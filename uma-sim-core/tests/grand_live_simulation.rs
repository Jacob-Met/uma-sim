//! Port of Kotlin `GrandLiveSimulationTest.kt` (~23 tests).

mod common;

use common::{base_state, settings_fast, with_deck, with_resources};
use std::collections::HashMap;
use uma_sim_core::calendar::CAREER_TURNS;
use uma_sim_core::deck::DeckPlacement;
use uma_sim_core::scenario::grand_live::{
    GrandLiveMechanics, CYCLE_SONGS_AT_START, DEFAULT_GREAT_SUCCESS_REQUIRED,
    MAKE_DEBUT_GRANT_TURN, PERF_CODES, PERF_MAX_BASE,
};
use uma_sim_core::scenario::grand_live_deck_support::GrandLiveDeckSupport;
use uma_sim_core::scenario::grand_live_lesson_board::SLOTS;
use uma_sim_core::scenario::{GrandConcertScenarioPlugin, ScenarioPlugin};
use uma_sim_core::state::{
    DeckSlot, DeckState, RunMeta, ScenarioResources, SimDate, TrainingFacility, TurnPhase,
};
use uma_sim_core::{SimEngine, SimRandom};

fn slot(id: &str, specialty: &str) -> DeckSlot {
    DeckSlot {
        support_id: id.into(),
        bond: 0,
        specialty: Some(specialty.into()),
        assigned_facility: None,
    }
}

#[test]
fn career_starts_without_make_debut_until_grant_turn() {
    let plugin = GrandConcertScenarioPlugin;
    let init = plugin.initial_scenario_resources(&RunMeta::new(1, "grand_concert", "Test"));
    assert_eq!(init.get("songs_learned"), 0);
    assert!(!GrandLiveMechanics::owns_song(&init, 1));
    assert_eq!(init.get("hype"), CYCLE_SONGS_AT_START);
    assert_eq!(
        init.get("great_success_required"),
        DEFAULT_GREAT_SUCCESS_REQUIRED
    );
    assert!(!GrandLiveMechanics::is_hype_maxed(&init));
    assert_eq!(init.get("perf_Da"), 0);

    let mut state = base_state("grand_concert", MAKE_DEBUT_GRANT_TURN);
    state.phase = "FREE".into();
    state.scenario_resources = init;
    let (after, lines) = plugin.on_turn_start(&state);
    assert!(GrandLiveMechanics::owns_song(&after.scenario_resources, 1));
    assert_eq!(after.scenario_resources.get("songs_learned"), 1);
    assert_eq!(after.scenario_resources.get("hype"), 1);
    assert_eq!(after.scenario_resources.get("cycle_song:1"), 1);
    assert_eq!(after.scenario_resources.get("perf_Da"), 10);
    assert!(lines.iter().any(|l| l.contains("Make Debut!")));
}

#[test]
fn debut_race_preserves_make_debut_cycle_hype() {
    let plugin = GrandConcertScenarioPlugin;
    let mut state = base_state("grand_concert", 12);
    state.date = SimDate {
        year: 1,
        month: 6,
        half: 2,
    };
    let (granted, _) =
        GrandLiveMechanics::grant_make_debut_song(&GrandLiveMechanics::initial_resources());
    state.scenario_resources = granted;
    assert_eq!(state.scenario_resources.get("hype"), 1);
    let (after, _) = plugin.on_race_complete(&state, "debut", true);
    assert_eq!(after.scenario_resources.get("hype"), 1);
    assert_eq!(after.scenario_resources.get("concert_index"), 0);
    assert!(GrandLiveMechanics::owns_song(&after.scenario_resources, 1));
}

#[test]
fn training_applies_603010_token_split() {
    let plugin = GrandConcertScenarioPlugin;
    let mut state = base_state("grand_concert", 5);
    state.facility_levels = HashMap::from([
        ("speed".into(), 3),
        ("stamina".into(), 1),
        ("power".into(), 1),
        ("guts".into(), 1),
        ("wit".into(), 1),
    ]);
    state = with_deck(
        state,
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[slot("s1", "speed"), slot("s2", "speed")]),
        },
    );
    let (after, lines) = plugin.on_training_complete(&state, TrainingFacility::Speed, true);
    assert!(after.scenario_resources.get("perf_Da") > 0);
    assert!(after.scenario_resources.get("perf_Pa") > 0);
    assert!(after.scenario_resources.get("perf_Vo") > 0);
    assert!(lines.iter().any(|l| l.contains("Da")));
}

