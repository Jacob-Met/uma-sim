//! Port of SimTelemetryExportTest.kt (asserting export shape, not a pure dump).

use uma_sim_core::state::{RunMeta, SimAction, SimActionKind, SimSettings, TurnPhase};
use uma_sim_core::SimEngine;

#[test]
fn export_jsonl_matches_android_turn_shape() {
    let mut engine = SimEngine::new(SimSettings {
        trace_telemetry: true,
        ..Default::default()
    });
    engine.start(RunMeta::new(4242, "ura", "Special Week"));

    // Clear mandatory debut (turn 1), then keep stepping until a successful TRAIN is recorded.
    // Kotlin asserts main_gain > 0; early-career failure rolls can zero a single attempt.
    let mut actions = 0;
    while actions < 40 {
        actions += 1;
        let phase = engine.state().phase.clone();
        if phase == TurnPhase::MandatoryRace.as_str() {
            engine.step(SimAction {
                kind: SimActionKind::Race,
                payload: None,
            });
            continue;
        }
        if phase == TurnPhase::Event.as_str() {
            engine.step(SimAction {
                kind: SimActionKind::Choose,
                payload: Some("0".into()),
            });
            continue;
        }
        engine.step(SimAction {
            kind: SimActionKind::Train,
            payload: Some("speed".into()),
        });
        let has_positive = engine
            .export_telemetry_jsonl()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .any(|v| {
                v["decision"]["action"].as_str() == Some("TRAIN")
                    && v["post"]["main_gain"].as_i64().unwrap_or(0) > 0
            });
        if has_positive {
            break;
        }
        // Recover energy after a failed train so the next attempt can succeed.
        if engine.state().phase == TurnPhase::Free.as_str() {
            engine.step(SimAction {
                kind: SimActionKind::Rest,
                payload: None,
            });
        }
    }

    engine.step(SimAction {
        kind: SimActionKind::Rest,
        payload: None,
    });

    let lines: Vec<_> = engine
        .export_telemetry_jsonl()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("jsonl line"))
        .collect();
    assert!(
        lines.len() >= 2,
        "expected at least train+rest lines, got {}",
        lines.len()
    );

    let train_line = lines
        .iter()
        .find(|v| {
            v["decision"]["action"].as_str() == Some("TRAIN")
                && v["post"]["main_gain"].as_i64().unwrap_or(0) > 0
        })
        .expect("TRAIN telemetry line with positive main_gain");
    assert_eq!(train_line["type"].as_str(), Some("turn"));
    assert_eq!(
        train_line["decision"]["trainingStat"].as_str(),
        Some("speed")
    );
    assert!(train_line["pre"]
        .as_object()
        .map(|o| o.contains_key("facilityLevels"))
        .unwrap_or(false));
    let main_gain = train_line["post"]["main_gain"].as_i64();
    assert!(
        main_gain.map(|g| g > 0).unwrap_or(false),
        "training should record positive main_gain"
    );
}

#[test]
fn bot_run_produces_calibration_ready_training_records() {
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 50,
        trace_telemetry: true,
        ..Default::default()
    });
    engine.start(RunMeta::new(99, "ura", "Special Week"));
    engine.play_to_completion_scoring(80);

    let train_records = engine
        .export_telemetry_jsonl()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["decision"]["action"].as_str() == Some("TRAIN"))
        .count();

    assert!(
        train_records >= 5,
        "bot partial run should emit multiple TRAIN telemetry rows"
    );
}
