//! File watching for hot-reload functionality
//!
//! This module provides automatic detection of .rune file changes
//! and triggers configuration reloads without downtime.

use crate::error::{Result, RUNEError};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tracing::{debug, error, info, trace};

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path that changed
    pub path: PathBuf,
    /// Type of change
    pub kind: ChangeKind,
    /// Timestamp of the change
    pub timestamp: std::time::Instant,
}

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was removed
    Removed,
}

/// File watcher for .rune configuration files
pub struct RUNEWatcher {
    /// The underlying notify watcher
    watcher: RecommendedWatcher,
    /// Channel receiver for events
    event_rx: Receiver<FileChangeEvent>,
    /// Channel sender (kept for cloning)
    event_tx: Sender<FileChangeEvent>,
    /// Paths being watched
    watched_paths: HashSet<PathBuf>,
    /// File extensions to watch
    extensions: Vec<String>,
}

impl RUNEWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();
        let tx_clone = tx.clone();

        // Create notify watcher with custom event handler
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                match result {
                    Ok(event) => {
                        if let Some(change_event) = process_notify_event(event) {
                            if let Err(e) = tx.send(change_event) {
                                error!("Failed to send file change event: {}", e);
                            }
                        }
                    }
                    Err(e) => error!("File watch error: {}", e),
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(1))
                .with_compare_contents(false),
        )
        .map_err(|e| RUNEError::ConfigError(format!("Failed to create watcher: {}", e)))?;

        Ok(RUNEWatcher {
            watcher,
            event_rx: rx,
            event_tx: tx_clone,
            watched_paths: HashSet::new(),
            extensions: vec!["rune".to_string(), "toml".to_string()],
        })
    }

    /// Watch a file or directory
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Check if already watching
        if self.watched_paths.contains(path) {
            debug!("Already watching path: {:?}", path);
            return Ok(());
        }

        // Determine recursive mode based on path type
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        // Start watching
        self.watcher
            .watch(path, mode)
            .map_err(|e| RUNEError::ConfigError(format!("Failed to watch {:?}: {}", path, e)))?;

        self.watched_paths.insert(path.to_path_buf());
        info!("Now watching: {:?} (mode: {:?})", path, mode);

        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if !self.watched_paths.contains(path) {
            return Ok(());
        }

        self.watcher
            .unwatch(path)
            .map_err(|e| RUNEError::ConfigError(format!("Failed to unwatch {:?}: {}", path, e)))?;

        self.watched_paths.remove(path);
        info!("Stopped watching: {:?}", path);

        Ok(())
    }

    /// Try to receive a file change event (non-blocking)
    pub fn try_recv(&self) -> Option<FileChangeEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Receive a file change event (blocking)
    pub fn recv(&self) -> Result<FileChangeEvent> {
        self.event_rx
            .recv()
            .map_err(|e| RUNEError::ConfigError(format!("Failed to receive event: {}", e)))
    }

    /// Receive with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Option<FileChangeEvent> {
        self.event_rx.recv_timeout(timeout).ok()
    }

    /// Get a clone of the event sender (for multi-threaded use)
    pub fn event_sender(&self) -> Sender<FileChangeEvent> {
        self.event_tx.clone()
    }

    /// Check if a file should be watched based on extension
    pub fn should_watch(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self.extensions.contains(&ext_str.to_string());
            }
        }
        false
    }

    /// Get watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.iter().cloned().collect()
    }

    /// Clear all watches
    pub fn clear(&mut self) -> Result<()> {
        let paths: Vec<PathBuf> = self.watched_paths.iter().cloned().collect();
        for path in paths {
            self.unwatch(&path)?;
        }
        Ok(())
    }
}