#[test]
fn performance_tokens_cap_at_200() {
    let res = ScenarioResources::from_map(HashMap::from([("perf_Da".into(), 195)]));
    let capped = GrandLiveMechanics::add_perf_tokens(&res, &HashMap::from([("Da".into(), 20)]));
    assert_eq!(capped.get("perf_Da"), PERF_MAX_BASE);
}

#[test]
fn soft_cap_halves_gains_above_1200() {
    assert_eq!(GrandLiveMechanics::soft_cap_gain(1210, 20), 10);
    assert_eq!(GrandLiveMechanics::soft_cap_gain(1190, 20), 15);
}

#[test]
fn promo_concert_mandatory_at_turn_24() {
    let plugin = GrandConcertScenarioPlugin;
    let mut state = base_state("grand_concert", 24);
    state.date = SimDate {
        year: 2,
        month: 6,
        half: 2,
    };
    let (next, lines) = plugin.on_turn_start(&state);
    assert_eq!(next.phase, TurnPhase::MandatoryRace.as_str());
    assert_eq!(next.pending_race_id.as_deref(), Some("promo_1"));
    assert!(lines.iter().any(|l| l.contains("Promo Concert 1")));
}

#[test]
fn all_six_concert_turns_mapped() {
    assert_eq!(GrandLiveMechanics::concert_race_id(1), Some("debut"));
    assert_eq!(GrandLiveMechanics::concert_race_id(24), Some("promo_1"));
    assert_eq!(GrandLiveMechanics::concert_race_id(36), Some("promo_2"));
    assert_eq!(GrandLiveMechanics::concert_race_id(48), Some("promo_3"));
    assert_eq!(GrandLiveMechanics::concert_race_id(60), Some("promo_4"));
    assert_eq!(
        GrandLiveMechanics::concert_race_id(72),
        Some("grand_concert")
    );
}

#[test]
fn song_purchase_increments_cycle_hype() {
    let plugin = GrandConcertScenarioPlugin;
    let state = with_resources(
        base_state("grand_concert", 10),
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
            ("techniques_learned".into(), 4),
            ("techniques_since_last_song".into(), 1),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 0),
            ("perf_Vo".into(), 50),
            ("perf_Me".into(), 50),
        ]),
    );
    let result = plugin.apply_side_action(&state, "gl_song_3");
    assert!(result.is_some());
    let (after, _) = result.unwrap();
    assert_eq!(after.scenario_resources.get("songs_learned"), 2);
    assert_eq!(after.scenario_resources.get("hype"), 1);
}

#[test]
fn is_hype_maxed_when_cycle_meets_required() {
    let res = ScenarioResources::from_map(HashMap::from([
        ("hype".into(), 3),
        ("great_success_required".into(), 3),
    ]));
    assert!(GrandLiveMechanics::is_hype_maxed(&res));
    assert!(!GrandLiveMechanics::is_hype_maxed(&res.set("hype", 2)));
}

#[test]
fn great_success_grand_concert_with_18_songs() {
    let plugin = GrandConcertScenarioPlugin;
    let mut values: HashMap<String, i32> =
        (1..=18).map(|i| (format!("song_owned:{i}"), 1)).collect();
    values.insert("songs_learned".into(), 18);
    values.insert("hype".into(), 3);
    values.insert("great_success_required".into(), 3);
    values.insert("cycle_song:18".into(), 1);
    values.insert("cycle_song:17".into(), 1);
    values.insert("cycle_song:16".into(), 1);
    let mut state = with_resources(base_state("grand_concert", 72), values);
    state.date = SimDate {
        year: 3,
        month: 12,
        half: 2,
    };
    state.fans = 5000;
    let (after, lines) = plugin.on_race_complete(&state, "grand_concert", true);
    assert!(lines.iter().any(|l| l.contains("Great Success")));
    assert_eq!(after.scenario_resources.get("grand_live_perfect"), 1);
    assert!(GrandLiveMechanics::owns_song(&after.scenario_resources, 24));
}

