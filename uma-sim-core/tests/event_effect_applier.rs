use uma_sim_core::{
    CareerState, EventEffectApplier, MoodLevel, RunMeta, SimDate, SimRandom, TraineeStats,
};

fn base_state(speed: i32, energy: i32, skill_points: i32) -> CareerState {
    CareerState {
        meta: RunMeta::new(1, "ura", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 2,
        stats: TraineeStats {
            speed,
            ..Default::default()
        },
        energy,
        max_energy: 100,
        mood: MoodLevel::Normal,
        fans: 0,
        skill_points,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: "FREE".into(),
        completed_races: vec![],
        facility_levels: uma_sim_core::default_facility_levels(),
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
    }
}

#[test]
fn applies_stat_and_energy() {
    let state = base_state(100, 50, 0);
    let mut rng = SimRandom::new(42);
    let (next, lines) = EventEffectApplier::apply(&state, "Energy +30\nSpeed +10", &mut rng);
    assert_eq!(next.energy, 80);
    assert_eq!(next.stats.speed, 110);
    assert!(!lines.is_empty());
}

#[test]
fn applies_skill_points_and_hints() {
    let state = base_state(0, 50, 10);
    let mut rng = SimRandom::new(1);
    let (next, _) =
        EventEffectApplier::apply(&state, "Skill points +45\nHydrate hint +1", &mut rng);
    assert_eq!(next.skill_points, 55);
    assert!(next.hint_levels.contains_key("Hydrate"));
}
