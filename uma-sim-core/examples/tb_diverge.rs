use uma_sim_core::{run_turn_trace_fixture, TurnTraceFixture};
fn main() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/turn_trace_2_trackblazer.json"
    );
    let expected: TurnTraceFixture =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let actual = run_turn_trace_fixture(2, "trackblazer");
    for (i, (e, a)) in expected
        .snapshots
        .iter()
        .zip(actual.snapshots.iter())
        .enumerate()
    {
        if e != a {
            println!("first diverge at index {i} turn={}", e.turn);
            println!(
                " expected: turn={} energy={} mood={} fans={} sp={} speed={} phase={} rng={}",
                e.turn,
                e.energy,
                e.mood,
                e.fans,
                e.skill_points,
                e.speed,
                e.phase,
                e.rng_call_count
            );
            println!(
                " actual:   turn={} energy={} mood={} fans={} sp={} speed={} phase={} rng={}",
                a.turn,
                a.energy,
                a.mood,
                a.fans,
                a.skill_points,
                a.speed,
                a.phase,
                a.rng_call_count
            );
            println!(" exp res: {:?}", e.scenario_resources);
            println!(" act res: {:?}", a.scenario_resources);
            return;
        }
    }
    println!(
        "all matched len_exp={} len_act={}",
        expected.snapshots.len(),
        actual.snapshots.len()
    );
}
