//! Hot-reload coordinator for zero-downtime configuration updates
//!
//! This module orchestrates automatic reloading when .rune files change,
//! using the file watcher to detect changes and the RUNEEngine's atomic swap
//! capabilities to update rules and policies without downtime.

use crate::engine::RUNEEngine;
use crate::error::{RUNEError, Result};
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

    // ========== Comprehensive Tests ==========

    #[test]
    fn test_reload_config_default() {
        let config = ReloadConfig::default();
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert_eq!(config.max_retry_attempts, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
        assert!(config.auto_reload);
    }

    #[test]
    fn test_reload_config_custom() {
        let config = ReloadConfig {
            debounce_duration: Duration::from_secs(2),
            max_retry_attempts: 5,
            retry_delay: Duration::from_millis(500),
            auto_reload: false,
        };
        assert_eq!(config.debounce_duration, Duration::from_secs(2));
        assert_eq!(config.max_retry_attempts, 5);
        assert_eq!(config.retry_delay, Duration::from_millis(500));
        assert!(!config.auto_reload);
    }

    #[tokio::test]
    async fn test_coordinator_with_custom_config() {
        let engine = Arc::new(RUNEEngine::new());
        let config = ReloadConfig {
            debounce_duration: Duration::from_secs(1),
            max_retry_attempts: 10,
            retry_delay: Duration::from_millis(100),
            auto_reload: false,
        };
        let coordinator = ReloadCoordinator::with_config(engine, config.clone());
        assert!(coordinator.is_ok());

        let coord = coordinator.unwrap();
        assert_eq!(coord.config.debounce_duration, Duration::from_secs(1));
        assert_eq!(coord.config.max_retry_attempts, 10);
        assert!(!coord.config.auto_reload);
    }

    #[tokio::test]
    async fn test_watch_nonexistent_file() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Try to watch non-existent file
        let result = coordinator.watch_file("/nonexistent/path/file.rune");
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RUNEError::ConfigError(msg) if msg.contains("does not exist"))
        );
        assert_eq!(coordinator.watched_files().len(), 0);
    }

    #[tokio::test]
    async fn test_watch_multiple_files() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create multiple temp files
        let mut temp_file1 = NamedTempFile::new().unwrap();
        writeln!(temp_file1, "version = \"1.0\"").unwrap();

        let mut temp_file2 = NamedTempFile::new().unwrap();
        writeln!(temp_file2, "version = \"1.0\"").unwrap();

        // Watch both files
        assert!(coordinator.watch_file(temp_file1.path()).is_ok());
        assert!(coordinator.watch_file(temp_file2.path()).is_ok());

        assert_eq!(coordinator.watched_files().len(), 2);
    }

    #[tokio::test]
    async fn test_subscribe_to_events() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Subscribe to events
        let mut rx = coordinator.subscribe();

        // Send a test event through the channel
        if let Some(tx) = &coordinator.event_tx {
            let event = ReloadEvent {
                path: PathBuf::from("test.rune"),
                result: ReloadResult::Success,
                timestamp: std::time::Instant::now(),
            };
            tx.send(event.clone()).unwrap();

            // Receive the event
            let received = rx.try_recv();
            assert!(received.is_ok());
            let received_event = received.unwrap();
            assert_eq!(received_event.path, PathBuf::from("test.rune"));
            assert_eq!(received_event.result, ReloadResult::Success);
        }
    }

    #[test]
    fn test_reload_result_equality() {
        assert_eq!(ReloadResult::Success, ReloadResult::Success);
        assert_eq!(
            ReloadResult::Failed("error".to_string()),
            ReloadResult::Failed("error".to_string())
        );
        assert_eq!(
            ReloadResult::Skipped("reason".to_string()),
            ReloadResult::Skipped("reason".to_string())
        );

        assert_ne!(
            ReloadResult::Success,
            ReloadResult::Failed("error".to_string())
        );
        assert_ne!(
            ReloadResult::Failed("error1".to_string()),
            ReloadResult::Failed("error2".to_string())
        );
    }

    #[test]
    fn test_reload_event_debug() {
        let event = ReloadEvent {
            path: PathBuf::from("/test/file.rune"),
            result: ReloadResult::Success,
            timestamp: std::time::Instant::now(),
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ReloadEvent"));
        assert!(debug_str.contains("file.rune"));
        assert!(debug_str.contains("Success"));
    }

    #[tokio::test]
    async fn test_reload_with_datalog_rules() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with Datalog rules
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[rules]
user(alice).
admin(alice).
can_access(U) :- user(U), admin(U).
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_with_cedar_policies() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with Cedar policies
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_mixed_rules_and_policies() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with both rules and policies
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[rules]
user(alice).

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_missing_version() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file without version
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[rules]\nuser(alice).").unwrap();
        temp_file.flush().unwrap();

        // Reload should fail with parse error
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(msg) if msg.contains("Parse error")));
    }

    #[tokio::test]
    async fn test_reload_invalid_toml() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with invalid TOML
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[data]
invalid toml here
key = no quotes
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should fail
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(msg) if msg.contains("Parse error")));
    }

    #[tokio::test]
    async fn test_stop_watching() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create and watch a temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "version = \"1.0\"").unwrap();
        coordinator.watch_file(temp_file.path()).unwrap();

        assert_eq!(coordinator.watched_files().len(), 1);

        // Stop watching
        let result = coordinator.stop();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reload_file_not_found() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Try to reload non-existent file
        let result = coordinator
            .manual_reload(Path::new("/nonexistent/file.rune"))
            .await;
        assert!(matches!(result, ReloadResult::Failed(msg) if msg.contains("Failed to read file")));
    }

    #[tokio::test]
    async fn test_reload_empty_file() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create empty file
        let temp_file = NamedTempFile::new().unwrap();

        // Reload should fail (no version)
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_reload_only_version() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create file with only version
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"version = "rune/1.0""#).unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed (empty rules and policies are valid)
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_event_timestamp() {
        use std::time::Duration;

        let before = std::time::Instant::now();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let event = ReloadEvent {
            path: PathBuf::from("test.rune"),
            result: ReloadResult::Success,
            timestamp: std::time::Instant::now(),
        };

        assert!(event.timestamp > before);
    }

    #[test]
    fn test_reload_result_clone() {
        let result1 = ReloadResult::Success;
        let result2 = result1.clone();
        assert_eq!(result1, result2);

        let result3 = ReloadResult::Failed("error".to_string());
        let result4 = result3.clone();
        assert_eq!(result3, result4);

        let result5 = ReloadResult::Skipped("skipped".to_string());
        let result6 = result5.clone();
        assert_eq!(result5, result6);
    }

    #[test]
    fn test_reload_event_clone() {
        let event1 = ReloadEvent {
            path: PathBuf::from("test.rune"),
            result: ReloadResult::Success,
            timestamp: std::time::Instant::now(),
        };

        let event2 = event1.clone();
        assert_eq!(event1.path, event2.path);
        assert_eq!(event1.result, event2.result);
    }

    #[test]
    fn test_reload_config_clone() {
        let config1 = ReloadConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.debounce_duration, config2.debounce_duration);
        assert_eq!(config1.max_retry_attempts, config2.max_retry_attempts);
        assert_eq!(config1.retry_delay, config2.retry_delay);
        assert_eq!(config1.auto_reload, config2.auto_reload);
    }

    #[tokio::test]
    async fn test_reload_with_invalid_cedar_policy() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with invalid Cedar policy syntax
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[policies]
invalid policy syntax here
permit (
    no proper structure
)
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should fail
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(msg) if msg.contains("Parse error") || msg.contains("Policy")));
    }

    #[tokio::test]
    async fn test_reload_with_invalid_datalog_rules() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with invalid Datalog syntax
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[rules]
invalid rule :--.
malformed(X) :- no_body
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should fail
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_auto_reload_disabled() {
        let engine = Arc::new(RUNEEngine::new());
        let config = ReloadConfig {
            debounce_duration: Duration::from_millis(10),
            max_retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
            auto_reload: false, // Disabled
        };
        let mut coordinator = ReloadCoordinator::with_config(engine, config).unwrap();

        // Create temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"version = "rune/1.0""#).unwrap();
        temp_file.flush().unwrap();

        // Watch file
        coordinator.watch_file(temp_file.path()).unwrap();

        // Verify auto_reload is disabled
        assert!(!coordinator.config.auto_reload);
    }

    #[tokio::test]
    async fn test_event_tx_none_case() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // event_tx should be None initially (no subscribers)
        assert!(coordinator.event_tx.is_none());

        // Manual reload should work without event_tx
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"version = "rune/1.0""#).unwrap();
        temp_file.flush().unwrap();

        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_subscribe_multiple_times() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Subscribe once
        let _rx1 = coordinator.subscribe();
        assert!(coordinator.event_tx.is_some());

        // Subscribe again (replaces previous subscription)
        let _rx2 = coordinator.subscribe();
        assert!(coordinator.event_tx.is_some());
    }

    #[tokio::test]
    async fn test_event_send_with_dropped_receiver() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Subscribe and immediately drop receiver
        {
            let _rx = coordinator.subscribe();
        } // Receiver dropped here

        // Try to send event through the channel (should log warning but not fail)
        if let Some(tx) = &coordinator.event_tx {
            let event = ReloadEvent {
                path: PathBuf::from("test.rune"),
                result: ReloadResult::Success,
                timestamp: std::time::Instant::now(),
            };
            // This should return Err because receiver is dropped
            let send_result = tx.send(event);
            assert!(send_result.is_err());
        }
    }

    #[tokio::test]
    async fn test_reload_result_debug_formats() {
        let success = ReloadResult::Success;
        let failed = ReloadResult::Failed("test error".to_string());
        let skipped = ReloadResult::Skipped("test skip".to_string());

        assert!(format!("{:?}", success).contains("Success"));
        assert!(format!("{:?}", failed).contains("Failed"));
        assert!(format!("{:?}", failed).contains("test error"));
        assert!(format!("{:?}", skipped).contains("Skipped"));
        assert!(format!("{:?}", skipped).contains("test skip"));
    }

    #[test]
    fn test_reload_config_debug() {
        let config = ReloadConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ReloadConfig"));
        assert!(debug_str.contains("debounce_duration"));
        assert!(debug_str.contains("max_retry_attempts"));
    }

    #[tokio::test]
    async fn test_watched_files_empty_initially() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        assert_eq!(coordinator.watched_files().len(), 0);
        assert!(coordinator.watched_files().is_empty());
    }

    #[tokio::test]
    async fn test_reload_with_multiple_policies() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with multiple Cedar policies
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);

