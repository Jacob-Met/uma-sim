use std::time::Instant;
use uma_sim_core::run_career_summary;

fn main() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kotlin/golden/summaries.json"
    );
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    println!("loaded {} fixtures", fixtures.len());
    let mut mismatches = 0u32;
    let t0 = Instant::now();
    for (i, f) in fixtures.iter().enumerate() {
        let seed = f["seed"].as_i64().unwrap();
        let scenario = f["scenario"].as_str().unwrap();
        eprint!(
            "[{}/{}] seed={} {} ... ",
            i + 1,
            fixtures.len(),
            seed,
            scenario
        );
        let t = Instant::now();
        let actual = run_career_summary(seed, scenario);
        let ok = actual.turn == f["turn"].as_i64().unwrap() as i32
            && actual.fans == f["fans"].as_i64().unwrap() as i32
            && actual.speed == f["speed"].as_i64().unwrap() as i32
            && actual.sp == f["sp"].as_i64().unwrap() as i32;
        if ok {
            eprintln!("ok ({:?})", t.elapsed());
        } else {
            mismatches += 1;
            eprintln!(
                "MISMATCH ({:?}) got {}/{}/{}/{} expected {}/{}/{}/{}",
                t.elapsed(),
                actual.turn,
                actual.fans,
                actual.speed,
                actual.sp,
                f["turn"],
                f["fans"],
                f["speed"],
                f["sp"]
            );
        }
    }
    println!("DONE mismatches={} elapsed={:?}", mismatches, t0.elapsed());
}
