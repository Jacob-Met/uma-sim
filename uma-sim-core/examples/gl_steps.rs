use std::time::Instant;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::{SimEngine, policy::default_auto_policy};
fn main() {
    let mut e = SimEngine::new(SimSettings { speed_multiplier: 50, ..Default::default() });
    e.start(RunMeta::new(42, "grand_concert", "Special Week"));
    for i in 0..20 {
        let t = Instant::now();
        let ch = e.choices();
        if ch.is_empty() { println!("empty choices turn={}", e.state().turn); break; }
        e.step(default_auto_policy(&ch));
        println!("step {i} turn={} phase={} elapsed={:?}", e.state().turn, e.state().phase, t.elapsed());
        if e.state().career_complete { break; }
    }
}
