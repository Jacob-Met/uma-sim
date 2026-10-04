//! Batch result analysis: turn `uma-sim batch` JSONL output into decision-ready
//! aggregate statistics.
//!
//! The simulator's core promise is comparing decks, policies and rule changes
//! run against run, but `batch` only emits raw JSON lines. This module closes
//! that loop: it reads the same lines `cmd_batch` writes and computes
//! descriptive statistics (score distribution, grade histogram, terminal-stat
//! means, top careers) plus an optional two-file comparison, so a policy or
//! deck change can be evaluated with one command instead of a hand-rolled
//! script.
//!
//! The record type is intentionally decoupled from the engine's
//! `CareerTerminalRecord`: analysis only needs the serialized shape, never
//! engine internals.

use std::collections::BTreeMap;

/// One career terminal record as written by `uma-sim batch` (JSONL, one
/// record per line). `score` and `grade` are required: a line missing either
/// is not analyzable and counts as skipped. Everything else tolerates absence
/// so older/newer batch files still analyze.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchRecord {
    pub seed: i64,
    pub scenario: String,
    pub grade: String,
    pub score: i32,
    #[serde(default)]
    pub score_before_shop: i32,
    #[serde(default)]
    pub u: f64,
    #[serde(default)]
    pub sp_spent: i32,
    #[serde(default)]
    pub stats: BatchStats,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct BatchStats {
    #[serde(default)]
    pub speed: i32,
    #[serde(default)]
    pub stamina: i32,
    #[serde(default)]
    pub power: i32,
    #[serde(default)]
    pub guts: i32,
    #[serde(default)]
    pub wit: i32,
}

/// One of the best careers in a batch, for drilling into interesting seeds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopCareer {
    pub seed: i64,
    pub grade: String,
    pub score: i32,
}

/// Aggregate statistics over one batch file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Summary {
    pub n: usize,
    pub skipped: usize,
    pub seed_min: i64,
    pub seed_max: i64,
    pub score_mean: f64,
    pub score_median: f64,
    pub score_min: i32,
    pub score_max: i32,
    pub score_stddev: f64,
    pub grades: BTreeMap<String, usize>,
    pub u_mean: f64,
    pub u_nonzero_frac: f64,
    pub sp_spent_mean: f64,
    /// Mean (score - score_before_shop): what the skill shop added on average.
    pub shop_gain_mean: f64,
    pub stat_means: BTreeMap<String, f64>,
    pub top: Vec<TopCareer>,
}

/// Delta of the analyzed input relative to a baseline file for `--compare`
/// runs. Positive deltas mean the input is better than the baseline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Comparison {
    pub baseline_n: usize,
    pub input_n: usize,
    pub score_mean_delta: f64,
    pub score_median_delta: f64,
    pub u_mean_delta: f64,
    pub sp_spent_mean_delta: f64,
    /// Per-grade count shift (input minus baseline); grades present in only
    /// one side appear with the other side counted as zero.
    pub grade_shifts: BTreeMap<String, i64>,
}

/// Analyze the full text of a batch JSONL file. Blank lines are ignored;
/// malformed lines (or lines missing `score`/`grade`) are skipped and
/// counted in `Summary.skipped` instead of aborting the run.
pub fn analyze_text(input: &str, top_n: usize) -> Result<Summary, String> {
    let mut records: Vec<BatchRecord> = Vec::new();
    let mut skipped = 0usize;
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<BatchRecord>(line) {
            Ok(rec) => records.push(rec),
            Err(_) => skipped += 1,
        }
    }
    if records.is_empty() {
        return Err(format!(
            "no valid career records found ({} line(s) skipped)",
            skipped
        ));
    }

    let n = records.len() as f64;
    let mut scores: Vec<i32> = records.iter().map(|r| r.score).collect();
    scores.sort_unstable();
    let score_sum: i64 = scores.iter().map(|s| *s as i64).sum();
    let score_mean = score_sum as f64 / n;
    let score_median = if scores.len() % 2 == 1 {
        scores[scores.len() / 2] as f64
    } else {
        let m = scores.len() / 2;
        (scores[m - 1] as f64 + scores[m] as f64) / 2.0
    };
    let variance = scores
        .iter()
        .map(|s| (*s as f64 - score_mean).powi(2))
        .sum::<f64>()
        / n;

    let mut grades: BTreeMap<String, usize> = BTreeMap::new();
    let mut u_sum = 0.0;
    let mut u_nonzero = 0usize;
    let mut sp_sum: i64 = 0;
    let mut shop_gain_sum: i64 = 0;
    let mut stat_sums: BTreeMap<String, i64> = BTreeMap::new();
    let mut seed_min = i64::MAX;
    let mut seed_max = i64::MIN;
    for r in &records {
        *grades.entry(r.grade.clone()).or_insert(0) += 1;
        u_sum += r.u;
        if r.u > 0.0 {
            u_nonzero += 1;
        }
        sp_sum += r.sp_spent as i64;
        shop_gain_sum += (r.score - r.score_before_shop) as i64;
        for (k, v) in [
            ("speed", r.stats.speed),
            ("stamina", r.stats.stamina),
            ("power", r.stats.power),
            ("guts", r.stats.guts),
            ("wit", r.stats.wit),
        ] {
            *stat_sums.entry(k.to_string()).or_insert(0) += v as i64;
        }
        seed_min = seed_min.min(r.seed);
        seed_max = seed_max.max(r.seed);
    }

    let mut top: Vec<TopCareer> = records
        .iter()
        .map(|r| TopCareer {
            seed: r.seed,
            grade: r.grade.clone(),
            score: r.score,
        })
        .collect();
    top.sort_by(|a, b| b.score.cmp(&a.score));
    top.truncate(top_n);

    Ok(Summary {
        n: records.len(),
        skipped,
        seed_min,
        seed_max,
        score_mean,
        score_median,
        score_min: scores[0],
        score_max: scores[scores.len() - 1],
        score_stddev: variance.sqrt(),
        grades,
        u_mean: u_sum / n,
        u_nonzero_frac: u_nonzero as f64 / n,
        sp_spent_mean: sp_sum as f64 / n,
        shop_gain_mean: shop_gain_sum as f64 / n,
        stat_means: stat_sums
            .into_iter()
            .map(|(k, v)| (k, v as f64 / n))
            .collect(),
        top,
    })
}

