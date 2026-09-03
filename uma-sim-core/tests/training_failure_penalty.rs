use uma_sim_core::{
    detect_repo_root, load_training_tables, MoodLevel, RunMeta, SimAction, SimActionKind, SimEngine,
    SimRandom, SimSettings, TrainingFacility, TrainingFailureConfig, TrainingResolver, INJURED,
    TurnPhase,
};

#[test]
fn certain_injury_chance_applies_injury_flag() {
    TrainingFailureConfig::load_from_json(Some(
        r#"{"failure_penalty":{"injury_chance":1.0,"mood_drop_chance":0.0,"energy_loss":10}}"#,
    ));
    let mut rng = SimRandom::new(0);
    let outcome = TrainingFailureConfig::resolve_failure(100, 20, 100, &mut rng);
    TrainingFailureConfig::reset_to_defaults();
    assert!(outcome.injured);
    assert_eq!(outcome.energy, 70);
    assert!(!outcome.mood_dropped);
}

fn engine_with_injury() -> SimEngine {
    let mut engine = SimEngine::new(SimSettings::default());
    engine.start(RunMeta::new(1, "ura", "Test"));
    let mut snap = engine.export();
    snap.state.statuses = vec![INJURED.to_string()];
    snap.state.phase = TurnPhase::Free.as_str().to_string();
    snap.state.awaiting_choice = false;
    snap.state.pending_event_title = None;
    snap.state.pending_event_options = vec![];
    engine.restore(snap);
    engine
}

#[test]
fn injured_trainee_blocked_from_training() {
    let mut engine = engine_with_injury();
    let result = engine.step(SimAction {
        kind: SimActionKind::Train,
        payload: Some("speed".into()),
    });
    assert!(result
        .text_lines
        .iter()
        .any(|l| l.to_lowercase().contains("injured")));
}

#[test]
fn rest_clears_injury() {
    let mut engine = engine_with_injury();
    engine.step(SimAction {
        kind: SimActionKind::Rest,
        payload: None,
    });
    assert!(!engine.state().is_injured());
}

#[test]
fn wit_training_restores_energy() {
    let root = detect_repo_root().expect("repo root");
    let resolver = TrainingResolver::new(load_training_tables(Some(&root)));
    let outcome =
        resolver.resolve_typical(TrainingFacility::Wit, 1, MoodLevel::Normal, None);
    assert!(
        outcome.energy_cost < 0,
        "Wit should restore energy (negative cost)"
    );
}
