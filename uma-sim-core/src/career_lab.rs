//! Saved-career library and branch-and-compare laboratory (Commons T100).
//!
//! The engine already supports export/restore of [`RunSnapshot`], but the CLI
//! keeps one fixed session file and the REST API keeps one in-memory engine.
//! This module adds:
//!
//! * a **durable named checkpoint library** (`.uma-sim/library/`): save, list,
//!   load, delete, import and export named career checkpoints with visible
//!   seed/scenario/turn, a content fingerprint for compatibility disclosure,
//!   atomic individual-file writes, and restoration after failed saves;
//! * **branch runs**: play a checkpoint forward to completion with one of the
//!   built-in policies (or per-turn action overrides), recording a full
//!   decision/timeline/outcome trace per branch with its own isolated
//!   telemetry;
//! * **comparison**: first-divergence detection, aligned timeline diffs,
//!   outcome comparison, and a downloadable Markdown/JSON report.
//!
//! RNG honesty: branches share the checkpoint's seed and RNG position, but
//! RNG streams diverge the moment actions differ. The comparison report says
//! so explicitly and never claims matched future randomness or causal
//! superiority from one seed.

use crate::catalog::event::active_event_catalog;
use crate::content::ContentPackRegistry;
use crate::engine::SimEngine;
use crate::factory::detect_repo_root;
use crate::session::parse_sim_action;
use crate::snapshot::RunSnapshot;
use crate::state::{
    CareerState, GeneratedSpark, SimAction, SimActionKind, SimChoice, SimDate, TraineeStats,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Snapshot schema version understood by this build. Bumped only on
/// intentionally breaking `RunSnapshot` changes; a mismatch is a hard,
/// non-destructive load error.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Core package version baked in at compile time.
const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// User-facing laboratory errors. Never panics; callers map these to HTTP
/// statuses or CLI messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabError {
    /// Entry name violates the naming rules.
    InvalidName(String),
    /// A durable entry or branch result with this name or ID already exists.
    AlreadyExists(String),
    /// No library entry with this name.
    NotFound(String),
    /// Raw snapshot text failed to decode as a `RunSnapshot`.
    InvalidSnapshot(String),
    /// The snapshot's schema is not readable by this build (non-destructive).
    IncompatibleSnapshot(String),
    /// Filesystem failure.
    Io(String),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabError::InvalidName(m) => write!(f, "invalid checkpoint name: {m}"),
            LabError::AlreadyExists(n) => {
                write!(f, "entry '{n}' already exists and was not replaced")
            }
            LabError::NotFound(n) => write!(f, "no checkpoint named '{n}'"),
            LabError::InvalidSnapshot(m) => {
                write!(f, "snapshot failed validation and was not imported: {m}")
            }
            LabError::IncompatibleSnapshot(m) => write!(f, "incompatible snapshot: {m}"),
            LabError::Io(m) => write!(f, "storage error: {m}"),
        }
    }
}

impl std::error::Error for LabError {}

// ---------------------------------------------------------------------------
// Time helpers (no chrono dependency)
// ---------------------------------------------------------------------------

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Format unix seconds as RFC3339 UTC (`2026-10-03T16:45:00Z`).
fn rfc3339_utc(secs: u64) -> String {
    // Days since civil 1970-01-01 → civil date (Howard Hinnant's algorithm).
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let rem = secs % 86_400;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        m,
        d,
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

/// Short human date label for a career date (`Y1 Jun · 2nd half`).
pub fn date_label(date: &SimDate) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS
        .get(date.month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("?");
    let half = if date.half <= 1 { "1st" } else { "2nd" };
    format!("Y{} {month} · {half} half", date.year)
}

// ---------------------------------------------------------------------------
// Content fingerprint
// ---------------------------------------------------------------------------

/// Identifies the content/catalog context a checkpoint was saved under, so
/// loading under a different context can disclose the mismatch instead of
/// silently producing a divergent career.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentFingerprint {
    pub core_version: String,
    pub snapshot_schema: u32,
    pub event_catalog_count: usize,
    pub content_pack_events: usize,
    pub repo_root_detected: bool,
}

impl ContentFingerprint {
    pub fn capture() -> Self {
        Self {
            core_version: CORE_VERSION.to_string(),
            snapshot_schema: SNAPSHOT_SCHEMA_VERSION,
            event_catalog_count: active_event_catalog().event_count(),
            content_pack_events: ContentPackRegistry::all().len(),
            repo_root_detected: detect_repo_root().is_some(),
        }
    }

