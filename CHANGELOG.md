Changelog](https://keepachangelog.com/); versions follow [semver](https://semver.org).
- `tests/config_matrix.rs`; per-knob behavior matrix for all 8 telemetry
  - `tests/wire_otlp.rs`; a real span exported as OTLP/protobuf
  - `tests/wire_prometheus.rs`; real metrics recorded through the global
  - `tests/wire_sentry.rs`; an event captured through the real Sentry