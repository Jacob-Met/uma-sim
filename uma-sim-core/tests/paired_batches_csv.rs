use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const HEADER: &str =
    "scenario,trainee,seed,baseline_score,input_score,score_delta,baseline_grade,input_grade\r\n";

struct Fixture(PathBuf);

impl Fixture {
    fn new(input: &[Value], baseline: &[Value]) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "uma-paired-csv-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join(".uma-sim")).unwrap();
        fs::write(
            dir.join(".uma-sim/session.json"),
            b"unread session sentinel\n",
        )
        .unwrap();
        for (name, rows) in [("input.jsonl", input), ("baseline.jsonl", baseline)] {
            let mut bytes = Vec::new();
            for row in rows {
                serde_json::to_writer(&mut bytes, row).unwrap();
                bytes.push(b'\n');
            }
            fs::write(dir.join(name), bytes).unwrap();
        }
        Self(dir)
    }

    fn run(&self) -> Output {
        let before: Vec<_> = ["input.jsonl", "baseline.jsonl", ".uma-sim/session.json"]
            .into_iter()
            .map(|name| {
                let path = self.0.join(name);
                let bytes = fs::read(&path).unwrap();
                let modified = fs::metadata(&path).unwrap().modified().unwrap();
                (path, bytes, modified)
            })
            .collect();
        let output = Command::new(env!("CARGO_BIN_EXE_uma-sim"))
            .current_dir(&self.0)
            .env("UMA_REPO_ROOT", self.0.join("absent-root"))
            .env("UMA_RACE_MODEL", "invalid-unused-setting")
            .args([
                "compare-batches",
                "--input=input.jsonl",
                "--baseline=baseline.jsonl",
                "--format=csv",
            ])
            .output()
            .unwrap();
        for (path, bytes, modified) in before {
            assert_eq!(fs::read(&path).unwrap(), bytes, "{}", path.display());
            assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        }
        output
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn row(scenario: &str, trainee: &str, seed: i64, score: i32, grade: &str) -> Value {
    json!({"scenario": scenario, "trainee": trainee, "seed": seed, "score": score, "grade": grade})
}

#[test]
fn csv_preserves_literal_fields_integer_endpoints_and_identity_order() {
    let scenario = "雨,\"風\"\r\n次";
    let trainee = "\t=literal, \"馬\"\n尾 ";
    let input = [
        row(scenario, trainee, i64::MAX, i32::MIN, "F"),
        row("A", "z", 0, 7, "=literal"),
        row(scenario, trainee, 10, 3, "\t"),
        row(scenario, trainee, i64::MIN, i32::MAX, "=SUM(1,2)\t"),
        row("A", "a", 9, -1, " "),
        row(scenario, trainee, -2, 0, "Q\""),
    ];
    let baseline = [
        row(scenario, trainee, -2, 0, "Q\""),
        row("A", "a", 9, -1, " "),
        row(scenario, trainee, i64::MIN, i32::MIN, "B,\"級\"\r\n"),
        row(scenario, trainee, 10, 1, "\t"),
        row("A", "z", 0, 10, "+literal"),
        row(scenario, trainee, i64::MAX, i32::MAX, "F"),
    ];
    let fixture = Fixture::new(&input, &baseline);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let expected = concat!(
        "scenario,trainee,seed,baseline_score,input_score,score_delta,baseline_grade,input_grade\r\n",
        "\"A\",\"a\",9,-1,-1,0,\" \",\" \"\r\n",
        "\"A\",\"z\",0,10,7,-3,\"+literal\",\"=literal\"\r\n",
        "\"雨,\"\"風\"\"\r\n次\",\"\t=literal, \"\"馬\"\"\n尾 \",-9223372036854775808,-2147483648,2147483647,4294967295,\"B,\"\"級\"\"\r\n\",\"=SUM(1,2)\t\"\r\n",
        "\"雨,\"\"風\"\"\r\n次\",\"\t=literal, \"\"馬\"\"\n尾 \",-2,0,0,0,\"Q\"\"\",\"Q\"\"\"\r\n",
        "\"雨,\"\"風\"\"\r\n次\",\"\t=literal, \"\"馬\"\"\n尾 \",10,1,3,2,\"\t\",\"\t\"\r\n",
        "\"雨,\"\"風\"\"\r\n次\",\"\t=literal, \"\"馬\"\"\n尾 \",9223372036854775807,2147483647,-2147483648,-4294967295,\"F\",\"F\"\r\n"
    );
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn csv_exports_every_pair_beyond_the_text_detail_limit() {
    let input: Vec<_> = (-6..=6)
        .rev()
        .map(|seed| row("s", "t", seed, 11, "B"))
        .collect();
    let baseline: Vec<_> = (-6..=6).map(|seed| row("s", "t", seed, 10, "A")).collect();
    let output = Fixture::new(&input, &baseline).run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let expected = HEADER.to_owned()
        + &(-6..=6)
            .map(|seed| format!("\"s\",\"t\",{seed},10,11,1,\"A\",\"B\"\r\n"))
            .collect::<String>();
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn csv_admits_both_complete_files_before_writing_even_the_header() {
    let valid = row("s", "t", 1, 10, "A");
    for (input, baseline) in [
        (vec![valid.clone(), json!({})], vec![valid.clone()]),
        (vec![valid.clone()], vec![valid.clone(), json!({})]),
        (vec![valid.clone(), valid.clone()], vec![valid.clone()]),
        (vec![valid.clone()], vec![valid.clone(), valid.clone()]),
        (vec![valid.clone()], vec![row("s", "t", 2, 10, "A")]),
        (vec![], vec![valid.clone()]),
        (vec![valid.clone()], vec![]),
    ] {
        let output = Fixture::new(&input, &baseline).run();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("compare-batches:"));
    }
}
