// Wire tests exercise the live metrics pipeline; unwrap/expect is the test
// signal here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#![cfg(feature = "prometheus")]

//! Prometheus wire test: initialize the Prometheus exporter, record real
//! metrics through the global meter, gather the exposition text, and serve
//! it over a TCP listener exactly like a `/metrics` scrape target, then
//! fetch and assert it over that socket.
//!
//! `init()` installs a global subscriber/meter provider, which can only
//! happen once per process — so this binary runs exactly one test owning
//! the full flow.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use otelkit::{Exporter, TelemetryConfig};

#[test]
fn prometheus_metrics_gather_and_serve_over_a_scrape() {
    let guard = otelkit::init(
        TelemetryConfig::new("otelkit-wire-prometheus")
            .exporter(Exporter::Prometheus)
            .log_level("warn"),
    )
    .unwrap();

    // Record real metrics through the global meter provider that
    // `init` installed.
    use opentelemetry::KeyValue;
    use opentelemetry::metrics::{Counter, Histogram};

    let meter = opentelemetry::global::meter("otelkit-wire");
    let requests: Counter<u64> = meter.u64_counter("otelkit_wire_requests_total").build();
    requests.add(3, &[KeyValue::new("route", "ok")]);
    requests.add(2, &[KeyValue::new("route", "fail")]);

    let latency: Histogram<u64> = meter
        .u64_histogram("otelkit_wire_request_duration_ms")
        .build();
    latency.record(12, &[]);
    latency.record(48, &[]);

    // Give the cumulative reader a beat to process the deltas.
    std::thread::sleep(Duration::from_millis(100));

    // 1. Gathered exposition text contains the metric families.
    let exposition = guard.gather_metrics().unwrap();
    assert!(
        exposition.contains("otelkit_wire_requests_total"),
        "counter missing from exposition:\n{exposition}"
    );
    assert!(
        exposition.contains("otelkit_wire_request_duration_ms"),
        "histogram missing from exposition:\n{exposition}"
    );
    assert!(
        exposition.contains("route=\"ok\""),
        "label dimensions missing:\n{exposition}"
    );
    // (Resource attributes are not exported as labels by the
    // opentelemetry-prometheus reader — the exposition is scope+metric
    // only, so nothing is asserted about service_name here.)

    // 2. Serve the exposition over a TCP listener like a real scrape
    //    target and fetch it back over the socket.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let body = exposition.clone();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        // Read the request head before responding, then close cleanly
        // (dropping the stream with unread client bytes would RST).
        let mut buf = [0u8; 1024];
        let mut got = 0;
        while got < buf.len() {
            let n = stream.read(&mut buf[got..]).unwrap();
            if n == 0 {
                break;
            }
            got += n;
            if buf[..got].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain; version=0.0.4\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
    });

    let mut client = std::net::TcpStream::connect(addr).unwrap();
    client
        .write_all(
            format!("GET /metrics HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\n\r\n")
                .as_bytes(),
        )
        .unwrap();
    let mut scraped = String::new();
    client.read_to_string(&mut scraped).unwrap();
    server.join().unwrap();

    assert!(
        scraped.starts_with("HTTP/1.1 200 OK"),
        "scrape failed: {scraped}"
    );
    assert!(
        scraped.contains("otelkit_wire_requests_total"),
        "scrape lost the counter"
    );
    assert!(
        scraped.contains("route=\"fail\""),
        "scrape lost label dimensions"
    );
}
