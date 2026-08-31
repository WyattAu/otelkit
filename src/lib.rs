#![forbid(unsafe_code)]

mod config;
pub mod error;

pub use config::{LogFormat, TelemetryConfig};
pub use error::TelemetryError;

/// RAII guard that flushes and shuts down telemetry on drop.
pub struct TelemetryGuard {
    #[cfg(feature = "otlp")]
    tracer_provider: Option<opentelemetry_sdk::trace::TracerProvider>,
    #[cfg(feature = "sentry")]
    sentry_guard: Option<sentry::ClientInitGuard>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        #[cfg(feature = "sentry")]
        {
            self.sentry_guard.take();
        }
        #[cfg(feature = "otlp")]
        {
            if let Some(provider) = self.tracer_provider.take() {
                if let Err(e) = provider.shutdown() {
                    eprintln!("otelkit: tracer provider shutdown error: {e}");
                }
            }
        }
    }
}

/// Initialize tracing and telemetry from the given configuration.
///
/// Returns a [`TelemetryGuard`] that must be held for the lifetime of the
/// program to ensure resources are flushed on shutdown.
pub fn init(config: TelemetryConfig) -> Result<TelemetryGuard, TelemetryError> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = tracing_subscriber::EnvFilter::try_new(&config.log_level)
        .map_err(|e| TelemetryError::InvalidConfig(e.to_string()))?;

    let mut guard = TelemetryGuard {
        #[cfg(feature = "otlp")]
        tracer_provider: None,
        #[cfg(feature = "sentry")]
        sentry_guard: None,
    };

    let registry = tracing_subscriber::registry().with(filter);

    match config.log_format {
        LogFormat::Text => {
            registry
                .with(tracing_subscriber::fmt::layer().with_target(true))
                .init();
        }
        LogFormat::Json => {
            registry
                .with(tracing_subscriber::fmt::layer().json().with_target(true))
                .init();
        }
    }

    #[cfg(feature = "otlp")]
    {
        let otlp_endpoint = config
            .otlp_endpoint
            .as_deref()
            .unwrap_or("http://localhost:4317");

        let resource = opentelemetry_sdk::Resource::builder()
            .with_attributes([opentelemetry::KeyValue::new(
                "service.name",
                config.service_name.clone(),
            )])
            .build();

        let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_resource(resource)
            .build();

        let tracer = tracer_provider.tracer(config.service_name.clone());

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::try_new(&config.log_level)
                .map_err(|e| TelemetryError::InvalidConfig(e.to_string()))?)
            .with(otel_layer)
            .init();

        guard.tracer_provider = Some(tracer_provider);
    }

    #[cfg(feature = "sentry")]
    {
        let sentry_dsn = config
            .sentry_dsn
            .as_deref()
            .ok_or_else(|| TelemetryError::InvalidConfig("sentry feature enabled but no DSN provided".into()))?;

        guard.sentry_guard = Some(sentry::init((
            sentry_dsn,
            sentry::ClientOptions {
                traces_sample_rate: config.sample_rate,
                release: Some(config.service_version.clone().into()),
                ..Default::default()
            },
        )));
    }

    Ok(guard)
}

/// Build a [`TelemetryConfig`] from environment variables.
///
/// Recognized variables:
/// - `OTEL_SERVICE_NAME` — service name (default: `"unknown"`)
/// - `OTEL_SERVICE_VERSION` — service version (default: `"0.0.0"`)
/// - `OTEL_LOG_LEVEL` — log level filter (default: `"info"`)
/// - `OTEL_LOG_FORMAT` — `"text"` or `"json"` (default: `"json"`)
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP endpoint
/// - `SENTRY_DSN` — Sentry DSN
/// - `OTEL_SAMPLE_RATE` — trace sample rate 0.0–1.0 (default: `1.0`)
pub fn from_env() -> Result<TelemetryConfig, TelemetryError> {
    TelemetryConfig::from_env()
}

