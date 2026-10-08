use serde_json::json;
use std::io::{self, Read};
use uma_sim_core::batch_pairs::{compare, render_text, PairedBatch, MAX_INPUT_BYTES};

fn record(seed: i64, scenario: &str, trainee: &str, score: i32, grade: &str) -> String {
    json!({"seed":seed,"scenario":scenario,"trainee":trainee,"score":score,"grade":grade})
        .to_string()
}

fn batch(lines: &[String]) -> PairedBatch {
    PairedBatch::parse(&lines.join("\n"), "fixture").unwrap()
}

#[test]
fn matches_complete_identities_instead_of_positions_or_seed_alone() {
    let baseline = batch(&[
        record(8, "ura", "A", 100, "C"),
        record(-2, "ura", "A", 50, "F"),
        record(8, "ura", "B", 70, "E"),
        record(8, "unity", "A", 80, "D"),
    ]);
    let input = batch(&[
        record(8, "ura", "B", 75, "D"),
        record(8, "unity", "A", 80, "D"),
        record(8, "ura", "A", 110, "B"),
        record(-2, "ura", "A", 49, "F"),
    ]);
    let report = compare(&input, &baseline).unwrap();
    assert_eq!(report.summary.count, 4);
    assert_eq!(
        (
            report.summary.wins,
            report.summary.ties,
            report.summary.losses
        ),
        (2, 1, 1)
    );
    assert_eq!(report.summary.mean_score_delta, 3.5);
    assert_eq!(report.summary.median_score_delta, 2.5);
    assert_eq!(report.summary.min_score_delta, -1);
    assert_eq!(report.summary.max_score_delta, 10);
    assert!((report.summary.population_stddev_score_delta - 19.25_f64.sqrt()).abs() < 1e-12);
    let identities: Vec<_> = report
        .comparisons
        .iter()
        .map(|p| {
            (
                p.scenario.as_str(),
                p.trainee.as_str(),
                p.seed,
                p.score_delta,
            )
        })
        .collect();
    assert_eq!(
        identities,
        vec![
            ("unity", "A", 8, 0),
            ("ura", "A", -2, -1),
            ("ura", "A", 8, 10),
            ("ura", "B", 8, 5)
        ]
    );
    assert_eq!(report.comparisons[2].baseline_grade, "C");
    assert_eq!(report.comparisons[2].input_grade, "B");
    assert_eq!(
        serde_json::to_value(&report).unwrap()["format"],
        "uma-sim.paired-batches.v1"
    );
}

#[test]
fn odd_median_and_full_signed_integer_endpoints_are_exact() {
    let baseline = batch(&[
        record(i64::MIN, "ura", "A", i32::MAX, "A"),
        record(0, "ura", "A", 7, "A"),
        record(i64::MAX, "ura", "A", i32::MIN, "A"),
    ]);
    let input = batch(&[
        record(i64::MAX, "ura", "A", i32::MAX, "A"),
        record(0, "ura", "A", 7, "A"),
        record(i64::MIN, "ura", "A", i32::MIN, "A"),
    ]);
    let report = compare(&input, &baseline).unwrap();
    assert_eq!(report.summary.min_score_delta, -4_294_967_295);
    assert_eq!(report.summary.max_score_delta, 4_294_967_295);
    assert_eq!(report.summary.mean_score_delta, 0.0);
    assert_eq!(report.summary.median_score_delta, 0.0);
    assert_eq!(report.comparisons[0].seed, i64::MIN);
    assert_eq!(report.comparisons[2].seed, i64::MAX);
}

#[test]
fn incomplete_or_duplicate_cohorts_refuse_in_both_directions() {
    let first = record(1, "ura", "A", 10, "F");
    let second = record(2, "ura", "A", 20, "F");
    let one = batch(std::slice::from_ref(&first));
    let two = batch(&[first.clone(), second]);
    assert!(compare(&one, &two)
        .unwrap_err()
        .contains("no input partner"));
    assert!(compare(&two, &one)
        .unwrap_err()
        .contains("no baseline partner"));
    assert!(
        PairedBatch::parse(&format!("{first}\n{first}\n"), "fixture")
            .unwrap_err()
            .contains("duplicate career")
    );
}

