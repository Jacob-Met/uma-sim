use uma_sim_core::{
    default_facility_levels, CareerState, DeckPlacement, DeckSlot, DeckState, MoodLevel, RunMeta,
    SimDate, TraineeStats, TrainingFacility, TrainingFailureConfig, TrainingResolver,
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
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    }
}

#[test]
fn low_energy_increases_failure() {
    let low = TrainingFailureConfig::failure_chance_pct(20, 100, MoodLevel::Normal, 1);
    let high = TrainingFailureConfig::failure_chance_pct(95, 100, MoodLevel::Normal, 1);
    assert!(low > high);
}

#[test]
fn awful_mood_increases_failure() {
    let awful = TrainingFailureConfig::failure_chance_pct(80, 100, MoodLevel::Awful, 1);
    let great = TrainingFailureConfig::failure_chance_pct(80, 100, MoodLevel::Great, 1);
    assert!(awful > great);
}

#[test]
fn rainbow_activates_friendship_for_all_cards_on_facility() {
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            DeckSlot {
                support_id: "support:speed:special".into(),
                bond: 85,
                specialty: Some("speed".into()),
                assigned_facility: None,
            },
            DeckSlot {
                support_id: "support:friend:1".into(),
                bond: 85,
                specialty: Some("friend".into()),
                assigned_facility: None,
            },
        ]),
    };
    // Both assigned to speed + wit — move friend to speed for test
    let stacked = DeckState {
        slots: vec![
            deck.slots[0].clone(),
            DeckSlot {
                assigned_facility: Some("speed".into()),
                ..deck.slots[1].clone()
            },
        ],
    };
    let state = ura_state(stacked);
    let empty = TrainingResolver::default().resolve_typical(
        TrainingFacility::Speed,
        3,
        MoodLevel::Normal,
        Some(&CareerState {
            deck: DeckState::default(),
            ..state.clone()
        }),
    );
    let rainbow = TrainingResolver::default().resolve_typical(
        TrainingFacility::Speed,
        3,
        MoodLevel::Normal,
        Some(&state),
    );
    assert!(rainbow.main_gain > empty.main_gain);
}
