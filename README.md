Tracing and telemetry initialization for Rust; OpenTelemetry, OTLP export, Sentry integration, and structured logging.
- **OpenTelemetry**; trace context propagation and span export.
- **OTLP export**; push traces to any OTLP-compatible collector (Jaeger, Grafana Tempo, etc.).
- **Sentry**; error tracking with release association and sample-rate control.
- **Structured logging**; `tracing-subscriber` with env-filter, JSON or text output, and an RAII guard that flushes on shutdown.
| `otlp` |; | OpenTelemetry OTLP trace export to any OTLP-compatible collector. |
| `stdout` |; | Stdout span exporter; hermetic, no network access needed. |
| `prometheus` |; | In-process Prometheus metrics exposition via `TelemetryGuard::gather_metrics`. |
| `sentry` |; | Sentry error tracking integration with release association and sample-rate control. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP endpoint |; |
| `SENTRY_DSN` | Sentry DSN |; |