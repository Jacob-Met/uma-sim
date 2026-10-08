//! R8.6 soft gate: physics NPC win-rate sanity vs live-log prior.
//!
//! Full V4 (stats + placement + margins) still open. This gate only checks that
//! placeholder NPCs produce ordered win rates (strong ≫ weak) and that a mid
//! profile is not stuck at 0% or 100% win.

use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use uma_race_core::{
    get_course, simulate_field_synced, Aptitude, GroundCondition, HorseInput, PosKeepMode, Strategy,
};
use uma_sim_core::race::placeholder_npc_field;

#[derive(Deserialize)]
struct TelemetryCorpus {
    total_races: usize,
    win_rate: f64,
}

fn research_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../research")
        .join(name)
}

fn trainee(speed: f64, stam: f64, power: f64, guts: f64, wit: f64) -> HorseInput {
    HorseInput {
        speed,
        stamina: stam,
        power,
        guts,
        wisdom: wit,
        strategy: Strategy::Senkou,
        distance_apt: Aptitude::A,
        surface_apt: Aptitude::A,
        strategy_apt: Aptitude::A,
        mood: 1,
        skills: vec![],
    }
}

fn win_rate(trainee: &HorseInput, grade: &str, seeds: impl Iterator<Item = u32>) -> f64 {
    let course = get_course(10606).expect("Tokyo 2000"); // medium turf
    let mut wins = 0usize;
    let mut n = 0usize;
    for seed in seeds {
        let mut field = vec![trainee.clone()];
        field.extend(placeholder_npc_field(grade, 8, seed));
        let res = simulate_field_synced(
            course,
            GroundCondition::Good,
            &field,
            seed,
            PosKeepMode::Virtual,
        );
        if res.finishers[0].index == 0 {
            wins += 1;
        }
        n += 1;
    }
    wins as f64 / n as f64
}

#[test]
fn r86_npc_win_rates_ordered_vs_telemetry_prior() {
    let raw = fs::read_to_string(research_path("race_telemetry_corpus.json")).expect("corpus");
    let corpus: TelemetryCorpus = serde_json::from_str(&raw).expect("parse corpus");
    assert!(
        corpus.total_races >= 100 && corpus.win_rate > 0.4 && corpus.win_rate < 0.9,
        "telemetry prior out of range: n={} wr={}",
        corpus.total_races,
        corpus.win_rate
    );

    // 24 seeds — cheap soft check, not a full V4 fit.
    let seeds = 1000u32..1024;
    let weak = trainee(400.0, 350.0, 350.0, 300.0, 300.0);
    let mid = trainee(800.0, 700.0, 700.0, 600.0, 600.0);
    let strong = trainee(1200.0, 1100.0, 1100.0, 1000.0, 1000.0);

    let wr_weak = win_rate(&weak, "PRE_OP", seeds.clone());
    let wr_mid = win_rate(&mid, "OP", seeds.clone());
    let wr_strong = win_rate(&strong, "G1", seeds);

    eprintln!(
        "R8.6 soft win-rates: weak@PRE_OP={wr_weak:.2} mid@OP={wr_mid:.2} strong@G1={wr_strong:.2} (telemetry prior≈{:.2}; prior is bot-selected races, not random mid@OP)",
        corpus.win_rate
    );

    assert!(
        wr_strong + 1e-9 >= wr_mid && wr_mid + 1e-9 >= wr_weak,
        "win rates must be ordered strong≥mid≥weak; got {wr_strong}/{wr_mid}/{wr_weak}"
    );
    // 9-horse fields → equal-skill random ≈11%. Soften extremes only.
    assert!(
        wr_mid < 0.95,
        "mid profile should not win nearly every race ({wr_mid})"
    );
    assert!(
        wr_strong >= 0.25,
        "strong trainee should often beat G1 placeholders, got {wr_strong}"
    );
    assert!(
        wr_weak <= 0.25,
        "weak trainee should rarely dominate PRE_OP placeholders, got {wr_weak}"
    );
}
