//! Config-knob behavior matrix for otelkit.
//!
//! Every public knob must OBSERVABLY change behavior. Knob inventory (8):
//!
//! | knob | behavior proof |
//! |------|----------------|
//! | service_name | `wire_otlp.rs` ships it in the OTLP resource (byte
//! |   assertion); `init_otlp.rs` sets it |
//! | service_version | `sentry_options.rs` asserts it lands in
//! |   `ClientOptions.release` |
//! | log_level | invalid values rejected (`init_text.rs` + below);
//! valid values drive the EnvFilter |
//! | log_format | `init_text.rs` vs `init_json.rs` install different
//! |   layers; parse surface below |
//! | otlp_endpoint | `wire_otlp.rs` receives the export at the configured
//! |   endpoint; env mapping in `from_env.rs` |
//! | sentry_dsn | missing DSN with `sentry` feature is a config error
//! |   (`init_text.rs`); present DSN binds the client (`sentry_options.rs`) |
//! | sample_rate | clamp below; `sentry_options.rs` asserts it lands in
//! |   `traces_sample_rate` |
//! | exporter | selection drives init dispatch (`init_stdout.rs`,
//! |   `init_prometheus.rs`, `init_otlp.rs`, `wire_*.rs`); env mapping
//! |   + feature-off rejections below |
//!
//! NOTE: `sample_rate` only affects the `sentry` backend
//! (`traces_sample_rate`); builds without `sentry` accept and store it
//! with no observable effect. Documented here, not a bug.
//!
//! Tracing allows one global subscriber per process, so tests that touch
//! `init()` either assert repeatable *failures* (no global state taken)
//! or live behind feature gates that keep them off the success path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use otelkit::{Exporter, LogFormat, TelemetryConfig, TelemetryError};

/// `init` variant for the failure assertions (`TelemetryGuard` has no
/// `Debug`, so `.unwrap_err()` is unusable on it).
fn init_err(config: TelemetryConfig) -> TelemetryError {
    match otelkit::init(config) {
        Err(e) => e,
        Ok(_) => panic!("expected init to fail"),
    }
}

// ---------------------------------------------------------------------------
// log_level: invalid values are rejected WITHOUT taking global state
// (repeatable — a second identical call must fail the same way, proving
// the subscriber was never installed).
// ---------------------------------------------------------------------------

#[test]
fn knob_log_level_invalid_rejected_repeatably() {
    for _ in 0..2 {
        let err = init_err(TelemetryConfig::default().log_level("foo=notalevel"));
        assert!(
            matches!(err, TelemetryError::InvalidConfig(_)),
            "invalid log level must be a config error, got: {err}"
        );
    }
}

#[test]
fn knob_log_format_parse_surface() {
    assert_eq!(LogFormat::from_str_opt("text"), LogFormat::Text);
    assert_eq!(LogFormat::from_str_opt("TEXT"), LogFormat::Text);
    assert_eq!(LogFormat::from_str_opt("plain"), LogFormat::Text);
    assert_eq!(LogFormat::from_str_opt("json"), LogFormat::Json);
    // Unknown input defaults to Json (pinned: from_env.rs relies on it).
    assert_eq!(LogFormat::from_str_opt("banana"), LogFormat::Json);
}

// ---------------------------------------------------------------------------
// sample_rate: builder clamps to [0.0, 1.0]; the clamped value (not the
// input) is what `init` hands to Sentry (see sentry_options.rs).
// ---------------------------------------------------------------------------

#[test]
fn knob_sample_rate_clamped_to_unit_range() {
    assert_eq!(TelemetryConfig::default().sample_rate(2.0).sample_rate, 1.0);
    assert_eq!(
        TelemetryConfig::default().sample_rate(-0.5).sample_rate,
        0.0
    );
    assert_eq!(
        TelemetryConfig::default().sample_rate(0.25).sample_rate,
        0.25
    );
}

// ---------------------------------------------------------------------------
// exporter: pure parse surface + env mapping (child process: env mutation
// is process-global, so re-exec like tests/from_env.rs does).
// ---------------------------------------------------------------------------

