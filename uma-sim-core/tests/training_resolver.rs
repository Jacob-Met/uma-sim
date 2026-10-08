use uma_sim_core::{
    default_facility_levels, detect_repo_root, load_training_tables, CareerState, MoodLevel,
    RunMeta, SimDate, StatName, TraineeStats, TrainingFacility, TrainingResolver,
};

#[test]
fn power_training_secondary_is_stamina() {
    let resolver = TrainingResolver::default();
    assert_eq!(
        resolver.secondary_facility(TrainingFacility::Power),
        TrainingFacility::Stamina
    );
    assert_eq!(
        resolver.secondary_stat(TrainingFacility::Power),
        StatName::Stamina
    );
}

#[test]
fn tertiary_gain_is_twenty_percent_of_main() {
    let root = detect_repo_root().expect("repo root");
    let resolver = TrainingResolver::new(load_training_tables(Some(&root)));
    let state = CareerState {
        meta: RunMeta::new(1, "ura", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 5,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 100,
        max_energy: 100,
        mood: MoodLevel::Normal,
        fans: 0,
        skill_points: 0,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: "FREE".into(),
        completed_races: vec![],
        facility_levels: default_facility_levels(),
        facility_train_counts: Default::default(),
        pending_event_options: vec![],
        hint_levels: Default::default(),
        statuses: vec![],
        performance_tokens: Default::default(),
        scenario_resources: Default::default(),
        legacy: Default::default(),
        learned_skill_ids: vec![],
        deck: Default::default(),
        log: vec![],
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    };
    let outcome =
        resolver.resolve_typical(TrainingFacility::Speed, 3, MoodLevel::Normal, Some(&state));
    assert!(outcome.tertiary_gain > 0);
    assert_eq!(
        (outcome.main_gain as f64 * 0.2) as i32,
        outcome.tertiary_gain
    );
    assert_eq!(
        resolver.tertiary_facility(TrainingFacility::Speed),
        TrainingFacility::Guts
    );
}
