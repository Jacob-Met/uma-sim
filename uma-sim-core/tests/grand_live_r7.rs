//! R7.2 Grand Live fidelity — concert outcomes, blocked perf, reserve, fan-scaled unique.

mod common;

use common::{base_state, with_deck, with_resources};
use std::collections::HashMap;
use uma_sim_core::deck::DeckPlacement;
use uma_sim_core::state::{DeckSlot, DeckState};
use uma_sim_core::{
    ConcertOutcome, GrandLiveDeckSupport, GrandLiveLessonBoard, GrandLiveMechanics,
    ScenarioResources, TrainingFacility,
};

#[test]
fn concert_outcome_great_success_normal_and_failure() {
    let mut gs = ScenarioResources::new();
    gs = gs
        .set("hype", 3)
        .set("cycle_song:3", 1)
        .set("cycle_song:4", 1)
        .set("cycle_song:5", 1);
    assert_eq!(
        GrandLiveMechanics::concert_outcome("promo_1", &gs),
        ConcertOutcome::GreatSuccess
    );

    let normal = ScenarioResources::new()
        .set("hype", 2)
        .set("cycle_song:3", 1)
        .set("cycle_song:4", 1);
    assert_eq!(
        GrandLiveMechanics::concert_outcome("promo_1", &normal),
        ConcertOutcome::Normal
    );

    assert_eq!(
        GrandLiveMechanics::concert_outcome_with_race("promo_1", &gs, false),
        ConcertOutcome::Failure
    );
    let forced = ScenarioResources::new()
        .set("force_concert_fail", 1)
        .set("hype", 3);
    assert_eq!(
        GrandLiveMechanics::concert_outcome("promo_1", &forced),
        ConcertOutcome::Failure
    );
}

#[test]
fn blocked_performance_types_zero_token_gains() {
    let mut res = ScenarioResources::new();
    res = GrandLiveMechanics::set_blocked_performance(&res, "Da", true);
    let gains =
        GrandLiveMechanics::training_token_gain(TrainingFacility::Speed, 3, 0, 0, Some(&res));
    assert!(!gains.contains_key("Da") || gains.get("Da") == Some(&0));
    assert!(gains.get("Pa").copied().unwrap_or(0) > 0 || gains.get("Vo").copied().unwrap_or(0) > 0);
}

#[test]
fn fan_scaled_unique_skill_power_multiply_fan_count() {
    assert_eq!(GrandLiveMechanics::unique_skill_power(0), 800);
    assert_eq!(GrandLiveMechanics::unique_skill_power(19_999), 800);
    assert_eq!(GrandLiveMechanics::unique_skill_power(20_000), 900);
    assert_eq!(GrandLiveMechanics::unique_skill_power(50_000), 1000);
    assert_eq!(GrandLiveMechanics::unique_skill_power(100_000), 1100);
    assert_eq!(GrandLiveMechanics::unique_skill_power(160_000), 1200);
    // Gold unique base velocity 3500 × 0.8 at low fans
    assert_eq!(GrandLiveMechanics::unique_skill_velocity(3500, 0), 2800);
    assert_eq!(
        GrandLiveMechanics::unique_skill_velocity(3500, 160_000),
        4200
    );
}

#[test]
fn reserve_square_id_prefers_board_slot() {
    let mut state = base_state("grand_concert", 10);
    state.phase = "FREE".into();
    state.scenario_resources = GrandLiveMechanics::initial_resources();
    state.scenario_resources =
        GrandLiveMechanics::set_reserve_square_id(&state.scenario_resources, 1);
    assert_eq!(
        GrandLiveMechanics::reserve_square_id(&state.scenario_resources),
        Some("1".into())
    );
    let _slots = GrandLiveLessonBoard::current_slots(&state);
}

