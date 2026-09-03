//! Rewrite Rust-owned rng/turn parity fixtures after intentional engine changes.
//!
//!   cargo run -p uma-sim-core --example refreeze_parity_traces --release
//!
//! Optional args: scenarios (default: grand_concert only).
use std::fs;
use std::path::PathBuf;
use uma_sim_core::{run_rng_trace_fixture, run_turn_trace_fixture};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scenarios: Vec<&str> = if args.is_empty() {
        vec!["grand_concert"]
    } else {
        args.iter().map(|s| s.as_str()).collect()
    };
    let seeds = [1i64, 42, 7];
    let dir = fixtures_dir();
    for seed in seeds {
        for scenario in &scenarios {
            let rng = run_rng_trace_fixture(seed, scenario);
            let rng_path = dir.join(format!("rng_trace_{seed}_{scenario}.json"));
            fs::write(
                &rng_path,
                serde_json::to_string_pretty(&rng).expect("serialize rng") + "\n",
            )
            .expect("write rng");
            println!("wrote {}", rng_path.display());

            let turn = run_turn_trace_fixture(seed, scenario);
            let turn_path = dir.join(format!("turn_trace_{seed}_{scenario}.json"));
            fs::write(
                &turn_path,
                serde_json::to_string_pretty(&turn).expect("serialize turn") + "\n",
            )
            .expect("write turn");
            println!("wrote {}", turn_path.display());
        }
    }
}
