//! Saved-career library: branch-and-compare laboratory (Commons T100).
//!
//! The CLI keeps one fixed session (`.uma-sim/session.json`) and the REST API
//! one in-memory engine, so there was no way to save before a meaningful
//! choice, play two independent continuations, resume either and compare what
//! actually diverged. This module adds a named library of run snapshots with
//! per-entry metadata and captured telemetry, plus a structural comparison.
//!
//! Entry layout under the library root (default `<cwd>/.uma-sim/library/`):
//! `<name>.snapshot.json`, `<name>.telemetry.jsonl`, `<name>.meta.json`.
//!
//! Game mechanics are untouched: entries are plain [`RunSnapshot`]s restored
//! through the normal [`SimEngine::restore`] path, and continuations use the
//! built-in `default`/`bot` policies only. No external-policy launcher, no
//! source-data changes, no deployment.

use crate::engine::SimEngine;
use crate::session::RunSession;
use crate::snapshot::{RunSnapshot, RunSnapshotCodec};
use crate::telemetry::TurnTelemetryRecord;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SNAPSHOT_EXT: &str = ".snapshot.json";
const TELEMETRY_EXT: &str = ".telemetry.jsonl";
const META_EXT: &str = ".meta.json";

/// Branch marker for the active session (`.uma-sim/session.branch`).
const BRANCH_MARKER: &str = "session.branch";

/// Continuation telemetry sidecar (`.uma-sim/session.telemetry.jsonl`).
///
/// The snapshot format carries no telemetry, so a completed `lab play` would
/// otherwise lose the action history when the process exits. The lab writes
/// the in-memory telemetry here at the end of each continuation run, and
/// [`CareerLab::save`] prefers it over the (fresh, empty) reloaded engine.
const SESSION_TELEMETRY: &str = "session.telemetry.jsonl";

/// Terminal outcome summary captured at save time, when the career is complete.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalSummary {
    pub u: f64,
    pub grade: String,
    pub score: i32,
    pub sp_spent: i32,
    pub fans: i32,
}

/// Metadata for one library entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerLabMeta {
    pub name: String,
    pub saved_at_unix: u64,
    pub turn: i32,
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub career_complete: bool,
    pub fans: i32,
    /// Policy used for the continuation that produced this entry (`manual`,
    /// `default`, `bot`). Never `external` — the lab has no process launcher.
    pub policy_label: String,
    /// Entry this branch was created from, if any.
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub terminal: Option<TerminalSummary>,
    /// Number of telemetry lines captured with this entry.
    pub telemetry_lines: usize,
}

/// A per-turn action used for divergence detection.
#[derive(Debug, Clone, PartialEq)]
pub struct LabAction {
    pub turn: i32,
    pub action: Option<String>,
}

/// Where two entries' action sequences first differ.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionDivergence {
    pub index: usize,
    pub turn_a: i32,
    pub turn_b: i32,
    pub action_a: Option<String>,
    pub action_b: Option<String>,
}

/// Structural comparison of two library entries.
#[derive(Debug, Clone)]
pub struct CareerComparison {
    pub entry_a: String,
    pub entry_b: String,
    pub same_origin: bool,
    pub related: bool,
    pub turn_a: i32,
    pub turn_b: i32,
    pub complete_a: bool,
    pub complete_b: bool,
    pub terminal_a: Option<TerminalSummary>,
    pub terminal_b: Option<TerminalSummary>,
    pub fans_a: i32,
    pub fans_b: i32,
    pub shared_action_prefix: usize,
    pub divergence: Option<ActionDivergence>,
    pub telemetry_missing: Vec<String>,
}

/// Named library of saved careers.
pub struct CareerLab {
    root: PathBuf,
}

impl CareerLab {
    /// Library root for the current working directory.
    pub fn default_root() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join("library")
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn cwd() -> Self {
        Self::with_root(Self::default_root())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Entry names must be filesystem-safe identifiers.
    pub fn validate_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            && name != "."
            && name != ".."
    }

