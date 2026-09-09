// Prometheus exporter init + metric gathering + guard drop shutdown. Runs
// as its own process (one successful tracing global init per process; the
// global meter provider is also installed here).
#![cfg(feature = "prometheus")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use otelkit::{Exporter, TelemetryConfig, TelemetryError};

#[test]
fn prometheus_init_gather_metrics_and_guard_shutdown() {
    let config = TelemetryConfig::new("otelkit-prometheus-tests")
        .exporter(Exporter::Prometheus)
        .log_level("info");

    let guard = otelkit::init(config)
        .map_err(|e| format!("init failed: {e}"))
        .unwrap();

    // Record a metric through the globally installed meter provider; the
    // Prometheus reader collects it synchronously on gather.
    let meter = opentelemetry::global::meter("otelkit-prometheus-tests");
    let counter = meter.u64_counter("otelkit_test_requests_total").build();
    counter.add(1, &[]);

    let exposition = guard
        .gather_metrics()
        .map_err(|e| format!("gather failed: {e}"))
        .unwrap();
    assert!(
        exposition.contains("otelkit_test_requests"),
        "exposition should contain the recorded counter, got: {exposition}"
    );

    // Drop triggers SdkMeterProvider::shutdown().
    drop(guard);
}

#[test]
fn prometheus_init_invalid_log_level_fails_before_global_init() {
    // `EnvFilter::try_new` rejects this directive before the global
    // subscriber or meter provider is touched, so the error path is safe
    // regardless of test ordering/parallelism. (Free-form junk like
    // "not a level!!!" parses as a target directive and would NOT fail.)
    let err = otelkit::init(
        TelemetryConfig::new("otelkit-prometheus-bad-level")
            .exporter(Exporter::Prometheus)
            .log_level("info=bogus"),
    );
    assert!(matches!(err, Err(TelemetryError::InvalidConfig(_))));
}
