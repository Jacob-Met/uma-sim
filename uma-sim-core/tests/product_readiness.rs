//! Port of Kotlin `ProductReadinessTest.kt`.

mod common;

use common::{settings_fast, KT_RES, SCENARIOS};
use std::path::PathBuf;
use uma_sim_core::calendar::CAREER_TURNS;
use uma_sim_core::state::{DialogueMode, RunMeta, SimSettings};
use uma_sim_core::SimEngine;

#[test]
fn four_scenarios_complete_72_turns() {
    for scenario in SCENARIOS {
        let mut engine = SimEngine::new(settings_fast(50));
        engine.start(RunMeta::new(99, scenario, "Test"));
        engine.play_to_completion(500);
        assert!(engine.state().career_complete, "{scenario}");
        assert!(engine.state().turn >= CAREER_TURNS, "{scenario}");
    }
}

#[test]
fn speed_multiplier_clamps_1_to_100() {
    assert_eq!(
        SimSettings {
            speed_multiplier: 1,
            ..Default::default()
        }
        .clamped_speed(),
        1
    );
    assert_eq!(
        SimSettings {
            speed_multiplier: 100,
            ..Default::default()
        }
        .clamped_speed(),
        100
    );
    assert_eq!(
        SimSettings {
            speed_multiplier: 999,
            ..Default::default()
        }
        .clamped_speed(),
        100
    );
}

#[test]
fn golden_fixture_file_present() {
    let path = PathBuf::from(KT_RES).join("golden/summaries.json");
    let raw = std::fs::read_to_string(&path).expect("golden/summaries.json");
    assert!(raw.len() > 1000);
}

#[test]
fn telemetry_replay_resources_present() {
    let base = PathBuf::from(KT_RES).join("telemetry_replay");
    assert!(base.join("fixtures.json").is_file());
    assert!(base.join("live_bot_sample.jsonl").is_file());
}

#[test]
fn deck_and_legacy_start_completes_career() {
    let mut engine = SimEngine::new(settings_fast(50));
    let mut meta = RunMeta::new(42, "ura", "Test");
    meta.deck_supports = vec!["support:10001".into()];
    meta.legacy_factors = vec!["factor:blue:1@3".into()];
    engine.start(meta);
    engine.play_to_completion_scoring(500);
    assert!(engine.state().career_complete);
    assert!(!engine.state().deck.slots.is_empty());
    assert!(!engine.state().legacy.factor_ids.is_empty());
}

#[test]
fn bot_policy_completes_all_four_scenarios() {
    for scenario in SCENARIOS {
        let mut engine = SimEngine::new(settings_fast(50));
        engine.start(RunMeta::new(123, scenario, "Bot"));
        engine.play_to_completion_scoring(500);
        assert!(engine.state().career_complete, "{scenario}");
    }
}

#[test]
fn full_career_under_ten_seconds_at_x20() {
    let start = std::time::Instant::now();
    let mut engine = SimEngine::create(SimSettings {
        speed_multiplier: 20,
        dialogue_mode: DialogueMode::Off,
        ..Default::default()
    });
    engine.start(RunMeta::new(42, "grand_concert", "Perf"));
    engine.play_to_completion_scoring(500);
    let elapsed_ms = start.elapsed().as_millis();
    assert!(
        elapsed_ms < 10_000,
        "x20 grand_concert career took {elapsed_ms}ms"
    );
}
