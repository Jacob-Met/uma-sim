//! Port of Kotlin `UraMechanicsTest.kt` (Gate R4).

use std::collections::HashMap;
use std::sync::Mutex;

use uma_sim_core::{
    CareerState, DuelContest, DuelPrediction, MoodLevel, RunMeta, ScenarioPlugin,
    ScenarioResources, SimDate, SimRandom, StatName, TraineeStats, TrainingFacility, UraMechanics,
    UraScenarioPlugin,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn base_state(resources: ScenarioResources) -> CareerState {
    CareerState {
        meta: RunMeta::new(1, "ura", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 10,
        stats: TraineeStats {
            speed: 200,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 80,
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
        scenario_resources: resources,
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
    }
}

#[test]
fn choose_duel_prefers_good_odds_target_stat() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(None);
    let options = vec![
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Guts),
                prediction: DuelPrediction::Bad,
            },
            0,
        ),
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Speed),
                prediction: DuelPrediction::Good,
            },
            0,
        ),
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Stamina),
                prediction: DuelPrediction::Bad,
            },
            0,
        ),
    ];
    assert_eq!(
        UraMechanics::choose_duel_contest_index(&options, &[StatName::Speed, StatName::Stamina]),
        1
    );
}

#[test]
fn choose_duel_falls_back_to_best_odds_when_no_good_target() {
    let _g = TEST_LOCK.lock().unwrap();
    let options = vec![
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Power),
                prediction: DuelPrediction::Great,
            },
            0,
        ),
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Stamina),
                prediction: DuelPrediction::Worst,
            },
            0,
        ),
    ];
    assert_eq!(
        UraMechanics::choose_duel_contest_index(&options, &[StatName::Speed]),
        0
    );
}

#[test]
fn duel_win_raises_cap_and_meek_level() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(Some(
        r#"{"happy_meek":{"win_rewards":{"stat_gain_by_level":[20],"cap_raise":50,"skill_points":10},
              "win_chance_by_prediction":{"great":1.0,"good":1.0,"bad":1.0,"worst":1.0}}}"#,
    ));
    let state = base_state(ScenarioResources::from_map(HashMap::from([
        ("happy_meek_level".into(), 0),
        ("happy_meek_badge".into(), 1),
    ])));
    let option = UraMechanics::format_contest_option(
        &DuelContest {
            facility: Some(TrainingFacility::Speed),
            prediction: DuelPrediction::Great,
        },
        0,
    );
    let mut rng = SimRandom::new(999);
    let (after, lines) = UraMechanics::resolve_duel(&state, &option, &mut rng);
    assert!(lines.iter().any(|l| l.contains("WON")));
    assert_eq!(after.scenario_resources.get("ura_cap_bonus_speed"), 50);
    assert_eq!(after.scenario_resources.get("happy_meek_level"), 1);
    assert!(after.stats.speed > 200);
}

#[test]
fn training_on_badged_facility_triggers_duel_event() {
    let _g = TEST_LOCK.lock().unwrap();
    let plugin = UraScenarioPlugin::new();
    let state = CareerState {
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        scenario_resources: ScenarioResources::from_map(HashMap::from([(
            "happy_meek_badge".into(),
            1,
        )])),
        ..base_state(ScenarioResources::new())
    };
    let (after, lines) = plugin.on_training_complete(&state, TrainingFacility::Speed, true);
    assert!(after
        .pending_event_title
        .as_ref()
        .is_some_and(|t| t.contains("Happy Meek")));
    assert!(after.pending_event_options.len() >= 5);
    assert!(lines.iter().any(|l| l.contains("Challenge")));
}

#[test]
fn duel_training_bias_boosts_badged_facility() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(Some(
        r#"{"happy_meek":{"duel_training_bias_moderate":1.25}}"#,
    ));
    let res = ScenarioResources::from_map(HashMap::from([("happy_meek_badge".into(), 2)]));
    let boosted =
        UraMechanics::apply_duel_training_bias(100.0, TrainingFacility::Stamina, &res, 10, 100);
    assert_eq!(boosted, 125.0);
    let unchanged =
        UraMechanics::apply_duel_training_bias(100.0, TrainingFacility::Speed, &res, 10, 100);
    assert_eq!(unchanged, 100.0);
}

#[test]
fn duel_win_grants_racing_spirit_hint() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(None);
    let state = base_state(ScenarioResources::from_map(HashMap::from([(
        "happy_meek_level".into(),
        1,
    )])));
    let option = UraMechanics::format_contest_option(
        &DuelContest {
            facility: Some(TrainingFacility::Speed),
            prediction: DuelPrediction::Great,
        },
        0,
    );
    // Great prediction → near-certain win
    let mut rng = SimRandom::new(0);
    let (after, lines) = UraMechanics::resolve_duel(&state, &option, &mut rng);
    assert!(
        after
            .hint_levels
            .get("racing_spirit_speed")
            .copied()
            .unwrap_or(0)
            >= 1
            || lines.iter().any(|l| l.contains("hint")),
        "expected racing spirit hint; lines={lines:?} hints={:?}",
        after.hint_levels
    );
}

#[test]
fn duel_accepts_bad_odds_when_failure_within_pct() {
    let _g = TEST_LOCK.lock().unwrap();
    // 60% acceptable failure → Bad (55% fail) allowed; Worst (78%) not.
    UraMechanics::load_research(Some(
        r#"{"happy_meek":{"duel_failure_acceptable_pct":60,"win_chance_by_prediction":{"great":0.92,"good":0.72,"bad":0.45,"worst":0.22}}}"#,
    ));
    assert!(UraMechanics::prediction_failure_acceptable(
        DuelPrediction::Bad
    ));
    assert!(!UraMechanics::prediction_failure_acceptable(
        DuelPrediction::Worst
    ));
    let options = vec![
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Guts),
                prediction: DuelPrediction::Worst,
            },
            0,
        ),
        UraMechanics::format_contest_option(
            &DuelContest {
                facility: Some(TrainingFacility::Speed),
                prediction: DuelPrediction::Bad,
            },
            0,
        ),
    ];
    let idx = UraMechanics::choose_duel_contest_index(&options, &[StatName::Speed, StatName::Guts]);
    assert_eq!(
        idx, 1,
        "prefer Bad Speed over Worst Guts when within acceptable fail %"
    );
}

#[test]
fn max_level_meek_win_unlocks_past_my_limits() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(None);
    let state = base_state(ScenarioResources::from_map(HashMap::from([(
        "happy_meek_level".into(),
        UraMechanics::max_meek_level(),
    )])));
    let option = UraMechanics::format_contest_option(
        &DuelContest {
            facility: Some(TrainingFacility::Speed),
            prediction: DuelPrediction::Great,
        },
        0,
    );
    let mut won = false;
    let mut after = state.clone();
    for seed in 0..40 {
        let mut rng = SimRandom::new(seed);
        let (s, lines) = UraMechanics::resolve_duel(&state, &option, &mut rng);
        if lines.iter().any(|l| l.contains("WON")) {
            after = s;
            won = true;
            break;
        }
    }
    assert!(won, "expected a Great-odds win within seeds");
    assert!(
        after
            .learned_skill_ids
            .iter()
            .any(|id| id == "skill:past_my_limits"),
        "Past My Limits missing: {:?}",
        after.learned_skill_ids
    );
}

#[test]
fn ura_finale_distance_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/ura_finale.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str() == Some("finale distance/surface from race history")));
}
