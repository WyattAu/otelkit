// Prometheus exporter with Text log format — complements
// tests/init_prometheus.rs (which uses the default JSON format). Each
// tracing global init needs its own process, so the Text arm of the
// format match lives here.
#![cfg(feature = "prometheus")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use otelkit::{Exporter, LogFormat, TelemetryConfig};

#[test]
fn prometheus_text_format_init_gather_and_shutdown() {
    let config = TelemetryConfig::new("otelkit-prometheus-text-tests")
        .exporter(Exporter::Prometheus)
        .log_format(LogFormat::Text)
        .log_level("info");

    let guard = otelkit::init(config)
        .map_err(|e| format!("init failed: {e}"))
        .unwrap();

    let meter = opentelemetry::global::meter("otelkit-prometheus-text-tests");
    let gauge = meter.u64_gauge("otelkit_text_probe").build();
    gauge.record(1, &[]);

    let exposition = guard
        .gather_metrics()
        .map_err(|e| format!("gather failed: {e}"))
        .unwrap();
    assert!(
        exposition.contains("otelkit_text_probe"),
        "exposition should contain the recorded gauge, got: {exposition}"
    );

    drop(guard);
}
