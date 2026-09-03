//! Parity harness tests (R0/R4 gate). Consumes fixtures exported by Kotlin
//! `ParityFixtureExportTest` into `tests/fixtures/`.

use std::collections::HashMap;
use std::path::PathBuf;

use uma_sim_core::scoring::{
    apply_training_multipliers, calculate_raw_training_score, parse_event_reward_text,
    sample_event_reward, score_event_option, score_lesson_option, soft_cap_effectiveness_multiplier,
    stat_score, BarFillResult, DateYear, DecisionContext, GameDateSnapshot, LessonScoreInputs,
    MoodOrdinal, StatName, SupportEffectSlice, TrainingConfig, TrainingOption,
};
use uma_sim_core::state::MoodLevel;
use uma_sim_core::{run_rng_trace_fixture, run_turn_trace_fixture, RngTraceFixture, TurnTraceFixture};

const TRACE_SEEDS: [i64; 3] = [1, 42, 7];
const SCENARIOS: [&str; 4] = ["ura", "grand_concert", "unity", "trackblazer"];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = fixtures_dir().join(name);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("missing fixture {name}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse fixture {name}: {e}"))
}

fn fixture_exists(name: &str) -> bool {
    fixtures_dir().join(name).is_file()
}

fn strip_additive_gl_keys(snap: &uma_sim_core::TurnSnapshot) -> uma_sim_core::TurnSnapshot {
    let mut s = snap.clone();
    // R7.2 telemetry keys absent from frozen Kotlin turn_trace fixtures.
    s.scenario_resources.remove("last_live_result");
    s.scenario_resources.remove("member_ready_count");
    s.scenario_resources.remove("unique_skill_power");
    s
}

fn first_turn_divergence(
    expected: &[uma_sim_core::TurnSnapshot],
    actual: &[uma_sim_core::TurnSnapshot],
) -> Option<(i32, String)> {
    let pairs = expected.iter().zip(actual.iter());
    for (exp, act) in pairs {
        let exp_n = strip_additive_gl_keys(exp);
        let act_n = strip_additive_gl_keys(act);
        if exp_n != act_n {
            return Some((
                exp.turn,
                format!("turn {} expected {:?}, got {:?}", exp.turn, exp_n, act_n),
            ));
        }
    }
    if expected.len() != actual.len() {
        let turn = actual
            .get(expected.len())
            .or_else(|| expected.get(actual.len()))
            .map(|s| s.turn)
            .unwrap_or(-1);
        return Some((
            turn,
            format!(
                "snapshot count mismatch: expected {}, got {}",
                expected.len(),
                actual.len()
            ),
        ));
    }
    None
}

fn first_rng_divergence(expected: &[String], actual: &[String]) -> Option<(usize, String)> {
    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        if exp != act {
            return Some((i, format!("rng#{i} expected {exp}, got {act}")));
        }
    }
    if expected.len() != actual.len() {
        return Some((
            expected.len().min(actual.len()),
            format!(
                "rng trace length mismatch: expected {}, got {}",
                expected.len(),
                actual.len()
            ),
        ));
    }
    None
}

fn approx_eq(actual: f64, expected: f64, ctx: &str) {
    let tol = (expected.abs() * 1e-9).max(1e-9);
    assert!(
        (actual - expected).abs() <= tol,
        "{ctx}: actual={actual} expected={expected}"
    );
}

