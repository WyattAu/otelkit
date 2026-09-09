# Coverage Notes — otelkit

## Measurement

```
cargo llvm-cov --summary-only --all-features
```

Current: **97.06% region / 98.43% line coverage**. All exporter backends
(OTLP, Stdout, Prometheus) and their lifecycle paths are exercised. Tests
added to close the gap:

- `tests/init_stdout.rs` — stdout backend init, span export, guard-drop
  shutdown, and the invalid-log-level error path (`EnvFilter::try_new`
  rejects `info=bogus` before any global state is touched).
- `tests/init_prometheus.rs` — prometheus backend init (JSON log format),
  counter recorded through the globally installed meter provider, successful
  `TelemetryGuard::gather_metrics()`, guard-drop shutdown, and the
  invalid-log-level error path.
- `tests/init_prometheus_text.rs` — prometheus backend with the Text log
  format arm (each global subscriber init needs its own process, so the
  format arms live in separate files).
- `src/lib.rs` — `guard_drop_reports_stdout_provider_shutdown_error` and
  `guard_drop_reports_meter_provider_shutdown_error`: dropping a
  `TelemetryGuard` holding an already-shut-down provider (opentelemetry-sdk
  returns `AlreadyShutdown` on the second `shutdown()`) exercises the stdout
  and meter error branches of `Drop for TelemetryGuard`.

Integration tests are kept in separate files on purpose: tracing allows one
successful global subscriber per process, and each scenario needs the global
state to itself. Error-path tests must fail *before* any global state is
touched (`EnvFilter::try_new` rejection) so they are safe under parallel
execution. Note that free-form junk like `"not a level!!!"` parses as a
target directive and does NOT fail; a directive with an invalid level such
as `info=bogus` does.

## Known unreachable defensive code

Several `map_err` closure regions remain uncovered by design:

- `init()`'s OTLP branch re-validates `EnvFilter::try_new` (the identical
  call earlier in the function already rejects any invalid level string, so
  the inner `map_err` arm is provably unreachable). It is kept deliberately:
  removing it would couple the branch's correctness to the earlier
  validation surviving refactors.
- `gather_metrics()`'s `encode` and `from_utf8` error arms cannot fail with
  the Prometheus text encoder's own output.
- The exporter-build `map_err` in `init_prometheus` only fires if the
  prometheus exporter constructor fails, which the registry/reader setup
  does not.