    /// Compare a saved fingerprint against the current one. Returns
    /// `(blocking, advisory)`: blocking issues make the load fail loudly
    /// (without touching anything); advisory issues are disclosed to the user.
    pub fn check_against(&self, current: &ContentFingerprint) -> (Vec<String>, Vec<String>) {
        let mut blocking = Vec::new();
        let mut advisory = Vec::new();
        if self.snapshot_schema != current.snapshot_schema {
            blocking.push(format!(
                "snapshot schema v{} is not readable by this build (expects v{})",
                self.snapshot_schema, current.snapshot_schema
            ));
        }
        if self.core_version != current.core_version {
            advisory.push(format!(
                "checkpoint was saved by uma-sim-core {}; this build is {}",
                self.core_version, current.core_version
            ));
        }
        if self.event_catalog_count != current.event_catalog_count {
            advisory.push(format!(
                "event catalog size changed since save ({} then, {} now); event draws may differ",
                self.event_catalog_count, current.event_catalog_count
            ));
        }
        if self.content_pack_events != current.content_pack_events {
            advisory.push(format!(
                "content-pack event count changed since save ({} then, {} now)",
                self.content_pack_events, current.content_pack_events
            ));
        }
        if self.repo_root_detected != current.repo_root_detected {
            advisory.push(
                "research/knowledge root detection differs from save time; catalog content may differ"
                    .to_string(),
            );
        }
        (blocking, advisory)
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// Metadata for one named checkpoint in the career library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub name: String,
    pub label: String,
    pub note: String,
    pub saved_at: String,
    pub saved_at_unix: u64,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub turn: i32,
    pub date_label: String,
    pub phase: String,
    pub rng_calls: u32,
    pub fingerprint: ContentFingerprint,
}

/// Validate a library/session/branch name. Public so the API layer can share
/// the same naming rules.
pub fn validate_entry_name(name: &str) -> Result<(), LabError> {
    validate_name(name)
}

/// Generate a unique session id from a human hint.
pub fn new_session_id(hint: &str) -> String {
    static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    let base: String = hint
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(24)
        .collect();
    let base = if base.is_empty() {
        "branch".to_string()
    } else {
        base
    };
    format!("{base}-{}-{:04}", now_unix(), n % 10_000)
}

fn validate_name(name: &str) -> Result<(), LabError> {
    if name.is_empty() || name.len() > 64 {
        return Err(LabError::InvalidName("must be 1-64 characters".to_string()));
    }
    if name == "." || name == ".." {
        return Err(LabError::InvalidName(
            "'.' and '..' are reserved".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(LabError::InvalidName(
            "only ASCII letters, digits, '-', '_' and '.' are allowed".to_string(),
        ));
    }
    Ok(())
}

/// Write `bytes` to `path` atomically: temp file in the same directory, then
/// rename. A crash mid-write leaves at most a stray `.tmp` file and never a
/// half-written checkpoint, so the last good career is preserved.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), LabError> {
    let parent = path
        .parent()
        .ok_or_else(|| LabError::Io(format!("no parent directory for {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|e| LabError::Io(e.to_string()))?;
    let tmp = parent.join(format!(
        ".tmp-{}-{}",
        std::process::id(),
        path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "blob".to_string())
    ));
    fs::write(&tmp, bytes).map_err(|e| LabError::Io(e.to_string()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        LabError::Io(format!("rename failed: {e}"))
    })?;
    Ok(())
}

/// Preserve the prior snapshot before replacing either part of a checkpoint.
/// The recovery name deliberately avoids `.tmp-`: a failed restoration must
/// not let the ordinary stale-temp sweep delete the remaining saved career.
fn preserve_snapshot(path: &Path) -> Result<Option<PathBuf>, LabError> {
    let mut source = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(LabError::Io(error.to_string())),
    };
    static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let backup_path = path.with_file_name(format!(
        ".rollback-{}-{}-{}",
        std::process::id(),
        BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed),
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    let mut backup = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .map_err(|error| {
            LabError::Io(format!(
                "could not preserve prior snapshot at {}: {error}",
                backup_path.display()
            ))
        })?;
    if let Err(error) = std::io::copy(&mut source, &mut backup).and_then(|_| backup.sync_all()) {
        drop(backup);
        let _ = fs::remove_file(&backup_path);
        return Err(LabError::Io(format!(
            "could not preserve prior snapshot: {error}"
        )));
    }
    Ok(Some(backup_path))
}

fn discard_snapshot_backup(backup: Option<&Path>) {
    if let Some(path) = backup {
        let _ = fs::remove_file(path);
    }
}

/// Return the original save error after restoration. If restoration itself is
/// refused, retain the complete backup and identify it in the returned error.
fn rollback_snapshot(path: &Path, backup: Option<&Path>, save_error: LabError) -> LabError {
    let restored = match backup {
        Some(previous) => fs::rename(previous, path),
        None => fs::remove_file(path),
    };
    match restored {
        Ok(()) => save_error,
        Err(error) if backup.is_none() && error.kind() == std::io::ErrorKind::NotFound => {
            save_error
        }
        Err(error) => {
            let recovery = match backup {
                Some(previous) => format!("prior snapshot retained at {}", previous.display()),
                None => format!("incomplete new snapshot remains at {}", path.display()),
            };
            LabError::Io(format!(
                "{save_error}; snapshot restoration failed: {error}; {recovery}"
            ))
        }
    }
}

/// Durable named checkpoint library under `.uma-sim/library/` (cwd-relative,
/// same convention as the CLI session file).
pub struct CareerLibrary {
    dir: PathBuf,
}

impl Default for CareerLibrary {
    fn default() -> Self {
        let dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join("library");
        Self { dir }
    }
}

impl CareerLibrary {
    /// Library rooted at the current directory's `.uma-sim/library/`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Library rooted at an explicit directory (used by tests).
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn snapshot_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.snapshot.json"))
    }

    fn meta_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.meta.json"))
    }

    /// Save `snapshot` as a named checkpoint. `overwrite=false` (the default)
    /// refuses to clobber existing metadata or an orphan snapshot. A returned
    /// metadata-write failure restores the prior snapshot; a refused restore
    /// retains its recovery copy and reports that path.
    pub fn save(
        &self,
        name: &str,
        label: Option<&str>,
        note: Option<&str>,
        snapshot: &RunSnapshot,
        overwrite: bool,
    ) -> Result<LibraryEntry, LabError> {
        validate_name(name)?;
        let meta_path = self.meta_path(name);
        let snapshot_path = self.snapshot_path(name);
        if !overwrite && (meta_path.exists() || snapshot_path.exists()) {
            return Err(LabError::AlreadyExists(name.to_string()));
        }
        let saved_at_unix = now_unix();
        let entry = LibraryEntry {
            name: name.to_string(),
            label: label
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(name)
                .to_string(),
            note: note.unwrap_or("").to_string(),
            saved_at: rfc3339_utc(saved_at_unix),
            saved_at_unix,
            seed: snapshot.meta.seed,
            scenario_id: snapshot.meta.scenario_id.clone(),
            trainee_name: snapshot.meta.trainee_name.clone(),
            turn: snapshot.state.turn,
            date_label: date_label(&snapshot.state.date),
            phase: snapshot.state.phase.clone(),
            rng_calls: snapshot.rng_calls,
            fingerprint: ContentFingerprint::capture(),
        };
        let snap_bytes =
            serde_json::to_string_pretty(snapshot).map_err(|e| LabError::Io(e.to_string()))?;
        let meta_bytes =
            serde_json::to_string_pretty(&entry).map_err(|e| LabError::Io(e.to_string()))?;
        // Retain the old bytes before publishing the replacement. Restoring
        // uses a rename, so a metadata write failure (including exhausted
        // space) does not need another full snapshot allocation to recover.
        let previous = preserve_snapshot(&snapshot_path)?;
        if let Err(error) = atomic_write(&snapshot_path, snap_bytes.as_bytes()) {
            discard_snapshot_backup(previous.as_deref());
            return Err(error);
        }
        if let Err(e) = atomic_write(&meta_path, meta_bytes.as_bytes()) {
            return Err(rollback_snapshot(&snapshot_path, previous.as_deref(), e));
        }
        discard_snapshot_backup(previous.as_deref());
        Ok(entry)
    }

    /// List all checkpoints, newest first. Unreadable entries are skipped.
    pub fn list(&self) -> Result<Vec<LibraryEntry>, LabError> {
        let mut entries = Vec::new();
        let read = fs::read_dir(&self.dir);
        let dir_entries = match read {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
            Err(e) => return Err(LabError::Io(e.to_string())),
        };
        for de in dir_entries.flatten() {
            let path = de.path();
            let is_meta = path
                .file_name()
                .map(|n| n.to_string_lossy().ends_with(".meta.json"))
                .unwrap_or(false);
            if !is_meta {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(entry) = serde_json::from_str::<LibraryEntry>(&raw) {
                    entries.push(entry);
                }
            }
        }
        entries.sort_by(|a, b| {
            b.saved_at_unix
                .cmp(&a.saved_at_unix)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(entries)
    }

    /// Load a checkpoint. Returns the snapshot, its entry, and advisory
    /// compatibility issues. A schema mismatch fails with
    /// [`LabError::IncompatibleSnapshot`] without touching anything.
    pub fn load(&self, name: &str) -> Result<(RunSnapshot, LibraryEntry, Vec<String>), LabError> {
        validate_name(name)?;
        let meta_raw = fs::read_to_string(self.meta_path(name))
            .map_err(|_| LabError::NotFound(name.to_string()))?;
        let entry: LibraryEntry =
            serde_json::from_str(&meta_raw).map_err(|e| LabError::Io(e.to_string()))?;
        let snap_raw = fs::read_to_string(self.snapshot_path(name))
            .map_err(|_| LabError::NotFound(name.to_string()))?;
        let snapshot: RunSnapshot = serde_json::from_str(&snap_raw)
            .map_err(|e| LabError::InvalidSnapshot(e.to_string()))?;
        let (blocking, advisory) = entry
            .fingerprint
            .check_against(&ContentFingerprint::capture());
        if !blocking.is_empty() {
            return Err(LabError::IncompatibleSnapshot(blocking.join("; ")));
        }
        Ok((snapshot, entry, advisory))
    }

    /// Delete a checkpoint (both files). Missing entries are a `NotFound`
    /// error, not a silent no-op.
    pub fn delete(&self, name: &str) -> Result<(), LabError> {
        validate_name(name)?;
        if !self.meta_path(name).exists() {
            return Err(LabError::NotFound(name.to_string()));
        }
        fs::remove_file(self.snapshot_path(name)).map_err(|e| LabError::Io(e.to_string()))?;
        fs::remove_file(self.meta_path(name)).map_err(|e| LabError::Io(e.to_string()))?;
        Ok(())
    }

    /// Validate and import external snapshot JSON as a new checkpoint.
    /// Validation happens before any write, so a bad import preserves the
    /// last good library state.
    pub fn import(
        &self,
        raw: &str,
        name: Option<&str>,
        overwrite: bool,
    ) -> Result<LibraryEntry, LabError> {
        let snapshot: RunSnapshot =
            serde_json::from_str(raw).map_err(|e| LabError::InvalidSnapshot(e.to_string()))?;
        let name = match name {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => auto_name(&snapshot),
        };
        self.save(&name, None, Some("imported"), &snapshot, overwrite)
    }

    /// Raw snapshot bytes for download.
    pub fn export_raw(&self, name: &str) -> Result<String, LabError> {
        validate_name(name)?;
        fs::read_to_string(self.snapshot_path(name))
            .map_err(|_| LabError::NotFound(name.to_string()))
    }
}

/// Default checkpoint name when the caller doesn't supply one.
pub fn auto_name(snapshot: &RunSnapshot) -> String {
    format!(
        "seed{}-t{}-{}",
        snapshot.meta.seed, snapshot.state.turn, snapshot.meta.scenario_id
    )
    .chars()
    .map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            c
        } else {
            '-'
        }
    })
    .collect()
}

// ---------------------------------------------------------------------------
// Branch runs
// ---------------------------------------------------------------------------

/// One action the user forces at a specific turn instead of the policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOverride {
    pub turn: i32,
    pub action_id: String,
}

