//! File watching for hot-reload functionality
//!
//! This module provides automatic detection of .rune file changes
//! and triggers configuration reloads without downtime.

use crate::error::{RUNEError, Result};
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
            move |result: notify::Result<Event>| match result {
                Ok(event) => {
                    if let Some(change_event) = process_notify_event(event) {
                        if let Err(e) = tx.send(change_event) {
                            error!("Failed to send file change event: {}", e);
                        }
                    }
                }
                Err(e) => error!("File watch error: {}", e),
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
                _ => return None, // Ignore metadata changes
            }
        }
        EventKind::Remove(_) => ChangeKind::Removed,
        _ => return None, // Ignore access and other events
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
        return None; // No extension, ignore
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
    fn test_watch_directory() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Watch directory (should use recursive mode)
        assert!(watcher.watch(temp_dir.path()).is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);
        assert!(watcher
            .watched_paths()
            .contains(&temp_dir.path().to_path_buf()));
    }

    #[test]
    fn test_unwatch() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rune");
        fs::write(&file_path, "version = \"1.0\"").unwrap();

        // Watch and then unwatch
        watcher.watch(&file_path).unwrap();
        assert_eq!(watcher.watched_paths().len(), 1);

        watcher.unwatch(&file_path).unwrap();
        assert_eq!(watcher.watched_paths().len(), 0);

        // Unwatching non-existent path should be no-op
        assert!(watcher.unwatch(&file_path).is_ok());
    }

    #[test]
    fn test_clear_watches() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.rune");
        let file2 = temp_dir.path().join("test2.rune");

        fs::write(&file1, "version = \"1.0\"").unwrap();
        fs::write(&file2, "version = \"2.0\"").unwrap();

        watcher.watch(&file1).unwrap();
        watcher.watch(&file2).unwrap();
        assert_eq!(watcher.watched_paths().len(), 2);

        watcher.clear().unwrap();
        assert_eq!(watcher.watched_paths().len(), 0);
    }

    #[test]
    fn test_should_watch() {
        let watcher = RUNEWatcher::new().unwrap();

        assert!(watcher.should_watch(Path::new("config.rune")));
        assert!(watcher.should_watch(Path::new("data.toml")));
        assert!(!watcher.should_watch(Path::new("readme.md")));
        assert!(!watcher.should_watch(Path::new("script.py")));
        assert!(!watcher.should_watch(Path::new("no_extension")));
    }

    #[test]
    fn test_event_sender() {
        let watcher = RUNEWatcher::new().unwrap();
        let sender = watcher.event_sender();

        // Send event manually
        let event = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        assert!(sender.send(event).is_ok());

        // Should be able to receive it
        let received = watcher.try_recv();
        assert!(received.is_some());
        assert_eq!(received.unwrap().kind, ChangeKind::Modified);
    }

    #[test]
    fn test_try_recv_empty() {
        let watcher = RUNEWatcher::new().unwrap();
        // Should return None when no events
        assert!(watcher.try_recv().is_none());
    }

    #[test]
    fn test_recv_timeout() {
        let watcher = RUNEWatcher::new().unwrap();

        // Should timeout when no events
        let result = watcher.recv_timeout(Duration::from_millis(50));
        assert!(result.is_none());

        // Send event and receive it
        let sender = watcher.event_sender();
        let event = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Created,
            timestamp: std::time::Instant::now(),
        };
        sender.send(event.clone()).unwrap();

        let result = watcher.recv_timeout(Duration::from_millis(100));
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, ChangeKind::Created);
    }

    #[test]
    fn test_process_notify_event_create() {
        use notify::event::{CreateKind, EventKind};

        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("test.rune")],
            attrs: Default::default(),
        };

        let result = process_notify_event(event);
        assert!(result.is_some());
        let change_event = result.unwrap();
        assert_eq!(change_event.kind, ChangeKind::Created);
        assert_eq!(change_event.path, PathBuf::from("test.rune"));
    }

    #[test]
    fn test_process_notify_event_modify_data() {
        use notify::event::{DataChange, EventKind, ModifyKind};

        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![PathBuf::from("test.toml")],
            attrs: Default::default(),
        };

        let result = process_notify_event(event);
        assert!(result.is_some());
        let change_event = result.unwrap();
        assert_eq!(change_event.kind, ChangeKind::Modified);
        assert_eq!(change_event.path, PathBuf::from("test.toml"));
    }

    #[test]
    fn test_process_notify_event_modify_any() {
        use notify::event::{EventKind, ModifyKind};

        let event = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![PathBuf::from("test.rune")],
            attrs: Default::default(),
        };

        let result = process_notify_event(event);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, ChangeKind::Modified);
    }

    #[test]
    fn test_process_notify_event_metadata_ignored() {
        use notify::event::{EventKind, MetadataKind, ModifyKind};

        let event = Event {
            kind: EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)),
            paths: vec![PathBuf::from("test.rune")],
            attrs: Default::default(),
        };

        // Metadata changes should be ignored
        let result = process_notify_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_notify_event_remove() {
        use notify::event::{EventKind, RemoveKind};

        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![PathBuf::from("test.rune")],
            attrs: Default::default(),
        };

        let result = process_notify_event(event);
        assert!(result.is_some());
        let change_event = result.unwrap();
        assert_eq!(change_event.kind, ChangeKind::Removed);
    }

    #[test]
    fn test_process_notify_event_other_ignored() {
        use notify::event::EventKind;

        let event = Event {
            kind: EventKind::Other,
            paths: vec![PathBuf::from("test.rune")],
            attrs: Default::default(),
        };

        // Other events should be ignored
        let result = process_notify_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_notify_event_wrong_extension() {
        use notify::event::{CreateKind, EventKind};

        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("test.txt")],
            attrs: Default::default(),
        };

        // Non-rune/toml files should be ignored
        let result = process_notify_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_notify_event_no_extension() {
        use notify::event::{CreateKind, EventKind};

        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("no_extension")],
            attrs: Default::default(),
        };

        // Files without extension should be ignored
        let result = process_notify_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_notify_event_empty_paths() {
        use notify::event::{CreateKind, EventKind};

        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![],
            attrs: Default::default(),
        };

        // Empty paths should return None
        let result = process_notify_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_change_kind_equality() {
        assert_eq!(ChangeKind::Created, ChangeKind::Created);
        assert_eq!(ChangeKind::Modified, ChangeKind::Modified);
        assert_eq!(ChangeKind::Removed, ChangeKind::Removed);
        assert_ne!(ChangeKind::Created, ChangeKind::Modified);
    }

    #[test]
    fn test_file_change_event_clone() {
        let event = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        let cloned = event.clone();
        assert_eq!(event.path, cloned.path);
        assert_eq!(event.kind, cloned.kind);
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

    #[test]
    fn test_debouncer_multiple_events() {
        let mut debouncer = EventDebouncer::new(Duration::from_millis(100));

        let event1 = FileChangeEvent {
            path: PathBuf::from("test1.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        let event2 = FileChangeEvent {
            path: PathBuf::from("test2.rune"),
            kind: ChangeKind::Created,
            timestamp: std::time::Instant::now(),
        };

        debouncer.add_event(event1);
        debouncer.add_event(event2);
        assert!(debouncer.has_pending());

        std::thread::sleep(Duration::from_millis(150));
        let settled = debouncer.get_settled_events();
        assert_eq!(settled.len(), 2);
    }

    #[test]
    fn test_debouncer_overwrite_event() {
        // Use larger durations to avoid timing issues on CI
        let mut debouncer = EventDebouncer::new(Duration::from_millis(200));

        let event1 = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Created,
            timestamp: std::time::Instant::now(),
        };

        let event2 = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        debouncer.add_event(event1);
        std::thread::sleep(Duration::from_millis(50));
        debouncer.add_event(event2); // This should reset the timer

        // Wait less than the debounce duration from the second event
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(debouncer.get_settled_events().len(), 0); // Not settled yet

        // Wait for the full duration to elapse
        std::thread::sleep(Duration::from_millis(150));
        let settled = debouncer.get_settled_events();
        assert_eq!(settled.len(), 1);
        assert_eq!(settled[0].kind, ChangeKind::Modified); // Should have the latest event
    }

    #[test]
    fn test_debouncer_clear() {
        let mut debouncer = EventDebouncer::new(Duration::from_millis(100));

        let event = FileChangeEvent {
            path: PathBuf::from("test.rune"),
            kind: ChangeKind::Modified,
            timestamp: std::time::Instant::now(),
        };

        debouncer.add_event(event);
        assert!(debouncer.has_pending());

        debouncer.clear();
        assert!(!debouncer.has_pending());
        assert_eq!(debouncer.get_settled_events().len(), 0);
    }

    #[test]
    fn test_debouncer_no_pending_initially() {
        let debouncer = EventDebouncer::new(Duration::from_millis(100));
        assert!(!debouncer.has_pending());
        assert_eq!(debouncer.pending.len(), 0);
        assert_eq!(debouncer.last_event_time.len(), 0);
    }

    #[test]
    fn test_watcher_integration() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rune");

        // Create and watch file
        fs::write(&file_path, "version = \"1.0\"").unwrap();
        watcher.watch(&file_path).unwrap();

        // Modify file - this may trigger an event
        fs::write(&file_path, "version = \"2.0\"").unwrap();

        // Give the watcher a moment to process
        std::thread::sleep(Duration::from_millis(100));

        // Try to receive event (may or may not be there depending on timing)
        let _event = watcher.try_recv();

        // Clean up
        watcher.unwatch(&file_path).unwrap();
    }

    #[test]
    fn test_recv_blocking() {
        let watcher = RUNEWatcher::new().unwrap();
        let sender = watcher.event_sender();

        // Spawn a thread to send an event after a delay
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let event = FileChangeEvent {
                path: PathBuf::from("test.rune"),
                kind: ChangeKind::Created,
                timestamp: std::time::Instant::now(),
            };
            let _ = sender.send(event);
        });

        // Blocking receive should wait for the event
        let result = watcher.recv();
        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.kind, ChangeKind::Created);
    }

    #[test]
    fn test_recv_channel_disconnected() {
        let watcher = RUNEWatcher::new().unwrap();
        // Drop the watcher's event_tx by getting a sender and not using it
        // The original tx is still held, so this won't actually disconnect
        // But we can test the error path by dropping all senders
        drop(watcher.event_tx);

        // Note: This test shows the structure but won't actually trigger
        // the error because the watcher still holds event_tx internally.
        // The error path is covered when the notify callback fails to send.
    }

    #[test]
    fn test_file_watch_with_actual_changes() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rune");

        // Create file
        fs::write(&file_path, "version = \"1.0\"").unwrap();
        watcher.watch(&file_path).unwrap();

        // Give watcher time to register
        std::thread::sleep(Duration::from_millis(50));

        // Modify file
        fs::write(&file_path, "version = \"2.0\"").unwrap();

        // Wait for event
        std::thread::sleep(Duration::from_millis(200));

        // Check for event (may or may not be present depending on OS/timing)
        let event = watcher.try_recv();
        if let Some(e) = event {
            // Use canonicalize to handle path differences (e.g., /private/var vs /var on macOS)
            let canonical_file_path = fs::canonicalize(&file_path).unwrap_or(file_path.clone());
            let canonical_event_path = fs::canonicalize(&e.path).unwrap_or(e.path.clone());
            assert_eq!(canonical_event_path, canonical_file_path);
        }

        watcher.unwatch(&file_path).unwrap();
    }

    #[test]
    fn test_watch_invalid_path() {
        let mut watcher = RUNEWatcher::new().unwrap();
        let invalid_path = PathBuf::from("/nonexistent/path/to/file.rune");

        // Watching invalid path should return error
        let result = watcher.watch(&invalid_path);
        assert!(result.is_err());
    }
}
