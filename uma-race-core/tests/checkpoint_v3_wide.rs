//! Wider V3 soft gate toward the full checkpoint corpus.
//!
//! Fixture: `research/race_checkpoint_v3_sample_240.json` (low-skill-biased extract).
//! Triage: `research/race_checkpoint_v3_triage_240.json`
//!
//! Floor is looser than the curated 120-case gate while high-skill residuals close.
//! Path toward 1000: re-extract with `V3_CASE_TARGET=1000` once pass rate holds.

use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use uma_race_core::{
    get_course, simulate_solo, simulate_with_default_pacer, Aptitude, GroundCondition, HorseInput,
    PosKeepMode, Strategy,
};

const DT: f64 = 1.0 / 15.0;
const MIN_PASS_RATE: f64 = 0.80;

#[derive(Deserialize)]
struct SampleFile {
    case_count: usize,
    cases: Vec<SampleCase>,
}

#[derive(Deserialize)]
struct SampleHorse {
    speed: f64,
    stamina: f64,
    power: f64,
    guts: f64,
    wisdom: f64,
    strategy: String,
    #[serde(rename = "distanceAptitude")]
    distance_aptitude: String,
    #[serde(rename = "surfaceAptitude")]
    surface_aptitude: String,
    #[serde(rename = "strategyAptitude")]
    strategy_aptitude: String,
}

#[derive(Deserialize)]
struct SampleCase {
    id: String,
    #[allow(dead_code)]
    kind: String,
    seed_u32: u32,
    course_id: u32,
    ground: String,
    mood: i8,
    pace_effects: bool,
    skills: Vec<String>,
    horse: SampleHorse,
    expected_finish: Option<f64>,
    extract_err: Option<String>,
}

fn research_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../research")
        .join(name)
}

fn parse_strategy(s: &str) -> Strategy {
    match s {
        "Nige" => Strategy::Nige,
        "Senkou" => Strategy::Senkou,
        "Sasi" => Strategy::Sasi,
        "Oikomi" => Strategy::Oikomi,
        "Oonige" => Strategy::Oonige,
        other => panic!("unknown strategy {other}"),
    }
}

fn parse_ground(s: &str) -> GroundCondition {
    match s {
        "Good" => GroundCondition::Good,
        "Yielding" => GroundCondition::Yielding,
        "Soft" => GroundCondition::Soft,
        "Heavy" => GroundCondition::Heavy,
        other => panic!("unknown ground {other}"),
    }
}

fn to_horse(c: &SampleCase) -> HorseInput {
    HorseInput {
        speed: c.horse.speed,
        stamina: c.horse.stamina,
        power: c.horse.power,
        guts: c.horse.guts,
        wisdom: c.horse.wisdom,
        strategy: parse_strategy(&c.horse.strategy),
        distance_apt: Aptitude::from_str_letter(&c.horse.distance_aptitude).expect("dist apt"),
        surface_apt: Aptitude::from_str_letter(&c.horse.surface_aptitude).expect("surf apt"),
        strategy_apt: Aptitude::from_str_letter(&c.horse.strategy_aptitude).expect("strat apt"),
        mood: c.mood,
        skills: c.skills.clone(),
    }
}

#[test]
fn r85_v3_wide_240_soft_gate() {
    let sample_path = research_path("race_checkpoint_v3_sample_240.json");
    let raw = fs::read_to_string(&sample_path).expect("race_checkpoint_v3_sample_240.json");
    let sample: SampleFile = serde_json::from_str(&raw).expect("parse sample");
    assert_eq!(sample.case_count, sample.cases.len());
    assert!(
        sample.cases.len() >= 200,
        "wide V3 sample should be ≥200 cases, got {}",
        sample.cases.len()
    );

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    let mut failures = Vec::new();

    for c in &sample.cases {
        if c.extract_err.as_ref().map(|e| !e.is_empty()).unwrap_or(false)
            || c.expected_finish.is_none()
        {
            skip += 1;
            continue;
        }
        let expected = c.expected_finish.unwrap();
        let course = get_course(c.course_id).unwrap_or_else(|| panic!("missing course {}", c.course_id));
        let ground = parse_ground(&c.ground);
        let horse = to_horse(c);
        let ours = if c.pace_effects {
            simulate_with_default_pacer(course, ground, &horse, c.seed_u32, PosKeepMode::Virtual)
                .finish_time
        } else {
            simulate_solo(course, ground, &horse, c.seed_u32).finish_time
        };
        let delta = (ours - expected).abs();
        if delta <= DT + 1e-6 {
            pass += 1;
        } else {
            fail += 1;
            failures.push(json!({
                "id": c.id,
                "delta": delta,
                "ours": ours,
                "expected": expected,
                "skills": c.skills,
            }));
        }
    }

    let scored = pass + fail;
    let rate = if scored > 0 {
        pass as f64 / scored as f64
    } else {
        0.0
    };
    let triage = json!({
        "fixture": "race_checkpoint_v3_sample_240.json",
        "pass": pass,
        "fail": fail,
        "skip": skip,
        "pass_rate": rate,
        "min_pass_rate_gate": MIN_PASS_RATE,
        "failures": failures,
    });
    let out = research_path("race_checkpoint_v3_triage_240.json");
    fs::write(&out, serde_json::to_string_pretty(&triage).unwrap()).expect("write triage");
    println!(
        "V3 wide triage: {pass} pass / {fail} fail / {skip} skip (rate={:.1}%) → {}",
        rate * 100.0,
        out.display()
    );

    assert!(
        rate + 1e-9 >= MIN_PASS_RATE,
        "wide V3 pass rate {:.1}% below soft floor {:.0}%",
        rate * 100.0,
        MIN_PASS_RATE * 100.0
    );
}