#[test]
fn debut_grants_no_tokens_make_debut_song_already_granted() {
    let plugin = GrandConcertScenarioPlugin;
    let mut state = base_state("grand_concert", 12);
    let (granted, _) =
        GrandLiveMechanics::grant_make_debut_song(&GrandLiveMechanics::initial_resources());
    state.scenario_resources = granted;
    let before_da = state.scenario_resources.get("perf_Da");
    let (after, lines) = plugin.on_race_complete(&state, "debut", true);
    assert_eq!(after.scenario_resources.get("perf_Da"), before_da);
    assert!(lines.iter().any(|l| l.contains("Make Debut race")));
}

#[test]
fn lesson_board_has_at_most_three_slots() {
    let plugin = GrandConcertScenarioPlugin;
    let mut state = base_state("grand_concert", 10);
    state.meta.seed = 42;
    state = with_resources(
        state,
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("techniques_learned".into(), 4),
            ("techniques_since_last_song".into(), 1),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 1),
            ("perf_Vo".into(), 100),
            ("perf_Me".into(), 100),
            ("perf_Da".into(), 100),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
        ]),
    );
    assert!(plugin.extra_choices(&state).len() <= SLOTS);
}

#[test]
fn cycle_max_blocks_song_when_full() {
    let plugin = GrandConcertScenarioPlugin;
    let state = with_resources(
        base_state("grand_concert", 10),
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("hype".into(), 4),
            ("great_success_required".into(), 3),
            ("perf_Vo".into(), 50),
            ("perf_Me".into(), 50),
        ]),
    );
    assert!(plugin.apply_side_action(&state, "gl_song_3").is_none());
}

#[test]
fn friendship_bonus_increases_training_multiplier() {
    let res = ScenarioResources::from_map(HashMap::from([("bonus_friendship_pct".into(), 10)]));
    assert!((GrandLiveMechanics::training_stat_multiplier(&res) - 1.1).abs() < 1e-9);
}

#[test]
fn part_four_songs_unlock_in_senior_dec_late() {
    let mut state = base_state("grand_concert", 70);
    state.date = SimDate {
        year: 3,
        month: 12,
        half: 2,
    };
    assert_eq!(GrandLiveMechanics::career_part(&state), 4);
}

#[test]
fn full_career_completes_with_engine() {
    let mut engine = SimEngine::new(settings_fast(50));
    engine.start(RunMeta::new(999, "grand_concert", "GL Test"));
    assert_eq!(engine.state().scenario_resources.get("songs_learned"), 0);
    engine.play_to_completion_scoring(500);
    assert!(engine.state().career_complete);
    assert!(engine.state().turn >= CAREER_TURNS);
    // Make Debut! should have been granted during the career.
    assert!(
        engine.state().scenario_resources.get("songs_learned") >= 1,
        "expected Make Debut grant during career"
    );
}

#[test]
fn uma_guide_performance_formula() {
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Speed, 1, 0, 0),
        10
    );
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Speed, 1, 2, 0),
        13
    );
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Speed, 1, 2, 1),
        15
    );
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Wit, 1, 0, 0),
        6
    );
    let split = GrandLiveMechanics::split_token_total(10, TrainingFacility::Speed);
    assert_eq!(split.get("Da").copied().unwrap_or(0), 6);
    assert_eq!(split.get("Pa").copied().unwrap_or(0), 3);
    assert_eq!(split.get("Vo").copied().unwrap_or(0), 1);
    // Largest remainder: 13 × 60/30/10 → 8/4/1 (calibration row), not 7/3/3.
    let split13 = GrandLiveMechanics::split_token_total(13, TrainingFacility::Speed);
    assert_eq!(split13.get("Da").copied().unwrap_or(0), 8);
    assert_eq!(split13.get("Pa").copied().unwrap_or(0), 4);
    assert_eq!(split13.get("Vo").copied().unwrap_or(0), 1);
}

#[test]
fn technique_gate_blocks_song_purchase() {
    let plugin = GrandConcertScenarioPlugin;
    let state = with_resources(
        base_state("grand_concert", 10),
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
            ("techniques_learned".into(), 0),
            ("techniques_since_last_song".into(), 0),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 0),
            ("perf_Vo".into(), 50),
            ("perf_Me".into(), 50),
        ]),
    );
    assert!(plugin.apply_side_action(&state, "gl_song_3").is_none());
    assert_eq!(
        GrandLiveMechanics::techniques_required_for_next_song(&state.scenario_resources),
        1
    );
}

