# Threat Model: otelkit
Assets: (A1) telemetry pipeline integrity; logs/traces go where configured;
- **OPEN-1; `TelemetryConfig` derives `Debug` including `sentry_dsn`.**
- **OPEN-2; `sample_rate` is not validated/clamped.** Negative or > 1.0
- **OPEN-3; no endpoint URL validation on `otlp_endpoint`.** A malformed
  endpoint (env-controlled) redirects telemetry; env is assumed trusted,
- **OPEN-4; no log redaction layer.** The crate transports whatever spans
**Residual risk:** `init` is process-global; calling it twice (tests,