#[cfg(test)]
mod tests {
    use crate::config::{LogFormat, TelemetryConfig};
    use crate::error::TelemetryError;

    // ---- LogFormat tests ----

    #[test]
    fn log_format_variants() {
        let text = LogFormat::Text;
        let json = LogFormat::Json;
        assert_ne!(text, json);
    }

    #[test]
    fn log_format_equality() {
        assert_eq!(LogFormat::Text, LogFormat::Text);
        assert_eq!(LogFormat::Json, LogFormat::Json);
    }

    #[test]
    fn log_format_from_str_text() {
        assert_eq!(LogFormat::from_str_opt("text"), LogFormat::Text);
        assert_eq!(LogFormat::from_str_opt("TEXT"), LogFormat::Text);
        assert_eq!(LogFormat::from_str_opt("plain"), LogFormat::Text);
    }

    #[test]
    fn log_format_from_str_json_default() {
        assert_eq!(LogFormat::from_str_opt("json"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_opt("anything"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_opt(""), LogFormat::Json);
    }

    // ---- TelemetryConfig defaults tests ----

    #[test]
    fn telemetry_config_default() {
        let cfg = TelemetryConfig::default();
        assert_eq!(cfg.service_name, "unknown");
        assert_eq!(cfg.service_version, "0.0.0");
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.log_format, LogFormat::Json);
        assert!(cfg.otlp_endpoint.is_none());
        assert!(cfg.sentry_dsn.is_none());
        assert_eq!(cfg.sample_rate, 1.0);
    }

    #[test]
    fn telemetry_config_builder() {
        let cfg = TelemetryConfig::default()
            .service_name("my-svc")
            .service_version("1.2.3")
            .log_level("debug")
            .log_format(LogFormat::Text)
            .otlp_endpoint("http://localhost:4317")
            .sentry_dsn("https://key@sentry.io/1")
            .sample_rate(0.5);

        assert_eq!(cfg.service_name, "my-svc");
        assert_eq!(cfg.service_version, "1.2.3");
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.log_format, LogFormat::Text);
        assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://localhost:4317"));
        assert_eq!(cfg.sentry_dsn.as_deref(), Some("https://key@sentry.io/1"));
        assert_eq!(cfg.sample_rate, 0.5);
    }

    #[test]
    fn sample_rate_clamped() {
        let cfg = TelemetryConfig::default().sample_rate(2.0);
        assert_eq!(cfg.sample_rate, 1.0);

        let cfg = TelemetryConfig::default().sample_rate(-0.5);
        assert_eq!(cfg.sample_rate, 0.0);
    }

    // ---- TelemetryError display tests ----

    #[test]
    fn error_otlp_connection_display() {
        let err = TelemetryError::OtlpConnection("refused".into());
        assert_eq!(err.to_string(), "OTLP connection error: refused");
    }

    #[test]
    fn error_invalid_config_display() {
        let err = TelemetryError::InvalidConfig("bad value".into());
        assert_eq!(err.to_string(), "invalid config: bad value");
    }

    // ---- from_env defaults (no env vars set) ----

    #[test]
    fn from_env_defaults() {
        let cfg = TelemetryConfig::from_env().unwrap();
        assert_eq!(cfg.service_name, "unknown");
        assert_eq!(cfg.service_version, "0.0.0");
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.log_format, LogFormat::Json);
        assert!(cfg.otlp_endpoint.is_none());
        assert!(cfg.sentry_dsn.is_none());
        assert_eq!(cfg.sample_rate, 1.0);
    }

    // ---- LogFormat Debug / Clone ----

    #[test]
    fn log_format_debug_and_clone() {
        let fmt = LogFormat::Text;
        let cloned = fmt;
        let debug_str = format!("{:?}", fmt);
        assert_eq!(debug_str, "Text");
        assert_eq!(fmt, cloned);
    }
}