#[test]
fn promo_concert_grants_stats_sp_and_raises_perf_cap() {
    let plugin = GrandConcertScenarioPlugin;
    let state = with_resources(
        {
            let mut s = base_state("grand_concert", 24);
            s.date = SimDate {
                year: 2,
                month: 6,
                half: 2,
            };
            s
        },
        HashMap::from([
            ("hype".into(), 3),
            ("great_success_required".into(), 3),
            ("cycle_techniques".into(), 2),
            ("concert_index".into(), 1),
        ]),
    );
    let (after, lines) = plugin.on_race_complete(&state, "promo_1", true);
    assert_eq!(after.stats.speed, 110);
    assert_eq!(after.skill_points, 85);
    assert_eq!(GrandLiveMechanics::perf_max(&after.scenario_resources), 250);
    assert!(lines.iter().any(|l| l.contains("Great Success")));
}

#[test]
fn make_debut_song_grants_all_performance_tokens() {
    let (after, lines) =
        GrandLiveMechanics::grant_make_debut_song(&GrandLiveMechanics::initial_resources());
    for code in PERF_CODES {
        assert_eq!(after.get(&GrandLiveMechanics::perf_resource_key(code)), 10);
    }
    assert!(lines.iter().any(|l| l.contains("performance tokens +10")));
}

#[test]
fn light_hello_grants_least_owned_on_proc() {
    let res = ScenarioResources::from_map(HashMap::from([
        ("perf_Da".into(), 50),
        ("perf_Pa".into(), 10),
        ("perf_Vo".into(), 11),
        ("perf_Vi".into(), 11),
        ("perf_Me".into(), 11),
    ]));
    assert_eq!(GrandLiveMechanics::least_owned_perf_code(&res), "Pa");
    let state = with_deck(
        {
            let mut s = with_resources(base_state("grand_concert", 5), res.values.clone());
            s.meta.seed = 99;
            s
        },
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[slot("support:30052", "friend")]),
        },
    );
    let mut found = false;
    for seed in 0..=500 {
        let mut rng = SimRandom::new(seed);
        let (gains, lines) = GrandLiveMechanics::roll_light_hello(
            &state.scenario_resources,
            &state,
            TrainingFacility::Wit,
            &mut rng,
        );
        if gains.is_empty() {
            continue;
        }
        found = true;
        assert_eq!(gains.get("Pa").copied(), Some(20));
        assert!(lines.iter().any(|l| l.contains("Light Hello")));
        break;
    }
    assert!(
        found,
        "expected at least one seed to proc Light Hello in 500 tries"
    );
}

#[test]
fn friendship_training_biases_secondary_to_least_owned() {
    let res = ScenarioResources::from_map(HashMap::from([
        ("bonus_friendship_pct".into(), 5),
        ("perf_Da".into(), 40),
        ("perf_Pa".into(), 20),
        ("perf_Vo".into(), 10),
        ("perf_Vi".into(), 10),
        ("perf_Me".into(), 10),
    ]));
    let raw = HashMap::from([("Da".into(), 6), ("Pa".into(), 3), ("Vo".into(), 1)]);
    let biased =
        GrandLiveMechanics::apply_friendship_training_bias(&raw, &res, TrainingFacility::Speed);
    assert_eq!(biased.get("Da").copied().unwrap_or(0), 6);
    assert_eq!(biased.get("Vo").copied().unwrap_or(0), 4);
    assert!(!biased.contains_key("Pa"));
}

#[test]
fn scenario_links_increase_formula_total() {
    let deck = DeckState {
        slots: DeckPlacement::assign_by_specialty(&[
            slot("support:30052", "friend"),
            slot("smart_falcon", "speed"),
        ]),
    };
    let state = with_deck(base_state("grand_concert", 5), deck);
    assert_eq!(
        GrandLiveDeckSupport::scenario_link_count(&state, TrainingFacility::Wit),
        1
    );
    assert_eq!(
        GrandLiveDeckSupport::scenario_link_count(&state, TrainingFacility::Speed),
        1
    );
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Speed, 1, 1, 1),
        13
    );
    assert_eq!(
        GrandLiveMechanics::performance_token_total(TrainingFacility::Wit, 1, 1, 1),
        8
    );
}

#[test]
fn days_to_concert_matches_scoring_shared() {
    assert_eq!(GrandLiveMechanics::days_to_concert(24), 0);
    assert_eq!(GrandLiveMechanics::days_to_concert(22), 2);
}
