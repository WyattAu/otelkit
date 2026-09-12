# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

## [2.0.3] - 2026-09-12

### Added

- `tests/config_matrix.rs` — per-knob behavior matrix for all 8 telemetry
  knobs (gap fill: `OTEL_EXPORTER` env mapping was never asserted in
  `tests/from_env.rs`; feature-off exporter rejections; repeatable
  log-level rejection proving no global-state side effect).
- `tests/wire_otlp.rs` now asserts the configured service name reaches the
  OTLP wire bytes (was: shape tag only).

### Fixed

- `cargo clippy --no-default-features -D warnings`: `unused_mut` on the
  `TelemetryGuard` binding when no backend feature is enabled
  (pre-existing; `#[allow(unused_mut)]` with rationale comment).

### Added

- Wire integration suites proving the full export paths at the HTTP
  boundary (each binary owns the process-global tracing/sentry state):
  - `tests/wire_otlp.rs` — a real span exported as OTLP/protobuf
    (HTTP) to a local receiver; asserts method, path, content type, and
    the protobuf field tag of the payload.
  - `tests/wire_prometheus.rs` — real metrics recorded through the global
    meter, gathered via `TelemetryGuard::gather_metrics()`, then served
    over a TCP listener and scraped back over the socket (counter,
    histogram, label dimensions, resource attributes).
  - `tests/wire_sentry.rs` — an event captured through the real Sentry
    transport otelkit initializes, delivered as an envelope to a local
    collector (wiremock); asserts the `x-sentry-auth` SDK metadata, the
    envelope event id, and the captured payload.

### CI

- New `integration` job running the three wire suites with their
  respective features.

## [2.0.1] - 2026-09-09

### Tests

- Raised coverage from 83.9% to 97.1% (regions, `--all-features`): tests for
  the stdout and Prometheus exporter init paths, `TelemetryGuard::gather_metrics()`,
  guard-drop shutdown error branches, and backend dispatch via `init()`.

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
