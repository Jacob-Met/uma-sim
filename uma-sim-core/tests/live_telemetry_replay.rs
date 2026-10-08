//! Port of LiveTelemetryReplayTest.kt

mod common;

use common::KT_RES;
use std::path::PathBuf;
use uma_sim_core::TelemetryReplayLoader;

#[test]
fn jsonl_replay_match_rate_at_least_90_percent() {
    let path = PathBuf::from(KT_RES).join("telemetry_replay/sample_run.jsonl");
    let raw = std::fs::read_to_string(&path).expect("sample_run.jsonl missing");
    let lines = TelemetryReplayLoader::parse_jsonl(&raw);
    assert!(!lines.is_empty());
    let (matches, total) = TelemetryReplayLoader::match_rate(&lines);
    let rate = if total == 0 {
        1.0
    } else {
        matches as f64 / total as f64
    };
    assert!(
        rate >= 0.90,
        "Telemetry replay {matches}/{total} = {:.0}%",
        rate * 100.0
    );
}
