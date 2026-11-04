// TelemetryGuard::drop + init() text-format paths.
//
// tracing installs one global subscriber per process, so init() can only
// succeed once per test binary. Each scenario lives in its own integration
// test file (separate process) and sequences its phases inside a single
// #[test] to keep ordering deterministic.
//
// unwrap/expect are the test signal here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use otelkit::{LogFormat, TelemetryConfig, TelemetryError};

/// `init` variant for the failure assertions (TelemetryGuard has no Debug,
/// so `.unwrap_err()` is not usable on `Result<TelemetryGuard, _>`).
fn init_err(config: TelemetryConfig) -> TelemetryError {
    match otelkit::init(config) {
        Err(e) => e,
        Ok(_) => panic!("expected init to fail"),
    }
}

#[test]
fn text_format_init_lifecycle() {
    // 1. Invalid log level is rejected before any global state is touched.
    //    (EnvFilter treats bare words as target names, so an invalid LEVEL
    //    after '=' is what actually fails to parse.)
    let err = init_err(TelemetryConfig::default().log_level("foo=notalevel"));
    assert!(
        matches!(err, TelemetryError::InvalidConfig(_)),
        "got: {err}"
    );

    // 2. Text format without a Sentry DSN and without an OTLP endpoint:
    //    refused BEFORE any global state is touched (v3.0.1: the validation
    //    moved ahead of subscriber installation, so the process can still
    //    init cleanly afterwards with a valid config).
    #[cfg(feature = "sentry")]
    {
        let err = init_err(TelemetryConfig::default().log_format(LogFormat::Text));
        assert!(
            matches!(&err, TelemetryError::InvalidConfig(msg) if msg.contains("DSN")),
            "got: {err}"
        );

        // 3. With a DSN set, init succeeds: the guard is created and the
        //    global subscriber is now taken.
        let guard = otelkit::init(
            TelemetryConfig::default()
                .log_format(LogFormat::Text)
                .sentry_dsn("https://key@sentry.io/1"),
        )
        .expect("valid config must init");
        drop(guard);

        // 4. Any further init fails because the global subscriber is taken.
        let err = init_err(
            TelemetryConfig::default()
                .log_format(LogFormat::Text)
                .sentry_dsn("https://key@sentry.io/1"),
        );
        assert!(
            matches!(err, TelemetryError::InvalidConfig(_)),
            "got: {err}"
        );
    }

    #[cfg(not(feature = "sentry"))]
    {
        // Without the sentry feature there is no export mechanism compiled
        // at all: a pathless config is refused (silent no-op telemetry),
        // with a DSN present but the feature off the refusal is the same.
        let err = init_err(
            TelemetryConfig::default()
                .log_format(LogFormat::Text)
                .log_level("info"),
        );
        assert!(
            matches!(&err, TelemetryError::InvalidConfig(msg)
                if msg.contains("no export path")),
            "got: {err}"
        );

        // A second init in the same process fails the same way (the
        // export-path check runs before the subscriber is touched, so the
        // refusal is deterministic rather than subscriber-order-dependent).
        let err = init_err(TelemetryConfig::default().log_format(LogFormat::Text));
        assert!(
            matches!(&err, TelemetryError::InvalidConfig(msg)
                if msg.contains("no export path")),
            "got: {err}"
        );
    }
}
