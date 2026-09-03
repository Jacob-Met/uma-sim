//! Port of TelemetryReplayTest.kt

mod common;

use common::{base_state, stats5, KT_RES};
use serde::Deserialize;
use std::path::PathBuf;
use uma_sim_core::state::{MoodLevel, TurnPhase};
use uma_sim_core::{scenario_plugin_for, BotDecisionAdapter, TrainingResolver};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryReplayFixture {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default = "default_energy")]
    energy: i32,
    #[serde(default = "default_mood")]
    mood: String,
    #[serde(default = "default_stat")]
    speed: i32,
    #[serde(default = "default_stat")]
    stamina: i32,
    #[serde(default = "default_stat")]
    power: i32,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    expected_index: i32,
    #[serde(default = "default_facility")]
    expected_facility: String,
    #[serde(default)]
    description: String,
}

fn default_energy() -> i32 {
    60
}
fn default_mood() -> String {
    "NORMAL".into()
}
fn default_stat() -> i32 {
    200
}
fn default_facility() -> String {
    "speed".into()
}

fn load_fixtures() -> Vec<TelemetryReplayFixture> {
    let path = PathBuf::from(KT_RES).join("telemetry_replay/fixtures.json");
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&raw).expect("parse fixtures.json")
}

fn mood_from(s: &str) -> MoodLevel {
    MoodLevel::from_scoring_name(s).unwrap_or(MoodLevel::Normal)
}

fn event_state(f: &TelemetryReplayFixture) -> uma_sim_core::CareerState {
    let mut state = base_state("ura", 5);
    state.stats = stats5(f.speed, f.stamina, f.power, 200, 200);
    state.energy = f.energy;
    state.mood = mood_from(&f.mood);
    state.phase = TurnPhase::Event.as_str().to_string();
    state.pending_event_title = Some("Replay".into());
    state.pending_event_options = f.options.clone();
    state.awaiting_choice = true;
    state
}

fn training_state(f: &TelemetryReplayFixture) -> uma_sim_core::CareerState {
    let mut state = base_state("ura", 10);
    state.stats = stats5(f.speed, f.stamina, f.power, 200, 200);
    state.energy = f.energy;
    state.mood = mood_from(&f.mood);
    state
}

#[test]
fn replay_fixtures_match_at_least_90_percent() {
    let fixtures = load_fixtures();
    assert!(!fixtures.is_empty(), "fixtures.json missing");
    let mut matches = 0;
    let plugin = scenario_plugin_for("ura");
    let resolver = TrainingResolver::default();
    for f in &fixtures {
        match f.kind.as_str() {
            "event" => {
                let idx = BotDecisionAdapter::choose_event_option(&event_state(f), plugin.as_ref());
                if idx == f.expected_index {
                    matches += 1;
                }
            }
            "training" => {
                let facility = BotDecisionAdapter::choose_training_facility(
                    &training_state(f),
                    plugin.as_ref(),
                    &resolver,
                );
                if facility.key() == f.expected_facility.to_lowercase() {
                    matches += 1;
                }
            }
            _ => {}
        }
    }
    let rate = matches as f64 / fixtures.len() as f64;
    assert!(
        rate >= 0.90,
        "Replay match {matches}/{} = {:.0}%",
        fixtures.len(),
        rate * 100.0
    );
}

#[test]
fn event_fixtures_exact_match() {
    let plugin = scenario_plugin_for("ura");
    for f in load_fixtures().into_iter().filter(|f| f.kind == "event") {
        assert_eq!(
            f.expected_index,
            BotDecisionAdapter::choose_event_option(&event_state(&f), plugin.as_ref()),
            "{}",
            f.description
        );
    }
}
