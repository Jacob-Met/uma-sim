//! Thin wrappers mirroring Kotlin `GoldenSeedTest` names.
//! Overlaps `golden_seeds.rs` — do not rewrite that file.

use uma_sim_core::{run_career_summary, CAREER_TURNS};

const SEEDS: std::ops::RangeInclusive<i64> = 1..=50;
const SCENARIOS: [&str; 4] = ["ura", "grand_concert", "unity", "trackblazer"];

#[test]
fn golden_seed_42_ura() {
    let a = run_career_summary(42, "ura");
    let b = run_career_summary(42, "ura");
    assert_eq!(a, b);
    assert_eq!(a.turn, CAREER_TURNS);
}

#[test]
fn all_fifty_seeds_reproducible() {
    for seed in SEEDS {
        for scenario in SCENARIOS {
            assert_eq!(
                run_career_summary(seed, scenario),
                run_career_summary(seed, scenario),
                "seed={seed} scenario={scenario}"
            );
        }
    }
}

#[test]
fn fixture_file_matches_engine() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kotlin/golden/summaries.json"
    );
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return, // Kotlin skips if resource not committed
    };
    #[derive(serde::Deserialize)]
    struct FixtureSummary {
        seed: i64,
        scenario: String,
        turn: i32,
        fans: i32,
        speed: i32,
        sp: i32,
    }
    let expected: Vec<FixtureSummary> = serde_json::from_str(&raw).expect("parse summaries");
    assert!(
        expected.len() >= 200,
        "Expected 50×4 fixtures, got {}",
        expected.len()
    );
    for exp in &expected {
        let actual = run_career_summary(exp.seed, &exp.scenario);
        assert_eq!(
            (
                actual.seed,
                actual.scenario.as_str(),
                actual.turn,
                actual.fans,
                actual.speed,
                actual.sp
            ),
            (
                exp.seed,
                exp.scenario.as_str(),
                exp.turn,
                exp.fans,
                exp.speed,
                exp.sp
            ),
            "seed={} scenario={}",
            exp.seed,
            exp.scenario
        );
    }
}
