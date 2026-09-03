use uma_sim_core::{
    default_facility_levels, CareerState, DeckPlacement, DeckSlot, DeckState, MoodLevel, RunMeta,
    SimDate, TraineeStats, TrainingFacility, TrainingResolver,
};

fn ura_state(deck: DeckState) -> CareerState {
    CareerState {
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
        deck,
        log: vec![],
    }
}

#[test]
fn deck_increases_training_gain_vs_empty_deck() {
    let resolver = TrainingResolver::default();
    let mood = MoodLevel::Normal;
    let empty = ura_state(DeckState::default());
    let decked = ura_state(DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            DeckSlot {
                support_id: "support:speed:special".into(),
                bond: 85,
                specialty: Some("speed".into()),
                assigned_facility: None,
            },
            DeckSlot {
                support_id: "support:friend:1".into(),
                bond: 80,
                specialty: Some("friend".into()),
                assigned_facility: None,
            },
        ]),
    });
    let gain_empty = resolver
        .resolve_typical(TrainingFacility::Speed, 3, mood, Some(&empty))
        .main_gain;
    let gain_deck = resolver
        .resolve_typical(TrainingFacility::Speed, 3, mood, Some(&decked))
        .main_gain;
    assert!(gain_deck >= gain_empty);
}
