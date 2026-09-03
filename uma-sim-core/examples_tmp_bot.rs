mod common;
use common::{base_state, stats5, with_resources};
use std::collections::HashMap;
use uma_sim_core::{scenario_plugin_for, BotDecisionAdapter, TrainingResolver};

fn main() {
    let resolver = TrainingResolver::default();
    let plugin = scenario_plugin_for("grand_concert");
    for (label, speed, stam, power, guts, wit) in [
        ("stam_lag", 400, 120, 400, 400, 400),
        ("power_lag", 500, 500, 200, 500, 500),
        ("speed_lag", 150, 300, 300, 300, 300),
    ] {
        let mut state = base_state("grand_concert", 15);
        state.stats = stats5(speed, stam, power, guts, wit);
        state.energy = 80;
        state = with_resources(state, HashMap::from([
            ("hype".into(), 1),
            ("great_success_required".into(), 3),
        ]));
        let fac = BotDecisionAdapter::choose_training_facility(&state, plugin.as_ref(), &resolver);
        println!("{label} -> {}", fac.key());
    }
}