/// How a branch is played forward from its checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchConfig {
    /// Built-in policy: `"bot"` (scoring) or `"default"` (heuristic).
    #[serde(default = "default_policy_name")]
    pub policy: String,
    #[serde(default = "default_max_actions")]
    pub max_actions: i32,
    #[serde(default)]
    pub overrides: Vec<ActionOverride>,
}

fn default_policy_name() -> String {
    "bot".to_string()
}

fn default_max_actions() -> i32 {
    500
}

impl Default for BranchConfig {
    fn default() -> Self {
        Self {
            policy: default_policy_name(),
            max_actions: default_max_actions(),
            overrides: Vec::new(),
        }
    }
}

fn action_choice_id(action: &SimAction) -> String {
    match (&action.kind, &action.payload) {
        (SimActionKind::Train, Some(p)) => format!("train_{p}"),
        (SimActionKind::Choose, Some(p)) => format!("event_{p}"),
        (SimActionKind::Lesson, Some(p)) => p.clone(),
        (SimActionKind::Rest, _) => "rest".to_string(),
        (SimActionKind::Recreation, _) => "recreation".to_string(),
        (SimActionKind::Race, _) => "race".to_string(),
        (SimActionKind::Advance, _) => "advance".to_string(),
        _ => "rest".to_string(),
    }
}

