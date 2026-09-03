//! Port of SoftCapTest.kt

mod common;

use uma_sim_core::legacy::LegacyApplicator;
use uma_sim_core::scoring::soft_cap_effectiveness_multiplier;

#[test]
fn shared_multiplier_matches_scoring_shared() {
    let cap = 1400;
    assert!((soft_cap_effectiveness_multiplier(1000, 20, cap) - 1.0).abs() < 1e-9);
    assert!((soft_cap_effectiveness_multiplier(1300, 20, cap) - 0.5).abs() < 1e-9);
    assert!((soft_cap_effectiveness_multiplier(1190, 20, cap) - 0.75).abs() < 1e-9);
    assert!((soft_cap_effectiveness_multiplier(1400, 20, cap) - 0.0).abs() < 1e-9);
}

#[test]
fn spark_cap_raises_hard_cap_before_soft_cap_blend() {
    let legacy = LegacyApplicator::build_legacy(&["factor:blue:1@3".into()], Vec::new());
    let hard_cap = LegacyApplicator::effective_stat_cap(1400, "speed", &legacy);
    assert_eq!(hard_cap, 1416);
    assert!((soft_cap_effectiveness_multiplier(1408, 16, hard_cap) - 0.25).abs() < 1e-9);
}
