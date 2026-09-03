use uma_sim_core::{run_career_summary, GoldenSummary};

#[derive(serde::Deserialize)]
struct FixtureSummary {
    seed: i64,
    scenario: String,
    turn: i32,
    fans: i32,
    speed: i32,
    sp: i32,
}

fn to_golden(f: &FixtureSummary) -> GoldenSummary {
    GoldenSummary {
        seed: f.seed,
        scenario: f.scenario.clone(),
        turn: f.turn,
        fans: f.fans,
        speed: f.speed,
        sp: f.sp,
    }
}

#[test]
fn seed_42_ura_reproducible() {
    let a = run_career_summary(42, "ura");
    let b = run_career_summary(42, "ura");
    assert_eq!(a, b);
    assert_eq!(a.turn, 72);
}

#[test]
fn kotlin_golden_fixture_seed_42_ura() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kotlin/golden/summaries.json"
    );
    let raw = std::fs::read_to_string(path).expect("summaries.json");
    let fixtures: Vec<FixtureSummary> = serde_json::from_str(&raw).expect("parse");
    let expected = fixtures
        .iter()
        .find(|f| f.seed == 42 && f.scenario == "ura")
        .expect("seed 42 ura fixture");
    let actual = run_career_summary(42, "ura");
    assert_eq!(
        actual,
        to_golden(expected),
        "Rust engine must match Kotlin golden for seed=42 ura"
    );
}

#[test]
fn all_kotlin_golden_summaries() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kotlin/golden/summaries.json"
    );
    let raw = std::fs::read_to_string(path).expect("summaries.json");
    let fixtures: Vec<FixtureSummary> = serde_json::from_str(&raw).expect("parse");
    assert!(fixtures.len() >= 200);
    for exp in &fixtures {
        let actual = run_career_summary(exp.seed, &exp.scenario);
        assert_eq!(actual, to_golden(exp), "seed={} scenario={}", exp.seed, exp.scenario);
    }
}
