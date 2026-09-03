//! Port of DeckPlacementTest.kt + DeckSupportBridgeTest.kt

mod common;

use common::{base_state, with_deck};
use uma_sim_core::deck::{DeckPlacement, DeckSpec};
use uma_sim_core::scenario::grand_live_deck_support::GrandLiveDeckSupport;
use uma_sim_core::state::{DeckSlot, DeckState, MoodLevel, RunMeta, TrainingFacility};
use uma_sim_core::{SimEngine, TrainingResolver};

fn slot(id: &str, bond: i32, specialty: &str) -> DeckSlot {
    DeckSlot {
        support_id: id.into(),
        bond,
        specialty: Some(specialty.into()),
        assigned_facility: None,
    }
}

#[test]
fn assigns_cards_to_specialty_facilities() {
    let placed = DeckPlacement::assign_by_specialty(&[
        slot("support:speed:special", 0, "speed"),
        slot("support:stamina:1", 0, "stamina"),
        slot("support:friend:1", 0, "friend"),
    ]);
    assert_eq!(placed[0].assigned_facility.as_deref(), Some("speed"));
    assert_eq!(placed[1].assigned_facility.as_deref(), Some("stamina"));
    assert_eq!(placed[2].assigned_facility.as_deref(), Some("wit"));
}

#[test]
fn training_gain_uses_only_cards_on_facility() {
    let resolver = TrainingResolver::default();
    let mood = MoodLevel::Normal;
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            slot("support:speed:special", 85, "speed"),
            slot("support:stamina:1", 85, "stamina"),
        ]),
    };
    let state = with_deck(base_state("ura", 5), deck.clone());
    let speed_gain = resolver
        .resolve_typical(TrainingFacility::Speed, 3, mood, Some(&state))
        .main_gain;
    let stamina_gain = resolver
        .resolve_typical(TrainingFacility::Stamina, 3, mood, Some(&state))
        .main_gain;
    assert!(speed_gain > 0);
    assert!(stamina_gain > 0);
    assert_eq!(deck.count_on_facility(TrainingFacility::Speed), 1);
    assert_eq!(deck.count_on_facility(TrainingFacility::Stamina), 1);
    assert_eq!(deck.count_on_facility(TrainingFacility::Power), 0);
}

#[test]
fn grand_live_scenario_links_only_on_trained_facility() {
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            slot("support:30052", 0, "friend"),
            slot("support:speed:special", 0, "speed"),
        ]),
    };
    let state = with_deck(base_state("grand_concert", 5), deck);
    assert_eq!(
        GrandLiveDeckSupport::scenario_link_count(&state, TrainingFacility::Wit),
        1
    );
    assert_eq!(
        GrandLiveDeckSupport::scenario_link_count(&state, TrainingFacility::Speed),
        0
    );
    assert!(GrandLiveDeckSupport::has_light_hello(&state, TrainingFacility::Wit));
    assert!(!GrandLiveDeckSupport::has_light_hello(&state, TrainingFacility::Speed));
}

#[test]
fn deck_spec_parses_manual_placement() {
    let spec = DeckSpec::parse("support:10001@speed:85");
    assert_eq!(spec.support_id, "support:10001");
    assert_eq!(spec.facility.as_deref(), Some("speed"));
    assert_eq!(spec.bond, 85);
}

#[test]
fn manual_placement_stacks_cards_on_facility() {
    let slots = DeckPlacement::build_from_specs(&[
        "support:speed:special@speed".into(),
        "support:stamina:1@speed".into(),
    ]);
    assert_eq!(
        slots
            .iter()
            .filter(|s| s.assigned_facility.as_deref() == Some("speed"))
            .count(),
        2
    );
    let deck = DeckState { slots };
    assert_eq!(deck.count_on_facility(TrainingFacility::Speed), 2);
}

#[test]
fn runtime_reassign_moves_card() {
    let mut engine = SimEngine::new(Default::default());
    let mut meta = RunMeta::new(1, "ura", "Test");
    meta.deck_supports = vec!["support:speed:special".into(), "support:stamina:1".into()];
    engine.start(meta);
    assert!(engine.assign_deck_slot("support:stamina:1", TrainingFacility::Speed));
    assert_eq!(
        engine.state().deck.count_on_facility(TrainingFacility::Speed),
        2
    );
}

#[test]
fn deck_increases_training_gain_vs_empty_deck() {
    let resolver = TrainingResolver::default();
    let mood = MoodLevel::Normal;
    let empty = base_state("ura", 5);
    let decked = with_deck(
        empty.clone(),
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[
                slot("support:speed:special", 85, "speed"),
                slot("support:friend:1", 80, "friend"),
            ]),
        },
    );
    let gain_empty = resolver
        .resolve_typical(TrainingFacility::Speed, 3, mood, Some(&empty))
        .main_gain;
    let gain_deck = resolver
        .resolve_typical(TrainingFacility::Speed, 3, mood, Some(&decked))
        .main_gain;
    assert!(gain_deck >= gain_empty);
}