/// Process notify event into our event type
fn process_notify_event(event: Event) -> Option<FileChangeEvent> {
    // Filter for relevant event kinds
    let kind = match event.kind {
        EventKind::Create(_) => ChangeKind::Created,
        EventKind::Modify(modify_kind) => {
            use notify::event::ModifyKind;
            match modify_kind {
                ModifyKind::Data(_) | ModifyKind::Any => ChangeKind::Modified,
                _ => return None,  // Ignore metadata changes
            }
        }
        EventKind::Remove(_) => ChangeKind::Removed,
        _ => return None,  // Ignore access and other events
    };

    // Get the first path (usually there's only one)
    let path = event.paths.into_iter().next()?;

    // Filter for .rune and .toml files
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_str()?;
        if ext_str != "rune" && ext_str != "toml" {
            trace!("Ignoring non-rune file: {:?}", path);
            return None;
        }
    } else {
        return None;  // No extension, ignore
    }

    Some(FileChangeEvent {
        path,
        kind,
        timestamp: std::time::Instant::now(),
    })
}

/// Debouncer for file change events
///
/// Files may be written in multiple chunks, causing multiple events.
/// This debouncer waits for events to settle before triggering reload.
pub struct EventDebouncer {
    /// Debounce duration
    duration: Duration,
    /// Pending events
    pending: HashMap<PathBuf, FileChangeEvent>,
    /// Last event time for each path
    last_event_time: HashMap<PathBuf, std::time::Instant>,
}

use std::collections::HashMap;

impl EventDebouncer {
    /// Create a new debouncer with specified duration
    pub fn new(duration: Duration) -> Self {
        EventDebouncer {
            duration,
            pending: HashMap::new(),
            last_event_time: HashMap::new(),
        }
    }

    /// Add an event to the debouncer
    pub fn add_event(&mut self, event: FileChangeEvent) {
        let now = std::time::Instant::now();
        self.pending.insert(event.path.clone(), event.clone());
        self.last_event_time.insert(event.path, now);
    }

    /// Get events that have settled (no new events for duration)
    pub fn get_settled_events(&mut self) -> Vec<FileChangeEvent> {
        let now = std::time::Instant::now();
        let mut settled = Vec::new();

        // Find events that have settled
        let settled_paths: Vec<PathBuf> = self
            .last_event_time
            .iter()
            .filter_map(|(path, time)| {
                if now.duration_since(*time) >= self.duration {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        // Extract settled events
        for path in settled_paths {
            if let Some(event) = self.pending.remove(&path) {
                self.last_event_time.remove(&path);
                settled.push(event);
            }
        }

        settled
    }

    /// Check if any events are pending
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all pending events
    pub fn clear(&mut self) {
        self.pending.clear();
        self.last_event_time.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_creation() {
        let watcher = RUNEWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_file() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rune");

        // Create file
        fs::write(&file_path, "version = \"1.0\"").unwrap();

        // Start watching
        assert!(watcher.watch(&file_path).is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);

        // Try watching again (should be no-op)
        assert!(watcher.watch(&file_path).is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);
    }

    #[test]
    fn test_should_watch() {
        let watcher = RUNEWatcher::new().unwrap();

        assert!(watcher.should_watch(Path::new("config.rune")));
        assert!(watcher.should_watch(Path::new("data.toml")));
        assert!(!watcher.should_watch(Path::new("readme.md")));
        assert!(!watcher.should_watch(Path::new("script.py")));
    }

    #[test]
    fn test_debouncer() {
        let mut debouncer = EventDebouncer::new(Duration::from_millis(100));

        let event = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        debouncer.add_event(event.clone());
        assert!(debouncer.has_pending());

        // Immediately checking should return nothing
        assert_eq!(debouncer.get_settled_events().len(), 0);

        // After waiting, should get the event
        std::thread::sleep(Duration::from_millis(150));
        let settled = debouncer.get_settled_events();
        assert_eq!(settled.len(), 1);
        assert_eq!(settled[0].path, event.path);

        // Should be empty now
        assert!(!debouncer.has_pending());
    }
}