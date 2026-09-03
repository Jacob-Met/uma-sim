use uma_sim_core::run_career_summary;

fn main() {
    for scenario in ["ura", "grand_concert", "unity", "trackblazer"] {
        let g = run_career_summary(42, scenario);
        println!(
            "{}: turn={} fans={} speed={} sp={}",
            scenario, g.turn, g.fans, g.speed, g.sp
        );
    }
}
