use uma_sim_core::batch_analysis::{analyze_text, render_text};

fn record(seed: i64, score: i32, before: i32) -> String {
    serde_json::json!({
        "seed": seed,
        "scenario": "ura",
        "grade": "F",
        "score": score,
        "score_before_shop": before
    })
    .to_string()
}

#[test]
fn shop_gain_retains_the_full_positive_i32_difference() {
    let summary = analyze_text(&record(1, i32::MAX, i32::MIN), 1).unwrap();
    assert_eq!(summary.n, 1);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.shop_gain_mean, 4_294_967_295.0);
    assert_eq!(summary.score_mean, f64::from(i32::MAX));
    assert_eq!(summary.top[0].score, i32::MAX);
    assert!(
        render_text(&summary, None, "extreme.jsonl").contains("mean gain 4294967295.0 pts/career")
    );
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["shop_gain_mean"].as_f64(), Some(4_294_967_295.0));
}

#[test]
fn shop_gain_retains_the_full_negative_i32_difference() {
    let summary = analyze_text(&record(2, i32::MIN, i32::MAX), 0).unwrap();
    assert_eq!(summary.shop_gain_mean, -4_294_967_295.0);
    assert_eq!(summary.score_min, i32::MIN);
    assert!(summary.top.is_empty());
}

#[test]
fn mixed_extreme_and_ordinary_gains_keep_their_mean() {
    let input = [
        record(3, i32::MAX, i32::MIN),
        record(4, i32::MIN, i32::MAX),
        record(5, 420, 300),
    ]
    .join("\n");
    let summary = analyze_text(&input, 3).unwrap();
    assert_eq!(summary.n, 3);
    assert_eq!(summary.shop_gain_mean, 40.0);
    assert_eq!(summary.top[0].seed, 3);
    assert_eq!(summary.top[1].seed, 5);
    assert_eq!(summary.top[2].seed, 4);
}

#[test]
fn absent_shop_score_and_malformed_records_keep_existing_semantics() {
    let input = concat!(
        "{\"seed\":6,\"scenario\":\"ura\",\"grade\":\"F\",\"score\":2147483647}\n",
        "{\"seed\":7,\"scenario\":\"ura\",\"grade\":\"F\",\"score\":2147483648}\n",
        "{\"seed\":8,\"scenario\":\"ura\",\"grade\":\"F\",\"score\":0,\"score_before_shop\":-2147483649}\n",
        "not JSON\n\n"
    );
    let summary = analyze_text(input, 1).unwrap();
    assert_eq!(summary.n, 1);
    assert_eq!(summary.skipped, 3);
    assert_eq!(summary.shop_gain_mean, f64::from(i32::MAX));
    assert_eq!(summary.seed_min, 6);
    assert_eq!(summary.seed_max, 6);
}

#[test]
fn equal_extreme_scores_have_zero_shop_gain() {
    let input = [
        record(9, i32::MAX, i32::MAX),
        record(10, i32::MIN, i32::MIN),
    ]
    .join("\n");
    let summary = analyze_text(&input, 0).unwrap();
    assert_eq!(summary.shop_gain_mean, 0.0);
    assert_eq!(summary.score_mean, -0.5);
}
