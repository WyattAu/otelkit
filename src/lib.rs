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
