# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [2.0.0] - 2026-09-08

### Added

- `Exporter` enum (`Otlp` / `Stdout` / `Prometheus`) selecting the telemetry
  backend, with `TelemetryConfig::exporter()` builder and `OTEL_EXPORTER`
  environment variable. Stdout and Prometheus paths are fully hermetic
  (no collector needed).
- `TelemetryConfig::new(service_name)` constructor.
- `TelemetryGuard::gather_metrics()` returning the Prometheus exposition
  text when built with the `prometheus` feature.
- `stdout` and `prometheus` features (opentelemetry-stdout 0.32,
  opentelemetry-prometheus 0.32, prometheus 0.14).

### Changed

- **Breaking:** `TelemetryConfig` gains a public `exporter` field.
- opentelemetry 0.28 → 0.32, opentelemetry_sdk 0.28 → 0.32.1,
  opentelemetry-otlp 0.28 → 0.32, tracing-opentelemetry 0.29 → 0.33.

## [1.0.0] - 2026-09-05

First stable release. The public API is now covered by the project's
semver guarantees: breaking changes require a major version bump.

### Changed

- MSRV is now 1.88 (was 1.85) so the `time` dependency can be bumped to
  ≥ 0.3.47, fixing RUSTSEC-2026-0009 (DoS via stack exhaustion, transitive
  through `sentry-types`).

## [0.1.0] - 2026-09-01

### Added

- OpenTelemetry trace context propagation and span export.
- OTLP export to any OTLP-compatible collector (Jaeger, Grafana Tempo,
  etc.) behind the `otlp` feature.
- Sentry error tracking with release association and sample-rate control
  behind the `sentry` feature.
- Structured logging via `tracing-subscriber`: env-filter, JSON or text
  output (`json` feature, on by default), and an RAII guard that flushes
  on shutdown.
