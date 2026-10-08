//! CLI/API session persistence to `.uma-sim/session.json`.

use crate::engine::SimEngine;
use crate::snapshot::{RunSnapshot, RunSnapshotCodec};
use crate::state::{SimAction, SimActionKind};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
        let json = RunSnapshotCodec::encode(&engine.export());
        publish_snapshot(&path, json.as_bytes())
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

/// Write a complete sibling before replacing a saved career. A handled write,
/// sync, or rename error leaves the previous snapshot intact. This does not
/// serialize concurrent CLI commands or promise directory durability on power loss.
fn publish_snapshot(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Keep the established behavior for a session file that is a symlink,
    // including a link to a not-yet-created file.
    let mut target = path.to_path_buf();
    let mut links = 0;
    let permissions = loop {
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if links == 40 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "too many session symlinks",
                    ));
                }
                links += 1;
                let link = fs::read_link(&target)?;
                target = if link.is_absolute() {
                    link
                } else {
                    target.parent().unwrap_or_else(|| Path::new(".")).join(link)
                };
            }
            Ok(metadata) if metadata.is_file() => {
                // An atomic rename must not silently bypass a write denial on
                // an existing session. Opening without truncation preserves it.
                fs::OpenOptions::new().write(true).open(&target)?;
                break Some(metadata.permissions());
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "session destination is not a regular file",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => break None,
            Err(error) => return Err(error),
        }
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let mut prepared = None;
    for _ in 0..128 {
        let temporary = parent.join(format!(
            ".session-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        // A dangling session symlink may name this candidate itself.
        if temporary == target {
            continue;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => {
                prepared = Some((temporary, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) = prepared.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a session temporary file",
        )
    })?;
    let written = (|| {
        file.write_all(bytes)?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()
    })();
    drop(file);
    let published = written.and_then(|()| fs::rename(&temporary, &target));
    if let Err(error) = published {
        if let Err(cleanup) = fs::remove_file(&temporary) {
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "{error}; failed to remove temporary session {}: {cleanup}",
                    temporary.display()
                ),
            ));
        }
        return Err(error);
    }
    Ok(())
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