    fn snapshot_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}{SNAPSHOT_EXT}"))
    }

    fn telemetry_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}{TELEMETRY_EXT}"))
    }

    fn meta_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}{META_EXT}"))
    }

    fn branch_marker_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join(BRANCH_MARKER)
    }

    /// Record which library entry the active session was branched from.
    pub fn set_session_parent(parent: &str) -> io::Result<()> {
        let path = Self::branch_marker_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, parent)
    }

    /// Parent entry recorded for the active session, if any.
    pub fn session_parent() -> Option<String> {
        fs::read_to_string(Self::branch_marker_path())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn clear_session_parent() {
        let _ = fs::remove_file(Self::branch_marker_path());
    }

    fn session_telemetry_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join(SESSION_TELEMETRY)
    }

    /// Persist an engine's in-memory telemetry for the next `save`.
    pub fn write_session_telemetry(engine: &SimEngine) -> io::Result<()> {
        let jsonl: String = engine
            .telemetry_log()
            .iter()
            .filter_map(|r| serde_json::to_string(r).ok())
            .collect::<Vec<_>>()
            .join("\n");
        let path = Self::session_telemetry_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, jsonl)
    }

    fn read_session_telemetry() -> Option<String> {
        fs::read_to_string(Self::session_telemetry_path())
            .ok()
            .filter(|s| !s.trim().is_empty())
    }

    pub fn clear_session_telemetry() {
        let _ = fs::remove_file(Self::session_telemetry_path());
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Save the engine's current run as a library entry.
    pub fn save(
        &self,
        engine: &SimEngine,
        name: &str,
        policy_label: &str,
        parent: Option<String>,
        note: &str,
    ) -> io::Result<CareerLabMeta> {
        if !Self::validate_name(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid library name '{name}': use 1-64 chars of [A-Za-z0-9._-]"),
            ));
        }
        fs::create_dir_all(&self.root)?;
        let state = engine.state();
        // NOTE: `export_telemetry_jsonl()` serializes the Android bridge lines
        // (event choices only), not the per-turn records. The lab needs the
        // per-turn action history, so serialize `telemetry_log()` as JSONL.
        // Prefer the session sidecar written by `lab play`: the snapshot
        // format carries no telemetry, so a reloaded engine's in-memory log is
        // empty even when the continuation that produced this save recorded
        // a full history.
        let telemetry: String = Self::read_session_telemetry().unwrap_or_else(|| {
            engine
                .telemetry_log()
                .iter()
                .filter_map(|r| serde_json::to_string(r).ok())
                .collect::<Vec<_>>()
                .join("\n")
        });
        let telemetry_lines = telemetry.lines().filter(|l| !l.trim().is_empty()).count();
        fs::write(
            self.snapshot_path(name),
            RunSnapshotCodec::encode(&engine.export()),
        )?;
        fs::write(self.telemetry_path(name), &telemetry)?;
        // The terminal record is ephemeral: it lives on the in-memory engine
        // and is not part of the snapshot, so recompute it from the final
        // state when the career is complete but no record survived the reload.
        let terminal_record = engine
            .last_terminal()
            .cloned()
            .or_else(|| state.career_complete.then(|| engine.evaluate_terminal()));
        let meta = CareerLabMeta {
            name: name.to_string(),
            saved_at_unix: Self::now_unix(),
            turn: state.turn,
            seed: state.meta.seed,
            scenario_id: state.meta.scenario_id.clone(),
            trainee_name: state.meta.trainee_name.clone(),
            career_complete: state.career_complete,
            fans: state.fans,
            policy_label: policy_label.to_string(),
            parent,
            note: note.to_string(),
            terminal: terminal_record.as_ref().map(|t| TerminalSummary {
                u: t.u,
                grade: t.grade.clone(),
                score: t.score,
                sp_spent: t.sp_spent,
                fans: state.fans,
            }),
            telemetry_lines,
        };
        let meta_json = serde_json::to_string_pretty(&meta).map_err(io::Error::other)?;
        fs::write(self.meta_path(name), meta_json)?;
        Ok(meta)
    }

    /// Load an entry back into a fresh engine, with telemetry tracing enabled
    /// so the continuation is captured for later comparison.
    pub fn load(&self, name: &str) -> io::Result<(SimEngine, CareerLabMeta)> {
        let meta = self.info(name)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no library entry named '{name}'"),
            )
        })?;
        let raw = fs::read_to_string(self.snapshot_path(name))?;
        let snapshot: RunSnapshot = RunSnapshotCodec::decode(&raw)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut settings = snapshot.settings.clone();
        settings.trace_telemetry = true;
        let snapshot = RunSnapshot {
            settings,
            ..snapshot
        };
        let mut engine = SimEngine::create(snapshot.settings.clone());
        engine.restore(snapshot);
        Ok((engine, meta))
    }

    pub fn info(&self, name: &str) -> io::Result<Option<CareerLabMeta>> {
        let path = self.meta_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(path)?;
        let meta: CareerLabMeta = serde_json::from_str(&raw)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(meta))
    }

    /// All entries, oldest first.
    pub fn list(&self) -> Vec<CareerLabMeta> {
        let mut metas: Vec<CareerLabMeta> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(META_EXT).map(str::to_string)
            })
            .filter_map(|name| self.info(&name).ok().flatten())
            .collect();
        metas.sort_by_key(|m| (m.saved_at_unix, m.name.clone()));
        metas
    }

    pub fn delete(&self, name: &str) -> io::Result<()> {
        let mut missing = 0;
        for path in [
            self.snapshot_path(name),
            self.telemetry_path(name),
            self.meta_path(name),
        ] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => missing += 1,
                Err(e) => return Err(e),
            }
        }
        if missing == 3 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no library entry named '{name}'"),
            ));
        }
        Ok(())
    }

    fn action_sequence(&self, name: &str) -> Vec<LabAction> {
        let Ok(raw) = fs::read_to_string(self.telemetry_path(name)) else {
            return Vec::new();
        };
        raw.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<TurnTelemetryRecord>(l).ok())
            .map(|r| LabAction {
                turn: r.turn,
                action: r.action,
            })
            .collect()
    }

    /// Compare two entries: origin, terminals, and first action divergence.
    pub fn compare(&self, a: &str, b: &str) -> io::Result<CareerComparison> {
        let meta_a = self.info(a)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no library entry named '{a}'"),
            )
        })?;
        let meta_b = self.info(b)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no library entry named '{b}'"),
            )
        })?;
        let seq_a = self.action_sequence(a);
        let seq_b = self.action_sequence(b);
        let mut telemetry_missing = Vec::new();
        if seq_a.is_empty() && meta_a.telemetry_lines > 0 {
            telemetry_missing.push(a.to_string());
        }
        if seq_b.is_empty() && meta_b.telemetry_lines > 0 {
            telemetry_missing.push(b.to_string());
        }
        let mut shared = 0usize;
        let mut divergence = None;
        for (i, (la, lb)) in seq_a.iter().zip(seq_b.iter()).enumerate() {
            if la.action == lb.action {
                shared = i + 1;
            } else {
                divergence = Some(ActionDivergence {
                    index: i,
                    turn_a: la.turn,
                    turn_b: lb.turn,
                    action_a: la.action.clone(),
                    action_b: lb.action.clone(),
                });
                break;
            }
        }
        // If one sequence is a strict prefix of the other, that is also a
        // divergence point (the shorter run stopped / played fewer turns).
        if divergence.is_none()
            && seq_a.len() != seq_b.len()
            && shared == seq_a.len().min(seq_b.len())
        {
            let (turn_a, turn_b) = match (seq_a.get(shared), seq_b.get(shared)) {
                (Some(la), Some(lb)) => (la.turn, lb.turn),
                (Some(la), None) => (la.turn, meta_b.turn),
                (None, Some(lb)) => (meta_a.turn, lb.turn),
                (None, None) => (meta_a.turn, meta_b.turn),
            };
            divergence = Some(ActionDivergence {
                index: shared,
                turn_a,
                turn_b,
                action_a: seq_a.get(shared).and_then(|l| l.action.clone()),
                action_b: seq_b.get(shared).and_then(|l| l.action.clone()),
            });
        }
        let related = meta_a.parent == Some(b.to_string())
            || meta_b.parent == Some(a.to_string())
            || (meta_a.parent.is_some() && meta_a.parent == meta_b.parent);
        Ok(CareerComparison {
            entry_a: a.to_string(),
            entry_b: b.to_string(),
            same_origin: meta_a.seed == meta_b.seed
                && meta_a.scenario_id == meta_b.scenario_id
                && meta_a.trainee_name == meta_b.trainee_name,
            related,
            turn_a: meta_a.turn,
            turn_b: meta_b.turn,
            complete_a: meta_a.career_complete,
            complete_b: meta_b.career_complete,
            terminal_a: meta_a.terminal.clone(),
            terminal_b: meta_b.terminal.clone(),
            fans_a: meta_a.fans,
            fans_b: meta_b.fans,
            shared_action_prefix: shared,
            divergence,
            telemetry_missing,
        })
    }
}

/// Convenience: save the active CLI session into the default library.
pub fn save_active_session(
    name: &str,
    policy_label: &str,
    parent: Option<String>,
    note: &str,
) -> io::Result<CareerLabMeta> {
    let Some((engine, _)) = RunSession::load() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no active session — run `start` first",
        ));
    };
    CareerLab::cwd().save(&engine, name, policy_label, parent, note)
}
