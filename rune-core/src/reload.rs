//! Hot-reload coordinator for zero-downtime configuration updates
//!
//! This module orchestrates automatic reloading when .rune files change,
//! using the file watcher to detect changes and the RUNEEngine's atomic swap
//! capabilities to update rules and policies without downtime.

use crate::engine::RUNEEngine;
use crate::error::{Result, RUNEError};
use crate::parser::parse_rune_file;
use crate::policy::PolicySet;
use crate::watcher::{EventDebouncer, RUNEWatcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Reload event sent when configuration is reloaded
#[derive(Debug, Clone)]
pub struct ReloadEvent {
    /// Path that triggered the reload
    pub path: PathBuf,
    /// Result of the reload
    pub result: ReloadResult,
    /// Timestamp of the reload
    pub timestamp: std::time::Instant,
}

/// Result of a reload attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadResult {
    /// Reload succeeded
    Success,
    /// Reload failed (old config retained)
    Failed(String),
    /// Reload skipped (e.g., file not relevant)
    Skipped(String),
}

/// Configuration for the reload coordinator
#[derive(Debug, Clone)]
pub struct ReloadConfig {
    /// Debounce duration (wait for file writes to settle)
    pub debounce_duration: Duration,
    /// Maximum reload attempts on failure
    pub max_retry_attempts: usize,
    /// Retry delay on failure
    pub retry_delay: Duration,
    /// Enable automatic reload on file changes
    pub auto_reload: bool,
}

impl Default for ReloadConfig {
    fn default() -> Self {
        ReloadConfig {
            debounce_duration: Duration::from_millis(500),
            max_retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
            auto_reload: true,
        }
    }
}

/// Hot-reload coordinator
///
/// Coordinates file watching, parsing, and atomic engine updates.
/// Runs as an async task that processes file change events.
pub struct ReloadCoordinator {
    /// The RUNE engine to reload
    engine: Arc<RUNEEngine>,
    /// File watcher
    watcher: RUNEWatcher,
    /// Event debouncer
    debouncer: EventDebouncer,
    /// Configuration
    config: ReloadConfig,
    /// Reload event channel sender
    event_tx: Option<mpsc::UnboundedSender<ReloadEvent>>,
    /// Watched files
    watched_files: Vec<PathBuf>,
}

impl ReloadCoordinator {
    /// Create a new reload coordinator
    pub fn new(engine: Arc<RUNEEngine>) -> Result<Self> {
        Self::with_config(engine, ReloadConfig::default())
    }

    /// Create a reload coordinator with custom configuration
    pub fn with_config(engine: Arc<RUNEEngine>, config: ReloadConfig) -> Result<Self> {
        let watcher = RUNEWatcher::new()?;
        let debouncer = EventDebouncer::new(config.debounce_duration);

        Ok(ReloadCoordinator {
            engine,
            watcher,
            debouncer,
            config,
            event_tx: None,
            watched_files: Vec::new(),
        })
    }

    /// Watch a configuration file for changes
    pub fn watch_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Verify file exists and is readable
        if !path.exists() {
            return Err(RUNEError::ConfigError(format!(
                "File does not exist: {:?}",
                path
            )));
        }

        // Start watching
        self.watcher.watch(path)?;
        self.watched_files.push(path.to_path_buf());

        info!("Watching configuration file: {:?}", path);
        Ok(())
    }

    /// Subscribe to reload events
    pub fn subscribe(&mut self) -> mpsc::UnboundedReceiver<ReloadEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Run the coordinator (async task)
    ///
    /// This method never returns under normal circumstances.
    /// It continuously watches for file changes and reloads configuration.
    pub async fn run(mut self) -> Result<()> {
        info!("Reload coordinator started");

        loop {
            // Check for file events (with timeout to check debouncer periodically)
            if let Some(event) = self.watcher.recv_timeout(Duration::from_millis(100)) {
                debug!("File change event: {:?}", event);
                self.debouncer.add_event(event);
            }

            // Check for settled events (debounced)
            let settled_events = self.debouncer.get_settled_events();

            for event in settled_events {
                if !self.config.auto_reload {
                    debug!("Auto-reload disabled, skipping: {:?}", event.path);
                    continue;
                }

                // Attempt reload
                let reload_result = self.reload_file(&event.path).await;

                // Send reload event
                if let Some(tx) = &self.event_tx {
                    let reload_event = ReloadEvent {
                        path: event.path.clone(),
                        result: reload_result,
                        timestamp: std::time::Instant::now(),
                    };

                    if tx.send(reload_event).is_err() {
                        warn!("Failed to send reload event (no subscribers)");
                    }
                }
            }

            // Small yield to prevent busy loop
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Reload configuration from a file
    async fn reload_file(&self, path: &Path) -> ReloadResult {
        // Read file
        let content = match tokio::fs::read_to_string(path).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read {:?}: {}", path, e);
                return ReloadResult::Failed(format!("Failed to read file: {}", e));
            }
        };

        // Parse configuration
        let config = match parse_rune_file(&content) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse {:?}: {}", path, e);
                return ReloadResult::Failed(format!("Parse error: {}", e));
            }
        };

        // Reload Datalog rules
        if !config.rules.is_empty() {
            if let Err(e) = self.engine.reload_datalog_rules(config.rules) {
                error!("Failed to reload Datalog rules: {}", e);
                return ReloadResult::Failed(format!("Datalog reload error: {}", e));
            }
            info!("Reloaded Datalog rules from {:?}", path);
        }

        // Reload Cedar policies
        if !config.policies.is_empty() {
            // Create new policy set
            let mut policy_set = PolicySet::new();

            // Add each policy
            for policy in config.policies {
                if let Err(e) = policy_set.add_policy(&policy.id, &policy.content) {
                    error!("Failed to add policy {}: {}", policy.id, e);
                    return ReloadResult::Failed(format!("Policy add error: {}", e));
                }
            }

            // Reload the policy set
            if let Err(e) = self.engine.reload_policies(policy_set) {
                error!("Failed to reload policies: {}", e);
                return ReloadResult::Failed(format!("Policy reload error: {}", e));
            }
            info!("Reloaded Cedar policies from {:?}", path);
        }

        info!("Successfully reloaded configuration from {:?}", path);
        ReloadResult::Success
    }

    /// Manually trigger a reload (for testing or explicit user request)
    pub async fn manual_reload(&self, path: &Path) -> ReloadResult {
        self.reload_file(path).await
    }

    /// Stop watching all files
    pub fn stop(&mut self) -> Result<()> {
        self.watcher.clear()
    }

    /// Get list of watched files
    pub fn watched_files(&self) -> &[PathBuf] {
        &self.watched_files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine);
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_watch_file() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "version = \"1.0\"").unwrap();

        // Watch the file
        let result = coordinator.watch_file(temp_file.path());
        assert!(result.is_ok());

        assert_eq!(coordinator.watched_files().len(), 1);
    }

    #[tokio::test]
    async fn test_manual_reload() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "version = \"rune/1.0\"\n\n[rules]\n").unwrap();
        temp_file.flush().unwrap();

        // Manual reload
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_invalid_config() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with invalid content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid syntax [[[").unwrap();
        temp_file.flush().unwrap();

        // Manual reload should fail
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(_)));
    }
}