permit (
    principal == User::"bob",
    action == Action::"write",
    resource == Document::"doc1"
);
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_with_multiple_datalog_rules() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with multiple Datalog rules
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[rules]
user(alice).
user(bob).
admin(alice).
role(U, admin) :- admin(U).
can_access(U, R) :- user(U), role(U, admin).
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Reload should succeed
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_reload_event_path_preservation() {
        let test_path = PathBuf::from("/test/path/config.rune");
        let event = ReloadEvent {
            path: test_path.clone(),
            result: ReloadResult::Success,
            timestamp: std::time::Instant::now(),
        };

        assert_eq!(event.path, test_path);
        assert_eq!(event.path.to_str().unwrap(), "/test/path/config.rune");
    }

    #[tokio::test]
    async fn test_coordinator_stop_with_no_files_watched() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Stop without watching any files
        let result = coordinator.stop();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_same_file_twice() {
        let engine = Arc::new(RUNEEngine::new());
        let mut coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "version = \"1.0\"").unwrap();

        // Watch the same file twice
        assert!(coordinator.watch_file(temp_file.path()).is_ok());
        assert!(coordinator.watch_file(temp_file.path()).is_ok());

        // Both watches should be tracked
        assert_eq!(coordinator.watched_files().len(), 2);
    }

    #[tokio::test]
    async fn test_reload_with_whitespace_only() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create temp file with only whitespace
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "   \n\n   \t\t   \n").unwrap();
        temp_file.flush().unwrap();

        // Reload should fail (no version)
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert!(matches!(result, ReloadResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_reload_result_variants_inequality() {
        // Test different variants are not equal
        assert_ne!(ReloadResult::Success, ReloadResult::Failed("err".to_string()));
        assert_ne!(ReloadResult::Success, ReloadResult::Skipped("skip".to_string()));
        assert_ne!(
            ReloadResult::Failed("err1".to_string()),
            ReloadResult::Skipped("skip".to_string())
        );
    }

    #[tokio::test]
    async fn test_coordinator_fields_initialization() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Verify initial state
        assert!(coordinator.event_tx.is_none());
        assert_eq!(coordinator.watched_files.len(), 0);
        assert_eq!(coordinator.config.debounce_duration, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_manual_reload_success_path() {
        let engine = Arc::new(RUNEEngine::new());
        let coordinator = ReloadCoordinator::new(engine).unwrap();

        // Create valid config
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"version = "rune/1.0"

[rules]
fact(value).
"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Manual reload through public API
        let result = coordinator.manual_reload(temp_file.path()).await;
        assert_eq!(result, ReloadResult::Success);
    }

    #[tokio::test]
    async fn test_config_all_fields_accessible() {
        let config = ReloadConfig {
            debounce_duration: Duration::from_millis(123),
            max_retry_attempts: 7,
            retry_delay: Duration::from_millis(456),
            auto_reload: true,
        };

        // Verify all fields are accessible
        assert_eq!(config.debounce_duration, Duration::from_millis(123));
        assert_eq!(config.max_retry_attempts, 7);
        assert_eq!(config.retry_delay, Duration::from_millis(456));
        assert!(config.auto_reload);
    }
}
