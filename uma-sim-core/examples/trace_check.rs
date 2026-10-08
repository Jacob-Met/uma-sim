use std::time::Instant;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::{run_career_summary, run_rng_trace_fixture, SimEngine};

fn main() {
    let t = Instant::now();
    let s = run_career_summary(42, "ura");
    println!("summary speed50: {:?} in {:?}", s, t.elapsed());

    let t = Instant::now();
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 1,
        ..Default::default()
    });
    engine.start(RunMeta::new(42, "ura", "Special Week"));
    engine.play_to_completion(500);
    println!(
        "play speed1 turn={} complete={} in {:?}",
        engine.state().turn,
        engine.state().career_complete,
        t.elapsed()
    );

    for seed in [1i64, 42, 7] {
        for scenario in ["ura", "grand_concert", "unity", "trackblazer"] {
            let t = Instant::now();
            let trace = run_rng_trace_fixture(seed, scenario);
            println!(
                "seed={seed} {scenario}: entries={} complete in {:?}",
                trace.entries.len(),
                t.elapsed()
            );
        }
    }
}