fn mood_label(mood: &crate::state::MoodLevel) -> &'static str {
    match mood {
        crate::state::MoodLevel::Great => "GREAT",
        crate::state::MoodLevel::Good => "GOOD",
        crate::state::MoodLevel::Normal => "NORMAL",
        crate::state::MoodLevel::Bad => "BAD",
        crate::state::MoodLevel::Awful => "AWFUL",
    }
}

/// One recorded step of a branch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchStep {
    pub step_index: usize,
    pub turn: i32,
    pub date_label: String,
    pub phase: String,
    pub action_id: String,
    pub action_label: String,
    /// Was a user override applied at this turn?
    pub override_applied: bool,
    /// Was a user override rejected (not a legal choice this turn)?
    pub override_rejected: bool,
    pub rng_calls_before: u32,
    pub rng_calls_after: u32,
    pub energy: i32,
    pub mood: String,
    pub fans: i32,
    pub skill_points: i32,
    pub stats: TraineeStats,
    /// Races completed by this step (ids appended since the previous step).
    pub new_races: Vec<String>,
    pub total_races: usize,
    pub total_skills: usize,
}

/// Final outcome of a branch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchOutcome {
    pub steps: usize,
    pub final_turn: i32,
    pub career_complete: bool,
    pub completed_races: Vec<String>,
    pub stats: TraineeStats,
    pub energy: i32,
    pub mood: String,
    pub fans: i32,
    pub skill_points: i32,
    pub learned_skills: Vec<String>,
    pub sparks: Vec<SparkSummary>,
    pub scenario_resources: BTreeMap<String, i32>,
    pub total_rng_calls: u32,
    pub telemetry_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparkSummary {
    pub color: String,
    pub factor_id: String,
    pub stars: i32,
    pub label: String,
}

impl From<&GeneratedSpark> for SparkSummary {
    fn from(s: &GeneratedSpark) -> Self {
        Self {
            color: s.color.clone(),
            factor_id: s.factor_id.clone(),
            stars: s.stars,
            label: s.label.clone(),
        }
    }
}

/// A completed branch: checkpoint + config + full timeline + outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabResult {
    pub id: String,
    pub name: String,
    pub checkpoint_name: String,
    pub checkpoint_turn: i32,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub config: BranchConfig,
    pub started_at: String,
    pub timeline: Vec<BranchStep>,
    pub outcome: BranchOutcome,
}

static BRANCH_COUNTER: AtomicU64 = AtomicU64::new(0);

fn branch_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let n = BRANCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    // Process identity and subsecond time distinguish independent invocations.
    // Publication still refuses collisions; generated IDs are not a lock.
    format!(
        "br-{}-{:09}-{}-{n:016x}",
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id()
    )
}

fn choose_action(
    engine: &SimEngine,
    choices: &[SimChoice],
    turn: i32,
    config: &BranchConfig,
) -> (String, String, SimAction, bool, bool) {
    // Returns (action_id, action_label, action, override_applied, override_rejected).
    if let Some(ov) = config.overrides.iter().find(|o| o.turn == turn) {
        if let Some(choice) = choices.iter().find(|c| c.id == ov.action_id) {
            return (
                choice.id.clone(),
                choice.label.clone(),
                parse_sim_action(&ov.action_id),
                true,
                false,
            );
        }
        // Override not legal this turn: fall through to the policy, flagged.
        let (id, label, action) = policy_action(engine, choices, config);
        return (id, label, action, false, true);
    }
    let (id, label, action) = policy_action(engine, choices, config);
    (id, label, action, false, false)
}

fn policy_action(
    engine: &SimEngine,
    choices: &[SimChoice],
    config: &BranchConfig,
) -> (String, String, SimAction) {
    let action = if config.policy == "default" {
        engine
            .default_action()
            .unwrap_or_else(|| parse_sim_action("rest"))
    } else {
        engine
            .scoring_action()
            .unwrap_or_else(|| parse_sim_action("rest"))
    };
    let id = action_choice_id(&action);
    let label = choices
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.label.clone())
        .unwrap_or_else(|| id.clone());
    (id, label, action)
}

