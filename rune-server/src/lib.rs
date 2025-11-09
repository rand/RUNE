//! RUNE HTTP Server - RESTful API for remote authorization
//!
//! This crate provides an HTTP API for RUNE authorization engine,
//! enabling remote authorization queries with sub-10ms latency.

pub mod api;
pub mod error;
pub mod handlers;
pub mod metrics;
pub mod state;
pub mod tracing;

pub use api::{AuthorizeRequest, AuthorizeResponse, HealthResponse};
pub use error::{ApiError, ApiResult};
pub use state::AppState;
