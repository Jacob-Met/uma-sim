// T162 gate-verification fixture: DO NOT MERGE.
// Regression test for the max_hp baseline formula (exercises the bugfix-requires-test gate).
use uma_race_core::{max_hp, Strategy};

#[test]
fn max_hp_matches_documented_formula() {
    // MaxHP = 0.8 x StrategyCoef x Stamina + CourseDistance; Sasi coef = 1.0
    let hp = max_hp(Strategy::Sasi, 500.0, 2000.0);
    assert!(
        (hp - 2400.0).abs() < 1e-9,
        "max_hp formula regression: expected ~2400.0, got {hp}"
    );
    assert!(hp.is_finite());
}
