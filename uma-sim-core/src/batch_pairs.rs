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
    let median = if count % 2 == 0 {
        (deltas[middle - 1] + deltas[middle]) as f64 / 2.0
    } else {
        deltas[middle] as f64
    };
    let variance = deltas
        .iter()
        .map(|delta| (*delta as f64 - mean).powi(2))
        .sum::<f64>()
        / count as f64;
    Ok(PairedComparison {
        format: "uma-sim.paired-batches.v1",
        direction: "input_minus_baseline",
        identity_fields: ["scenario", "trainee", "seed"],
        summary: PairSummary {
            count,
            wins: deltas.iter().filter(|delta| **delta > 0).count(),
            ties: deltas.iter().filter(|delta| **delta == 0).count(),
            losses: deltas.iter().filter(|delta| **delta < 0).count(),
            mean_score_delta: mean,
            median_score_delta: median,
            min_score_delta: deltas[0],
            max_score_delta: deltas[count - 1],
            population_stddev_score_delta: variance.sqrt(),
        },
        comparisons,
    })
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
    text
}

fn identity_text(identity: &CareerIdentity) -> String {
    serde_json::to_string(&(&identity.scenario, &identity.trainee, identity.seed))
        .expect("serializing strings and an integer cannot fail")
}
