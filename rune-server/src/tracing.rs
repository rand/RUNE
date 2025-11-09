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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[test]
    fn test_get_sampler_always_on() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "1.0");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOn));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_always_off() {
        // Clear any existing value first
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.0");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOff));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_ratio_based() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.5");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::TraceIdRatioBased(_)));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_invalid_value() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "invalid");
        let sampler = get_sampler();
        // Should default to AlwaysOn when parse fails
        assert!(matches!(sampler, Sampler::AlwaysOn));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_greater_than_one() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "2.0");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOn));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_negative() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "-0.5");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOff));
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }

    #[test]
    fn test_get_sampler_no_env() {
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        let sampler = get_sampler();
        // Should default to AlwaysOn (1.0)
        assert!(matches!(sampler, Sampler::AlwaysOn));
    }

    #[test]
    fn test_shutdown_telemetry() {
        // This should not panic
        shutdown_telemetry();
    }

    #[test]
    fn test_create_authorization_span() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let span = create_authorization_span("user123", "read", "/data/file.txt");

            // Verify span is created with correct name
            assert_eq!(span.metadata().unwrap().name(), "authorize_request");

            // Enter the span to test it
            let _guard = span.enter();
        });
    }

    #[test]
    fn test_record_decision() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let span = tracing::info_span!("test_span");
            let _guard = span.enter();

            // Should not panic
            record_decision("PERMIT", 12.5);
        });
    }

    #[test]
    fn test_record_error() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let span = tracing::info_span!("test_span");
            let _guard = span.enter();

            // Should not panic
            record_error("Authorization failed");
        });
    }

    #[test]
    fn test_trace_datalog_evaluation() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let result = trace_datalog_evaluation(10, || {
                // Simulate some evaluation
                42
            });
            assert_eq!(result, 42);
        });
    }

    #[test]
    fn test_trace_cedar_evaluation() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let result = trace_cedar_evaluation(5, || {
                // Simulate some evaluation
                "allowed"
            });
            assert_eq!(result, "allowed");
        });
    }

    #[test]
    fn test_trace_parse_request() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let result = trace_parse_request(|| {
                // Simulate parsing
                Ok::<_, &str>("parsed")
            });
            assert_eq!(result, Ok("parsed"));
        });
    }

    #[test]
    fn test_trace_format_response() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let result = trace_format_response(|| {
                // Simulate formatting
                "{\"status\": \"ok\"}"
            });
            assert_eq!(result, "{\"status\": \"ok\"}");
        });
    }

    #[tokio::test]
    async fn test_trace_cache_lookup() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            async {
                let result = trace_cache_lookup(async {
                    // Simulate cache lookup
                    Some("cached_value")
                })
                .await;
                assert_eq!(result, Some("cached_value"));
            }
        })
        .await;
    }

    #[test]
    fn test_trace_datalog_evaluation_with_error() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let result = trace_datalog_evaluation(0, || {
                // Simulate evaluation that returns error
                Err::<(), _>("no rules")
            });
            assert_eq!(result, Err("no rules"));
        });
    }

    #[test]
    fn test_trace_cedar_evaluation_with_policies() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let policies_evaluated = Arc::new(Mutex::new(false));
            let policies_evaluated_clone = policies_evaluated.clone();

            let result = trace_cedar_evaluation(3, || {
                *policies_evaluated_clone.lock().unwrap() = true;
                "decision"
            });

            assert_eq!(result, "decision");
            assert!(*policies_evaluated.lock().unwrap());
        });
    }

    #[test]
    fn test_multiple_span_operations() {
        let subscriber = Registry::default();
        with_default(subscriber, || {
            let span = create_authorization_span("admin", "delete", "/users/123");
            let _guard = span.enter();

            // Record multiple operations
            record_decision("DENY", 5.2);
            record_error("Insufficient permissions");

            // Nested trace operations
            let parse_result = trace_parse_request(|| "parsed");
            let format_result = trace_format_response(|| "formatted");

            assert_eq!(parse_result, "parsed");
            assert_eq!(format_result, "formatted");
        });
    }

    #[test]
    fn test_get_sampler_boundary_values() {
        // Test exact boundary of 0.0
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.0");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOff));

        // Test exact boundary of 1.0
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "1.0");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::AlwaysOn));

        // Test just below 1.0
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.999");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::TraceIdRatioBased(_)));

        // Test just above 0.0
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.001");
        let sampler = get_sampler();
        assert!(matches!(sampler, Sampler::TraceIdRatioBased(_)));

        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
    }
}
