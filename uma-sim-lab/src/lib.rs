//! uma-sim saved-career branch-and-compare laboratory.
//!
//! A career library of named, durable checkpoints. Fork any checkpoint into
//! independent branches, play each branch forward with its own RNG stream
//! (restored from the checkpoint's seed + RNG call count, then evolved
//! independently per branch), and compare the branches side by side.
//!
//! One branch can never mutate its checkpoint of origin or a sibling:
//! every `fork` deep-copies the checkpoint snapshot, and every `play`
//! persists only the branch's own head snapshot.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use uma_sim_core::session::parse_sim_action;
use uma_sim_core::snapshot::{RunSnapshot, RunSnapshotCodec};
use uma_sim_core::state::{CareerState, SimAction, SimActionKind, SimChoice};
use uma_sim_core::{default_auto_policy, SimEngine, SimSettings};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum LabError {
    Io(String),
    Json(String),
    NotFound(String),
    Exists(String),
    Engine(String),
    Policy(String),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabError::Io(e) => write!(f, "io: {e}"),
            LabError::Json(e) => write!(f, "json: {e}"),
            LabError::NotFound(e) => write!(f, "not found: {e}"),
            LabError::Exists(e) => write!(f, "already exists: {e}"),
            LabError::Engine(e) => write!(f, "engine: {e}"),
            LabError::Policy(e) => write!(f, "policy: {e}"),
        }
    }
}

impl std::error::Error for LabError {}

pub type LabResult<T> = Result<T, LabError>;

fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

fn check_name(name: &str) -> LabResult<()> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(LabError::Engine(format!(
            "name must be 1-64 chars of [a-zA-Z0-9_-], got {name:?}"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/// A named, durable checkpoint: full engine snapshot plus the visible
/// seed / scenario / turn context the player needs to recognize it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub name: String,
    pub created_at: String,
    pub note: String,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub turn: i32,
    pub career_complete: bool,
    pub rng_calls: u32,
    pub snapshot: RunSnapshot,
}

/// One independent continuation forked from a checkpoint. Its `snapshot` is
/// the branch head — a deep copy at fork time, evolved only by this branch's
/// own play steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRecord {
    pub name: String,
    pub parent_checkpoint: String,
    pub created_at: String,
    pub note: String,
    pub snapshot: RunSnapshot,
}

/// Compact listing info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub name: String,
    pub created_at: String,
    pub note: String,
    pub seed: i64,
    pub scenario_id: String,
    pub turn: i32,
    pub career_complete: bool,
    pub rng_calls: u32,
}

/// One recorded action while playing a branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    pub step: u32,
    pub turn: i32,
    pub action_id: String,
    pub energy: i32,
    pub mood: String,
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
    pub fans: i32,
    pub skill_points: i32,
    pub races_completed: usize,
    pub phase: String,
    pub career_complete: bool,
    pub rng_calls: u32,
}

/// A recorded play-through of a branch head.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub branch: String,
    pub policy: String,
    pub played_at: String,
    pub start_turn: i32,
    pub end_turn: i32,
    pub steps: Vec<TurnRecord>,
    pub hash: String,
}

