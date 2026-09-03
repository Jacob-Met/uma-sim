//! Port of TraineeCatalogTest.kt

use uma_sim_core::{
    detect_repo_root, load_training_tables, CareerState, DeckPlacement, DeckSlot, DeckState,
    MoodLevel, RunMeta, SimDate, TraineeCatalog, TraineeStats, TrainingFacility,
    TrainingGainContext, TrainingResolver,
};

#[test]
fn loads_special_week_growth_from_kb() {
    let root = detect_repo_root().expect("repo root");
    TraineeCatalog::init_from_repo(Some(&root));
    let meta = TraineeCatalog::lookup("Special Week")
        .or_else(|| TraineeCatalog::lookup("Special Dreamer"))
        .expect("Expected Special Week trainee card with stat_bonus");
    assert_eq!(meta.growth_bonus_pct[1], 20);
}

#[test]
fn growth_pct_boosts_training_gain() {
    let root = detect_repo_root().unwrap();
    TraineeCatalog::init_from_repo(Some(&root));
    TrainingGainContext::set_trainee_growth_lookup(Some(TraineeCatalog::growth_pct));
    let tables = load_training_tables(Some(&root));
    let resolver = TrainingResolver::new(tables);
    let base_state = CareerState {
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
        completed_races: Vec::new(),
        facility_levels: uma_sim_core::default_facility_levels(),
        facility_train_counts: Default::default(),
        pending_event_options: Vec::new(),
        hint_levels: Default::default(),
        statuses: Vec::new(),
        performance_tokens: Default::default(),
        scenario_resources: Default::default(),
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: DeckState {
            slots: DeckPlacement::assign_by_specialty(&[DeckSlot {
                support_id: "support:10001".into(),
                bond: 85,
                specialty: Some("guts".into()),
                assigned_facility: None,
            }]),
        },
        log: Vec::new(),
    };
    let mut sw_state = base_state.clone();
    sw_state.meta.trainee_name = "Special Week".into();
    let test_gain = resolver
        .resolve_typical(TrainingFacility::Stamina, 3, MoodLevel::Normal, Some(&base_state))
        .main_gain;
    let sw_gain = resolver
        .resolve_typical(TrainingFacility::Stamina, 3, MoodLevel::Normal, Some(&sw_state))
        .main_gain;
    assert!(
        sw_gain > test_gain,
        "Special Week +20% stamina growth should beat generic trainee"
    );
}
