# Coverage Notes: otelkit
- `tests/init_stdout.rs`; stdout backend init, span export, guard-drop
- `tests/init_prometheus.rs`; prometheus backend init (JSON log format),
- `tests/init_prometheus_text.rs`; prometheus backend with the Text log
- `src/lib.rs`; `guard_drop_reports_stdout_provider_shutdown_error` and