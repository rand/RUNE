//! Tracing and logging setup for RUNE

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Initialize tracing with default configuration
pub fn init_tracing() {
    init_tracing_with_config(TracingConfig::default());
}

/// Initialize tracing with custom configuration
pub fn init_tracing_with_config(config: TracingConfig) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(config.default_level));

    let fmt_layer = fmt::layer()
        .with_span_events(config.span_events.clone())
        .with_target(config.show_target)
        .with_thread_ids(config.show_thread_ids)
        .with_thread_names(config.show_thread_names)
        .with_file(config.show_file)
        .with_line_number(config.show_line_number);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if config.json_output {
        let json_layer = fmt::layer()
            .json()
            .with_span_events(config.span_events)
            .with_target(true)
            .with_thread_ids(true);

        subscriber.with(json_layer).init();
    } else {
        subscriber.init();
    }
}

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Default log level if RUST_LOG is not set
    pub default_level: String,
    /// Show span events (enter, exit, close)
    pub span_events: FmtSpan,
    /// Show target module in logs
    pub show_target: bool,
    /// Show thread IDs
    pub show_thread_ids: bool,
    /// Show thread names
    pub show_thread_names: bool,
    /// Show source file
    pub show_file: bool,
    /// Show line numbers
    pub show_line_number: bool,
    /// Output logs as JSON
    pub json_output: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            default_level: "rune=debug,info".to_string(),
            span_events: FmtSpan::CLOSE,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file: false,
            show_line_number: false,
            json_output: false,
        }
    }
}

impl TracingConfig {
    /// Create a production configuration
    pub fn production() -> Self {
        Self {
            default_level: "rune=info,warn".to_string(),
            span_events: FmtSpan::NONE,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file: false,
            show_line_number: false,
            json_output: true, // JSON for structured logging
        }
    }

    /// Create a development configuration
    pub fn development() -> Self {
        Self {
            default_level: "rune=debug,info".to_string(),
            span_events: FmtSpan::CLOSE,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file: true,
            show_line_number: true,
            json_output: false,
        }
    }

    /// Create a verbose configuration for debugging
    pub fn verbose() -> Self {
        Self {
            default_level: "rune=trace,debug".to_string(),
            span_events: FmtSpan::ENTER | FmtSpan::EXIT | FmtSpan::CLOSE,
            show_target: true,
            show_thread_ids: true,
            show_thread_names: true,
            show_file: true,
            show_line_number: true,
            json_output: false,
        }
    }
}

/// Tracing macros for RUNE-specific events
#[macro_export]
macro_rules! trace_authorization {
    ($result:expr, $duration:expr, $request:expr) => {
        tracing::info!(
            target: "rune::authorization",
            result = ?$result,
            duration_ms = $duration.as_millis() as u64,
            principal = ?$request.principal,
            action = ?$request.action,
            resource = ?$request.resource,
            "Authorization decision"
        );
    };
}

#[macro_export]
macro_rules! trace_datalog_evaluation {
    ($rules:expr, $facts_derived:expr, $duration:expr) => {
        tracing::debug!(
            target: "rune::datalog",
            rules = $rules,
            facts_derived = $facts_derived,
            duration_ms = $duration.as_millis() as u64,
            "Datalog evaluation completed"
        );
    };
}

#[macro_export]
macro_rules! trace_policy_evaluation {
    ($policy_id:expr, $result:expr, $duration:expr) => {
        tracing::debug!(
            target: "rune::cedar",
            policy_id = $policy_id,
            result = ?$result,
            duration_ms = $duration.as_millis() as u64,
            "Policy evaluation completed"
        );
    };
}

#[macro_export]
macro_rules! trace_cache_access {
    ($cache_name:expr, $key:expr, $hit:expr) => {
        tracing::trace!(
            target: "rune::cache",
            cache = $cache_name,
            key = $key,
            hit = $hit,
            "Cache access"
        );
    };
}

#[macro_export]
macro_rules! trace_hot_reload {
    ($config_type:expr, $duration:expr, $success:expr) => {
        tracing::info!(
            target: "rune::hot_reload",
            config_type = $config_type,
            duration_ms = $duration.as_millis() as u64,
            success = $success,
            "Hot-reload event"
        );
    };
}

#[macro_export]
macro_rules! trace_error {
    ($error:expr, $context:expr) => {
        tracing::error!(
            target: "rune::error",
            error = %$error,
            context = $context,
            "Error occurred"
        );
    };
}

/// Performance span for instrumenting functions
#[macro_export]
macro_rules! perf_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::DEBUG, $name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::span!(tracing::Level::DEBUG, $name, $($field)*)
    };
}

/// OpenTelemetry integration (optional)
#[cfg(feature = "opentelemetry")]
pub mod otel {
    use opentelemetry::{global, sdk::Resource, KeyValue};
    use opentelemetry_otlp::WithExportConfig;
    use tracing_subscriber::layer::SubscriberExt;

    /// Initialize OpenTelemetry tracing
    pub fn init_opentelemetry(
        endpoint: &str,
        service_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(
                opentelemetry::sdk::trace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])),
            )
            .install_batch(opentelemetry::runtime::Tokio)?;

        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(telemetry)
            .with(tracing_subscriber::fmt::layer())
            .init();

        Ok(())
    }

    /// Shutdown OpenTelemetry
    pub fn shutdown_opentelemetry() {
        global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.default_level, "rune=debug,info");
        assert!(!config.json_output);
        assert!(config.show_target);
    }

    #[test]
    fn test_tracing_config_production() {
        let config = TracingConfig::production();
        assert_eq!(config.default_level, "rune=info,warn");
        assert!(config.json_output);
        assert!(!config.show_file);
    }

    #[test]
    fn test_tracing_config_development() {
        let config = TracingConfig::development();
        assert!(config.show_file);
        assert!(config.show_line_number);
        assert!(!config.json_output);
    }

    #[test]
    fn test_tracing_config_verbose() {
        let config = TracingConfig::verbose();
        assert_eq!(config.default_level, "rune=trace,debug");
        assert!(config.show_thread_ids);
        assert!(config.show_thread_names);
        assert!(config.show_file);
        assert!(config.show_line_number);
    }
}
