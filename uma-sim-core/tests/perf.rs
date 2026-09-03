//! Port of PerfTest.kt — timing assertions, ported faithfully (no #[ignore]).

use std::time::Instant;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

#[test]
fn full_career_under_3s_at_x100() {
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 100,
        ..Default::default()
    });
    let start = Instant::now();
    engine.start(RunMeta::new(4242, "ura", "Perf"));
    engine.play_to_completion(500);
    let elapsed = start.elapsed().as_millis();
    assert!(elapsed < 3000, "Expected <3s at x100, got {elapsed}ms");
}

#[test]
fn full_career_under_10s_at_x20() {
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 20,
        ..Default::default()
    });
    let start = Instant::now();
    engine.start(RunMeta::new(4242, "ura", "Perf"));
    engine.play_to_completion(500);
    let elapsed = start.elapsed().as_millis();
    assert!(elapsed < 10_000, "Expected <10s at x20, got {elapsed}ms");
}
