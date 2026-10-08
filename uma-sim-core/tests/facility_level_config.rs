use std::collections::HashMap;

use uma_sim_core::{default_facility_levels, FacilityLevelConfig, TrainingFacility};

#[test]
fn level_rises_every_four_trains() {
    assert_eq!(FacilityLevelConfig::level_for_train_count(0), 1);
    assert_eq!(FacilityLevelConfig::level_for_train_count(3), 1);
    assert_eq!(FacilityLevelConfig::level_for_train_count(4), 2);
    assert_eq!(FacilityLevelConfig::level_for_train_count(8), 3);
    assert_eq!(FacilityLevelConfig::level_for_train_count(16), 5);
    assert_eq!(FacilityLevelConfig::level_for_train_count(99), 5);
}

#[test]
fn ura_uses_train_count_leveling_unity_does_not() {
    assert!(FacilityLevelConfig::uses_train_count_leveling("ura"));
    assert!(!FacilityLevelConfig::uses_train_count_leveling("unity_cup"));
}

#[test]
fn successful_train_increments_level_after_fourth_use() {
    let (after_one, counts) = FacilityLevelConfig::apply_successful_train(
        TrainingFacility::Speed,
        &default_facility_levels(),
        &HashMap::new(),
    );
    assert_eq!(after_one.get("speed").copied().unwrap_or(0), 1);
    assert_eq!(counts.get("speed").copied().unwrap_or(0), 1);

    let mut levels = after_one;
    let mut train_counts = counts;
    for _ in 0..3 {
        let next = FacilityLevelConfig::apply_successful_train(
            TrainingFacility::Speed,
            &levels,
            &train_counts,
        );
        levels = next.0;
        train_counts = next.1;
    }
    assert_eq!(levels.get("speed").copied().unwrap_or(0), 2);
    assert_eq!(train_counts.get("speed").copied().unwrap_or(0), 4);
}