#[test]
fn knob_exporter_from_str_opt() {
    assert_eq!(Exporter::from_str_opt("otlp"), Exporter::Otlp);
    assert_eq!(Exporter::from_str_opt("OTLP"), Exporter::Otlp);
    assert_eq!(Exporter::from_str_opt("stdout"), Exporter::Stdout);
    assert_eq!(Exporter::from_str_opt("STDOUT"), Exporter::Stdout);
    assert_eq!(Exporter::from_str_opt("prometheus"), Exporter::Prometheus);
    // Unknown input defaults to Otlp (historical behavior, pinned).
    assert_eq!(Exporter::from_str_opt("banana"), Exporter::Otlp);
    assert_eq!(Exporter::default(), Exporter::Otlp);
}

const CHILD_MARKER: &str = "OTELKIT_MATRIX_CHILD";
const CHILD_OUT: &str = "OTELKIT_MATRIX_OUT";

fn run_child(tag: &str, vars: &[(&str, &str)]) -> String {
    let out = std::env::temp_dir().join(format!("otelkit-matrix-{}-{tag}.txt", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .env(CHILD_MARKER, "1")
        .env(CHILD_OUT, &out)
        .envs(vars.iter().copied())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn child");
    assert!(status.success(), "child exited with {status}");
    std::fs::read_to_string(&out).expect("read child output")
}

#[test]
fn knob_exporter_env_mapping() {
    if std::env::var(CHILD_MARKER).is_ok() {
        let cfg = otelkit::from_env().expect("from_env should not fail");
        let name = match cfg.exporter {
            Exporter::Otlp => "otlp",
            Exporter::Stdout => "stdout",
            Exporter::Prometheus => "prometheus",
        };
        std::fs::write(std::env::var(CHILD_OUT).unwrap(), name).unwrap();
        std::process::exit(0);
    }

    // GAP FILL: tests/from_env.rs never asserted OTEL_EXPORTER.
    assert_eq!(
        run_child("exp-stdout", &[("OTEL_EXPORTER", "stdout")]),
        "stdout"
    );
    assert_eq!(
        run_child("exp-prom", &[("OTEL_EXPORTER", "prometheus")]),
        "prometheus"
    );
    assert_eq!(run_child("exp-otlp", &[("OTEL_EXPORTER", "otlp")]), "otlp");
    assert_eq!(
        run_child("exp-case", &[("OTEL_EXPORTER", "STDOUT")]),
        "stdout",
        "exporter parsing must be case-insensitive"
    );
    assert_eq!(
        run_child("exp-bogus", &[("OTEL_EXPORTER", "banana")]),
        "otlp",
        "unknown exporter must fall back to otlp"
    );
    // Unset → compiled-in default.
    assert_eq!(run_child("exp-default", &[]), "otlp");
}

// ---------------------------------------------------------------------------
// exporter without its feature: hard config error (repeatable — returned
// before any subscriber install). Only compiled where the feature is off,
// since with the feature on these selections take the success path owned
// by init_stdout.rs / init_prometheus.rs.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "stdout"))]
#[test]
fn knob_exporter_stdout_without_feature_rejected() {
    for _ in 0..2 {
        let err = init_err(TelemetryConfig::default().exporter(Exporter::Stdout));
        assert!(
            matches!(&err, TelemetryError::InvalidConfig(m) if m.contains("stdout")),
            "got: {err}"
        );
    }
}

#[cfg(not(feature = "prometheus"))]
#[test]
fn knob_exporter_prometheus_without_feature_rejected() {
    for _ in 0..2 {
        let err = init_err(TelemetryConfig::default().exporter(Exporter::Prometheus));
        assert!(
            matches!(&err, TelemetryError::InvalidConfig(m) if m.contains("prometheus")),
            "got: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// Builder setters store what init reads (unit-level plumbing pins).
// ---------------------------------------------------------------------------

#[test]
fn knob_builder_setters_store_values() {
    let cfg = TelemetryConfig::new("svc-a")
        .service_version("1.2.3")
        .log_level("debug")
        .log_format(LogFormat::Text)
        .otlp_endpoint("http://collector:4317")
        .sentry_dsn("https://key@sentry.io/9")
        .sample_rate(0.5)
        .exporter(Exporter::Stdout);
    assert_eq!(cfg.service_name, "svc-a");
    assert_eq!(cfg.service_version, "1.2.3");
    assert_eq!(cfg.log_level, "debug");
    assert_eq!(cfg.log_format, LogFormat::Text);
    assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://collector:4317"));
    assert_eq!(cfg.sentry_dsn.as_deref(), Some("https://key@sentry.io/9"));
    assert_eq!(cfg.sample_rate, 0.5);
    assert_eq!(cfg.exporter, Exporter::Stdout);
}
