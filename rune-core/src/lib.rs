//! RUNE Core - High-performance authorization and configuration engine
//!
//! This crate provides the core RUNE engine with sub-millisecond authorization
//! decisions and high-throughput policy evaluation.

#![warn(missing_docs)]
#![deny(unsafe_code)] // Most modules should not use unsafe code

pub mod datalog;
pub mod engine;
pub mod error;
pub mod facts;
pub mod parser;
pub mod policy;
pub mod request;
pub mod types;
pub mod watcher;

pub use engine::{AuthorizationResult, Decision, RUNEEngine};
pub use error::{RUNEError, Result};
pub use facts::{Fact, FactStore};
pub use parser::parse_rune_file;
pub use policy::PolicySet;
pub use request::{Request, RequestBuilder};
pub use types::{Action, Entity, Principal, Resource, Value};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // VERSION is a compile-time constant from CARGO_PKG_VERSION
        // Just verify it has semantic version format
        assert!(VERSION.contains('.'));
    }
}