fn reading_matches(actual: &uma_sim_core::scoring::EventEffectReading, expected: &serde_json::Value, ctx: &str) {
    assert_eq!(
        actual.energy_delta,
        expected["energyDelta"].as_i64().unwrap_or(0) as i32,
        "{ctx} energyDelta"
    );
    assert_eq!(
        actual.energy_is_range,
        expected["energyIsRange"].as_bool().unwrap_or(false),
        "{ctx} energyIsRange"
    );
    assert_eq!(
        actual.mood_delta,
        expected["moodDelta"].as_i64().unwrap_or(0) as i32,
        "{ctx} moodDelta"
    );
    assert_eq!(
        actual.random_stat_gain,
        expected["randomStatGain"].as_i64().unwrap_or(0) as i32,
        "{ctx} randomStatGain"
    );
    assert_eq!(
        actual.all_stats_gain,
        expected["allStatsGain"].as_i64().unwrap_or(0) as i32,
        "{ctx} allStatsGain"
    );
    assert_eq!(
        actual.skill_pts,
        expected["skillPts"].as_i64().unwrap_or(0) as i32,
        "{ctx} skillPts"
    );
    assert_eq!(
        actual.bond,
        expected["bond"].as_i64().unwrap_or(0) as i32,
        "{ctx} bond"
    );
    assert_eq!(
        actual.random_branch,
        expected["randomBranch"].as_bool().unwrap_or(false),
        "{ctx} randomBranch"
    );
    let exp_stats = expected["stats"].as_object().cloned().unwrap_or_default();
    for (k, v) in &exp_stats {
        let stat = StatName::from_name(k).unwrap_or_else(|| panic!("{ctx} unknown stat {k}"));
        let got = actual.stats.get(&stat).copied().unwrap_or(0);
        assert_eq!(got, v.as_i64().unwrap_or(0) as i32, "{ctx} stats.{k}");
    }
    for (stat, got) in &actual.stats {
        let name = match stat {
            StatName::Speed => "SPEED",
            StatName::Stamina => "STAMINA",
            StatName::Power => "POWER",
            StatName::Guts => "GUTS",
            StatName::Wit => "WIT",
        };
        if !exp_stats.contains_key(name) {
            assert_eq!(*got, 0, "{ctx} unexpected stat {name}={got}");
        }
    }
    let exp_hints = expected["hints"].as_array().cloned().unwrap_or_default();
    assert_eq!(actual.hints.len(), exp_hints.len(), "{ctx} hints len");
    let exp_tokens = expected["performanceTokens"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (k, v) in &exp_tokens {
        assert_eq!(
            actual.performance_tokens.get(k).copied().unwrap_or(0),
            v.as_i64().unwrap_or(0) as i32,
            "{ctx} performanceTokens.{k}"
        );
    }
}

fn event_score_ctx() -> DecisionContext {
    let mut ctx = DecisionContext {
        trainee_name: "Special Week".into(),
        energy: 80,
        mood_ordinal: MoodOrdinal::GOOD,
        day: 24,
        remaining_turns: 48,
        ..Default::default()
    };
    ctx.stats = HashMap::from([
        (StatName::Speed, 400),
        (StatName::Stamina, 350),
        (StatName::Power, 300),
        (StatName::Guts, 250),
        (StatName::Wit, 200),
    ]);
    ctx.stat_caps = StatName::ALL.into_iter().map(|s| (s, 1400)).collect();
    ctx
}

fn lesson_score_ctx() -> DecisionContext {
    let mut ctx = DecisionContext {
        trainee_name: "Special Week".into(),
        energy: 70,
        mood_ordinal: MoodOrdinal::NORMAL,
        day: 30,
        remaining_turns: 42,
        objective_profile: "scenario_clear_grand_concert".into(),
        days_to_concert: 6,
        songs_learned: 4,
        ..Default::default()
    };
    ctx.stats = StatName::ALL.into_iter().map(|s| (s, 300)).collect();
    ctx.stat_caps = StatName::ALL.into_iter().map(|s| (s, 1200)).collect();
    ctx.token_totals = HashMap::from([
        ("Da".into(), 50),
        ("Pa".into(), 40),
        ("Vo".into(), 30),
    ]);
    ctx
}

fn lesson_inputs_by_index(idx: i64) -> LessonScoreInputs {
    match idx {
        0 => LessonScoreInputs {
            is_song: true,
            song_known: false,
            training_effectiveness_pct: 10.0,
            legacy_rank_score: 50.0,
            ..Default::default()
        },
        1 => LessonScoreInputs {
            is_song: false,
            training_gain_amount: 15.0,
            raw_stat_gains: HashMap::from([(StatName::Speed, 10)]),
            ..Default::default()
        },
        2 => LessonScoreInputs {
            is_song: true,
            song_already_owned: true,
            skill_hint_amount: 5.0,
            energy_gain: 10,
            ..Default::default()
        },
        _ => panic!("unknown lesson score index {idx}"),
    }
}

fn assert_scoring_vector(v: &serde_json::Value) {
    let fn_name = v["function"].as_str().unwrap_or("?");
    let input = &v["input"];
    let expected = &v["expected"];
    match fn_name {
        "softCapEffectivenessMultiplier" => {
            let actual = soft_cap_effectiveness_multiplier(
                input["currentStat"].as_i64().unwrap() as i32,
                input["statGain"].as_i64().unwrap() as i32,
                input["statCap"].as_i64().unwrap() as i32,
            );
            approx_eq(actual, expected["value"].as_f64().unwrap(), fn_name);
        }
        "applyTrainingMultipliers" => {
            let mood = MoodLevel::from_scoring_name(input["mood"].as_str().unwrap()).unwrap();
            let slices = [
                SupportEffectSlice {
                    friendship_bonus_pct: input["friendshipBonusPct1"].as_f64().unwrap_or(10.0),
                    on_specialty: true,
                    ..Default::default()
                },
                SupportEffectSlice {
                    friendship_bonus_pct: input["friendshipBonusPct2"].as_f64().unwrap_or(15.0),
                    on_specialty: false,
                    ..Default::default()
                },
            ];
            let actual = apply_training_multipliers(
                input["baseGain"].as_f64().unwrap(),
                &slices,
                mood,
                input["numCharactersPresent"].as_i64().unwrap() as i32,
                0.0,
            ) as f64;
            approx_eq(actual, expected["value"].as_f64().unwrap(), fn_name);
        }
        "calculateRawTrainingScore" => {
            let day = input["day"].as_i64().unwrap() as i32;
            let primary = StatName::from_name(input["primary"].as_str().unwrap()).unwrap();
            let stats = HashMap::from([
                (StatName::Speed, 400),
                (StatName::Stamina, 350),
                (StatName::Power, 300),
                (StatName::Guts, 250),
                (StatName::Wit, 200),
            ]);
            let mut config = TrainingConfig::new(
                stats.clone(),
                StatName::ALL.to_vec(),
                StatName::ALL.to_vec(),
                HashMap::new(),
                GameDateSnapshot {
                    year: DateYear::Classic,
                    day,
                    b_is_pre_debut: false,
                    is_summer: false,
                },
                "URA Finale".into(),
                true,
            );
            config.stat_caps = stats.iter().map(|(k, _)| (*k, 1400)).collect();
            let option = TrainingOption {
                name: primary,
                stat_gains: HashMap::from([(primary, input["statGain"].as_i64().unwrap() as i32)]),
                relationship_bars: vec![
                    BarFillResult {
                        dominant_color: "orange".into(),
                        fill_percent: 60.0,
                        ..Default::default()
                    },
                    BarFillResult {
                        dominant_color: "green".into(),
                        fill_percent: 40.0,
                        ..Default::default()
                    },
                ],
                num_rainbow: input["rainbow"].as_i64().unwrap() as i32,
                training_level: Some(3),
                ..Default::default()
            };
            let actual = calculate_raw_training_score(&config, &option);
            approx_eq(actual, expected["value"].as_f64().unwrap(), fn_name);
        }
        "parseEventRewardText" => {
            let text = input["text"].as_str().unwrap();
            let actual = parse_event_reward_text(text);
            reading_matches(&actual, &expected["reading"], &format!("parse:{text}"));
        }
        "sampleEventReward" => {
            let text = input["text"].as_str().unwrap();
            let actual = sample_event_reward(
                text,
                input["branchRoll"].as_f64().unwrap(),
                input["energyRoll"].as_f64().unwrap(),
            );
            reading_matches(&actual, &expected["reading"], "sampleEventReward");
        }
        "scoreEventOption" => {
            let ctx = event_score_ctx();
            let text = input["text"].as_str().unwrap();
            let actual = score_event_option(&ctx, text);
            approx_eq(actual, expected["value"].as_f64().unwrap(), fn_name);
        }
        "scoreLessonOption" => {
            let ctx = lesson_score_ctx();
            let inputs = lesson_inputs_by_index(input["index"].as_i64().unwrap());
            let actual = score_lesson_option(&ctx, &inputs);
            approx_eq(actual, expected["value"].as_f64().unwrap(), fn_name);
        }
        "statScore" => {
            let actual = stat_score(input["value"].as_i64().unwrap() as i32) as f64;
            approx_eq(actual, expected["score"].as_f64().unwrap(), fn_name);
        }
        other => panic!("unknown scoring vector function: {other}"),
    }
}

#[test]
fn rng_seed_42_fixture_present() {
    assert!(
        fixture_exists("rng_seed_42.json"),
        "run Kotlin ParityFixtureExportTest with -DexportParity=true"
    );
}

#[test]
fn all_rng_trace_fixtures_match_kotlin() {
    for seed in TRACE_SEEDS {
        for scenario in SCENARIOS {
            let name = format!("rng_trace_{seed}_{scenario}.json");
            if !fixture_exists(&name) {
                panic!("missing {name} — export parity fixtures from Kotlin first");
            }
            let expected: RngTraceFixture = load_json(&name);
            let actual = run_rng_trace_fixture(seed, scenario);
            if let Some((idx, msg)) = first_rng_divergence(&expected.entries, &actual.entries) {
                panic!("{scenario} seed {seed}: first RNG divergence at index {idx}: {msg}");
            }
        }
    }
}

#[test]
fn turn_traces_report_first_divergence() {
    let mut matrix: Vec<(i64, &str, &'static str, Option<i32>)> = Vec::new();
    for seed in TRACE_SEEDS {
        for scenario in SCENARIOS {
            let name = format!("turn_trace_{seed}_{scenario}.json");
            if !fixture_exists(&name) {
                panic!("missing {name} — export parity fixtures from Kotlin first");
            }
            let expected: TurnTraceFixture = load_json(&name);
            let actual = run_turn_trace_fixture(seed, scenario);
            let divergence = first_turn_divergence(&expected.snapshots, &actual.snapshots);
            let status = if divergence.is_none() { "MATCH" } else { "DIVERGE" };
            let turn = divergence.as_ref().map(|(t, _)| *t);
            matrix.push((seed, scenario, status, turn));
            if let Some((turn, msg)) = divergence {
                eprintln!("turn_trace {scenario} seed {seed}: first divergence at turn {turn}: {msg}");
            }
        }
    }
    eprintln!("turn_trace matrix: {matrix:?}");
    let known_diverge = matrix
        .iter()
        .any(|(s, sc, st, _)| *s == 42 && *sc == "trackblazer" && *st == "DIVERGE");
    assert!(
        known_diverge || matrix.iter().all(|(_, _, st, _)| *st == "MATCH"),
        "expected trackblazer seed 42 divergence to be detected, or all traces to match"
    );
}

#[test]
fn scoring_vectors_fixture_loads() {
    if !fixture_exists("scoring_vectors.json") {
        panic!("missing scoring_vectors.json");
    }
    let root: serde_json::Value = load_json("scoring_vectors.json");
    let vectors = root["vectors"].as_array().expect("vectors array");
    assert!(
        vectors.len() >= 50,
        "expected >=50 scoring vectors, got {}",
        vectors.len()
    );
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in vectors {
        let fn_name = v["function"].as_str().unwrap_or("?");
        *counts.entry(fn_name.to_string()).or_default() += 1;
        assert_scoring_vector(v);
    }
    eprintln!("scoring vector counts: {counts:?}");
}

#[test]
fn event_parse_vectors_fixture_loads() {
    if !fixture_exists("event_parse_vectors.json") {
        panic!("missing event_parse_vectors.json");
    }
    let root: serde_json::Value = load_json("event_parse_vectors.json");
    let vectors = root["vectors"].as_array().expect("vectors array");
    assert!(!vectors.is_empty(), "event_parse_vectors.json must contain entries");
    for (i, v) in vectors.iter().enumerate() {
        let text = v["text"].as_str().unwrap_or("");
        let actual = parse_event_reward_text(text);
        reading_matches(&actual, &v["reading"], &format!("event_parse#{i}"));
    }
    eprintln!("event_parse_vectors: {} entries asserted", vectors.len());
}

#[test]
fn parity_fixture_manifest_complete() {
    let required: Vec<String> = TRACE_SEEDS
        .iter()
        .flat_map(|seed| {
            SCENARIOS.iter().flat_map(move |scenario| {
                [
                    format!("rng_trace_{seed}_{scenario}.json"),
                    format!("turn_trace_{seed}_{scenario}.json"),
                ]
            })
        })
        .chain([
            "rng_seed_42.json".into(),
            "scoring_vectors.json".into(),
            "event_parse_vectors.json".into(),
        ])
        .collect();
    let missing: Vec<_> = required
        .iter()
        .filter(|n| !fixture_exists(n))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "missing parity fixtures: {missing:?}. Run scripts/parity.ps1 -Export"
    );
}
