use uma_sim_core::{
    default_facility_levels, CareerState, DeckSlot, DeckState, EventProbabilityConfig, MoodLevel,
    RunMeta, SimDate, SimRandom, TraineeStats,
};

fn empty_career() -> CareerState {
    CareerState {
        meta: RunMeta::new(1, "ura", "T"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 1,
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
    }
}

#[test]
fn deck_raises_event_chance() {
    let empty = empty_career();
    let decked = CareerState {
        deck: DeckState {
            slots: vec![DeckSlot {
                support_id: "support:10001".into(),
                bond: 80,
                specialty: None,
                assigned_facility: None,
            }],
        },
        ..empty.clone()
    };
    assert!(
        EventProbabilityConfig::event_chance_for(&decked)
            > EventProbabilityConfig::event_chance_for(&empty)
    );
}

#[test]
fn energy_variance_picks_from_outcomes() {
    let mut rng = SimRandom::new(999);
    let deltas: std::collections::HashSet<i32> = (0..20)
        .map(|_| EventProbabilityConfig::pick_energy_variance(&mut rng))
        .collect();
    assert!(deltas.iter().all(|&d| d == -5 || d == -20));
}

#[test]
fn matches_energy_variance_pattern() {
    assert!(EventProbabilityConfig::matches_energy_variance(
        "Bad luck\nEnergy -5/-20"
    ));
}
