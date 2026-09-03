//! Port of Kotlin `GrandLiveFormulaCalibrationTest.kt` (+ LegacyInheritanceTest from same file).

use std::collections::HashMap;
use std::sync::Mutex;

use uma_sim_core::{
    CareerState, GrandLiveMechanics, LegacyApplicator, LegacyDeckConfig, LegacyFactorContext,
    LegacyFactorMeta, LegacyState, MoodLevel, RunMeta, ScenarioResources, SimDate, TraineeStats,
    TrainingFacility,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn calibration_rows_match_uma_guide_formula_total() {
    let cases = [
        ((TrainingFacility::Speed, 1, 0), 10),
        ((TrainingFacility::Speed, 1, 2), 13),
        ((TrainingFacility::Wit, 1, 0), 6),
    ];
    for ((fac, level, deck), expected) in cases {
        assert_eq!(
            GrandLiveMechanics::performance_token_total(fac, level, deck, 0),
            expected,
            "{fac:?} L{level} deck={deck}"
        );
    }
}

#[test]
fn split_sums_to_formula_total() {
    for facility in TrainingFacility::ALL {
        let total = GrandLiveMechanics::performance_token_total(facility, 3, 2, 1);
        let split = GrandLiveMechanics::split_token_total(total, facility);
        assert_eq!(
            total,
            split.values().sum::<i32>(),
            "{}",
            facility.name()
        );
    }
}

fn factor_lookup(id: &str) -> Option<LegacyFactorMeta> {
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
fn skill_factors_become_inherited_skills() {
    let _g = TEST_LOCK.lock().unwrap();
    LegacyDeckConfig::load_from_json(Some(
        r#"{"inherited_skill_slots":2,"spark_stat_cap_bonus":{"per_star":20,"max_bonus":400}}"#,
    ));
    LegacyFactorContext::set_lookup(Some(factor_lookup));
    let legacy = LegacyApplicator::build_legacy(
        &["factor:blue:1@3".into(), "factor:skill:20001@2".into()],
        Vec::new(),
    );
    assert_eq!(legacy.inherited_skill_ids, vec!["skill:200012".to_string()]);
    assert_eq!(legacy.spark_caps.get("speed").copied(), Some(60));
    LegacyFactorContext::set_lookup(None);
}

#[test]
fn inheritance_choice_applies_skills() {
    let state = CareerState {
        meta: RunMeta::new(1, "ura", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 13,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 100,
        max_energy: 100,
        mood: MoodLevel::Normal,
        fans: 0,
        skill_points: 0,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: String::new(),
        completed_races: Vec::new(),
        facility_levels: HashMap::new(),
        facility_train_counts: HashMap::new(),
        pending_event_options: Vec::new(),
        hint_levels: HashMap::new(),
        statuses: Vec::new(),
        performance_tokens: HashMap::new(),
        scenario_resources: ScenarioResources::new(),
        legacy: LegacyState {
            inherited_skill_ids: vec!["skill:200012".into()],
            ..Default::default()
        },
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
    };
    let after = LegacyApplicator::apply_inheritance_choice(&state, 0);
    assert_eq!(after.learned_skill_ids, vec!["skill:200012".to_string()]);
}
