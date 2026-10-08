//! Receiving checks for returned filesystem failures during checkpoint saves.

mod common;

use common::config_lock;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use uma_sim_core::career_lab::{CareerLibrary, LabError};
use uma_sim_core::race::RaceModel;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct LibraryDir(PathBuf);

impl LibraryDir {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "uma-library-replacement-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).expect("unique receiving directory");
        Self(dir)
    }

    fn library(&self) -> CareerLibrary {
        CareerLibrary::at(self.0.clone())
    }

    fn bytes(&self, name: &str) -> (Vec<u8>, Vec<u8>) {
        (
            fs::read(self.0.join(format!("{name}.snapshot.json"))).unwrap(),
            fs::read(self.0.join(format!("{name}.meta.json"))).unwrap(),
        )
    }

    fn refuse_metadata_staging(&self, name: &str) {
        fs::create_dir(
            self.0
                .join(format!(".tmp-{}-{name}.meta.json", std::process::id())),
        )
        .unwrap();
    }

    fn rollback_files(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".rollback-")
            })
            .collect()
    }
}

impl Drop for LibraryDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn engine() -> SimEngine {
    let mut engine = SimEngine::create(SimSettings {
        race_model: RaceModel::Stub,
        ..Default::default()
    });
    engine.start(RunMeta::new(42, "ura", "Special Week"));
    engine
}

#[test]
fn failed_overwrite_restores_the_complete_previous_checkpoint() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    let mut engine = engine();
    let previous = engine.export();
    lib.save(
        "keep",
        Some("Before race"),
        Some("Keep this note"),
        &previous,
        false,
    )
    .unwrap();
    lib.save("sibling", None, None, &previous, false).unwrap();
    let original = dir.bytes("keep");
    let sibling = dir.bytes("sibling");
    engine.auto_step_scoring();
    assert_ne!(engine.state().turn, previous.state.turn);
    dir.refuse_metadata_staging("keep");

    assert!(matches!(
        lib.save("keep", Some("Replacement"), None, &engine.export(), true),
        Err(LabError::Io(_))
    ));
    assert_eq!(
        dir.bytes("keep"),
        original,
        "returned failure changed saved bytes"
    );
    assert_eq!(dir.bytes("sibling"), sibling);
    let (restored, entry, _) = lib.load("keep").unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(previous).unwrap()
    );
    assert_eq!(entry.label, "Before race");
    assert_eq!(entry.note, "Keep this note");
    assert!(
        dir.rollback_files().is_empty(),
        "completed restoration left a redundant copy"
    );
}

#[test]
fn failed_import_overwrite_preserves_an_existing_checkpoint() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    let mut engine = engine();
    lib.save("keep", None, None, &engine.export(), false)
        .unwrap();
    let previous = dir.bytes("keep");
    engine.auto_step_scoring();
    let replacement = serde_json::to_string(&engine.export()).unwrap();
    dir.refuse_metadata_staging("keep");
    assert!(matches!(
        lib.import(&replacement, Some("keep"), true),
        Err(LabError::Io(_))
    ));
    assert_eq!(dir.bytes("keep"), previous);
    assert!(lib.load("keep").is_ok());
    assert!(dir.rollback_files().is_empty());
}

#[test]
fn snapshot_staging_failure_preserves_the_old_pair_and_cleans_its_backup() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    let mut engine = engine();
    lib.save("keep", None, None, &engine.export(), false)
        .unwrap();
    let previous = dir.bytes("keep");
    engine.auto_step_scoring();
    fs::create_dir(
        dir.0
            .join(format!(".tmp-{}-keep.snapshot.json", std::process::id())),
    )
    .unwrap();
    assert!(matches!(
        lib.save("keep", None, None, &engine.export(), true),
        Err(LabError::Io(_))
    ));
    assert_eq!(dir.bytes("keep"), previous);
    assert!(dir.rollback_files().is_empty());
}

#[test]
fn a_failed_new_save_does_not_create_a_visible_partial_checkpoint() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    dir.refuse_metadata_staging("new");
    assert!(matches!(
        lib.save("new", None, None, &engine().export(), false),
        Err(LabError::Io(_))
    ));
    assert!(!dir.0.join("new.snapshot.json").exists());
    assert!(!dir.0.join("new.meta.json").exists());
    assert!(lib.list().unwrap().is_empty());
    assert!(dir.rollback_files().is_empty());
}

#[test]
fn no_overwrite_preserves_orphan_snapshot_bytes() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    let saved = serde_json::to_vec(&engine().export()).unwrap();
    let path = dir.0.join("orphan.snapshot.json");
    fs::write(&path, &saved).unwrap();
    assert!(matches!(
        lib.save("orphan", None, None, &engine().export(), false),
        Err(LabError::AlreadyExists(_))
    ));
    assert_eq!(fs::read(path).unwrap(), saved);
    assert!(!dir.0.join("orphan.meta.json").exists());
}

#[test]
fn successful_overwrite_replaces_state_and_metadata_without_recovery_files() {
    let _lock = config_lock();
    let dir = LibraryDir::new();
    let lib = dir.library();
    let mut engine = engine();
    lib.save(
        "keep",
        Some("Old"),
        Some("Old note"),
        &engine.export(),
        false,
    )
    .unwrap();
    engine.auto_step_scoring();
    let new = engine.export();
    lib.save("keep", Some("New"), Some("New note"), &new, true)
        .unwrap();
    let (loaded, entry, _) = lib.load("keep").unwrap();
    assert_eq!(
        serde_json::to_value(loaded).unwrap(),
        serde_json::to_value(new).unwrap()
    );
    assert_eq!(entry.label, "New");
    assert_eq!(entry.note, "New note");
    assert!(dir.rollback_files().is_empty());
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}