#[test]
fn every_required_field_is_strict_and_late_bad_records_are_not_skipped() {
    let good = json!({"seed":4,"scenario":"ura","trainee":"A","grade":"F","score":123});
    for field in ["seed", "scenario", "trainee", "grade", "score"] {
        let mut missing = good.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            PairedBatch::parse(&missing.to_string(), "fixture").is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("seed", json!(1.0)),
        ("seed", json!("1")),
        ("score", json!(1.0)),
        ("score", json!(2_147_483_648_i64)),
        ("score", json!(-2_147_483_649_i64)),
        ("trainee", json!(null)),
        ("scenario", json!([])),
    ] {
        let mut invalid = good.clone();
        invalid[field] = value;
        assert!(
            PairedBatch::parse(&invalid.to_string(), "fixture").is_err(),
            "{field}"
        );
    }
    for suffix in ["not-json", "{}", "[]", "null", "{\"seed\":1,\"seed\":2,\"scenario\":\"ura\",\"trainee\":\"A\",\"grade\":\"F\",\"score\":1}"] {
        assert!(PairedBatch::parse(&format!("{good}\n{suffix}"), "fixture").is_err());
    }
    for empty in ["", " \r\n\t\n"] {
        assert!(PairedBatch::parse(empty, "fixture").is_err());
    }
}

#[test]
fn string_limits_count_utf8_bytes_and_retain_literal_whitespace() {
    let exact = record(1, &"é".repeat(128), &"馬".repeat(85), 0, &"é".repeat(32));
    assert_eq!(batch(&[exact]).len(), 1);
    for bad in [
        record(1, &"é".repeat(129), "A", 0, "F"),
        record(1, "ura", &"馬".repeat(86), 0, "F"),
        record(1, "ura", "A", 0, &"é".repeat(33)),
        record(1, "", "A", 0, "F"),
        record(1, "ura", "", 0, "F"),
        record(1, "ura", "A", 0, ""),
    ] {
        assert!(PairedBatch::parse(&bad, "fixture").is_err());
    }
    let literal = batch(&[record(1, " ", "\t\n", 0, " ")]);
    let report = compare(&literal, &literal).unwrap();
    assert_eq!(report.comparisons[0].trainee, "\t\n");
    assert!(render_text(&report).contains("[\" \",\"\\t\\n\",1]"));
}

#[test]
fn reader_bounds_utf8_and_io_errors_are_reported() {
    let record = record(1, "ura", "A", 0, "F");
    let mut exact = record.into_bytes();
    exact.resize(MAX_INPUT_BYTES, b' ');
    assert_eq!(
        PairedBatch::from_reader(exact.as_slice(), "fixture")
            .unwrap()
            .len(),
        1
    );
    exact.push(b' ');
    assert!(PairedBatch::from_reader(exact.as_slice(), "fixture")
        .unwrap_err()
        .contains("exceeds"));
    assert!(PairedBatch::from_reader([0xff].as_slice(), "fixture")
        .unwrap_err()
        .contains("UTF-8"));
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("authored read failure"))
        }
    }
    assert!(PairedBatch::from_reader(Broken, "fixture")
        .unwrap_err()
        .contains("authored read failure"));
}

#[test]
fn text_ranks_ten_largest_absolute_deltas_and_json_keeps_all_pairs() {
    let baseline: Vec<_> = (-6..=6)
        .rev()
        .map(|seed| record(seed, "ura", "A", 20, "F"))
        .collect();
    let input: Vec<_> = (-6..=6)
        .map(|seed| record(seed, "ura", "A", 20 + seed as i32, "F"))
        .collect();
    let report = compare(&batch(&input), &batch(&baseline)).unwrap();
    let text = render_text(&report);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 15);
    assert_eq!(lines[1], "Pairs: 13; wins: 6; ties: 1; losses: 6");
    assert_eq!(lines[5], "-6\t20\t14\t[\"ura\",\"A\",-6]");
    assert_eq!(lines[6], "+6\t20\t26\t[\"ura\",\"A\",6]");
    assert_eq!(lines[14], "+2\t20\t22\t[\"ura\",\"A\",2]");
    assert!(text.ends_with('\n'));
    assert_eq!(
        serde_json::to_value(&report).unwrap()["comparisons"]
            .as_array()
            .unwrap()
            .len(),
        13
    );
}
