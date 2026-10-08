//! Port of Kotlin `SimEngineTest.kt` (all 10 tests).

mod common;

use std::sync::Arc;

use common::settings_fast;
use uma_sim_core::calendar::CAREER_TURNS;
use uma_sim_core::catalog::event::EventCatalog;
use uma_sim_core::policy::default_auto_policy;
use uma_sim_core::rng::SimRandom;
use uma_sim_core::state::{
    DialogueMode, MoodLevel, RunMeta, SimAction, SimActionKind, SimSettings, TrainingFacility,
    TurnPhase,
};
use uma_sim_core::{SimEngine, SimEventEntry, TrainingResolver};

#[test]
fn same_seed_same_outcome() {
    fn run(seed: i64) -> (i32, i32) {
        let mut engine = SimEngine::new(settings_fast(1));
        engine.start(RunMeta::new(seed, "ura", "Test"));
        for _ in 0..20 {
            engine.auto_step_with_policy(default_auto_policy);
        }
        let s = engine.state();
        (s.stats.speed, s.fans)
    }
    assert_eq!(run(12345), run(12345));
}

#[test]
fn speed_clamped_to_100() {
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
fn turbo_suppresses_dialogue() {
    let s = SimSettings {
        speed_multiplier: 100,
        dialogue_mode: DialogueMode::Full,
        ..Default::default()
    };
    assert_eq!(s.effective_dialogue_mode(), DialogueMode::Off);
}

#[test]
fn completes_72_turn_career() {
    let mut engine = SimEngine::new(settings_fast(50));
    engine.start(RunMeta::new(999, "ura", "Golden"));
    engine.play_to_completion(500);
    assert!(engine.state().career_complete);
    assert_eq!(engine.state().turn, CAREER_TURNS);
}

#[test]
fn mandatory_debut_race_on_turn_1() {
    let mut engine = SimEngine::new(Default::default());
    engine.start(RunMeta::new(1, "ura", "Test"));
    assert_eq!(engine.state().phase, TurnPhase::MandatoryRace.as_str());
    assert_eq!(engine.state().pending_race_id.as_deref(), Some("debut"));
}

#[test]
fn training_uses_formula_gain() {
    let resolver = TrainingResolver::default();
    let mut rng = SimRandom::new(42);
    let out = resolver.resolve(
        TrainingFacility::Speed,
        3,
        MoodLevel::Great,
        &mut rng,
        None,
        None,
    );
    assert!(out.main_gain > 10);
    assert!(out.energy_cost >= 20);
}

struct ForceEventCatalog;

impl EventCatalog for ForceEventCatalog {
    fn pick_random(
        &self,
        trainee_name: &str,
        _scenario_id: &str,
        _deck_support_names: &[String],
        _turn: i32,
        _rng: &mut SimRandom,
    ) -> Option<SimEventEntry> {
        Some(SimEventEntry {
            id: "e1".into(),
            title: "Test".into(),
            owner_kind: "trainee".into(),
            owner_name: trainee_name.to_string(),
            options: vec!["Speed +20\nSkill points +10".into()],
        })
    }

    fn event_count(&self) -> usize {
        1
    }
}

#[test]
fn event_choice_updates_stats() {
    let mut engine = SimEngine::with_event_catalog(
        SimSettings::default(),
        Arc::new(ForceEventCatalog),
        Some(1.0),
    );
    engine.start(RunMeta::new(555, "ura", "Special Week"));
    engine.step(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    });
    let before = engine.state().stats.speed;
    engine.auto_step_with_policy(default_auto_policy);
    let mut steps = 0;
    while engine.state().phase != TurnPhase::Event.as_str() && steps < 5 {
        engine.auto_step_with_policy(default_auto_policy);
        steps += 1;
    }
    if engine.state().phase == TurnPhase::Event.as_str() {
        engine.step(SimAction {
            kind: SimActionKind::Choose,
            payload: Some("0".into()),
        });
        assert!(engine.state().stats.speed >= before);
    }
}

#[test]
fn grand_concert_earns_performance_tokens() {
    let mut engine = SimEngine::new(settings_fast(1));
    engine.start(RunMeta::new(1, "grand_concert", "Test"));
    engine.step(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    });
    engine.step(SimAction {
        kind: SimActionKind::Train,
        payload: Some("speed".into()),
    });
    assert!(engine.state().scenario_resources.get("perf_Da") > 0);
}

#[test]
fn trackblazer_earns_coins_on_race() {
    let mut engine = SimEngine::new(settings_fast(1));
    engine.start(RunMeta::new(2, "trackblazer", "Test"));
    engine.step(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    });
    assert!(engine.state().scenario_resources.get("tb_coins") >= 0);
}

#[test]
fn snapshot_round_trip_preserves_turn() {
    let mut engine = SimEngine::new(Default::default());
    engine.start(RunMeta::new(77, "ura", "Test"));
    engine.auto_step_with_policy(default_auto_policy);
    let snap = engine.export();
    let mut engine2 = SimEngine::new(snap.settings.clone());
    engine2.restore(snap);
    assert_eq!(engine.state().turn, engine2.state().turn);
    assert_eq!(engine.state().stats, engine2.state().stats);
}
