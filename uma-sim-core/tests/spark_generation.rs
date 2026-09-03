//! End-of-career spark generation tables.

use uma_sim_core::factory::init_from_detected_repo;
use uma_sim_core::rng::SimRandom;
use uma_sim_core::spark_generation::{
    blue_star_from_stat, generate_end_of_career_sparks, grade_quality_stars, red_generation_stars,
};
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

#[test]
fn blue_bands_never_roll_3star_below_600() {
    let mut rng = SimRandom::with_trace(1, false);
    for _ in 0..40 {
        assert!(blue_star_from_stat(599, &mut rng) <= 2);
    }
}

#[test]
fn red_star_distribution_covers_1_2_3() {
    let mut rng = SimRandom::with_trace(42, false);
    let mut seen = [false; 4];
    for _ in 0..200 {
        seen[red_generation_stars(&mut rng) as usize] = true;
    }
    assert!(seen[1] && seen[2] && seen[3]);
}

#[test]
fn grade_quality_ue_can_roll_3star() {
    let mut rng = SimRandom::with_trace(7, false);
    let mut got3 = false;
    for _ in 0..80 {
        if grade_quality_stars(30_000, &mut rng) == 3 {
            got3 = true;
            break;
        }
    }
    assert!(got3);
}

#[test]
fn completed_career_emits_blue_and_green_sparks() {
    let _ = init_from_detected_repo(true);
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 100,
        race_model: uma_sim_core::race::RaceModel::Stub,
        ..Default::default()
    });
    engine.start(RunMeta::new(11, "ura", "Special Week"));
    engine.play_to_completion(500);
    let sparks = &engine.state().generated_sparks;
    assert!(
        sparks.iter().any(|s| s.color == "blue"),
        "missing blue: {sparks:?}"
    );
    assert!(
        sparks.iter().any(|s| s.color == "green"),
        "missing green unique: {sparks:?}"
    );
}

#[test]
fn spark_generation_does_not_change_career_rng_call_count() {
    let _ = init_from_detected_repo(true);
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 100,
        race_model: uma_sim_core::race::RaceModel::Stub,
        trace_rng: true,
        ..Default::default()
    });
    engine.start(RunMeta::new(3, "ura", "Special Week"));
    engine.play_to_completion(500);
    let calls = engine.rng_call_count();
    let mut state = engine.state().clone();
    let before = state.generated_sparks.clone();
    state.generated_sparks.clear();
    generate_end_of_career_sparks(&mut state, 20_000);
    assert!(!state.generated_sparks.is_empty());
    assert_eq!(engine.rng_call_count(), calls);
    assert_ne!(before.len() + state.generated_sparks.len(), 0);
}