#[test]
fn member_ready_count_tracks_deck_size() {
    let state = base_state("grand_concert", 5);
    assert_eq!(GrandLiveMechanics::member_ready_count(&state), 0);
    let with = with_deck(
        base_state("grand_concert", 5),
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[DeckSlot {
                support_id: "smart_falcon".into(),
                bond: 0,
                specialty: Some("speed".into()),
                assigned_facility: None,
            }]),
        },
    );
    assert_eq!(GrandLiveMechanics::member_ready_count(&with), 1);
    assert!(GrandLiveDeckSupport::any_scenario_link_in_deck(&with));
}

#[test]
fn consolation_unique_path_sets_power() {
    let mut values = HashMap::new();
    values.insert("songs_learned".into(), 10);
    values.insert("hype".into(), 3);
    values.insert("cycle_song:2".into(), 1);
    values.insert("cycle_song:3".into(), 1);
    values.insert("cycle_song:4".into(), 1);
    for c in ["Da", "Pa", "Vo", "Vi", "Me"] {
        values.insert(format!("perf_{c}"), 100);
    }
    let mut state = with_resources(base_state("grand_concert", 72), values);
    state.fans = 25_000;
    state.pending_race_id = Some("grand_concert".into());
    let plugin = uma_sim_core::scenario_plugin_for("grand_concert");
    let (after, lines) = plugin.on_race_complete(&state, "grand_concert", true);
    assert!(lines
        .iter()
        .any(|l| l.contains("consolation") || l.contains("Dream")));
    assert_eq!(
        after.scenario_resources.get("unique_skill_power"),
        GrandLiveMechanics::unique_skill_power(25_000)
    );
    assert_eq!(
        after.scenario_resources.get("last_live_result"),
        ConcertOutcome::GreatSuccess.as_resource_value()
    );
}

#[test]
fn normal_concert_path_without_great_success() {
    let state = with_resources(
        base_state("grand_concert", 24),
        HashMap::from([
            ("hype".into(), 2),
            ("great_success_required".into(), 3),
            ("cycle_techniques".into(), 1),
            ("concert_index".into(), 1),
            ("perf_Da".into(), 50),
        ]),
    );
    let plugin = uma_sim_core::scenario_plugin_for("grand_concert");
    let (after, lines) = plugin.on_race_complete(&state, "promo_1", true);
    assert!(lines.iter().any(|l| l.contains("Concert complete")));
    assert!(!lines.iter().any(|l| l.contains("Great Success")));
    assert_eq!(
        after.scenario_resources.get("last_live_result"),
        ConcertOutcome::Normal.as_resource_value()
    );
    assert_eq!(after.stats.speed, 103); // +3 normal bonus
}

#[test]
fn concert_failure_on_race_loss() {
    let state = with_resources(
        base_state("grand_concert", 24),
        HashMap::from([
            ("hype".into(), 3),
            ("great_success_required".into(), 3),
            ("cycle_techniques".into(), 2),
            ("concert_index".into(), 1),
        ]),
    );
    let plugin = uma_sim_core::scenario_plugin_for("grand_concert");
    let (after, lines) = plugin.on_race_complete(&state, "promo_1", false);
    assert!(lines
        .iter()
        .any(|l| l.contains("FAILED") || l.contains("failure")));
    assert_eq!(
        after.scenario_resources.get("last_live_result"),
        ConcertOutcome::Failure.as_resource_value()
    );
}

#[test]
fn lesson_board_three_slots_documents_next_square_approximation() {
    // Board is a deterministic 3-slot pool (MDB square weights still approximate).
    let mut state = base_state("grand_concert", 10);
    state.meta.seed = 42;
    state.phase = "FREE".into();
    state = with_resources(
        state,
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("techniques_learned".into(), 4),
            ("techniques_since_last_song".into(), 2),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 1),
            ("perf_Vo".into(), 100),
            ("perf_Me".into(), 100),
            ("perf_Da".into(), 100),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
        ]),
    );
    let slots = GrandLiveLessonBoard::current_slots(&state);
    assert!(slots.len() <= 3);
    assert!(slots
        .iter()
        .any(|s| s.square_type >= 1 && s.square_type <= 4));
}

