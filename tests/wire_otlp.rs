// Wire tests exercise a real OTLP export; unwrap/expect is the test signal.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#![cfg(feature = "otlp")]

//! OTLP wire test: export a real span over HTTP/protobuf to a local
//! receiver and assert the request shape at the HTTP boundary.
//!
//! `init()` installs a global tracing subscriber, which can only happen
//! once per process — so this binary runs exactly one test that owns the
//! full flow: spin up a minimal OTLP/HTTP receiver on loopback, initialize
//! otelkit against it, emit a span, drop the guard (flush + shutdown), and
//! verify the receiver got a well-formed OTLP/protobuf export.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use otelkit::{Exporter, TelemetryConfig};

/// A minimal HTTP receiver that accepts one OTLP/protobuf POST and hands
/// the request head + body back to the test.
fn spawn_otlp_receiver() -> (std::net::SocketAddr, thread::JoinHandle<(String, Vec<u8>)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];

        // Read until we have the full head and the body announced by
        // content-length.
        let head_end = loop {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0, "client closed before sending a full request");
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n")
                && body_complete(&buf, pos + 4)
            {
                break pos + 4;
            }
        };

        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let body = buf[head_end..].to_vec();

        // Respond 200 and close so the exporter is not left hanging.
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .unwrap();
        stream.flush().unwrap();
        (head, body)
    });
    (addr, handle)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn body_complete(buf: &[u8], body_start: usize) -> bool {
    let head = String::from_utf8_lossy(&buf[..body_start]).to_ascii_lowercase();
    let len: usize = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    buf.len() >= body_start + len
}

#[test]
fn otlp_export_ships_protobuf_spans_to_the_receiver() {
    let (addr, receiver) = spawn_otlp_receiver();

    let config = TelemetryConfig::new("otelkit-wire-otlp")
        .exporter(Exporter::Otlp)
        .otlp_endpoint(format!("http://{addr}"))
        .log_level("info");
    assert_eq!(config.exporter, Exporter::Otlp);

    // Global subscriber init — this test owns the process's one shot.
    let guard = otelkit::init(config).unwrap();

    // Emit a span; the batch exporter ships it on guard drop (shutdown
    // flush).
    tracing::info_span!("otelkit-wire-operation", operation = "export-check").in_scope(|| {
        tracing::debug!("inside the span");
    });

    drop(guard);

    let (head, body) = receiver.join().unwrap();

    // The export must be a protobuf POST (opentelemetry-otlp `http-proto`
    // default). The configured endpoint is used verbatim as the path.
    assert!(head.starts_with("POST "), "unexpected request line: {head}");
    let head_lower = head.to_ascii_lowercase();
    assert!(
        head_lower.contains("content-type: application/x-protobuf"),
        "expected protobuf content type, got: {head}"
    );

    // OTLP ExportTraceServiceRequest: field 1 (resource_spans), wire type 2
    // (length-delimited) → the body must start with tag byte 0x0A and
    // carry a non-trivial payload.
    assert!(!body.is_empty(), "protobuf body must not be empty");
    assert_eq!(
        body[0], 0x0A,
        "body must start with the resource_spans field tag"
    );
    assert!(
        body.len() > 16,
        "suspiciously small export: {} bytes",
        body.len()
    );

    // The configured service name must reach the wire inside the OTLP
    // resource attributes (protobuf embeds the raw UTF-8 bytes).
    let name = b"otelkit-wire-otlp";
    assert!(
        body.windows(name.len()).any(|w| w == name),
        "export must carry the configured service name"
    );
}
