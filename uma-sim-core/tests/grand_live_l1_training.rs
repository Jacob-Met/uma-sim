//! Grand Live L1 training base table (GameTora / grand_concert.md).

mod common;

use common::base_state;
use uma_sim_core::{MoodLevel, TrainingFacility, TrainingResolver};

#[test]
fn grand_concert_l1_speed_matches_gametora_base() {
    let state = base_state("grand_concert", 5);
    let resolver = TrainingResolver::from_installed_tables();
    let out = resolver.resolve_typical(TrainingFacility::Speed, 1, MoodLevel::Normal, Some(&state));
    assert_eq!(out.main_gain, 8);
    assert_eq!(out.secondary_gain, 4);
    assert_eq!(out.tertiary_gain, 0);
    assert_eq!(out.energy_cost, 19);
    assert_eq!(out.skill_points, 4);
}

#[test]
fn grand_concert_l1_wit_recovers_energy() {
    let state = base_state("grand_concert", 5);
    let resolver = TrainingResolver::from_installed_tables();
    let out = resolver.resolve_typical(TrainingFacility::Wit, 1, MoodLevel::Normal, Some(&state));
    assert_eq!(out.main_gain, 6);
    assert_eq!(out.secondary_gain, 2);
    assert_eq!(out.energy_cost, -5);
    assert_eq!(out.skill_points, 5);
}

#[test]
fn ura_l1_unchanged_by_gl_override() {
    let state = base_state("ura", 5);
    let resolver = TrainingResolver::from_installed_tables();
    let out = resolver.resolve_typical(TrainingFacility::Speed, 1, MoodLevel::Normal, Some(&state));
    // Generic typical is 10, not GL's 8.
    assert_eq!(out.main_gain, 10);
    assert_eq!(out.energy_cost, 20);
}