#[test]
fn lesson_board_includes_unaffordable_and_song_when_unlocked() {
    let mut state = base_state("grand_concert", 10);
    state.meta.seed = 7;
    state.phase = "FREE".into();
    // Gate unlocked, but almost no PP — songs/techs still appear with affordable=false.
    state = with_resources(
        state,
        HashMap::from([
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("techniques_learned".into(), 4),
            ("techniques_since_last_song".into(), 4),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 1),
            ("perf_Vo".into(), 0),
            ("perf_Me".into(), 0),
            ("perf_Da".into(), 0),
            ("perf_Pa".into(), 0),
            ("perf_Vi".into(), 0),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
        ]),
    );
    let slots = GrandLiveLessonBoard::current_slots(&state);
    assert!(!slots.is_empty());
    assert!(
        slots.iter().any(|s| s.is_song),
        "expected a song slot when gate unlocked"
    );
    assert!(
        slots.iter().any(|s| !s.affordable),
        "board should surface unaffordable lessons like the live API"
    );
}

#[test]
fn concert_outcome_respects_race_result_flag() {
    // Concert economy still keys off race win/lose; mid-run placement comes from uma-race-core (R8).
    let outcome = GrandLiveMechanics::concert_outcome_with_race(
        "promo_1",
        &ScenarioResources::new().set("hype", 3),
        true,
    );
    assert_eq!(outcome, ConcertOutcome::GreatSuccess);
    let loss = GrandLiveMechanics::concert_outcome_with_race(
        "promo_1",
        &ScenarioResources::new().set("hype", 3),
        false,
    );
    assert_eq!(loss, ConcertOutcome::Failure);
}

#[test]
fn dating_light_hello_event_can_fire_and_unlocks_pal_date() {
    let mut state = with_deck(
        base_state("grand_concert", 8),
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[DeckSlot {
                support_id: "support:30052".into(),
                bond: 45,
                specialty: Some("friend".into()),
                assigned_facility: None,
            }]),
        },
    );
    state.phase = "FREE".into();
    state.scenario_resources = GrandLiveMechanics::initial_resources();
    // Force dating-starts by scanning seeds until the turn-start event fires.
    let plugin = uma_sim_core::scenario_plugin_for("grand_concert");
    let mut fired = false;
    for seed in 0..80 {
        state.meta.seed = seed;
        let (after, lines) = plugin.on_turn_start(&state);
        if lines.iter().any(|l| l.contains("dating-starts"))
            || after.pending_event_title.as_deref() == Some("Embrace Those Emotions!")
        {
            fired = true;
            assert!(after.awaiting_choice);
            assert!(after
                .pending_event_options
                .iter()
                .any(|o| o.to_lowercase().contains("can start dating")));
            break;
        }
    }
    assert!(
        fired,
        "expected Light Hello dating-starts within seed search"
    );
}

#[test]
fn song_mastery_train_bonuses_apply_on_purchase() {
    let Some(root) = uma_sim_core::detect_repo_root() else {
        return;
    };
    uma_sim_core::GrandLiveCatalogLoader::init_from_repo(Some(&root));
    let song =
        uma_sim_core::GrandLiveCatalog::find_song("2").expect("Run for Our Dream should load");
    assert!(matches!(
        song.mastery,
        uma_sim_core::GrandLiveMasteryBonus::TrainSkillPoints { value: 2 }
    ));
    let (res, _stats, lines) = GrandLiveMechanics::apply_song_mastery(
        &ScenarioResources::new(),
        &song.mastery,
        &uma_sim_core::TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
    );
    assert_eq!(res.get("mastery_train_sp"), 2);
    assert!(lines.iter().any(|l| l.contains("training SP")));
}

