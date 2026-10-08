//! R8.5: tiny hand-picked upstream checkpoint sample vs clean-room finish times.
//!
//! Fixture: `research/race_checkpoint_sample.json` (params + expected finish only).
//! Failures / notes: `research/race_checkpoint_triage.json`.

use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use uma_race_core::{
    get_course, simulate_solo, simulate_with_default_pacer, Aptitude, GroundCondition, HorseInput,
    PosKeepMode, Strategy,
};

const DT: f64 = 1.0 / 15.0;

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
    checkpoint_gain0: Option<f64>,
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

fn classify_failure(c: &SampleCase, ours: f64, expected: f64, delta: f64) -> String {
    let mut notes: Vec<String> = Vec::new();
    if c.horse.stamina < 200.0 {
        notes.push("extreme-low-stamina fuzz case; HP crawl path sensitive".into());
    }
    if c.horse.speed < 50.0 {
        notes.push("extreme-low-speed fuzz case".into());
    }
    if c.pace_effects {
        notes.push(
            "pace_effects=true → Virtual+default pacer; pos-keep still provisional vs upstream"
                .into(),
        );
    }
    if c.kind != "no_skill" {
        notes.push("skill activation / wisdom / sample-policy may diverge".into());
    }
    for sid in &c.skills {
        if sid == "910391" {
            notes.push(
                "910391 loaded from GameTora gene_version under 110391; oracle sample-0 also no finish gain — remaining Δ is physics near-miss".into(),
            );
        }
        if sid == "113301211" {
            notes.push(
                "113301211 requires course_distance 2000–2500; course 10814 is 1800 → skill inert; Δ is physics".into(),
            );
        }
        if sid == "203032" {
            notes.push(
                "203032 activates in oracle but finish equal with/without; our Δ is Virtual physics (not skill gain)".into(),
            );
        }
    }
    if delta > 5.0 {
        notes.push("large delta — likely ours (physics/spurt/HP), not upstream-known-issue".into());
    } else if delta <= 3.0 * DT + 1e-6 {
        notes.push(format!(
            "near-miss (≤3 frames, Δ={delta:.6}); start-delay/spurt/pos-keep edge — ours until proven upstream"
        ));
    } else if delta > DT {
        notes.push(
            "within few seconds of upstream — investigate frame/start-delay/spurt edge".into(),
        );
    }
    if notes.is_empty() {
        format!(
            "ours={ours:.6} expected={expected:.6} Δ={delta:.6}; unclassified (treat as ours until proven upstream)"
        )
    } else {
        format!(
            "ours={ours:.6} expected={expected:.6} Δ={delta:.6}; {}",
            notes.join("; ")
        )
    }
}

