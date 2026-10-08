//! Regenerate golden summaries under `race_model=physics` (R8.8 re-freeze).
//!
//! Usage (from repo root):
//!   cargo run -p uma-sim-core --example golden_refreeze_physics
//!
//! Writes `uma-sim-core/tests/fixtures/kotlin/golden/summaries.json`
//! and `research/R88_REFREEZE.md` evidence.

use std::time::Instant;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::{RaceModel, SimEngine};

fn main() {
    let out_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kotlin/golden/summaries.json"
    );
    let note_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/R88_REFREEZE.md");

    let seeds: Vec<i64> = (1..=50).collect();
    let scenarios = ["ura", "grand_concert", "unity", "trackblazer"];
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let t0 = Instant::now();

    for &seed in &seeds {
        for scenario in scenarios {
            eprint!("seed={seed} {scenario} ... ");
            let t = Instant::now();
            let mut engine = SimEngine::new(SimSettings {
                speed_multiplier: 50,
                race_model: RaceModel::Physics,
                ..Default::default()
            });
            engine.start(RunMeta::new(seed, scenario, "Special Week"));
            engine.play_to_completion(500);
            let s = engine.run_summary();
            eprintln!(
                "turn={} fans={} spd={} sp={} ({:?})",
                s.turn,
                s.fans,
                s.speed,
                s.sp,
                t.elapsed()
            );
            rows.push(serde_json::json!({
                "seed": s.seed,
                "scenario": s.scenario,
                "turn": s.turn,
                "fans": s.fans,
                "speed": s.speed,
                "sp": s.sp,
            }));
        }
    }

    let json = serde_json::to_string_pretty(&rows).expect("serialize");
    std::fs::write(out_path, format!("{json}\n")).expect("write summaries.json");

    let note = format!(
        "# R8.8 golden re-freeze (physics default)\n\n\
         **Date:** 2026-09-02\n\n\
         **Why:** Mid-run races now use `uma-race-core` frame-stepped physics \
         (`race_model=physics`). Stub win-by-default is no longer the product default. \
         Career summaries (fans / SP / speed) change because placements are real.\n\n\
         **Invariant preserved:** Race PRNG is derived from `(career_seed, turn, race_id)` \
         with **zero** draws on the career RNG stream — training RNG parity is unchanged; \
         only race outcomes and hooks that depend on placement diverge from stub.\n\n\
         **Command:** `cargo run -p uma-sim-core --example golden_refreeze_physics`\n\n\
         **Fixture:** `uma-sim-core/tests/fixtures/kotlin/golden/summaries.json` \
         ({n} rows = 50 seeds × 4 scenarios).\n\n\
         **Elapsed:** {:?}\n\n\
         **Rollback:** set `race_model=stub` (CLI `--race-model=stub` / env `UMA_RACE_MODEL=stub`) \
         for legacy stub behaviour; stub path remains supported.\n",
        t0.elapsed(),
        n = rows.len()
    );
    std::fs::write(note_path, note).expect("write R88_REFREEZE.md");
    println!(
        "Wrote {} summaries to {out_path}\nNote: {note_path}\nTotal {:?}",
        rows.len(),
        t0.elapsed()
    );
}