/// Play `snapshot` forward to completion (or `max_actions` steps) and record
/// the full timeline. The branch runs in a fresh engine with its own isolated
/// telemetry; it never touches the library or any other engine.
pub fn run_branch(
    snapshot: &RunSnapshot,
    name: &str,
    checkpoint_name: &str,
    config: &BranchConfig,
) -> LabResult {
    // Per-branch telemetry: a fresh engine means this branch's records are
    // isolated from the checkpoint source and from sibling branches. Note
    // `restore` adopts the snapshot's settings wholesale, so the flag must be
    // set on the snapshot copy *before* restoring.
    let mut snap = snapshot.clone();
    snap.settings.trace_telemetry = true;
    let mut engine = SimEngine::create(snap.settings.clone());
    engine.restore(snap);

    let mut timeline = Vec::new();
    let mut steps: usize = 0;
    let max = config.max_actions.max(1) as usize;
    while !engine.state().career_complete && steps < max {
        let choices = engine.choices();
        if choices.is_empty() {
            break;
        }
        let state: &CareerState = engine.state();
        let turn = state.turn;
        let pre_races = state.completed_races.clone();
        let pre_label = date_label(&state.date);
        let pre_phase = state.phase.clone();
        let rng_before = engine.rng_calls();
        let (action_id, action_label, action, ov_applied, ov_rejected) =
            choose_action(&engine, &choices, turn, config);
        engine.step(action);
        let rng_after = engine.rng_calls();
        let post: &CareerState = engine.state();
        // Races completed by this step: post-step ids not present pre-step
        // (order-preserving; robust to any list reordering).
        let new_races: Vec<String> = post
            .completed_races
            .iter()
            .filter(|r| !pre_races.contains(r))
            .cloned()
            .collect();
        timeline.push(BranchStep {
            step_index: steps,
            turn,
            date_label: pre_label,
            phase: pre_phase,
            action_id,
            action_label,
            override_applied: ov_applied,
            override_rejected: ov_rejected,
            rng_calls_before: rng_before,
            rng_calls_after: rng_after,
            energy: post.energy,
            mood: mood_label(&post.mood).to_string(),
            fans: post.fans,
            skill_points: post.skill_points,
            stats: post.stats.clone(),
            new_races,
            total_races: post.completed_races.len(),
            total_skills: post.learned_skill_ids.len(),
        });
        steps += 1;
    }

    let final_state = engine.state();
    let outcome = BranchOutcome {
        steps,
        final_turn: final_state.turn,
        career_complete: final_state.career_complete,
        completed_races: final_state.completed_races.clone(),
        stats: final_state.stats.clone(),
        energy: final_state.energy,
        mood: mood_label(&final_state.mood).to_string(),
        fans: final_state.fans,
        skill_points: final_state.skill_points,
        learned_skills: final_state.learned_skill_ids.clone(),
        sparks: final_state
            .generated_sparks
            .iter()
            .map(SparkSummary::from)
            .collect(),
        scenario_resources: final_state
            .scenario_resources
            .values
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        total_rng_calls: engine.rng_calls(),
        telemetry_records: engine.telemetry_log().len(),
    };

    LabResult {
        id: branch_id(),
        name: name.to_string(),
        checkpoint_name: checkpoint_name.to_string(),
        checkpoint_turn: snapshot.state.turn,
        seed: snapshot.meta.seed,
        scenario_id: snapshot.meta.scenario_id.clone(),
        trainee_name: snapshot.meta.trainee_name.clone(),
        config: config.clone(),
        started_at: rfc3339_utc(now_unix()),
        timeline,
        outcome,
    }
}

/// Persisted branch-result store (`.uma-sim/lab/<id>.json`, atomic writes).
pub struct BranchStore {
    dir: PathBuf,
}

impl Default for BranchStore {
    fn default() -> Self {
        let dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join("lab");
        Self { dir }
    }
}

