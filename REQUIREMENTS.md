# Requirements — otelkit

Numbered, testable requirements. Every requirement maps to at least one named
test; every security-relevant test cites at least one requirement. Doc
comments on the implementing public item carry `REQ-OTL-NNN` tags.

## Functional

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-OTL-001 | `TelemetryConfig::new` sets the service name; builder setters (`service_version`, `log_level`, `log_format`, `otlp_endpoint`, `sentry_dsn`, `sample_rate`, `exporter`) overwrite prior values and all fields are accessible | MUST |
| REQ-OTL-002 | `TelemetryConfig::from_env` returns documented defaults when no environment variables are set | MUST |
| REQ-OTL-003 | `TelemetryConfig::from_env` applies environment overrides on top of builder values (env wins) | MUST |
| REQ-OTL-004 | `from_env` rejects an unparseable `OTELKIT_SAMPLE_RATE` with `Err(TelemetryError)` instead of silently defaulting | MUST |
| REQ-OTL-005 | `sample_rate` values are clamped to `[0.0, 1.0]`, including boundary inputs (negative, > 1, NaN-adjacent) | MUST |
| REQ-OTL-006 | `LogFormat::from_str_opt` parses `text`/`json` case-sensitively; unknown input yields `None`/documented default | MUST |
| REQ-OTL-007 | `Exporter::from_str_opt` parses exporter names; default exporter is OTLP; variants are distinct; `Debug` renders without leaking DSNs | MUST |
| REQ-OTL-008 | `init` with `Exporter::Stdout` succeeds without network access and installs a working subscriber | MUST |
| REQ-OTL-009 | `init` with a JSON `LogFormat` completes a full lifecycle (init → emit → guard drop) | MUST |
| REQ-OTL-010 | `init` with a Text `LogFormat` completes a full lifecycle; invalid filter/log-level input returns `Err` (`init_err`) | MUST |
| REQ-OTL-011 | `init` with OTLP establishes the provider pipeline and `TelemetryGuard` drop shuts it down cleanly | MUST |
| REQ-OTL-012 | A malformed OTLP endpoint maps to `TelemetryError::OtlpConnection` (connection-class error), not a panic | MUST |
| REQ-OTL-013 | `init` without an OTLP endpoint or Sentry DSN returns `TelemetryError::InvalidConfig` | MUST |
| REQ-OTL-014 | `init` with Sentry carries the configured `sample_rate` and release into Sentry options | SHOULD |
| REQ-OTL-015 | A second `init` (global subscriber rebind) fails with `Err` for JSON, OTLP, and text paths — the global subscriber is bound exactly once | MUST |
| REQ-OTL-016 | `TelemetryGuard::drop` flushes/shuts down owned providers (OTLP, stdout, Prometheus meter) exactly once; shutdown errors are reported, not swallowed silently | MUST |
| REQ-OTL-017 | `gather_metrics` returns the Prometheus text exposition for a Prometheus-backed guard | MUST |
| REQ-OTL-018 | `gather_metrics` on a non-Prometheus guard returns `TelemetryError::InvalidConfig` (never panics) | MUST |
| REQ-OTL-019 | `TelemetryError` implements `std::error::Error`, renders stable `Display` messages (including long messages), and has a usable `Debug` | MUST |
| REQ-OTL-020 | `LogFormat` has sound `Eq`/`Clone`/`Debug` semantics (equality is symmetric, clones are independent) | SHOULD |
| REQ-OTL-021 | `from_env()` free function mirrors `TelemetryConfig::from_env` semantics | SHOULD |

## Security

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-OTL-100 | No public function panics on invalid configuration — bad endpoints, bad log levels, bad sample rates, double init all return `Err(TelemetryError)` | MUST |
| REQ-OTL-101 | The Sentry DSN and OTLP endpoint are treated as configuration, not telemetry content — `Debug`/`Display` of config, exporter, and error types must not embed the DSN value | MUST |
| REQ-OTL-102 | Telemetry export failures (network down, malformed endpoint) degrade to errors returned to the caller; they never crash or abort the process | MUST |
| REQ-OTL-103 | The crate forbids `unsafe` code; environment-variable parsing cannot panic on arbitrary hostile values | MUST |
| REQ-OTL-104 | Global initialization is single-owner: a second `init` cannot silently replace the subscriber chain (telemetry tamper resistance) | MUST |

## Robustness

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-OTL-200 | Guard lifetime defines provider lifetime: dropping the guard shuts down exporters so buffered spans/metrics are flushed before process exit | MUST |
| REQ-OTL-201 | Builder calls are idempotent/overwritable — later setters replace earlier ones deterministically (log level, log format, endpoint, DSN, name, version) | SHOULD |
| REQ-OTL-202 | `from_env` works identically in child processes with clean environments (no reliance on ambient process state) | SHOULD |
| REQ-OTL-203 | Invalid `EnvFilter` strings are rejected at init time with `InvalidConfig` rather than silently producing a no-op filter | MUST |

## Traceability Matrix

