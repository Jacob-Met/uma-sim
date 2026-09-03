use uma_sim_core::{
    default_facility_levels, BondGainConfig, CareerState, DeckPlacement, DeckSlot, DeckState,
    DeckTrainingSignals, MoodLevel, RunMeta, SimDate, TraineeStats, TrainingFacility,
};

fn slot(support_id: &str, bond: i32, specialty: &str) -> DeckSlot {
    DeckSlot {
        support_id: support_id.into(),
        bond,
        specialty: Some(specialty.into()),
        assigned_facility: None,
    }
}

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
fn training_increases_bond_on_facility_cards() {
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            slot("support:speed:special", 70, "speed"),
            slot("support:stamina:1", 70, "stamina"),
        ]),
    };
    let after = BondGainConfig::apply_training_bond(&deck, TrainingFacility::Speed);
    assert_eq!(after.slots[0].bond, 77);
    assert_eq!(after.slots[1].bond, 70);
}

#[test]
fn rainbow_count_requires_bond_on_specialty() {
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[slot(
            "support:speed:special",
            85,
            "speed",
        )]),
    };
    let state = ura_state(deck);
    assert_eq!(
        DeckTrainingSignals::num_rainbow(&state, TrainingFacility::Speed),
        1
    );
    assert_eq!(
        DeckTrainingSignals::num_rainbow(&state, TrainingFacility::Stamina),
        0
    );
    assert!(!DeckTrainingSignals::relationship_bars(&state, TrainingFacility::Speed).is_empty());
}