impl BranchStore {
    /// Store rooted at the current directory's `.uma-sim/lab/`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store rooted at an explicit directory (used by tests).
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, id: &str) -> Result<PathBuf, LabError> {
        if id.contains('/') || id.contains('\\') || id.contains('.') {
            return Err(LabError::InvalidName(
                "branch ids are opaque 'br-…' strings".to_string(),
            ));
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    /// Create an immutable branch result; an existing ID is never replaced.
    /// The complete temporary file is published atomically with a hard link,
    /// which refuses an existing destination even across competing processes.
    pub fn save(&self, result: &LabResult) -> Result<PathBuf, LabError> {
        let path = self.path_for(&result.id)?;
        let bytes =
            serde_json::to_string_pretty(result).map_err(|e| LabError::Io(e.to_string()))?;
        fs::create_dir_all(&self.dir).map_err(|e| LabError::Io(e.to_string()))?;
        let tmp = self.dir.join(format!(".tmp-{}-{}", branch_id(), result.id));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| LabError::Io(format!("create temporary branch file: {e}")))?;
        let ready = file
            .write_all(bytes.as_bytes())
            .and_then(|_| file.sync_all());
        drop(file);
        // Do not use a replacing rename here: two processes can independently
        // generate or receive the same ID before either result is published.
        let published = ready.and_then(|_| fs::hard_link(&tmp, &path));
        let _ = fs::remove_file(&tmp);
        published.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                LabError::AlreadyExists(result.id.clone())
            } else {
                LabError::Io(format!("publish branch result '{}': {e}", result.id))
            }
        })?;
        Ok(path)
    }

    /// List persisted branch results, newest first (by filename ordering).
    pub fn list(&self) -> Result<Vec<LabResult>, LabError> {
        let mut results = Vec::new();
        let read = fs::read_dir(&self.dir);
        let dir_entries = match read {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(results),
            Err(e) => return Err(LabError::Io(e.to_string())),
        };
        let mut paths: Vec<PathBuf> = dir_entries
            .flatten()
            .map(|de| de.path())
            .filter(|p| {
                p.extension().map(|e| e == "json").unwrap_or(false)
                    && p.file_stem()
                        .map(|s| s.to_string_lossy().starts_with("br-"))
                        .unwrap_or(false)
            })
            .collect();
        paths.sort();
        paths.reverse();
        for path in paths {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(r) = serde_json::from_str::<LabResult>(&raw) {
                    results.push(r);
                }
            }
        }
        Ok(results)
    }

    /// Load one persisted branch result by id.
    pub fn load(&self, id: &str) -> Result<LabResult, LabError> {
        let path = self.path_for(id)?;
        let raw = fs::read_to_string(&path).map_err(|_| LabError::NotFound(id.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| LabError::Io(e.to_string()))
    }

    /// Delete a persisted branch result.
    pub fn delete(&self, id: &str) -> Result<(), LabError> {
        let path = self.path_for(id)?;
        if !path.exists() {
            return Err(LabError::NotFound(id.to_string()));
        }
        fs::remove_file(&path).map_err(|e| LabError::Io(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// One aligned row of the A/B timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompare {
    pub step_index: usize,
    pub turn_a: Option<i32>,
    pub turn_b: Option<i32>,
    pub action_a: Option<String>,
    pub action_b: Option<String>,
    pub label_a: Option<String>,
    pub label_b: Option<String>,
    pub same_action: bool,
    pub energy_a: Option<i32>,
    pub energy_b: Option<i32>,
    pub mood_a: Option<String>,
    pub mood_b: Option<String>,
    pub fans_a: Option<i32>,
    pub fans_b: Option<i32>,
    pub skill_points_a: Option<i32>,
    pub skill_points_b: Option<i32>,
    /// Post-step observable state matched this row (stats/energy/mood/fans/
    /// skill points/races/skills).
    pub same_outcome: bool,
}

/// The first step where the branches visibly differ.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Divergence {
    pub step_index: usize,
    pub turn: i32,
    pub date_label: String,
    pub phase: String,
    /// `"decision"` (different actions chosen) or `"outcome"` (same action,
    /// different result — RNG or state had already diverged).
    pub kind: String,
    pub action_a: String,
    pub action_b: String,
    pub label_a: String,
    pub label_b: String,
    pub rng_calls_a_after: u32,
    pub rng_calls_b_after: u32,
    pub note: String,
}

/// Full A/B comparison of two branch results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchComparison {
    pub a_id: String,
    pub a_name: String,
    pub b_id: String,
    pub b_name: String,
    pub checkpoint_name: String,
    pub checkpoint_turn: i32,
    pub seed: i64,
    pub same_checkpoint: bool,
    pub compared_at: String,
    pub first_divergence: Option<Divergence>,
    pub aligned: Vec<TurnCompare>,
    pub outcome_a: BranchOutcome,
    pub outcome_b: BranchOutcome,
    /// Honesty caveats; always present.
    pub caveats: Vec<String>,
}

/// Canonical post-step observable state used for outcome matching.
fn outcome_key(step: &BranchStep) -> String {
    serde_json::json!({
        "stats": step.stats,
        "energy": step.energy,
        "mood": step.mood,
        "fans": step.fans,
        "skillPoints": step.skill_points,
        "races": step.total_races,
        "skills": step.total_skills,
    })
    .to_string()
}

fn standard_caveats() -> Vec<String> {
    vec![
        "Both branches share the checkpoint's seed and RNG position, but RNG streams diverge the moment actions differ; later randomness is NOT matched between branches."
            .to_string(),
        "This comparison describes observed divergence only. It does not establish that one branch's decisions caused its outcome, nor that either branch is superior."
            .to_string(),
        "Action overrides that were not legal on their turn were rejected and fell back to the branch policy (flagged per step as overrideRejected)."
            .to_string(),
    ]
}

/// Compare two branch results step-by-step.
pub fn compare_branches(a: &LabResult, b: &LabResult) -> BranchComparison {
    let n = a.timeline.len().max(b.timeline.len());
    let mut aligned = Vec::with_capacity(n);
    let mut first_divergence: Option<Divergence> = None;

    for i in 0..n {
        let sa = a.timeline.get(i);
        let sb = b.timeline.get(i);
        let same_action = match (sa, sb) {
            (Some(x), Some(y)) => x.action_id == y.action_id,
            _ => false,
        };
        let same_outcome = match (sa, sb) {
            (Some(x), Some(y)) => outcome_key(x) == outcome_key(y),
            _ => false,
        };
        if first_divergence.is_none() {
            if let (Some(x), Some(y)) = (sa, sb) {
                if !same_action {
                    first_divergence = Some(Divergence {
                        step_index: i,
                        turn: x.turn,
                        date_label: x.date_label.clone(),
                        phase: x.phase.clone(),
                        kind: "decision".to_string(),
                        action_a: x.action_id.clone(),
                        action_b: y.action_id.clone(),
                        label_a: x.action_label.clone(),
                        label_b: y.action_label.clone(),
                        rng_calls_a_after: x.rng_calls_after,
                        rng_calls_b_after: y.rng_calls_after,
                        note: format!(
                            "A chose '{}' while B chose '{}' on turn {} ({}). RNG positions after the step: A={} calls, B={} calls.",
                            x.action_label, y.action_label, x.turn, x.date_label, x.rng_calls_after, y.rng_calls_after
                        ),
                    });
                } else if !same_outcome {
                    first_divergence = Some(Divergence {
                        step_index: i,
                        turn: x.turn,
                        date_label: x.date_label.clone(),
                        phase: x.phase.clone(),
                        kind: "outcome".to_string(),
                        action_a: x.action_id.clone(),
                        action_b: y.action_id.clone(),
                        label_a: x.action_label.clone(),
                        label_b: y.action_label.clone(),
                        rng_calls_a_after: x.rng_calls_after,
                        rng_calls_b_after: y.rng_calls_after,
                        note: format!(
                            "Both branches chose '{}' on turn {} ({}), but the post-step state differs — the RNG streams or prior state had already diverged.",
                            x.action_label, x.turn, x.date_label
                        ),
                    });
                }
            } else {
                // One branch ended earlier: that itself is the divergence.
                let (side, step) = match (sa, sb) {
                    (Some(x), None) => ("B", x),
                    (None, Some(y)) => ("A", y),
                    _ => continue,
                };
                first_divergence = Some(Divergence {
                    step_index: i,
                    turn: step.turn,
                    date_label: step.date_label.clone(),
                    phase: step.phase.clone(),
                    kind: "outcome".to_string(),
                    action_a: sa.map(|x| x.action_id.clone()).unwrap_or_default(),
                    action_b: sb.map(|x| x.action_id.clone()).unwrap_or_default(),
                    label_a: sa.map(|x| x.action_label.clone()).unwrap_or_default(),
                    label_b: sb.map(|x| x.action_label.clone()).unwrap_or_default(),
                    rng_calls_a_after: sa.map(|x| x.rng_calls_after).unwrap_or(0),
                    rng_calls_b_after: sb.map(|x| x.rng_calls_after).unwrap_or(0),
                    note: format!(
                        "Branch {side} ended after {} steps while the other continued.",
                        if side == "A" {
                            a.timeline.len()
                        } else {
                            b.timeline.len()
                        }
                    ),
                });
            }
        }
        aligned.push(TurnCompare {
            step_index: i,
            turn_a: sa.map(|x| x.turn),
            turn_b: sb.map(|x| x.turn),
            action_a: sa.map(|x| x.action_id.clone()),
            action_b: sb.map(|x| x.action_id.clone()),
            label_a: sa.map(|x| x.action_label.clone()),
            label_b: sb.map(|x| x.action_label.clone()),
            same_action,
            energy_a: sa.map(|x| x.energy),
            energy_b: sb.map(|x| x.energy),
            mood_a: sa.map(|x| x.mood.clone()),
            mood_b: sb.map(|x| x.mood.clone()),
            fans_a: sa.map(|x| x.fans),
            fans_b: sb.map(|x| x.fans),
            skill_points_a: sa.map(|x| x.skill_points),
            skill_points_b: sb.map(|x| x.skill_points),
            same_outcome,
        });
    }

    BranchComparison {
        a_id: a.id.clone(),
        a_name: a.name.clone(),
        b_id: b.id.clone(),
        b_name: b.name.clone(),
        checkpoint_name: a.checkpoint_name.clone(),
        checkpoint_turn: a.checkpoint_turn,
        seed: a.seed,
        same_checkpoint: a.checkpoint_name == b.checkpoint_name
            && a.seed == b.seed
            && a.checkpoint_turn == b.checkpoint_turn,
        compared_at: rfc3339_utc(now_unix()),
        first_divergence,
        aligned,
        outcome_a: a.outcome.clone(),
        outcome_b: b.outcome.clone(),
        caveats: standard_caveats(),
    }
}

// ---------------------------------------------------------------------------
// Report rendering
// ---------------------------------------------------------------------------

fn stats_line(s: &TraineeStats) -> String {
    format!(
        "Spd {} / Sta {} / Pow {} / Gut {} / Wit {}",
        s.speed, s.stamina, s.power, s.guts, s.wit
    )
}

/// Keep report data literal in prose, headings and GFM table cells. Line
/// breaks stay inside their existing block instead of creating rows/headings.
fn markdown_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' | '\n' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str("<br>");
            }
            ch if ch.is_ascii_punctuation() => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out
}

