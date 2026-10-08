//! Phase 5 Outcomes v1 — placement multipliers + epithet stubs.

use std::sync::Mutex;
use uma_sim_core::{RaceOutcomeConfig, RacePlacement, SimRandom};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn place_and_show_fans_are_lower_than_win() {
    let _g = TEST_LOCK.lock().unwrap();
    RaceOutcomeConfig::load_from_json(Some(
        r#"{"win":{"fans_multiplier":1.0,"skill_points_base":45},"place_2_5":{"fans_multiplier":0.6,"skill_points_base":35},"show":{"fans_multiplier":0.3,"skill_points_base":20}}"#,
    ));
    let mut rng_a = SimRandom::new(7);
    let mut rng_b = SimRandom::new(7);
    let mut rng_c = SimRandom::new(7);
    let win =
        RaceOutcomeConfig::fan_gain_placed(true, "optional", RacePlacement::First, &mut rng_a);
    let place =
        RaceOutcomeConfig::fan_gain_placed(true, "optional", RacePlacement::Place25, &mut rng_b);
    let show =
        RaceOutcomeConfig::fan_gain_placed(true, "optional", RacePlacement::Show, &mut rng_c);
    assert!(
        win > place && place > show,
        "win={win} place={place} show={show}"
    );
    assert_eq!(
        RaceOutcomeConfig::skill_points_for(true, RacePlacement::Place25),
        35
    );
    assert_eq!(
        RaceOutcomeConfig::skill_points_for(true, RacePlacement::Show),
        20
    );
}

#[test]
fn g1_and_climax_wins_grant_stub_epithets() {
    let _g = TEST_LOCK.lock().unwrap();
    assert_eq!(
        RaceOutcomeConfig::epithet_for_win("tokyo_yushun_G1"),
        Some("epithet:sim_g1_win")
    );
    assert_eq!(
        RaceOutcomeConfig::epithet_for_win("climax_1"),
        Some("epithet:sim_climax_win")
    );
    let mut statuses = Vec::new();
    assert_eq!(
        RaceOutcomeConfig::grant_epithet(&mut statuses, "climax_3"),
        Some("epithet:sim_climax_win")
    );
    assert_eq!(
        RaceOutcomeConfig::grant_epithet(&mut statuses, "climax_3"),
        None,
        "duplicate epithet not re-granted"
    );
    assert_eq!(statuses, vec!["epithet:sim_climax_win".to_string()]);
}

#[test]
fn mid_run_race_physics_documented_in_research() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/grand_concert.json"
    ))
    .expect("research");
    assert!(
        raw.contains("uma-race-core") || raw.contains("race_note"),
        "grand_concert research should cite mid-run race physics (R8)"
    );
    let outcomes = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/race_outcomes.json"
    ))
    .expect("outcomes");
    assert!(
        outcomes.contains("physics"),
        "race_outcomes notes should mention physics default"
    );
}
