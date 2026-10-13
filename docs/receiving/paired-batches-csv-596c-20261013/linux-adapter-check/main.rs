extern crate self as uma_sim_core;
#[path = "../candidate/uma-sim-core/src/batch_pairs.rs"] pub mod batch_pairs;
#[path = "../candidate/uma-sim-core/src/bin/uma_sim/paired_batches.rs"] mod paired_batches;
fn main() { let args: Vec<String> = std::env::args().skip(2).collect(); std::process::exit(paired_batches::run(&args)); }