#[test]
fn closer_together_fires_at_senior_nov_with_16_songs() {
    let mut state = with_deck(
        base_state("grand_concert", 58),
        DeckState {
            slots: DeckPlacement::assign_by_specialty(&[DeckSlot {
                support_id: "support:30210".into(), // Smart Falcon
                bond: 80,
                specialty: Some("speed".into()),
                assigned_facility: None,
            }]),
        },
    );
    state.phase = "FREE".into();
    state.date = uma_sim_core::SimDate {
        year: 3,
        month: 11,
        half: 1,
    };
    state.scenario_resources = GrandLiveMechanics::initial_resources()
        .set("songs_learned", 16)
        .set("closer_together_done", 0);
    let plugin = uma_sim_core::scenario_plugin_for("grand_concert");
    let (after, lines) = plugin.on_turn_start(&state);
    assert!(
        lines.iter().any(|l| l.contains("Closer Together"))
            || after.pending_event_title.as_deref() == Some("Closer Together"),
        "lines={lines:?} title={:?}",
        after.pending_event_title
    );
    assert!(after
        .pending_event_options
        .iter()
        .any(|o| o.contains("Full Speed!")));

    // Trainee Smart Falcon without Falcon support still unlocks Full Speed!
    let mut trainee_only = base_state("grand_concert", 58);
    trainee_only.phase = "FREE".into();
    trainee_only.meta.trainee_name = "Smart Falcon".into();
    trainee_only.date = uma_sim_core::SimDate {
        year: 3,
        month: 11,
        half: 1,
    };
    trainee_only.scenario_resources = GrandLiveMechanics::initial_resources()
        .set("songs_learned", 16)
        .set("closer_together_done", 0);
    let (after2, _) = plugin.on_turn_start(&trainee_only);
    assert!(after2
        .pending_event_options
        .iter()
        .any(|o| o.contains("Full Speed!")));
}

#[test]
fn specialty_bonus_biases_roll_toward_specialty_facility() {
    let slots = vec![DeckSlot {
        support_id: "support:speed-card".into(),
        bond: 80,
        specialty: Some("speed".into()),
        assigned_facility: Some("wit".into()),
    }];
    let mut with_bonus = 0;
    let mut without_bonus = 0;
    for seed in 0..400 {
        let mut rng = uma_sim_core::SimRandom::new(seed);
        let placed = DeckPlacement::roll_for_turn(&slots, &mut rng, 120);
        if placed[0].assigned_facility.as_deref() == Some("speed") {
            with_bonus += 1;
        }
        let mut rng2 = uma_sim_core::SimRandom::new(seed);
        let placed2 = DeckPlacement::roll_for_turn(&slots, &mut rng2, 0);
        if placed2[0].assigned_facility.as_deref() == Some("speed") {
            without_bonus += 1;
        }
    }
    assert!(
        with_bonus > without_bonus + 20,
        "specialty bonus should raise specialty hits: with={with_bonus} without={without_bonus}"
    );
}

#[test]
fn live_type_and_result_state_packet_mapping() {
    assert_eq!(GrandLiveMechanics::live_type_for_race("promo_3"), 3);
    assert_eq!(GrandLiveMechanics::live_type_for_race("grand_concert"), 5);
    assert_eq!(
        GrandLiveMechanics::result_state_for_outcome(ConcertOutcome::GreatSuccess),
        2
    );
    assert_eq!(
        GrandLiveMechanics::result_state_for_outcome(ConcertOutcome::Normal),
        1
    );
    assert_eq!(
        GrandLiveMechanics::result_state_for_outcome(ConcertOutcome::Failure),
        0
    );
    let bonuses = GrandLiveMechanics::training_bonuses_packet(
        &ScenarioResources::new()
            .set("bonus_support_chain_pct", 2)
            .set("bonus_friendship_pct", 5),
    );
    assert!(bonuses.iter().any(|(t, v)| *t == 6 && *v == 2));
    assert!(bonuses.iter().any(|(t, v)| *t == 1 && *v == 5));
}

