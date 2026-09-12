// Wire tests exercise a real Sentry transport; unwrap/expect is the test
// signal here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#![cfg(feature = "sentry")]

//! Sentry wire test: point a real DSN at a local collector (wiremock),
//! capture an event through the sentry transport otelkit initialized, and
//! assert the envelope shape that arrives at the HTTP boundary.
//!
//! `sentry::init` installs a process-global client — this binary runs
//! exactly one test owning that lifetime.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sentry_events_reach_the_dsn_endpoint_as_envelopes() {
    let server = MockServer::start().await;

    // A DSN pointing at the local collector: project id 42.
    let host = server.address().ip();
    let port = server.address().port();
    let dsn = format!("http://pubkey-otelkit@{host}:{port}/42");

    Mock::given(method("POST"))
        .and(path("/api/42/envelope/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    let guard = otelkit::init(
        otelkit::TelemetryConfig::new("otelkit-wire-sentry")
            .sentry_dsn(&dsn)
            .sample_rate(1.0),
    )
    .unwrap();

    // Capture through the global hub the init installed. The event id
    // comes back synchronously; delivery is async in the transport's
    // background worker.
    let event_id = sentry::capture_message("otelkit-sentry-wire-smoke", sentry::Level::Error);
    let event_id_str = event_id.to_string();
    assert!(
        !event_id_str.is_empty(),
        "capture_message must return an event id"
    );

    // Dropping the guard drains the transport (sentry::ClientInitGuard
    // shutdown flush), giving the background worker a bounded window to
    // deliver before the mock server's expectations are checked.
    drop(guard);

    let requests = server
        .received_requests()
        .await
        .expect("request log enabled");
    assert_eq!(requests.len(), 1, "exactly one envelope must arrive");

    let req = &requests[0];
    // (wiremock's request log does not retain the content-type header; the
    // envelope identity is carried in the `x-sentry-auth` header and the
    // body shape below.)
    let auth = req
        .headers
        .get("x-sentry-auth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        auth.contains("sentry_client=sentry.rust"),
        "Sentry SDK client metadata missing from auth header: {auth}"
    );

    let body = String::from_utf8(req.body.clone()).unwrap();
    // Envelope format: first line is the envelope header JSON, then item
    // headers + payloads. The header carries the event id and the SDK name;
    // the event payload carries our message.
    assert!(
        body.contains(&format!("\"event_id\":\"{}\"", event_id_str)),
        "envelope header must carry the event id:\n{body}"
    );
    assert!(
        body.contains("otelkit-sentry-wire-smoke"),
        "event payload must carry the captured message:\n{body}"
    );
    assert!(
        body.contains("sentry.rust"),
        "SDK client metadata missing:\n{body}"
    );

    server.verify().await;
}
