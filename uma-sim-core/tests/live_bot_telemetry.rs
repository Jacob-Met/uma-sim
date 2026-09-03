//! Port of LiveBotTelemetryTest.kt

mod common;

use common::KT_RES;
use std::path::PathBuf;
use uma_sim_core::TelemetryReplayLoader;

#[test]
fn live_bot_sample_match_rate() {
    let path = PathBuf::from(KT_RES).join("telemetry_replay/live_bot_sample.jsonl");
    let raw = std::fs::read_to_string(&path).expect("live_bot_sample.jsonl missing");
    let lines = TelemetryReplayLoader::parse_jsonl(&raw);
    let actionable: Vec<_> = lines
        .into_iter()
        .filter(|l| l.kind == "training" || (l.kind == "event" && l.options.len() >= 2))
        .collect();
    assert!(!actionable.is_empty());
    let (matches, total) = TelemetryReplayLoader::match_rate(&actionable);
    let rate = if total == 0 {
        1.0
    } else {
        matches as f64 / total as f64
    };
    assert!(rate >= 0.90, "Live sample replay {matches}/{total}");
}
