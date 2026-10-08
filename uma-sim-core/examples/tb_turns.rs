use uma_sim_core::policy::default_auto_policy;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

fn main() {
    let mut e = SimEngine::new(SimSettings {
        speed_multiplier: 1,
        ..Default::default()
    });
    e.start(RunMeta::new(2, "trackblazer", "Special Week"));
    let mut last = -1;
    println!("turn,speed,energy,mood,fans,sp,phase,rng");
    loop {
        let s = e.state();
        if s.turn != last {
            println!(
                "{},{},{},{:?},{},{},{},{}",
                s.turn,
                s.stats.speed,
                s.energy,
                s.mood,
                s.fans,
                s.skill_points,
                s.phase,
                e.rng_call_count()
            );
            last = s.turn;
        }
        if s.career_complete {
            break;
        }
        let ch = e.choices();
        if ch.is_empty() {
            break;
        }
        e.step(default_auto_policy(&ch));
    }
}