/// Code spans do not interpret text escapes or entities. Choose a delimiter
/// longer than any backtick run in the value and protect significant padding.
fn markdown_code(value: &str) -> String {
    let value = value.replace("\r\n", " ").replace(['\r', '\n'], " ");
    if value.is_empty() {
        return String::new();
    }
    let mut longest = 0;
    let mut run = 0;
    for ch in value.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let delimiter = "`".repeat(longest + 1);
    let padding = if value.starts_with('`')
        || value.ends_with('`')
        || (value.starts_with(' ') && value.ends_with(' ') && !value.chars().all(|ch| ch == ' '))
    {
        " "
    } else {
        ""
    };
    format!("{delimiter}{padding}{value}{padding}{delimiter}")
}

/// Render the comparison as a downloadable Markdown report.
pub fn render_markdown(c: &BranchComparison) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Branch comparison: {} vs {}\n\n",
        markdown_text(&c.a_name),
        markdown_text(&c.b_name)
    ));
    out.push_str(&format!(
        "- Checkpoint: {} (seed {}, turn {})\n",
        markdown_code(&c.checkpoint_name),
        c.seed,
        c.checkpoint_turn
    ));
    out.push_str(&format!(
        "- Same checkpoint: {}\n- Compared at: {}\n\n",
        c.same_checkpoint,
        markdown_text(&c.compared_at)
    ));

    out.push_str("## First divergence\n\n");
    match &c.first_divergence {
        Some(d) => {
            out.push_str(&format!(
                "- Step {} · turn {} ({}) · phase {} · kind {}\n- A: {} — {}\n- B: {} — {}\n- {}\n",
                d.step_index,
                d.turn,
                markdown_text(&d.date_label),
                markdown_code(&d.phase),
                markdown_code(&d.kind),
                markdown_code(&d.action_a),
                markdown_text(&d.label_a),
                markdown_code(&d.action_b),
                markdown_text(&d.label_b),
                markdown_text(&d.note)
            ));
        }
        None => out.push_str("No divergence: the recorded timelines are identical.\n"),
    }

    out.push_str("\n## Timeline (per step)\n\n");
    out.push_str("| step | turn | A action | B action | Δ | energy A→B | fans A→B | SP A→B |\n");
    out.push_str("| ---: | ---: | -------- | -------- | - | ---------: | -------: | -----: |\n");
    for row in &c.aligned {
        let mark = if row.same_action && row.same_outcome {
            ""
        } else if !row.same_action {
            "≠ act"
        } else {
            "≠ out"
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {}→{} | {}→{} | {}→{} |\n",
            row.step_index,
            row.turn_a
                .map(|t| t.to_string())
                .unwrap_or_else(|| "–".into()),
            markdown_text(row.label_a.as_deref().unwrap_or("–")),
            markdown_text(row.label_b.as_deref().unwrap_or("–")),
            mark,
            row.energy_a
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
            row.energy_b
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
            row.fans_a
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
            row.fans_b
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
            row.skill_points_a
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
            row.skill_points_b
                .map(|e| e.to_string())
                .unwrap_or_else(|| "–".into()),
        ));
    }

    out.push_str("\n## Final outcomes\n\n");
    out.push_str("|  | A | B |\n| - | - | - |\n");
    let oa = &c.outcome_a;
    let ob = &c.outcome_b;
    let row = |k: &str, a: String, b: String| {
        format!("| {k} | {} | {} |\n", markdown_text(&a), markdown_text(&b))
    };
    out.push_str(&row("Branch", c.a_name.clone(), c.b_name.clone()));
    out.push_str(&row("Steps", oa.steps.to_string(), ob.steps.to_string()));
    out.push_str(&row(
        "Career complete",
        oa.career_complete.to_string(),
        ob.career_complete.to_string(),
    ));
    out.push_str(&row(
        "Final turn",
        oa.final_turn.to_string(),
        ob.final_turn.to_string(),
    ));
    out.push_str(&row("Stats", stats_line(&oa.stats), stats_line(&ob.stats)));
    out.push_str(&row(
        "Energy / mood",
        format!("{} / {}", oa.energy, oa.mood),
        format!("{} / {}", ob.energy, ob.mood),
    ));
    out.push_str(&row("Fans", oa.fans.to_string(), ob.fans.to_string()));
    out.push_str(&row(
        "Skill points",
        oa.skill_points.to_string(),
        ob.skill_points.to_string(),
    ));
    out.push_str(&row(
        "Races",
        format!(
            "{} ({})",
            oa.completed_races.len(),
            oa.completed_races.join(", ")
        ),
        format!(
            "{} ({})",
            ob.completed_races.len(),
            ob.completed_races.join(", ")
        ),
    ));
    out.push_str(&row(
        "Skills learned",
        format!(
            "{}: {}",
            oa.learned_skills.len(),
            oa.learned_skills.join(", ")
        ),
        format!(
            "{}: {}",
            ob.learned_skills.len(),
            ob.learned_skills.join(", ")
        ),
    ));
    out.push_str(&row(
        "Sparks",
        oa.sparks
            .iter()
            .map(|s| format!("{}★{} {}", s.color, s.stars, s.label))
            .collect::<Vec<_>>()
            .join("; "),
        ob.sparks
            .iter()
            .map(|s| format!("{}★{} {}", s.color, s.stars, s.label))
            .collect::<Vec<_>>()
            .join("; "),
    ));
    out.push_str(&row(
        "RNG calls",
        oa.total_rng_calls.to_string(),
        ob.total_rng_calls.to_string(),
    ));

    out.push_str("\n## Caveats\n\n");
    for caveat in &c.caveats {
        out.push_str(&format!("- {}\n", markdown_text(caveat)));
    }
    out.push_str("\n_Generated by uma-sim career lab._\n");
    out
}