/// Side-by-side comparison of two branch timelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub branch_a: String,
    pub branch_b: String,
    pub steps_compared: usize,
    pub first_divergence_step: Option<u32>,
    pub first_divergence_field: Option<String>,
    pub rows: Vec<CompareRow>,
    pub outcome_a: OutcomeSummary,
    pub outcome_b: OutcomeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareRow {
    pub step: u32,
    pub turn_a: i32,
    pub turn_b: i32,
    pub action_a: String,
    pub action_b: String,
    pub energy_a: i32,
    pub energy_b: i32,
    pub stats_a: String,
    pub stats_b: String,
    pub fans_a: i32,
    pub fans_b: i32,
    pub diverged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSummary {
    pub branch: String,
    pub career_complete: bool,
    pub final_turn: i32,
    pub stats: String,
    pub fans: i32,
    pub skill_points: i32,
    pub races_completed: usize,
    pub rng_calls: u32,
}

// ---------------------------------------------------------------------------
// Policies (built-in; game mechanics untouched)
// ---------------------------------------------------------------------------

fn train_choice(choices: &[SimChoice], id: &str) -> Option<SimAction> {
    choices.iter().any(|c| c.id == id).then_some(SimAction {
        kind: SimActionKind::Train,
        payload: Some(id.trim_start_matches("train_").to_string()),
    })
}

fn race_choice(choices: &[SimChoice]) -> Option<SimAction> {
    choices.iter().any(|c| c.id == "race").then_some(SimAction {
        kind: SimActionKind::Race,
        payload: None,
    })
}

fn event_choice(choices: &[SimChoice]) -> Option<SimAction> {
    let id = choices.iter().find(|c| c.id.starts_with("event_"))?;
    Some(SimAction {
        kind: SimActionKind::Choose,
        payload: Some(id.id.trim_start_matches("event_").to_string()),
    })
}

/// Built-in branch policies. Each sees the current choices and the career
/// state (so e.g. `restful` can react to low energy). Game mechanics are
/// untouched — policies only select among offered actions.
/// Names: `default`, `speed`, `stamina`, `restful`, `racer`.
pub type LabPolicy = fn(&[SimChoice], &CareerState) -> SimAction;

pub fn resolve_policy(name: &str) -> LabResult<LabPolicy> {
    match name {
        "default" => Ok(|c, _| default_auto_policy(c)),
        "speed" => Ok(|c, _| {
            event_choice(c)
                .or_else(|| race_choice_mandatory_only(c))
                .or_else(|| train_choice(c, "train_speed"))
                .unwrap_or_else(|| default_auto_policy(c))
        }),
        "stamina" => Ok(|c, _| {
            event_choice(c)
                .or_else(|| race_choice_mandatory_only(c))
                .or_else(|| train_choice(c, "train_stamina"))
                .unwrap_or_else(|| default_auto_policy(c))
        }),
        "restful" => Ok(|c, s| {
            if s.energy < 35 {
                if let Some(rest) = c.iter().any(|x| x.id == "rest").then_some(SimAction {
                    kind: SimActionKind::Rest,
                    payload: None,
                }) {
                    return rest;
                }
            }
            event_choice(c)
                .or_else(|| race_choice_mandatory_only(c))
                .or_else(|| train_choice(c, "train_speed"))
                .unwrap_or_else(|| default_auto_policy(c))
        }),
        "racer" => Ok(|c, _| {
            event_choice(c)
                .or_else(|| race_choice(c))
                .or_else(|| train_choice(c, "train_speed"))
                .unwrap_or_else(|| default_auto_policy(c))
        }),
        other => Err(LabError::Policy(format!(
            "unknown policy {other:?}; expected one of default|speed|stamina|restful|racer"
        ))),
    }
}

fn race_choice_mandatory_only(choices: &[SimChoice]) -> Option<SimAction> {
    choices
        .iter()
        .any(|c| c.id == "race" && c.label.to_lowercase().contains("mandatory"))
        .then_some(SimAction {
            kind: SimActionKind::Race,
            payload: None,
        })
}

pub fn policy_names() -> &'static [&'static str] {
    &["default", "speed", "stamina", "restful", "racer"]
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// Rooted career library: `checkpoints/`, `branches/`, `timelines/` below `dir`.
#[derive(Debug, Clone)]
pub struct LabLibrary {
    dir: PathBuf,
}

impl LabLibrary {
    /// Default location: `$UMA_LAB_DIR`, else `~/.uma-sim/lab`.
    pub fn default_dir() -> PathBuf {
        if let Ok(d) = std::env::var("UMA_LAB_DIR") {
            if !d.is_empty() {
                return PathBuf::from(d);
            }
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".uma-sim").join("lab")
    }

    pub fn new(dir: PathBuf) -> LabResult<Self> {
        for sub in ["checkpoints", "branches", "timelines"] {
            fs::create_dir_all(dir.join(sub)).map_err(|e| LabError::Io(e.to_string()))?;
        }
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn cp_path(&self, name: &str) -> PathBuf {
        self.dir.join("checkpoints").join(format!("{name}.json"))
    }

    fn br_path(&self, name: &str) -> PathBuf {
        self.dir.join("branches").join(format!("{name}.json"))
    }

    fn tl_path(&self, name: &str) -> PathBuf {
        self.dir.join("timelines").join(format!("{name}.json"))
    }

    fn write_json<T: Serialize>(path: &Path, value: &T) -> LabResult<()> {
        let s = serde_json::to_string_pretty(value).map_err(|e| LabError::Json(e.to_string()))?;
        fs::write(path, s).map_err(|e| LabError::Io(e.to_string()))
    }

    fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> LabResult<T> {
        let raw = fs::read_to_string(path).map_err(|e| LabError::Io(e.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| LabError::Json(e.to_string()))
    }

    fn list_dir(sub: &str, dir: &Path) -> LabResult<Vec<String>> {
        let mut out = Vec::new();
        let rd = fs::read_dir(dir.join(sub)).map_err(|e| LabError::Io(e.to_string()))?;
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    out.push(stem.to_string());
                }
            }
        }
        out.sort();
        Ok(out)
    }

    // -- checkpoints -------------------------------------------------------

    /// Save the engine's current state as a named durable checkpoint.
    pub fn save_checkpoint(
        &self,
        engine: &SimEngine,
        name: &str,
        note: &str,
    ) -> LabResult<CheckpointRecord> {
        check_name(name)?;
        let path = self.cp_path(name);
        if path.exists() {
            return Err(LabError::Exists(format!("checkpoint {name}")));
        }
        let snapshot = engine.export();
        let rec = CheckpointRecord {
            name: name.to_string(),
            created_at: now_stamp(),
            note: note.to_string(),
            seed: snapshot.meta.seed,
            scenario_id: snapshot.meta.scenario_id.clone(),
            trainee_name: snapshot.meta.trainee_name.clone(),
            turn: snapshot.state.turn,
            career_complete: snapshot.state.career_complete,
            rng_calls: snapshot.rng_calls,
            snapshot,
        };
        Self::write_json(&path, &rec)?;
        Ok(rec)
    }

    pub fn load_checkpoint(&self, name: &str) -> LabResult<CheckpointRecord> {
        let path = self.cp_path(name);
        if !path.exists() {
            return Err(LabError::NotFound(format!("checkpoint {name}")));
        }
        Self::read_json(&path)
    }

    pub fn delete_checkpoint(&self, name: &str) -> LabResult<()> {
        let path = self.cp_path(name);
        if !path.exists() {
            return Err(LabError::NotFound(format!("checkpoint {name}")));
        }
        fs::remove_file(&path).map_err(|e| LabError::Io(e.to_string()))
    }

    pub fn list_checkpoints(&self) -> LabResult<Vec<EntrySummary>> {
        let mut out = Vec::new();
        for name in Self::list_dir("checkpoints", &self.dir)? {
            let rec: CheckpointRecord = Self::read_json(&self.cp_path(&name))?;
            out.push(EntrySummary {
                name: rec.name,
                created_at: rec.created_at,
                note: rec.note,
                seed: rec.seed,
                scenario_id: rec.scenario_id,
                turn: rec.turn,
                career_complete: rec.career_complete,
                rng_calls: rec.rng_calls,
            });
        }
        Ok(out)
    }

    /// Export a checkpoint to a portable JSON file (importable elsewhere).
    pub fn export_checkpoint(&self, name: &str, file: &Path) -> LabResult<()> {
        let rec = self.load_checkpoint(name)?;
        Self::write_json(file, &rec)
    }

    /// Import a checkpoint exported elsewhere. Fails if the name is taken.
    pub fn import_checkpoint(&self, file: &Path) -> LabResult<CheckpointRecord> {
        let rec: CheckpointRecord = Self::read_json(file)?;
        check_name(&rec.name)?;
        let path = self.cp_path(&rec.name);
        if path.exists() {
            return Err(LabError::Exists(format!("checkpoint {}", rec.name)));
        }
        // Validate the embedded snapshot decodes through the codec path too.
        let _ = RunSnapshotCodec::decode(&RunSnapshotCodec::encode(&rec.snapshot))
            .map_err(|e| LabError::Json(e.to_string()))?;
        Self::write_json(&path, &rec)?;
        Ok(rec)
    }

    // -- branches ----------------------------------------------------------

    /// Fork a checkpoint into an independent branch. The branch head is a
    /// deep copy of the checkpoint snapshot — the checkpoint and any sibling
    /// branches are never touched by this branch's later play steps.
    pub fn fork_checkpoint(
        &self,
        checkpoint: &str,
        branch: &str,
        note: &str,
    ) -> LabResult<BranchRecord> {
        check_name(branch)?;
        let path = self.br_path(branch);
        if path.exists() {
            return Err(LabError::Exists(format!("branch {branch}")));
        }
        let cp = self.load_checkpoint(checkpoint)?;
        let rec = BranchRecord {
            name: branch.to_string(),
            parent_checkpoint: cp.name,
            created_at: now_stamp(),
            note: note.to_string(),
            snapshot: cp.snapshot,
        };
        Self::write_json(&path, &rec)?;
        Ok(rec)
    }

    pub fn load_branch(&self, name: &str) -> LabResult<BranchRecord> {
        let path = self.br_path(name);
        if !path.exists() {
            return Err(LabError::NotFound(format!("branch {name}")));
        }
        Self::read_json(&path)
    }

    pub fn delete_branch(&self, name: &str) -> LabResult<()> {
        let path = self.br_path(name);
        if !path.exists() {
            return Err(LabError::NotFound(format!("branch {name}")));
        }
        fs::remove_file(&path).map_err(|e| LabError::Io(e.to_string()))?;
        let _ = fs::remove_file(self.tl_path(name));
        Ok(())
    }

    pub fn list_branches(&self) -> LabResult<Vec<EntrySummary>> {
        let mut out = Vec::new();
        for name in Self::list_dir("branches", &self.dir)? {
            let rec: BranchRecord = Self::read_json(&self.br_path(&name))?;
            out.push(EntrySummary {
                name: rec.name,
                created_at: rec.created_at,
                note: rec.note,
                seed: rec.snapshot.meta.seed,
                scenario_id: rec.snapshot.meta.scenario_id.clone(),
                turn: rec.snapshot.state.turn,
                career_complete: rec.snapshot.state.career_complete,
                rng_calls: rec.snapshot.rng_calls,
            });
        }
        Ok(out)
    }

    // -- engine binding ----------------------------------------------------

    /// Restore an engine from a checkpoint (or branch) snapshot.
    pub fn restore_engine(snapshot: &RunSnapshot) -> LabResult<SimEngine> {
        let mut engine = SimEngine::create(snapshot.settings.clone());
        let result = engine.restore(snapshot.clone());
        if result.text_lines.iter().any(|l| l.contains("error")) {
            return Err(LabError::Engine(format!(
                "restore failed: {}",
                result.text_lines.join(" | ")
            )));
        }
        Ok(engine)
    }

    /// Start a fresh career and return the engine.
    pub fn start_career(
        seed: i64,
        scenario: &str,
        trainee: &str,
        settings: SimSettings,
    ) -> LabResult<SimEngine> {
        let mut engine = SimEngine::create(settings);
        let meta = uma_sim_core::RunMeta::new(seed, scenario, trainee);
        engine.start(meta);
        Ok(engine)
    }

    /// Apply one action to a branch head and persist the new head.
    /// Returns the action's result text for interactive play.
    pub fn step_branch(&self, branch: &str, action_id: &str) -> LabResult<Vec<String>> {
        let mut rec = self.load_branch(branch)?;
        let mut engine = Self::restore_engine(&rec.snapshot)?;
        let action = parse_sim_action(action_id);
        let result = engine.step(action);
        rec.snapshot = engine.export();
        Self::write_json(&self.br_path(branch), &rec)?;
        Ok(result.text_lines)
    }

    /// Play a branch forward up to `turns` turns with `policy`, recording a
    /// per-action timeline. Only this branch's head snapshot is updated.
    pub fn play_branch(&self, branch: &str, policy_name: &str, turns: u32) -> LabResult<Timeline> {
        let policy = resolve_policy(policy_name)?;
        let mut rec = self.load_branch(branch)?;
        let mut engine = Self::restore_engine(&rec.snapshot)?;
        let start_turn = engine.state().turn;
        let target_turn = start_turn + turns as i32;
        let mut steps: Vec<TurnRecord> = Vec::new();
        let mut step_no: u32 = 0;
        let max_actions = turns.saturating_mul(6).max(turns).saturating_add(24);
        while !engine.state().career_complete
            && engine.state().turn < target_turn
            && step_no < max_actions
        {
            let choices = engine.choices();
            if choices.is_empty() {
                break;
            }
            let action = policy(&choices, engine.state());
            let action_id = action_id_of(&action);
            engine.step(action);
            step_no += 1;
            let s = engine.state();
            steps.push(TurnRecord {
                step: step_no,
                turn: s.turn,
                action_id,
                energy: s.energy,
                mood: format!("{:?}", s.mood),
                speed: s.stats.speed,
                stamina: s.stats.stamina,
                power: s.stats.power,
                guts: s.stats.guts,
                wit: s.stats.wit,
                fans: s.fans,
                skill_points: s.skill_points,
                races_completed: s.completed_races.len(),
                phase: s.phase.clone(),
                career_complete: s.career_complete,
                rng_calls: engine.rng_call_count(),
            });
        }
        rec.snapshot = engine.export();
        Self::write_json(&self.br_path(branch), &rec)?;
        let tl = Timeline {
            branch: branch.to_string(),
            policy: policy_name.to_string(),
            played_at: now_stamp(),
            start_turn,
            end_turn: engine.state().turn,
            hash: timeline_hash(&steps),
            steps,
        };
        Self::write_json(&self.tl_path(branch), &tl)?;
        Ok(tl)
    }

    pub fn load_timeline(&self, branch: &str) -> LabResult<Timeline> {
        let path = self.tl_path(branch);
        if !path.exists() {
            return Err(LabError::NotFound(format!("timeline for branch {branch}")));
        }
        Self::read_json(&path)
    }

    /// Export a branch head to a portable JSON file.
    pub fn export_branch(&self, name: &str, file: &Path) -> LabResult<()> {
        let rec = self.load_branch(name)?;
        Self::write_json(file, &rec)
    }
}

