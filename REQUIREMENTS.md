# Requirements: otelkit
| REQ-OTL-015 | A second `init` (global subscriber rebind) fails with `Err` for JSON, OTLP, and text paths; the global subscriber is bound exactly once | MUST |
| REQ-OTL-100 | No public function panics on invalid configuration; bad endpoints, bad log levels, bad sample rates, double init all return `Err(TelemetryError)` | MUST |
| REQ-OTL-101 | The Sentry DSN and OTLP endpoint are treated as configuration, not telemetry content; `Debug`/`Display` of config, exporter, and error types must not embed the DSN value | MUST |
| REQ-OTL-201 | Builder calls are idempotent/overwritable; later setters replace earlier ones deterministically (log level, log format, endpoint, DSN, name, version) | SHOULD |