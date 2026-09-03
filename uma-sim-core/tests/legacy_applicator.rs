//! Port of Kotlin `LegacyApplicatorTest.kt` (+ inheritance helpers from formula calibration).

mod common;

use std::sync::Mutex;

use common::base_state;
use uma_sim_core::legacy::{
    LegacyApplicator, LegacyDeckConfig, LegacyFactorContext, LegacyFactorMeta,
};
use uma_sim_core::state::{LegacyState, RunMeta, SimSettings, TraineeStats};
use uma_sim_core::SimEngine;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn pink_race_lookup(id: &str) -> Option<LegacyFactorMeta> {
    if id.starts_with("factor:pink:") {
        Some(LegacyFactorMeta {
            id: id.into(),
            category: "pink".into(),
            stat_key: None,
            skill_id: None,
            pink_tag: Some("turf".into()),
            race_name: None,
        })
    } else if id.starts_with("factor:race:") {
        Some(LegacyFactorMeta {
            id: id.into(),
            category: "race".into(),
            stat_key: None,
            skill_id: None,
            pink_tag: None,
            race_name: Some("February S".into()),
        })
    } else {
        None
    }
}

fn skill_lookup(id: &str) -> Option<LegacyFactorMeta> {
    match id {
        "factor:skill:20001" => Some(LegacyFactorMeta {
            id: id.into(),
            category: "skill".into(),
            stat_key: None,
            skill_id: Some("skill:200012".into()),
            pink_tag: None,
            race_name: None,
        }),
        "factor:blue:1" => Some(LegacyFactorMeta {
            id: id.into(),
            category: "blue".into(),
            stat_key: Some("speed".into()),
            skill_id: None,
            pink_tag: None,
            race_name: None,
        }),
        _ => None,
    }
}

#[test]
fn spark_caps_stack_per_star() {
    let _g = TEST_LOCK.lock().unwrap();
    let legacy = LegacyApplicator::build_legacy(
        &["factor:blue:1@3".into(), "factor:blue:1@2".into()],
        Vec::new(),
    );
    // Cap table: 3★=+16, 2★=+9 → 25; start: 21+12=33
    assert_eq!(legacy.spark_caps.get("speed").copied().unwrap_or(0), 25);
    assert_eq!(
        legacy.blue_start_bonuses.get("speed").copied().unwrap_or(0),
        33
    );
}

#[test]
fn effective_cap_raises_above_base() {
    let _g = TEST_LOCK.lock().unwrap();
    let legacy = LegacyApplicator::build_legacy(&["factor:blue:1@3".into()], Vec::new());
    assert_eq!(
        LegacyApplicator::effective_stat_cap(1400, "speed", &legacy),
        1416
    );
}

#[test]
fn spark_run_differs_from_ace_run() {
    let _g = TEST_LOCK.lock().unwrap();
    fn summary(factors: Vec<String>) -> i32 {
        let mut engine = SimEngine::new(SimSettings {
            speed_multiplier: 50,
            ..Default::default()
        });
        let mut meta = RunMeta::new(42, "ura", "Test");
        meta.legacy_factors = factors;
        engine.start(meta);
        engine.play_to_completion(500);
        engine.state().stats.speed
    }
    let ace = summary(Vec::new());
    let spark = summary(vec!["factor:blue:1@3".into(), "factor:blue:2@3".into()]);
    assert!(spark >= ace);
}

