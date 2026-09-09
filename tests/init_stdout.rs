// Stdout exporter init success path + guard drop shutdown. Fully hermetic:
// spans are written to stdout, no collector or network access required.
// Runs as its own process (one successful tracing global init per process).
#![cfg(feature = "stdout")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use otelkit::{Exporter, TelemetryConfig, TelemetryError};

#[test]
fn stdout_init_emit_span_and_guard_shutdown() {
    let config = TelemetryConfig::new("otelkit-stdout-tests")
        .exporter(Exporter::Stdout)
        .log_level("info");

    let guard = otelkit::init(config)
        .map_err(|e| format!("init failed: {e}"))
        .unwrap();

    // Spans flow through the stdout exporter layer and appear in test
    // output; the pipeline must be wired up end to end.
    tracing::info!("otelkit stdout init verified");

    // Drop triggers SdkTracerProvider::shutdown(), flushing the batch
    // exporter to stdout.
    drop(guard);
}

#[test]
fn stdout_init_invalid_log_level_fails_before_global_init() {
    // `EnvFilter::try_new` rejects this directive before the global
    // subscriber is touched, so the error path is safe regardless of test
    // ordering/parallelism. (Free-form junk like "not a level!!!" parses
    // as a target directive and would NOT fail.)
    let err = otelkit::init(
        TelemetryConfig::new("otelkit-stdout-bad-level")
            .exporter(Exporter::Stdout)
            .log_level("info=bogus"),
    );
    assert!(matches!(err, Err(TelemetryError::InvalidConfig(_))));
}
