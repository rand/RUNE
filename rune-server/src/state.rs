//! Application state

use rune_core::RUNEEngine;
use std::sync::Arc;
use std::time::Instant;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// The RUNE authorization engine
    pub engine: Arc<RUNEEngine>,

    /// Server start time
    pub start_time: Instant,

    /// Debug mode flag
    pub debug: bool,
}

impl AppState {
    /// Create new application state
    pub fn new(engine: Arc<RUNEEngine>) -> Self {
        Self {
            engine,
            start_time: Instant::now(),
            debug: false,
        }
    }

    /// Create application state with debug mode
    pub fn with_debug(engine: Arc<RUNEEngine>, debug: bool) -> Self {
        Self {
            engine,
            start_time: Instant::now(),
            debug,
        }
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
