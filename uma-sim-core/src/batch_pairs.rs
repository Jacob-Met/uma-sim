//! Compare complete batches by their recorded career identities.
//!
//! This is descriptive pairing of saved results, not authentication of the
//! simulator version, policy, deck, or random-number consumption.

use std::collections::BTreeMap;
use std::io::Read;

pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RECORDS: usize = 100_000;
pub const MAX_IDENTITY_BYTES: usize = 256;
pub const MAX_GRADE_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct CareerIdentity {
    pub scenario: String,
    pub trainee: String,
    pub seed: i64,
}

#[derive(Debug, serde::Deserialize)]
struct Record {
    seed: i64,
    scenario: String,
    trainee: String,
    grade: String,
    score: i32,
}

/// Validated, nonempty and unique records from one bounded JSONL input.
#[derive(Debug)]
pub struct PairedBatch {
    records: BTreeMap<CareerIdentity, Record>,
}

impl PairedBatch {
    /// Read at most the byte limit plus one byte, then validate the whole file.
    /// The label is used only in diagnostics.
    pub fn from_reader(reader: impl Read, label: &str) -> Result<Self, String> {
        let mut bytes = Vec::new();
        reader
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("{label}: could not read batch: {error}"))?;
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(format!("{label}: input exceeds {MAX_INPUT_BYTES} bytes"));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| format!("{label}: input is not UTF-8: {error}"))?;
        Self::parse(text, label)
    }

    /// Parse all nonblank lines. No malformed record is silently skipped.
    /// Identity and grade strings are literal; nonempty whitespace is retained.
    pub fn parse(text: &str, label: &str) -> Result<Self, String> {
        if text.len() > MAX_INPUT_BYTES {
            return Err(format!("{label}: input exceeds {MAX_INPUT_BYTES} bytes"));
        }
        let mut records = BTreeMap::new();
        let mut count = 0usize;
        for (offset, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            count += 1;
            let line_number = offset + 1;
            if count > MAX_RECORDS {
                return Err(format!("{label}: more than {MAX_RECORDS} records"));
            }
            let record: Record = serde_json::from_str(line)
                .map_err(|error| format!("{label} line {line_number}: invalid record: {error}"))?;
            for (field, value, limit) in [
                ("scenario", record.scenario.as_str(), MAX_IDENTITY_BYTES),
                ("trainee", record.trainee.as_str(), MAX_IDENTITY_BYTES),
                ("grade", record.grade.as_str(), MAX_GRADE_BYTES),
            ] {
                if value.is_empty() || value.len() > limit {
                    return Err(format!(
                        "{label} line {line_number}: {field} must contain 1..={limit} UTF-8 bytes"
                    ));
                }
            }
            let identity = CareerIdentity {
                scenario: record.scenario.clone(),
                trainee: record.trainee.clone(),
                seed: record.seed,
            };
            if records.contains_key(&identity) {
                return Err(format!(
                    "{label} line {line_number}: duplicate career {}",
                    identity_text(&identity)
                ));
            }
            records.insert(identity, record);
        }
        if records.is_empty() {
            return Err(format!("{label}: no career records"));
        }
        Ok(Self { records })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PairOutcome {
    pub scenario: String,
    pub trainee: String,
    pub seed: i64,
    pub baseline_score: i32,
    pub input_score: i32,
    pub score_delta: i64,
    pub baseline_grade: String,
    pub input_grade: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PairSummary {
    pub count: usize,
    pub wins: usize,
    pub ties: usize,
    pub losses: usize,
    pub mean_score_delta: f64,
    pub median_score_delta: f64,
    pub min_score_delta: i64,
    pub max_score_delta: i64,
    pub population_stddev_score_delta: f64,
    /// Standard error of the paired mean (sample variance; unavailable for one pair).
    pub score_delta_standard_error: Option<f64>,
    /// Approximate normal-theory 95% interval for mean paired score lift.
    /// Only emitted for at least 30 matched careers; it is not a game-model validity claim.
    pub mean_score_delta_95pct_normal_interval: Option<[f64; 2]>,
    /// Why this normal interval was or was not reported. A reported interval
    /// remains a model-based approximation, not verified 95% coverage.
    pub mean_score_delta_normal_interval_status: &'static str,
    /// Fraction of decisive pairs won by the input; ties are excluded.
    pub decisive_win_rate: Option<f64>,
    /// Wilson score 95% interval for the decisive win fraction.
    pub decisive_win_rate_95pct_wilson_interval: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PairedComparison {
    pub format: &'static str,
    pub direction: &'static str,
    pub identity_fields: [&'static str; 3],
    pub summary: PairSummary,
    /// Complete pairs, sorted by scenario, trainee, then numeric signed seed.
    pub comparisons: Vec<PairOutcome>,
}

/// Compare input minus baseline. Both complete identity sets must match.
pub fn compare(input: &PairedBatch, baseline: &PairedBatch) -> Result<PairedComparison, String> {
    if let Some(identity) = input
        .records
        .keys()
        .find(|key| !baseline.records.contains_key(*key))
    {
        return Err(format!(
            "input career {} has no baseline partner; complete matching cohorts are required",
            identity_text(identity)
        ));
    }
    if let Some(identity) = baseline
        .records
        .keys()
        .find(|key| !input.records.contains_key(*key))
    {
        return Err(format!(
            "baseline career {} has no input partner; complete matching cohorts are required",
            identity_text(identity)
        ));
    }
    let comparisons: Vec<PairOutcome> = input
        .records
        .iter()
        .map(|(identity, record)| {
            let original = &baseline.records[identity];
            PairOutcome {
                scenario: identity.scenario.clone(),
                trainee: identity.trainee.clone(),
                seed: identity.seed,
                baseline_score: original.score,
                input_score: record.score,
                // Cast before subtraction: both i32 endpoints are valid inputs.
                score_delta: i64::from(record.score) - i64::from(original.score),
                baseline_grade: original.grade.clone(),
                input_grade: record.grade.clone(),
            }
        })
        .collect();
    let mut deltas: Vec<i64> = comparisons.iter().map(|pair| pair.score_delta).collect();
    deltas.sort_unstable();
    let count = deltas.len();
    // Admission bounds count and every delta, so this exact i64 sum cannot overflow.
    let mean = deltas.iter().sum::<i64>() as f64 / count as f64;
    let middle = count / 2;
    let median = if count.is_multiple_of(2) {
        (deltas[middle - 1] + deltas[middle]) as f64 / 2.0
    } else {
        deltas[middle] as f64
    };
    let variance = deltas
        .iter()
        .map(|delta| (*delta as f64 - mean).powi(2))
        .sum::<f64>()
        / count as f64;
    let wins = deltas.iter().filter(|delta| **delta > 0).count();
    let ties = deltas.iter().filter(|delta| **delta == 0).count();
    let losses = count - wins - ties;
    // Bessel-corrected paired variance / n. The normal interval is deliberately
    // omitted for small n rather than suggesting unjustified precision.
    let standard_error = (count > 1).then(|| (variance / (count - 1) as f64).sqrt());
    // Thirty samples alone cannot establish normality or rule out rare tails.
    // In particular, identical observed deltas produce a zero-width interval
    // even though an unseen rare effect may exist. Do not print that as a 95% CI.
    let normal_interval_status = if count < 30 {
        "omitted_fewer_than_30_pairs"
    } else if variance == 0.0 {
        "omitted_no_observed_variation"
    } else {
        "reported_normal_assumptions_unverified"
    };
    let normal_interval = if normal_interval_status == "reported_normal_assumptions_unverified" {
        standard_error.map(|se| [mean - Z_95 * se, mean + Z_95 * se])
    } else {
        None
    };
    let decisive = wins + losses;
    let win_rate = (decisive > 0).then(|| wins as f64 / decisive as f64);
    let win_interval = wilson_95(wins, decisive);
    Ok(PairedComparison {
        format: "uma-sim.paired-batches.v1",
        direction: "input_minus_baseline",
        identity_fields: ["scenario", "trainee", "seed"],
        summary: PairSummary {
            count,
            wins,
            ties,
            losses,
            mean_score_delta: mean,
            median_score_delta: median,
            min_score_delta: deltas[0],
            max_score_delta: deltas[count - 1],
            population_stddev_score_delta: variance.sqrt(),
            score_delta_standard_error: standard_error,
            mean_score_delta_95pct_normal_interval: normal_interval,
            mean_score_delta_normal_interval_status: normal_interval_status,
            decisive_win_rate: win_rate,
            decisive_win_rate_95pct_wilson_interval: win_interval,
        },
        comparisons,
    })
}

// Two-sided 95% normal quantile. These intervals describe the variation
// across supplied career identities; they do not validate policies or rules.
const Z_95: f64 = 1.959_963_984_540_054;

fn wilson_95(successes: usize, trials: usize) -> Option<[f64; 2]> {
    if trials == 0 {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = Z_95 * Z_95;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width = Z_95 * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt() / denominator;
    Some([
        (center - half_width).max(0.0),
        (center + half_width).min(1.0),
    ])
}

/// Human-readable summary with the ten largest absolute differences.
/// JSON exports retain every pair; this view explicitly limits its detail rows.
pub fn render_text(comparison: &PairedComparison) -> String {
    use std::fmt::Write;
    let summary = &comparison.summary;
    let mut text = String::new();
    text.push_str("Paired batch comparison (input minus baseline)\n");
    writeln!(
        text,
        "Pairs: {}; wins: {}; ties: {}; losses: {}",
        summary.count, summary.wins, summary.ties, summary.losses
    )
    .expect("writing a String cannot fail");
    writeln!(
        text,
        "Score delta: mean {:.6}; median {:.6}; min {}; max {}; population stddev {:.6}",
        summary.mean_score_delta,
        summary.median_score_delta,
        summary.min_score_delta,
        summary.max_score_delta,
        summary.population_stddev_score_delta
    )
    .expect("writing a String cannot fail");
    text.push_str("Largest absolute score changes (up to 10; ties by scenario, trainee, seed):\n");
    text.push_str("delta\tbaseline\tinput\t[scenario,trainee,seed]\n");
    let mut rows: Vec<&PairOutcome> = comparison.comparisons.iter().collect();
    rows.sort_by(|a, b| {
        b.score_delta
            .unsigned_abs()
            .cmp(&a.score_delta.unsigned_abs())
            .then_with(|| a.scenario.cmp(&b.scenario))
            .then_with(|| a.trainee.cmp(&b.trainee))
            .then_with(|| a.seed.cmp(&b.seed))
    });
    for row in rows.into_iter().take(10) {
        let identity = serde_json::to_string(&(&row.scenario, &row.trainee, row.seed))
            .expect("serializing strings and an integer cannot fail");
        writeln!(
            text,
            "{:+}\t{}\t{}\t{}",
            row.score_delta, row.baseline_score, row.input_score, identity
        )
        .expect("writing a String cannot fail");
    }
    text.push('\n');
    if let Some(se) = summary.score_delta_standard_error {
        writeln!(text, "Paired mean standard error: {:.6}", se)
            .expect("writing a String cannot fail");
    } else {
        text.push_str("Paired mean standard error: unavailable (one pair)\n");
    }
    if let Some([low, high]) = summary.mean_score_delta_95pct_normal_interval {
        writeln!(
            text,
            "Mean score delta approx. 95% normal interval: [{low:.3}, {high:.3}]"
        )
        .expect("writing a String cannot fail");
    } else if summary.count >= 30 {
        text.push_str("Mean score interval: omitted (no observed variation; zero-width sample interval cannot rule out rare effects)\n");
    } else {
        text.push_str("Mean score interval: omitted (need at least 30 matched pairs)\n");
    }
    if let (Some(rate), Some([low, high])) = (
        summary.decisive_win_rate,
        summary.decisive_win_rate_95pct_wilson_interval,
    ) {
        writeln!(
            text,
            "Decisive win rate (ties excluded): {:.1}% (95% Wilson [{:.1}%, {:.1}%])",
            rate * 100.0,
            low * 100.0,
            high * 100.0
        )
        .expect("writing a String cannot fail");
    } else {
        text.push_str("Decisive win rate: unavailable (all paired scores tied)\n");
    }
    if summary.count >= 30 && summary.ties >= summary.count / 2 {
        writeln!(
            text,
            "Sparse changes: {} of {} pairs tied. Rare gains/losses can dominate the normal interval.",
            summary.ties, summary.count
        )
        .expect("writing a String cannot fail");
    }
    text.push_str("Normal mean interval is only a model-based approximation: 30+ pairs do not guarantee 95% coverage with rare/heavy-tailed deltas. Intervals assume independent careers; they neither validate the simulator nor prove causality.\n");
    text
}

fn identity_text(identity: &CareerIdentity) -> String {
    serde_json::to_string(&(&identity.scenario, &identity.trainee, identity.seed))
        .expect("serializing strings and an integer cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(scores: &[(i64, i32)]) -> PairedBatch {
        let text = scores
            .iter()
            .map(|(seed, score)| {
                format!(
                    r#"{{"seed":{seed},"scenario":"ura","trainee":"Special Week","grade":"F","score":{score}}}"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        PairedBatch::parse(&text, "test").unwrap()
    }

    #[test]
    fn one_pair_has_wilson_but_no_mean_interval() {
        let b = batch(&[(7, 100)]);
        let a = batch(&[(7, 110)]);
        let c = compare(&a, &b).unwrap();
        assert_eq!(c.summary.mean_score_delta, 10.0);
        assert_eq!(c.summary.score_delta_standard_error, None);
        assert_eq!(c.summary.mean_score_delta_95pct_normal_interval, None);
        assert_eq!(c.summary.decisive_win_rate, Some(1.0));
        let [lo, hi] = c.summary.decisive_win_rate_95pct_wilson_interval.unwrap();
        assert!(lo > 0.20 && lo < 0.21);
        assert!((hi - 1.0).abs() < 1e-12);
    }

    #[test]
    fn balanced_thirty_pairs_get_credible_uncertainty() {
        let original: Vec<_> = (0..30).map(|seed| (seed, 100)).collect();
        let changes: Vec<_> = (0..30)
            .map(|seed| (seed, if seed < 15 { 101 } else { 99 }))
            .collect();
        let c = compare(&batch(&changes), &batch(&original)).unwrap();
        assert_eq!(
            (c.summary.wins, c.summary.ties, c.summary.losses),
            (15, 0, 15)
        );
        assert_eq!(c.summary.decisive_win_rate, Some(0.5));
        let [lo, hi] = c.summary.mean_score_delta_95pct_normal_interval.unwrap();
        assert!(lo < 0.0 && hi > 0.0);
        assert!((lo + hi).abs() < 1e-12);
        let [lo, hi] = c.summary.decisive_win_rate_95pct_wilson_interval.unwrap();
        assert!(lo < 0.5 && hi > 0.5);
        let report = render_text(&c);
        assert!(report.contains("95% Wilson"));
        assert!(report.contains("neither validate the simulator"));
    }

    #[test]
    fn all_ties_have_no_directional_win_estimate() {
        let career: Vec<_> = (0..30).map(|seed| (seed, 100)).collect();
        let c = compare(&batch(&career), &batch(&career)).unwrap();
        assert_eq!(c.summary.decisive_win_rate, None);
        assert_eq!(c.summary.decisive_win_rate_95pct_wilson_interval, None);
        assert_eq!(c.summary.mean_score_delta_95pct_normal_interval, None);
        assert_eq!(
            c.summary.mean_score_delta_normal_interval_status,
            "omitted_no_observed_variation"
        );
        let report = render_text(&c);
        assert!(report.contains("zero-width sample interval cannot rule out rare effects"));
    }

    #[test]
    fn one_rare_gain_in_thirty_is_visibly_uncertain() {
        let baseline: Vec<_> = (0..30).map(|seed| (seed, 100)).collect();
        let input: Vec<_> = (0..30)
            .map(|seed| (seed, if seed == 29 { 1100 } else { 100 }))
            .collect();
        let result = compare(&batch(&input), &batch(&baseline)).unwrap();
        assert_eq!(result.summary.ties, 29);
        assert_eq!(
            result.summary.mean_score_delta_normal_interval_status,
            "reported_normal_assumptions_unverified"
        );
        let report = render_text(&result);
        assert!(report.contains("Sparse changes: 29 of 30 pairs tied."));
        assert!(report.contains("do not guarantee 95% coverage"));
    }

    #[test]
    fn near_limit_scores_stay_finite_without_overflow() {
        let b = batch(&[(1, i32::MIN), (2, i32::MAX)]);
        let a = batch(&[(1, i32::MAX), (2, i32::MIN)]);
        let c = compare(&a, &b).unwrap();
        assert_eq!(c.summary.mean_score_delta, 0.0);
        assert!(c.summary.score_delta_standard_error.unwrap().is_finite());
        assert!(c.summary.mean_score_delta_95pct_normal_interval.is_none());
        assert_eq!(c.comparisons[0].score_delta, i64::from(u32::MAX));
    }
}
