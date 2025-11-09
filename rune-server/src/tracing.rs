//! OpenTelemetry tracing integration for RUNE server

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Initialize OpenTelemetry with OTLP exporter
pub fn init_telemetry(service_name: &str) -> anyhow::Result<opentelemetry_sdk::trace::Tracer> {
    // Get OTLP endpoint from environment or use default
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    // Configure resource attributes
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // Configure OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(3));

    // Build the trace pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(get_sampler())
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)?;

    Ok(tracer)
}

/// Get sampler configuration from environment
fn get_sampler() -> Sampler {
    let sample_rate = std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0); // Default to 100% sampling

    if sample_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if sample_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(sample_rate)
    }
}

/// Initialize the complete tracing stack (console + OpenTelemetry)
pub fn init_tracing_stack(service_name: &str) -> anyhow::Result<()> {
    // Initialize OpenTelemetry
    let tracer = init_telemetry(service_name)?;

    // Create OpenTelemetry layer
    let otel_layer = OpenTelemetryLayer::new(tracer);

    // Create console layer for local logging
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true);

    // Create env filter
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,rune=debug"));

    // Combine all layers
    Registry::default()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    Ok(())
}

/// Shutdown OpenTelemetry provider
pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// Create authorization span with context
#[tracing::instrument(
    name = "authorize_request",
    skip_all,
    fields(
        principal = %principal,
        action = %action,
        resource = %resource,
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
    )
)]
pub fn create_authorization_span(principal: &str, action: &str, resource: &str) -> tracing::Span {
    tracing::info_span!(
        "authorize_request",
        principal = %principal,
        action = %action,
        resource = %resource,
        otel.kind = "server",
    )
}

/// Record decision in current span
pub fn record_decision(decision: &str, latency_ms: f64) {
    tracing::Span::current().record("decision", decision);
    tracing::Span::current().record("latency_ms", latency_ms);
    tracing::Span::current().record("otel.status_code", "OK");
}

/// Record error in current span
pub fn record_error(error: &str) {
    tracing::Span::current().record("otel.status_code", "ERROR");
    tracing::Span::current().record("error", error);
}

/// Create a child span for cache lookup
#[tracing::instrument(name = "cache_lookup", skip_all)]
pub async fn trace_cache_lookup<F, R>(f: F) -> R
where
    F: std::future::Future<Output = R>,
{
    f.await
}

/// Create a child span for Datalog evaluation
#[tracing::instrument(
    name = "datalog_evaluation",
    skip_all,
    fields(rules_count = tracing::field::Empty)
)]
pub fn trace_datalog_evaluation<F, R>(rules_count: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    tracing::Span::current().record("rules_count", rules_count);
    f()
}

/// Create a child span for Cedar evaluation
#[tracing::instrument(
    name = "cedar_evaluation",
    skip_all,
    fields(policies_count = tracing::field::Empty)
)]
pub fn trace_cedar_evaluation<F, R>(policies_count: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    tracing::Span::current().record("policies_count", policies_count);
    f()
}

/// Create a child span for request parsing
#[tracing::instrument(name = "parse_request", skip_all)]
pub fn trace_parse_request<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}

/// Create a child span for response formatting
#[tracing::instrument(name = "format_response", skip_all)]
pub fn trace_format_response<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}