fn action_id_of(action: &SimAction) -> String {
    match action.kind {
        SimActionKind::Train => format!("train_{}", action.payload.as_deref().unwrap_or("?")),
        SimActionKind::Rest => "rest".to_string(),
        SimActionKind::Recreation => "recreation".to_string(),
        SimActionKind::Race => "race".to_string(),
        SimActionKind::Choose => format!("event_{}", action.payload.as_deref().unwrap_or("?")),
        SimActionKind::Lesson => format!("gl_{}", action.payload.as_deref().unwrap_or("?")),
        SimActionKind::Advance => "advance".to_string(),
    }
}

fn timeline_hash(steps: &[TurnRecord]) -> String {
    // FNV-1a over the JSON of the step records: a stable content fingerprint.
    let s = serde_json::to_string(steps).unwrap_or_default();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

fn stats_str(r: &TurnRecord) -> String {
    format!("{}/{}/{}/{}/{}", r.speed, r.stamina, r.power, r.guts, r.wit)
}

fn outcome_of(tl: &Timeline) -> OutcomeSummary {
    match tl.steps.last() {
        Some(last) => OutcomeSummary {
            branch: tl.branch.clone(),
            career_complete: last.career_complete,
            final_turn: last.turn,
            stats: stats_str(last),
            fans: last.fans,
            skill_points: last.skill_points,
            races_completed: last.races_completed,
            rng_calls: last.rng_calls,
        },
        None => OutcomeSummary {
            branch: tl.branch.clone(),
            career_complete: false,
            final_turn: tl.start_turn,
            stats: "0/0/0/0/0".to_string(),
            fans: 0,
            skill_points: 0,
            races_completed: 0,
            rng_calls: 0,
        },
    }
}

/// Compare two timelines step by step. Divergence is detected on the
/// recorded stat/energy/fans/phase/action fields; RNG call counts are
/// reported but two branches may legitimately share counts while differing
/// in outcomes (or vice versa).
pub fn compare_timelines(a: &Timeline, b: &Timeline) -> Comparison {
    let n = a.steps.len().min(b.steps.len());
    let mut rows = Vec::with_capacity(n);
    let mut first_div: Option<(u32, String)> = None;
    for i in 0..n {
        let ra = &a.steps[i];
        let rb = &b.steps[i];
        let fields: Vec<(&str, String, String)> = vec![
            ("action", ra.action_id.clone(), rb.action_id.clone()),
            ("turn", ra.turn.to_string(), rb.turn.to_string()),
            ("energy", ra.energy.to_string(), rb.energy.to_string()),
            ("stats", stats_str(ra), stats_str(rb)),
            ("fans", ra.fans.to_string(), rb.fans.to_string()),
            (
                "skill_points",
                ra.skill_points.to_string(),
                rb.skill_points.to_string(),
            ),
            (
                "races",
                ra.races_completed.to_string(),
                rb.races_completed.to_string(),
            ),
            ("phase", ra.phase.clone(), rb.phase.clone()),
            ("mood", ra.mood.clone(), rb.mood.clone()),
        ];
        let div_field = fields
            .iter()
            .find(|(_, x, y)| x != y)
            .map(|(k, _, _)| k.to_string());
        if first_div.is_none() {
            if let Some(f) = div_field.clone() {
                first_div = Some((ra.step, f));
            }
        }
        rows.push(CompareRow {
            step: ra.step,
            turn_a: ra.turn,
            turn_b: rb.turn,
            action_a: ra.action_id.clone(),
            action_b: rb.action_id.clone(),
            energy_a: ra.energy,
            energy_b: rb.energy,
            stats_a: stats_str(ra),
            stats_b: stats_str(rb),
            fans_a: ra.fans,
            fans_b: rb.fans,
            diverged: div_field.is_some(),
        });
    }
    if first_div.is_none() && a.steps.len() != b.steps.len() {
        first_div = Some((n as u32 + 1, "length".to_string()));
    }
    Comparison {
        branch_a: a.branch.clone(),
        branch_b: b.branch.clone(),
        steps_compared: n,
        first_divergence_step: first_div.as_ref().map(|(s, _)| *s),
        first_divergence_field: first_div.as_ref().map(|(_, f)| f.clone()),
        rows,
        outcome_a: outcome_of(a),
        outcome_b: outcome_of(b),
    }
}

/// Render a comparison as a human-readable side-by-side table.
pub fn render_comparison(c: &Comparison) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== branch-and-compare: {} vs {} ===\n",
        c.branch_a, c.branch_b
    ));
    out.push_str(&format!("steps compared: {}\n", c.steps_compared));
    match (c.first_divergence_step, c.first_divergence_field.as_deref()) {
        (Some(s), Some(f)) => out.push_str(&format!("first divergence: step {s} (field: {f})\n")),
        _ => out.push_str("first divergence: none — branches identical over compared steps\n"),
    }
    out.push_str(&format!(
        "{:<6} {:<6} {:<14} {:<14} {:<7} {:<7} {:<22} {:<22} {:<6}\n",
        "step", "turn", "action_a", "action_b", "nrg_a", "nrg_b", "stats_a", "stats_b", "div?"
    ));
    for r in &c.rows {
        out.push_str(&format!(
            "{:<6} {:<6} {:<14} {:<14} {:<7} {:<7} {:<22} {:<22} {:<6}\n",
            r.step,
            format!("{}/{}", r.turn_a, r.turn_b),
            trunc(&r.action_a, 13),
            trunc(&r.action_b, 13),
            r.energy_a,
            r.energy_b,
            trunc(&r.stats_a, 21),
            trunc(&r.stats_b, 21),
            if r.diverged { "*" } else { "" },
        ));
    }
    out.push_str("\n--- outcomes ---\n");
    for o in [&c.outcome_a, &c.outcome_b] {
        out.push_str(&format!(
            "{}: complete={} turn={} stats={} fans={} sp={} races={} rng_calls={}\n",
            o.branch,
            o.career_complete,
            o.final_turn,
            o.stats,
            o.fans,
            o.skill_points,
            o.races_completed,
            o.rng_calls
        ));
    }
    out
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
}

/// One-line status for a branch head.
pub fn status_line(rec: &BranchRecord) -> String {
    let s = &rec.snapshot.state;
    format!(
        "branch {} (from checkpoint {}): turn={} phase={} energy={} stats={}/{}/{}/{}/{} fans={} sp={} races={} complete={} rng_calls={}",
        rec.name,
        rec.parent_checkpoint,
        s.turn,
        s.phase,
        s.energy,
        s.stats.speed,
        s.stats.stamina,
        s.stats.power,
        s.stats.guts,
        s.stats.wit,
        s.fans,
        s.skill_points,
        s.completed_races.len(),
        s.career_complete,
        rec.snapshot.rng_calls,
    )
}
