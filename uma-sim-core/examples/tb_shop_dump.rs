use uma_sim_core::scenario::TrackblazerMechanics;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

fn main() {
    let mut e = SimEngine::new(SimSettings {
        speed_multiplier: 1,
        ..Default::default()
    });
    e.start(RunMeta::new(2, "trackblazer", "Special Week"));
    for _ in 0..500 {
        if e.state().career_complete {
            break;
        }
        let title = e.state().pending_event_title.clone();
        if title.as_deref() == Some("Trackblazer Pro Shop") {
            println!(
                "turn={} coins={} options:",
                e.state().turn,
                e.state().scenario_resources.get("tb_coins")
            );
            for (i, opt) in e.state().pending_event_options.iter().enumerate() {
                println!("  [{i}] {}", opt.replace('\n', " | "));
            }
        }
        let ch = e.choices();
        if ch.is_empty() {
            break;
        }
        e.auto_step_with_policy(uma_sim_core::policy::default_auto_policy);
    }
    let s = e.state();
    println!(
        "final speed={} fans={} sp={} levels={:?}",
        s.stats.speed, s.fans, s.skill_points, s.facility_levels
    );
}