/// Compare the analyzed input against a baseline: positive deltas favor the
/// input. `baseline` is the summary of the `--compare` file.
pub fn compare(baseline: &Summary, input: &Summary) -> Comparison {
    let mut grade_shifts: BTreeMap<String, i64> = BTreeMap::new();
    for g in baseline.grades.keys().chain(input.grades.keys()) {
        grade_shifts.entry(g.clone()).or_insert(0);
    }
    for (g, shift) in grade_shifts.iter_mut() {
        let b = baseline.grades.get(g).copied().unwrap_or(0) as i64;
        let o = input.grades.get(g).copied().unwrap_or(0) as i64;
        *shift = o - b;
    }
    Comparison {
        baseline_n: baseline.n,
        input_n: input.n,
        score_mean_delta: input.score_mean - baseline.score_mean,
        score_median_delta: input.score_median - baseline.score_median,
        u_mean_delta: input.u_mean - baseline.u_mean,
        sp_spent_mean_delta: input.sp_spent_mean - baseline.sp_spent_mean,
        grade_shifts,
    }
}

/// Human-readable report. `label` names the analyzed file; the comparison
/// tuple carries the delta-vs-baseline and the baseline file's label.
pub fn render_text(s: &Summary, cmp: Option<(&Comparison, &str)>, label: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Batch analysis: {} ({} careers, seeds {}..{}",
        label, s.n, s.seed_min, s.seed_max
    ));
    if s.skipped > 0 {
        out.push_str(&format!(", {} malformed line(s) skipped", s.skipped));
    }
    out.push_str(")\n");
    out.push_str(&format!(
        "  score   mean {:8.1}  median {:8.1}  min {:6}  max {:6}  stddev {:7.1}\n",
        s.score_mean, s.score_median, s.score_min, s.score_max, s.score_stddev
    ));
    out.push_str(&format!(
        "  U       mean {:6.3}  nonzero {:.0}%\n",
        s.u_mean,
        s.u_nonzero_frac * 100.0
    ));
    out.push_str(&format!(
        "  shop    mean gain {:6.1} pts/career   sp_spent mean {:6.1}\n",
        s.shop_gain_mean, s.sp_spent_mean
    ));
    let stats: Vec<String> = ["speed", "stamina", "power", "guts", "wit"]
        .iter()
        .map(|k| format!("{} {:5.0}", k, s.stat_means.get(*k).copied().unwrap_or(0.0)))
        .collect();
    out.push_str(&format!("  stats   {}\n", stats.join("  ")));
    let grades: Vec<String> = s
        .grades
        .iter()
        .map(|(g, c)| format!("{}:{}", g, c))
        .collect();
    out.push_str(&format!("  grades  {}\n", grades.join("  ")));
    if !s.top.is_empty() {
        out.push_str("  top careers (seed grade score):\n");
        for t in &s.top {
            out.push_str(&format!("    {:<10} {:<4} {}\n", t.seed, t.grade, t.score));
        }
    }
    if let Some((c, baseline_label)) = cmp {
        out.push_str(&format!(
            "\nComparison: {} vs baseline {} (deltas = input minus baseline, positive favors input):\n",
            label, baseline_label
        ));
        out.push_str(&format!(
            "  score mean {:+.1}   median {:+.1}   U mean {:+.3}   sp_spent {:+.1}\n",
            c.score_mean_delta, c.score_median_delta, c.u_mean_delta, c.sp_spent_mean_delta
        ));
        let shifts: Vec<String> = c
            .grade_shifts
            .iter()
            .filter(|(_, d)| **d != 0)
            .map(|(g, d)| format!("{}{:+}", g, d))
            .collect();
        if shifts.is_empty() {
            out.push_str("  grade shifts: none\n");
        } else {
            out.push_str(&format!("  grade shifts: {}\n", shifts.join("  ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(seed: i64, grade: &str, score: i32) -> String {
        format!(
            "{{\"seed\":{seed},\"scenario\":\"ura\",\"trainee\":\"Special Week\",\
             \"grade\":\"{grade}\",\"score\":{score},\"score_before_shop\":{},\
             \"u\":1.5,\"sp_spent\":120,\
             \"stats\":{{\"speed\":800,\"stamina\":810,\"power\":820,\"guts\":830,\"wit\":840}}}}",
            score - 40
        )
    }

    fn sample() -> String {
        ["F", "E", "D", "C", "B"]
            .iter()
            .zip([100, 200, 300, 400, 500])
            .enumerate()
            .map(|(i, (g, s))| line(i as i64 + 1, g, s))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn aggregates_known_values() {
        let s = analyze_text(&sample(), 5).unwrap();
        assert_eq!(s.n, 5);
        assert_eq!(s.skipped, 0);
        assert!((s.score_mean - 300.0).abs() < 1e-9);
        assert!((s.score_median - 300.0).abs() < 1e-9);
        assert_eq!(s.score_min, 100);
        assert_eq!(s.score_max, 500);
        assert!((s.score_stddev - 141.4213562373095).abs() < 1e-6);
        assert_eq!(s.grades.get("B"), Some(&1));
        assert_eq!(s.grades.len(), 5);
        assert!((s.u_mean - 1.5).abs() < 1e-9);
        assert!((s.u_nonzero_frac - 1.0).abs() < 1e-9);
        assert!((s.sp_spent_mean - 120.0).abs() < 1e-9);
        assert!((s.shop_gain_mean - 40.0).abs() < 1e-9);
        assert!((s.stat_means["speed"] - 800.0).abs() < 1e-9);
        assert_eq!(s.top[0].score, 500);
        assert_eq!(s.top[0].seed, 5);
        assert_eq!(s.top.len(), 5);
    }

    #[test]
    fn malformed_lines_skipped_not_fatal() {
        let input = format!(
            "{}\n{{not json}}\n{{\"score\":10}}\n\n{}",
            line(1, "F", 100),
            line(2, "E", 200)
        );
        let s = analyze_text(&input, 5).unwrap();
        assert_eq!(s.n, 2);
        assert_eq!(s.skipped, 2); // bad JSON + missing grade
        assert!((s.score_mean - 150.0).abs() < 1e-9);
    }

    #[test]
    fn empty_input_is_error() {
        assert!(analyze_text("", 5).is_err());
        assert!(analyze_text("\n  \n", 5).is_err());
        assert!(analyze_text("{bad}\n{also bad}", 5).is_err());
    }

    #[test]
    fn compare_deltas() {
        let base = analyze_text(&sample(), 0).unwrap();
        let better = ["F", "E", "D", "C", "A"]
            .iter()
            .zip([150, 250, 350, 450, 900])
            .enumerate()
            .map(|(i, (g, s))| line(i as i64 + 1, g, s))
            .collect::<Vec<_>>()
            .join("\n");
        let other = analyze_text(&better, 0).unwrap();
        let c = compare(&base, &other);
        assert!((c.score_mean_delta - 120.0).abs() < 1e-9);
        assert_eq!(c.grade_shifts.get("B"), Some(&-1));
        assert_eq!(c.grade_shifts.get("A"), Some(&1));
        assert_eq!(c.grade_shifts.get("F"), Some(&0));
    }

    #[test]
    fn top_n_truncates() {
        let s = analyze_text(&sample(), 2).unwrap();
        assert_eq!(s.top.len(), 2);
        assert_eq!(s.top[0].score, 500);
        assert_eq!(s.top[1].score, 400);
    }

    #[test]
    fn text_render_contains_key_sections() {
        let s = analyze_text(&sample(), 1).unwrap();
        let t = render_text(&s, None, "demo.jsonl");
        assert!(t.contains("5 careers"));
        assert!(t.contains("300.0"));
        assert!(t.contains("grades"));
        assert!(t.contains("top careers"));
    }
}