/// Render the comparison as pretty JSON (same shape as the API payload).
pub fn render_json(c: &BranchComparison) -> String {
    serde_json::to_string_pretty(c).unwrap_or_else(|_| "{}".to_string())
}

/// Delete a persisted branch result (cwd store convenience).
pub fn delete_lab_result(id: &str) -> Result<(), LabError> {
    BranchStore::new().delete(id)
}

/// Short JSON-able summary of a branch result for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabResultSummary {
    pub id: String,
    pub name: String,
    pub checkpoint_name: String,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub policy: String,
    pub steps: usize,
    pub career_complete: bool,
    pub final_turn: i32,
    pub fans: i32,
    pub started_at: String,
}

impl From<&LabResult> for LabResultSummary {
    fn from(r: &LabResult) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            checkpoint_name: r.checkpoint_name.clone(),
            seed: r.seed,
            scenario_id: r.scenario_id.clone(),
            trainee_name: r.trainee_name.clone(),
            policy: r.config.policy.clone(),
            steps: r.outcome.steps,
            career_complete: r.outcome.career_complete,
            final_turn: r.outcome.final_turn,
            fans: r.outcome.fans,
            started_at: r.started_at.clone(),
        }
    }
}

/// Session info for the API session registry (fork/resume support).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub label: String,
    pub turn: i32,
    pub phase: String,
    pub career_complete: bool,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
}

/// Remove stale `.tmp-*` files left by interrupted atomic writes.
pub fn sweep_stale_tmp(dir: &Path) -> usize {
    let mut removed = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for de in entries.flatten() {
            let name = de.file_name().to_string_lossy().into_owned();
            if name.starts_with(".tmp-") {
                if fs::remove_file(de.path()).is_ok() {
                    removed += 1;
                }
            }
        }
    }
    removed
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn refused_snapshot_restoration_retains_recoverable_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "uma-refused-rollback-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let target = dir.join("keep.snapshot.json");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("unrelated"), b"preserve").unwrap();
        let backup = dir.join(".rollback-previous.snapshot.json");
        let previous = br#"{"previous":"saved career"}"#;
        fs::write(&backup, previous).unwrap();

        let error = rollback_snapshot(
            &target,
            Some(&backup),
            LabError::Io("metadata publication was refused".into()),
        );
        assert!(matches!(error, LabError::Io(_)));
        let message = error.to_string();
        assert!(message.contains("metadata publication was refused"));
        assert!(message.contains("snapshot restoration failed"));
        assert!(message.contains(&backup.display().to_string()));
        assert_eq!(fs::read(&backup).unwrap(), previous);
        assert_eq!(fs::read(target.join("unrelated")).unwrap(), b"preserve");
        fs::write(dir.join(".tmp-stale"), b"partial").unwrap();
        assert_eq!(sweep_stale_tmp(&dir), 1);
        assert_eq!(fs::read(&backup).unwrap(), previous);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rfc3339_formats_epoch() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        // 2026-10-03T00:00:00Z
        assert_eq!(rfc3339_utc(1790985600), "2026-10-03T00:00:00Z");
    }

    #[test]
    fn name_validation() {
        assert!(validate_name("before-debut").is_ok());
        assert!(validate_name("a_b.c-1").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("semi;colon").is_err());
        assert!(validate_name(&"x".repeat(65)).is_err());
        assert!(validate_name("../evil").is_err());
    }

    #[test]
    fn action_choice_id_round_trips() {
        for id in [
            "train_speed",
            "train_stamina",
            "rest",
            "recreation",
            "race",
            "event_3",
            "gl_song_foo",
        ] {
            let action = parse_sim_action(id);
            assert_eq!(action_choice_id(&action), id, "id {id}");
        }
    }

    #[test]
    fn fingerprint_check_flags_schema_but_not_version() {
        let a = ContentFingerprint {
            core_version: "0.1.0".into(),
            snapshot_schema: 1,
            event_catalog_count: 10,
            content_pack_events: 0,
            repo_root_detected: true,
        };
        let mut b = a.clone();
        b.core_version = "0.2.0".into();
        let (blocking, advisory) = a.check_against(&b);
        assert!(blocking.is_empty());
        assert_eq!(advisory.len(), 1);
        b.snapshot_schema = 2;
        let (blocking, _) = a.check_against(&b);
        assert_eq!(blocking.len(), 1);
    }
}