#[test]
fn pink_and_race_factors_tracked() {
    let _g = TEST_LOCK.lock().unwrap();
    LegacyFactorContext::set_lookup(Some(pink_race_lookup));
    let legacy = LegacyApplicator::build_legacy(
        &["factor:pink:11@2".into(), "factor:race:10001@3".into()],
        Vec::new(),
    );
    assert_eq!(legacy.pink_factor_ids.len(), 1);
    assert_eq!(legacy.race_factor_ids.len(), 1);
    assert_eq!(legacy.pink_star_totals.get("turf").copied(), Some(2));
    let stats = LegacyApplicator::apply_pink_aptitude(
        TraineeStats {
            speed: 0,
            stamina: 0,
            power: 0,
            guts: 0,
            wit: 10,
        },
        &legacy,
    );
    // Pink no longer proxies as wit.
    assert_eq!(stats.wit, 10);
    assert_eq!(legacy.pink_aptitude_tags, vec!["turf".to_string()]);
    assert_eq!(LegacyApplicator::race_win_skill_bonus(&legacy), 5);
    let apts = LegacyApplicator::apply_pink_aptitudes(
        std::collections::HashMap::from([("turf".into(), "G".into())]),
        &legacy,
    );
    assert_eq!(apts.get("turf").map(String::as_str), Some("F")); // ★2 → +1
    LegacyFactorContext::set_lookup(None);
}

#[test]
fn inheritance_choice_1_boosts_spark_stats() {
    let _g = TEST_LOCK.lock().unwrap();
    let mut state = base_state("ura", 13);
    state.stats.speed = 200;
    state.stats.stamina = 200;
    let after = LegacyApplicator::apply_inheritance_choice(&state, 1);
    assert_eq!(after.stats.speed, 220);
    assert_eq!(after.stats.stamina, 220);
}

#[test]
fn skill_factors_become_inherited_skills() {
    let _g = TEST_LOCK.lock().unwrap();
    LegacyDeckConfig::load_from_json(Some(r#"{"inherited_skill_slots":2}"#));
    LegacyFactorContext::set_lookup(Some(skill_lookup));
    let legacy = LegacyApplicator::build_legacy(
        &["factor:blue:1@3".into(), "factor:skill:20001@2".into()],
        Vec::new(),
    );
    assert_eq!(legacy.inherited_skill_ids, vec!["skill:200012".to_string()]);
    assert_eq!(legacy.spark_caps.get("speed").copied().unwrap_or(0), 16);
    assert_eq!(
        legacy.blue_start_bonuses.get("speed").copied().unwrap_or(0),
        21
    );
    LegacyFactorContext::set_lookup(None);
}

#[test]
fn inheritance_choice_applies_skills() {
    let _g = TEST_LOCK.lock().unwrap();
    let mut state = base_state("ura", 13);
    state.legacy = LegacyState {
        inherited_skill_ids: vec!["skill:200012".into()],
        ..Default::default()
    };
    let after = LegacyApplicator::apply_inheritance_choice(&state, 0);
    assert_eq!(after.learned_skill_ids, vec!["skill:200012".to_string()]);
}

#[test]
fn legacy_tree_preferred_over_flat_factors() {
    let _g = TEST_LOCK.lock().unwrap();
    use uma_sim_core::state::{LegacyTree, SparkSlot};

    let mut tree = LegacyTree::default();
    tree.parent_a.uma = "Oguri Cap".into();
    tree.parent_a.blue = SparkSlot {
        factor_id: "factor:blue:1".into(),
        stars: 3,
    };
    tree.parent_b.uma = "Special Week".into();

    let mut meta = RunMeta::new(1, "ura", "Silence Suzuka");
    meta.legacy_factors = vec!["factor:blue:2@1".into()]; // flat fallback — must be ignored
    meta.legacy_tree = Some(tree);

    assert_eq!(
        meta.effective_legacy_factors(),
        vec!["factor:blue:1@3".to_string()]
    );
    assert_eq!(
        meta.effective_parent_names(),
        vec!["Oguri Cap".to_string(), "Special Week".to_string()]
    );

    let mut engine = SimEngine::new(SimSettings {
        race_model: uma_sim_core::race::RaceModel::Stub,
        ..Default::default()
    });
    engine.start(meta);
    assert_eq!(
        engine
            .state()
            .legacy
            .blue_start_bonuses
            .get("speed")
            .copied()
            .unwrap_or(0),
        21
    );
    assert_eq!(
        engine.state().legacy.parent_names,
        vec!["Oguri Cap".to_string(), "Special Week".to_string()]
    );
}
