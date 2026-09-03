//! CLI/API session persistence to `.uma-sim/session.json`.

use crate::engine::SimEngine;
use crate::snapshot::{RunSnapshot, RunSnapshotCodec};
use crate::state::{SimAction, SimActionKind};
use std::fs;
use std::path::PathBuf;

pub struct RunSession;

impl RunSession {
    pub fn session_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".uma-sim")
            .join("session.json")
    }

    pub fn save(engine: &SimEngine) -> std::io::Result<()> {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = RunSnapshotCodec::encode(&engine.export());
        fs::write(path, json)
    }

    pub fn load() -> Option<(SimEngine, RunSnapshot)> {
        let path = Self::session_path();
        if !path.exists() {
            return None;
        }
        let raw = fs::read_to_string(&path).ok()?;
        let snapshot = RunSnapshotCodec::decode(&raw).ok()?;
        let mut engine = SimEngine::create(snapshot.settings.clone());
        engine.restore(snapshot.clone());
        Some((engine, snapshot))
    }

    pub fn clear() {
        let path = Self::session_path();
        let _ = fs::remove_file(path);
    }
}

/// Parse CLI/API action ids into [`SimAction`] (Kotlin `parseAction` parity).
pub fn parse_sim_action(action_id: &str) -> SimAction {
    if action_id.starts_with("gl_") {
        SimAction {
            kind: SimActionKind::Lesson,
            payload: Some(action_id.to_string()),
        }
    } else if let Some(rest) = action_id.strip_prefix("event_") {
        SimAction {
            kind: SimActionKind::Choose,
            payload: Some(rest.to_string()),
        }
    } else if let Some(rest) = action_id.strip_prefix("train_") {
        SimAction {
            kind: SimActionKind::Train,
            payload: Some(rest.to_string()),
        }
    } else if action_id == "rest" {
        SimAction {
            kind: SimActionKind::Rest,
            payload: None,
        }
    } else if action_id == "recreation" {
        SimAction {
            kind: SimActionKind::Recreation,
            payload: None,
        }
    } else if action_id == "race" {
        SimAction {
            kind: SimActionKind::Race,
            payload: None,
        }
    } else {
        SimAction {
            kind: SimActionKind::Rest,
            payload: None,
        }
    }
}