| Requirement | Test (fn, file) | Property class |
|-------------|-----------------|----------------|
| REQ-OTL-001 | `telemetry_config_new_sets_service_name`, `telemetry_config_builder`, `telemetry_config_all_fields_accessible`, `telemetry_config_builder_overwrite_*`, `telemetry_config_full_builder_chain`, `telemetry_config_exporter_builder` (`src/config.rs`) | unit |
| REQ-OTL-002 | `from_env_defaults`, `from_env_returns_defaults_when_no_vars_set` (`src/config.rs`), `from_env_reads_overrides_and_defaults` (`tests/from_env.rs`) | unit |
| REQ-OTL-003 | `from_env_reads_overrides_and_defaults` (`tests/from_env.rs`), `from_env_preserves_builder_pattern_semantics` (`src/config.rs`) | integration |
| REQ-OTL-004 | `from_env_invalid_sample_rate_unparseable` (`src/config.rs`) | unit |
| REQ-OTL-005 | `sample_rate_clamped`, `sample_rate_clamp_boundary_values` (`src/config.rs`) | unit |
| REQ-OTL-006 | `log_format_from_str_text`, `log_format_from_str_json_default`, `log_format_from_str_opt_case_sensitivity`, `log_format_from_str_opt_unknown_defaults_to_json` (`src/config.rs`) | unit |
| REQ-OTL-007 | `exporter_from_str_opt`, `exporter_default_is_otlp`, `exporter_variants_distinct`, `exporter_debug_format` (`src/exporter.rs`) | unit |
| REQ-OTL-008 | `init_stdout_succeeds_without_network` (`src/lib.rs`), `text_format_init_lifecycle` (`tests/init_text.rs`) | unit/integration |
| REQ-OTL-009 | `json_format_init_lifecycle` (`tests/init_json.rs`) | integration |
| REQ-OTL-010 | `text_format_init_lifecycle`, `init_err` (`tests/init_text.rs`) | integration |
| REQ-OTL-011 | `otlp_init_and_guard_shutdown` (`tests/init_otlp.rs`) | integration |
| REQ-OTL-012 | `init_otlp_malformed_endpoint_maps_to_connection_error` (`tests/init_otlp_errors.rs`) | integration |
| REQ-OTL-013 | `init_without_otlp_endpoint_or_sentry_dsn_is_invalid_config` (`tests/init_sentry_errors.rs`) | integration |
| REQ-OTL-014 | `sentry_options_carry_sample_rate_and_release` (`tests/sentry_options.rs`) | integration |
| REQ-OTL-015 | `init_json_twice_fails_to_rebind_global_subscriber` (`tests/init_json_errors.rs`), `init_otlp_twice_fails_to_rebind_global_subscriber` (`tests/init_otlp_errors.rs`) | integration |
| REQ-OTL-016 | `guard_drop_reports_provider_shutdown_error` (`src/lib.rs`), `otlp_init_and_guard_shutdown` (`tests/init_otlp.rs`) | unit/integration |
| REQ-OTL-017 | `init_prometheus_guard_gather_without_init` (`src/lib.rs`), prometheus lifecycle in `src/lib.rs` `init_prometheus` | unit |
| REQ-OTL-018 | `gather_metrics` `InvalidConfig` path (`src/lib.rs`), `init_prometheus_guard_gather_without_init` (`src/lib.rs`) | unit |
| REQ-OTL-019 | `error_debug_format`, `error_invalid_config_display`, `error_invalid_config_long_message`, `error_otlp_connection_display`, `error_otlp_connection_long_message`, `error_is_std_error` (`src/error.rs`) | unit |
| REQ-OTL-020 | `log_format_equality`, `log_format_equality_asymmetric`, `log_format_clone_independence`, `log_format_debug_and_clone`, `log_format_json_debug`, `log_format_variants` (`src/config.rs`) | unit |
| REQ-OTL-021 | `from_env_reads_overrides_and_defaults` (`tests/from_env.rs`), `from_env_defaults` (`src/config.rs`) | unit/integration |
| REQ-OTL-100 | `init_err` (`tests/init_text.rs`), `init_otlp_malformed_endpoint_maps_to_connection_error` (`tests/init_otlp_errors.rs`), `from_env_invalid_sample_rate_unparseable` (`src/config.rs`), double-init tests (`tests/init_json_errors.rs`, `tests/init_otlp_errors.rs`) | unit/integration |
| REQ-OTL-101 | `telemetry_config_debug_format` (`src/config.rs`), `exporter_debug_format` (`src/exporter.rs`) | unit |
| REQ-OTL-102 | `init_otlp_malformed_endpoint_maps_to_connection_error` (`tests/init_otlp_errors.rs`), `guard_drop_reports_provider_shutdown_error` (`src/lib.rs`) | unit/integration |
| REQ-OTL-103 | `#![forbid(unsafe_code)]` (`src/lib.rs`); `from_env_reads_overrides_and_defaults` with hostile env values (`tests/from_env.rs`) | unit |
| REQ-OTL-104 | `init_json_twice_fails_to_rebind_global_subscriber` (`tests/init_json_errors.rs`), `init_otlp_twice_fails_to_rebind_global_subscriber` (`tests/init_otlp_errors.rs`) | integration |
| REQ-OTL-200 | `otlp_init_and_guard_shutdown` (`tests/init_otlp.rs`), `guard_drop_reports_provider_shutdown_error` (`src/lib.rs`) | integration/unit |
| REQ-OTL-201 | `telemetry_config_builder_overwrite_log_level`, `telemetry_config_builder_overwrite_log_format`, `telemetry_config_builder_overwrite_otlp_endpoint`, `telemetry_config_builder_overwrite_sentry_dsn`, `telemetry_config_builder_overwrite_service_name`, `telemetry_config_builder_overwrite_service_version` (`src/config.rs`) | unit |
| REQ-OTL-202 | `run_child` / `write_child_output` / `child_dump` harness in `tests/from_env.rs` | integration |
| REQ-OTL-203 | `init_err` (`tests/init_text.rs`), invalid-filter mapping in `init` (`src/lib.rs`) | integration |
