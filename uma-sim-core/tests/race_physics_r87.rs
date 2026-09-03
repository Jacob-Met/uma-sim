//! R8.7 — physics race_model mid-run integration (stub default untouched).

use std::collections::HashSet;

use uma_sim_core::state::{
    DialogueMode, RunMeta, SimAction, SimActionKind, SimSettings, TurnPhase,
};
use uma_sim_core::{RaceModel, SimEngine};

#[test]
fn physics_debut_race_returns_real_order_not_always_first() {
    let mut engine = SimEngine::new(SimSettings {
        dialogue_mode: DialogueMode::ChoicesOnly,
        speed_multiplier: 1,
        race_model: RaceModel::Physics,
        ..Default::default()
    });
    engine.start(RunMeta::new(99, "ura", "Test"));
    assert_eq!(engine.state().phase, TurnPhase::MandatoryRace.as_str());
    assert_eq!(engine.state().pending_race_id.as_deref(), Some("debut"));

    engine.step(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    });

    let log = engine.state().log.join("\n");
    assert!(
        log.contains("physics t=") && log.contains("debut"),
        "log should record physics finish: {log}"
    );
    // Weak start stats vs PRE_OP placeholders → Show band (SP 20), not win (SP 45).
    assert_eq!(
        engine.state().skill_points,
        20,
        "expected show SP for non-1st physics place; log={log}"
    );
    assert!(
        !log.contains(" 1st "),
        "weak trainee should not place 1st under physics: {log}"
    );
}

#[test]
fn stub_default_debut_still_places_first() {
    let mut engine = SimEngine::new(SimSettings {
        dialogue_mode: DialogueMode::ChoicesOnly,
        speed_multiplier: 1,
        race_model: RaceModel::Stub,
        ..Default::default()
    });
    engine.start(RunMeta::new(99, "ura", "Test"));
    engine.step(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    });
    let log = engine.state().log.join("\n");
    assert!(
        log.contains("Race debut +"),
        "stub log format: {log}"
    );
    assert!(!log.contains("physics t="));
    assert_eq!(
        engine.state().skill_points,
        45,
        "stub win-by-default grants win SP"
    );
}

/// Full URA career under `race_model=physics` must complete and show non-trivial placements.
#[test]
fn physics_full_career_completes_with_varied_or_non_first_places() {
    let mut engine = SimEngine::new(SimSettings {
        dialogue_mode: DialogueMode::Off,
        speed_multiplier: 50,
        race_model: RaceModel::Physics,
        ..Default::default()
    });
    engine.start(RunMeta::new(42, "ura", "PhysicsCareer"));
    engine.play_to_completion(500);

    assert!(
        engine.state().career_complete,
        "physics career should complete without panic"
    );
    assert!(
        !engine.state().completed_races.is_empty(),
        "expected at least one completed race"
    );

    let mut places = HashSet::new();
    let mut saw_non_first = false;
    for line in &engine.state().log {
        if !line.starts_with("Race ") || !line.contains("physics t=") {
            continue;
        }
        // `Race {id} {place} +{fans} fans [physics …]`
        let place_tok = line.split_whitespace().nth(2).unwrap_or("");
        if place_tok.ends_with("st")
            || place_tok.ends_with("nd")
            || place_tok.ends_with("rd")
            || place_tok.ends_with("th")
        {
            places.insert(place_tok.to_string());
            if place_tok != "1st" {
                saw_non_first = true;
            }
        }
    }
    assert!(
        !places.is_empty(),
        "expected physics race log lines with places; log=\n{}",
        engine.state().log.join("\n")
    );
    assert!(
        saw_non_first || places.len() >= 2,
        "expected a non-First place or varied places across races; places={places:?}"
    );
}