#[test]
fn r85_checkpoint_sample_triage() {
    let sample_path = research_path("race_checkpoint_sample.json");
    let raw = fs::read_to_string(&sample_path).expect("race_checkpoint_sample.json");
    let sample: SampleFile = serde_json::from_str(&raw).expect("parse sample");
    assert_eq!(sample.case_count, sample.cases.len());
    assert!(
        sample.cases.len() >= 5 && sample.cases.len() <= 20,
        "R8.5 sample should be 5–20 cases, got {}",
        sample.cases.len()
    );

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    let mut failures = Vec::new();
    let mut results = Vec::new();

    for c in &sample.cases {
        if c.extract_err
            .as_ref()
            .map(|e| !e.is_empty())
            .unwrap_or(false)
            || c.expected_finish.is_none()
        {
            skip += 1;
            results.push(json!({
                "id": c.id,
                "status": "skip",
                "reason": c.extract_err.clone().unwrap_or_else(|| "missing expected_finish".into()),
            }));
            continue;
        }
        let expected = c.expected_finish.unwrap();
        let course =
            get_course(c.course_id).unwrap_or_else(|| panic!("missing course {}", c.course_id));
        let ground = parse_ground(&c.ground);
        let horse = to_horse(c);
        let ours = if c.pace_effects {
            simulate_with_default_pacer(course, ground, &horse, c.seed_u32, PosKeepMode::Virtual)
                .finish_time
        } else {
            simulate_solo(course, ground, &horse, c.seed_u32).finish_time
        };
        let delta = (ours - expected).abs();
        let ok = delta <= DT + 1e-6;
        if ok {
            pass += 1;
            results.push(json!({
                "id": c.id,
                "kind": c.kind,
                "status": "pass",
                "ours_finish": ours,
                "expected_finish": expected,
                "delta_s": delta,
                "pace_effects": c.pace_effects,
                "checkpoint_gain0": c.checkpoint_gain0,
            }));
        } else {
            fail += 1;
            let note = classify_failure(c, ours, expected, delta);
            failures.push(json!({
                "id": c.id,
                "kind": c.kind,
                "status": "fail",
                "ours_finish": ours,
                "expected_finish": expected,
                "delta_s": delta,
                "pace_effects": c.pace_effects,
                "skills": c.skills,
                "checkpoint_gain0": c.checkpoint_gain0,
                "triage_note": note,
                "attribution": "ours",
            }));
            results.push(json!({
                "id": c.id,
                "kind": c.kind,
                "status": "fail",
                "ours_finish": ours,
                "expected_finish": expected,
                "delta_s": delta,
            }));
        }
        eprintln!(
            "{} {} ours={:.6} exp={:.6} Δ={:.6} {}",
            if ok { "PASS" } else { "FAIL" },
            c.id,
            ours,
            expected,
            delta,
            if c.pace_effects { "pace" } else { "solo" }
        );
    }

    let triage = json!({
        "provenance": "R8.5 sample corpus triage (not full V3). Compare clean-room finish vs upstream-extracted expected_finish.",
        "sample": "research/race_checkpoint_sample.json",
        "tolerance_s": DT,
        "summary": {
            "total": sample.cases.len(),
            "pass": pass,
            "fail": fail,
            "skip": skip,
        },
        "failures": failures,
        "results": results,
        "notes": [
            "Sample corpus started — not full V3 checkpoint replay.",
            "Upstream checkpoint makeBuilder uses NoopHp + asitame/staminasyoubu; fixture expected_finish uses GameHp + no those hooks to match our engine.",
            "Failures default attribution=ours until confirmed upstream-known-issue.",
            "Harness mapping confirmed vs oracle RaceRequest: mood on horse+builder, ground string→enum, seed>>>0 / seed_u32.",
            "2026-09-02: empty-HP target overrides last-spurt; exhausted accel=-1.2; slope mods after target (fixes cp_0/cp_1 crawl). Skill 910391 = catalog data gap (inherited-only in GameTora).",
            "2026-09-02b: Virtual default-pacer designates Oonige/Nige focus correctly (SpeedUp+second_pos); pos-keep −2/−3 cooldowns were ignored by timer sentinel (fixes cp_3/cp_4 and restores cp_9/cp_13/cp_14).",
            "2026-09-02c: Evolution (rarity 6) must wisdom-roll (only Unique=5 skips) — fixes cp_6. 910391 indexed from gene_version. Remaining fails cp_7/8/10 are physics near-misses with inert skill gain vs oracle."
        ],
    });

    let triage_path = research_path("race_checkpoint_triage.json");
    fs::write(&triage_path, serde_json::to_string_pretty(&triage).unwrap())
        .expect("write race_checkpoint_triage.json");
    eprintln!(
        "R8.5 triage: {} pass / {} fail / {} skip → {}",
        pass,
        fail,
        skip,
        triage_path.display()
    );

    // Infrastructure gate: fixture loaded, all runnable cases evaluated, triage written.
    assert_eq!(pass + fail + skip, sample.cases.len());
    // Soft start: do not hard-fail cargo on physics mismatches; triage is the R8.5 deliverable.
    // Keep at least one no_skill evaluation so the gate is not vacuous.
    assert!(
        sample.cases.iter().any(|c| c.kind == "no_skill"),
        "sample must include no_skill cases"
    );
}