#[test]
fn technique_tiers_gate_advanced_lessons_until_later_concerts() {
    let Some(root) = uma_sim_core::detect_repo_root() else {
        return;
    };
    if !root
        .join("knowledge/canonical/by_kind/lesson.json")
        .is_file()
    {
        return;
    }
    uma_sim_core::GrandLiveCatalogLoader::init_from_repo(Some(&root));

    // Before first promo: advanced Stat+12 / SP+12 techniques stay off the board.
    let early = with_resources(
        {
            let mut s = base_state("grand_concert", 10);
            s.phase = "FREE".into();
            s
        },
        HashMap::from([
            ("concert_index".into(), 0),
            ("songs_learned".into(), 1),
            ("song_owned:1".into(), 1),
            ("techniques_since_last_song".into(), 0),
            ("song_slot_index".into(), 0),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
            ("perf_Da".into(), 200),
            ("perf_Pa".into(), 200),
            ("perf_Vo".into(), 200),
            ("perf_Vi".into(), 200),
            ("perf_Me".into(), 200),
        ]),
    );
    let early_techs = uma_sim_core::GrandLiveCatalog::board_techniques(&early);
    assert!(
        !early_techs.is_empty(),
        "early board should still have basic techniques"
    );
    let early_has_advanced = early_techs.iter().any(|t| {
        let e = t.effect_text.to_lowercase();
        e.contains("skill pts +12")
            || e.contains("skill points +12")
            || e.contains("+12\n")
            || e.ends_with("+12")
    });
    assert!(
        !early_has_advanced,
        "Stat/SP +12 should not appear before first concert"
    );

    let late = with_resources(
        {
            let mut s = base_state("grand_concert", 65);
            s.phase = "FREE".into();
            s
        },
        HashMap::from([
            ("concert_index".into(), 4),
            ("songs_learned".into(), 10),
            ("song_owned:1".into(), 1),
            ("techniques_since_last_song".into(), 0),
            ("song_slot_index".into(), 0),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
            ("perf_Da".into(), 200),
            ("perf_Pa".into(), 200),
            ("perf_Vo".into(), 200),
            ("perf_Vi".into(), 200),
            ("perf_Me".into(), 200),
        ]),
    );
    let late_techs = uma_sim_core::GrandLiveCatalog::board_techniques(&late);
    assert!(
        late_techs.len() > early_techs.len(),
        "later concerts unlock more technique tiers: early={} late={}",
        early_techs.len(),
        late_techs.len()
    );
    assert!(
        late_techs.iter().any(|t| {
            let e = t.effect_text.to_lowercase();
            e.contains("skill pts +12") || e.contains("+12")
        }),
        "before Grand Concert, +12 techniques should be available"
    );
}

#[test]
fn technique_pivot_skips_forced_song_slot_at_21_songs() {
    let Some(root) = uma_sim_core::detect_repo_root() else {
        return;
    };
    if !root
        .join("knowledge/canonical/by_kind/lesson.json")
        .is_file()
    {
        return;
    }
    uma_sim_core::GrandLiveCatalogLoader::init_from_repo(Some(&root));

    let mut state = with_resources(
        {
            let mut s = base_state("grand_concert", 66);
            s.phase = "FREE".into();
            s
        },
        HashMap::from([
            ("concert_index".into(), 4),
            ("songs_learned".into(), 21),
            ("song_owned:1".into(), 1),
            ("techniques_since_last_song".into(), 0),
            ("song_slot_index".into(), 0),
            ("hype".into(), 0),
            ("great_success_required".into(), 3),
            ("perf_Da".into(), 500),
            ("perf_Pa".into(), 500),
            ("perf_Vo".into(), 500),
            ("perf_Vi".into(), 500),
            ("perf_Me".into(), 500),
        ]),
    );
    // Mark many songs owned so remaining pool is thin; pivot must not force a song.
    for id in 2..=21 {
        state.scenario_resources = state.scenario_resources.set(&format!("song_owned:{id}"), 1);
    }
    let board = GrandLiveLessonBoard::current_slots(&state);
    let song_count = board.iter().filter(|s| s.is_song).count();
    let tech_count = board.iter().filter(|s| !s.is_song).count();
    assert!(
        tech_count >= song_count,
        "21-song pivot should prefer techniques: songs={song_count} techs={tech_count} board={board:?}"
    );
}
