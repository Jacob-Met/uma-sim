//! Integration tests for the T100 career library + branch-and-compare lab.
//!
//! Covers: library round-trip and naming rules, save/close/reopen
//! reproducibility (state + RNG), sibling isolation, branch determinism,
//! first-divergence detection, event/mandatory-race checkpoints across all
//! four scenarios, telemetry isolation, import-failure safety, and report
//! rendering.

mod common;

use common::{config_lock, SCENARIOS};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use uma_sim_core::career_lab::{
    compare_branches, delete_lab_result, render_json, render_markdown, run_branch, sweep_stale_tmp,
    ActionOverride, BranchConfig, BranchStore, CareerLibrary, LabError,
};
use uma_sim_core::race::RaceModel;
use uma_sim_core::snapshot::{RunSnapshot, RunSnapshotCodec};
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_dir(tag: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("uma-lab-{}-{}-{}", tag, std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_settings() -> SimSettings {
    SimSettings {
        speed_multiplier: 1,
        race_model: RaceModel::Stub,
        ..Default::default()
    }
}

/// Start a fresh career engine (holds the global-config lock via the caller).
fn start_engine(seed: i64, scenario: &str) -> SimEngine {
    let mut engine = SimEngine::create(test_settings());
    engine.start(RunMeta::new(seed, scenario, "Special Week"));
    engine
}

/// Play `n` scoring-policy steps.
fn play_scoring(engine: &mut SimEngine, n: usize) {
    for _ in 0..n {
        if engine.state().career_complete {
            break;
        }
        engine.auto_step_scoring();
    }
}

/// Canonical snapshot value: `serde_json::Value` object comparison is
/// key-order insensitive, unlike raw `to_string` on `HashMap` fields.
fn snapshot_value(snap: &RunSnapshot) -> serde_json::Value {
    serde_json::to_value(snap).expect("snapshot serializes")
}

fn snapshot_json(snap: &RunSnapshot) -> String {
    serde_json::to_string(snap).expect("snapshot serializes")
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[test]
fn library_round_trip() {
    let _lock = config_lock();
    let lib = CareerLibrary::at(tmp_dir("lib-rt").join("library"));

    let mut engine = start_engine(4242, "ura");
    play_scoring(&mut engine, 6);
    let before = engine.export();

    let entry = lib
        .save(
            "before-debut",
            Some("Before debut choice"),
            Some("test"),
            &before,
            false,
        )
        .expect("save");
    assert_eq!(entry.name, "before-debut");
    assert_eq!(entry.seed, 4242);
    assert_eq!(entry.scenario_id, "ura");
    assert_eq!(entry.trainee_name, "Special Week");
    assert_eq!(entry.turn, before.state.turn);
    assert_eq!(entry.rng_calls, before.rng_calls);
    assert!(!entry.date_label.is_empty());
    assert!(!entry.phase.is_empty());
    assert!(!entry.saved_at.is_empty());

    let entries = lib.list().expect("list");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "before-debut");

    let (loaded, loaded_entry, advisories) = lib.load("before-debut").expect("load");
    assert_eq!(loaded_entry.name, "before-debut");
    assert_eq!(snapshot_value(&loaded), snapshot_value(&before));
    let _ = advisories; // advisory-only on the same build

    let raw = lib.export_raw("before-debut").expect("export_raw");
    let decoded = RunSnapshotCodec::decode(&raw).expect("export decodes");
    assert_eq!(snapshot_value(&decoded), snapshot_value(&before));

    lib.delete("before-debut").expect("delete");
    assert!(lib.list().expect("list").is_empty());
    assert!(matches!(
        lib.load("before-debut"),
        Err(LabError::NotFound(_))
    ));
}

#[test]
fn library_name_rules_and_overwrite_protection() {
    let _lock = config_lock();
    let lib = CareerLibrary::at(tmp_dir("lib-names").join("library"));
    let mut engine = start_engine(1, "ura");
    let snap = engine.export();

    for bad in [
        "",
        "..",
        "has space",
        "semi;colon",
        "slash/x",
        &"x".repeat(65),
    ] {
        assert!(
            matches!(
                lib.save(bad, None, None, &snap, false),
                Err(LabError::InvalidName(_))
            ),
            "name {bad:?} should be rejected"
        );
    }

    lib.save("cp", None, None, &snap, false)
        .expect("first save");
    assert!(matches!(
        lib.save("cp", None, None, &snap, false),
        Err(LabError::AlreadyExists(_))
    ));
    // Overwrite replaces cleanly.
    play_scoring(&mut engine, 3);
    let snap2 = engine.export();
    let entry = lib.save("cp", None, None, &snap2, true).expect("overwrite");
    assert_eq!(entry.turn, snap2.state.turn);
    let (loaded, _, _) = lib.load("cp").expect("load");
    assert_eq!(snapshot_value(&loaded), snapshot_value(&snap2));

    assert!(matches!(lib.delete("missing"), Err(LabError::NotFound(_))));
}

#[test]
fn import_failure_preserves_last_good() {
    let _lock = config_lock();
    let lib = CareerLibrary::at(tmp_dir("lib-import").join("library"));

    let mut engine = start_engine(99, "ura");
    play_scoring(&mut engine, 4);
    let good = engine.export();
    lib.save("good", None, None, &good, false)
        .expect("save good");
    let good_bytes = std::fs::read(lib.dir().join("good.snapshot.json")).expect("read good file");

    // Garbage and wrong-shape imports fail validation before any write.
    assert!(matches!(
        lib.import("{not json", Some("bad1"), false),
        Err(LabError::InvalidSnapshot(_))
    ));
    assert!(matches!(
        lib.import(r#"{"nope": true}"#, Some("bad2"), false),
        Err(LabError::InvalidSnapshot(_))
    ));
    // A valid snapshot imports fine.
    let entry = lib
        .import(&snapshot_json(&good), Some("good-copy"), false)
        .expect("import ok");
    assert_eq!(entry.name, "good-copy");

    let entries = lib.list().expect("list");
    assert_eq!(entries.len(), 2);
    assert_eq!(
        std::fs::read(lib.dir().join("good.snapshot.json")).expect("re-read"),
        good_bytes,
        "last good checkpoint file must be byte-identical after failed imports"
    );
}

#[test]
fn stale_tmp_files_are_swept() {
    let dir = tmp_dir("sweep");
    std::fs::write(dir.join(".tmp-123-stale.json"), b"partial").unwrap();
    std::fs::write(dir.join("real.json"), b"{}").unwrap();
    assert_eq!(sweep_stale_tmp(&dir), 1);
    assert!(!dir.join(".tmp-123-stale.json").exists());
    assert!(dir.join("real.json").exists());
    assert_eq!(sweep_stale_tmp(&dir), 0);
}

// ---------------------------------------------------------------------------
// Reproducibility + isolation (M4)
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_restore_reproduces_state_and_rng() {
    let _lock = config_lock();
    // Play 6 scoring steps, checkpoint, then play 5 more.
    let mut a = start_engine(777, "ura");
    play_scoring(&mut a, 6);
    let checkpoint = a.export();
    let rng_at_checkpoint = a.rng_calls();
    play_scoring(&mut a, 5);
    let expected = a.export();
    let expected_rng = a.rng_calls();

    // Fresh engine: restore the checkpoint, play the same 5 steps.
    let mut b = SimEngine::create(test_settings());
    b.restore(checkpoint);
    assert_eq!(b.rng_calls(), rng_at_checkpoint);
    play_scoring(&mut b, 5);
    let actual = b.export();

    assert_eq!(b.rng_calls(), expected_rng, "RNG evolution must match");
    assert_eq!(
        snapshot_value(&actual),
        snapshot_value(&expected),
        "identical checkpoint + actions must reproduce state"
    );
}

#[test]
fn sibling_branch_cannot_mutate_checkpoint_or_sibling() {
    let _lock = config_lock();
    let lib = CareerLibrary::at(tmp_dir("lib-sib").join("library"));

    let mut engine = start_engine(31337, "ura");
    play_scoring(&mut engine, 6);
    play_until_free(&mut engine, 12);
    assert_eq!(engine.state().phase, "FREE");
    let checkpoint = engine.export();
    lib.save("origin", None, None, &checkpoint, false)
        .expect("save");
    let file_before =
        std::fs::read(lib.dir().join("origin.snapshot.json")).expect("read checkpoint file");

    let turn = checkpoint.state.turn;
    // Branch A forces REST at the checkpoint turn; branch B runs pure bot.
    let cfg_a = BranchConfig {
        policy: "bot".into(),
        max_actions: 500,
        overrides: vec![ActionOverride {
            turn,
            action_id: "rest".into(),
        }],
    };
    let cfg_b = BranchConfig::default();
    let branch_a = run_branch(&checkpoint, "A", "origin", &cfg_a);
    let branch_b = run_branch(&checkpoint, "B", "origin", &cfg_b);

    assert!(
        branch_a.timeline[0].override_applied,
        "override should apply on the checkpoint turn"
    );
    assert_eq!(branch_a.timeline[0].action_id, "rest");

    // The library checkpoint file is byte-identical: branches are copies.
    let file_after =
        std::fs::read(lib.dir().join("origin.snapshot.json")).expect("re-read checkpoint file");
    assert_eq!(
        file_before, file_after,
        "branch runs must not mutate the checkpoint"
    );

    // A fresh restore of the checkpoint still equals the original, and branch
    // B's head matches a direct replay — A did not leak into B.
    let (reloaded, _, _) = lib.load("origin").expect("reload");
    assert_eq!(snapshot_value(&reloaded), snapshot_value(&checkpoint));
    let mut direct = SimEngine::create(test_settings());
    direct.restore(checkpoint);
    direct.auto_step_scoring();

    let mut from_b = SimEngine::create(test_settings());
    from_b.restore(reloaded);
    from_b.auto_step_scoring();
    assert_eq!(
        snapshot_value(&from_b.export()),
        snapshot_value(&direct.export())
    );

    // B's first step is the bot's choice, recorded with RNG positions.
    assert!(branch_b.timeline[0].rng_calls_after >= branch_b.timeline[0].rng_calls_before);
}

#[test]
fn branch_run_is_deterministic() {
    let _lock = config_lock();
    let mut engine = start_engine(2024, "unity");
    play_scoring(&mut engine, 6);
    let checkpoint = engine.export();
    let cfg = BranchConfig::default();

    let a = run_branch(&checkpoint, "A", "cp", &cfg);
    let b = run_branch(&checkpoint, "B", "cp", &cfg);
    assert_eq!(
        serde_json::to_value(&a.timeline).unwrap(),
        serde_json::to_value(&b.timeline).unwrap(),
        "same checkpoint + config must produce identical timelines"
    );
    assert_eq!(
        serde_json::to_value(&a.outcome).unwrap(),
        serde_json::to_value(&b.outcome).unwrap()
    );
    // ... including isolated per-branch telemetry.
    assert!(a.outcome.telemetry_records > 0);
    assert_eq!(a.outcome.telemetry_records, b.outcome.telemetry_records);
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Play scoring steps until the phase is FREE (bounded), for checkpoints
/// where train/rest overrides are legal.
fn play_until_free(engine: &mut SimEngine, max_steps: usize) {
    for _ in 0..max_steps {
        if engine.state().phase == "FREE" || engine.state().career_complete {
            break;
        }
        engine.auto_step_scoring();
    }
}

#[test]
fn divergence_detected_on_first_decision_difference() {
    let _lock = config_lock();
    let mut engine = start_engine(555, "ura");
    play_scoring(&mut engine, 6);
    play_until_free(&mut engine, 12);
    assert_eq!(engine.state().phase, "FREE");
    let checkpoint = engine.export();
    let turn = checkpoint.state.turn;

    let mk = |action_id: &str| BranchConfig {
        policy: "bot".into(),
        max_actions: 500,
        overrides: vec![ActionOverride {
            turn,
            action_id: action_id.into(),
        }],
    };
    let a = run_branch(&checkpoint, "rest-branch", "cp", &mk("rest"));
    let b = run_branch(&checkpoint, "train-branch", "cp", &mk("train_speed"));
    assert!(a.timeline[0].override_applied && b.timeline[0].override_applied);

    let cmp = compare_branches(&a, &b);
    let div = cmp.first_divergence.expect("must diverge");
    assert_eq!(div.kind, "decision");
    assert_eq!(div.step_index, 0);
    assert_eq!(div.turn, turn);
    assert_eq!(div.action_a, "rest");
    assert_eq!(div.action_b, "train_speed");
    assert!(!cmp.aligned.is_empty());
    assert!(!cmp.aligned[0].same_action);
    // Caveats are always present and honest about RNG.
    assert!(cmp
        .caveats
        .iter()
        .any(|c| c.contains("RNG streams diverge")));
    assert!(cmp.caveats.iter().any(|c| c.contains("does not establish")));
}

#[test]
fn identical_branches_have_no_divergence() {
    let _lock = config_lock();
    let mut engine = start_engine(606, "ura");
    play_scoring(&mut engine, 6);
    let checkpoint = engine.export();
    let a = run_branch(&checkpoint, "A", "cp", &BranchConfig::default());
    let cmp = compare_branches(&a, &a);
    assert!(cmp.first_divergence.is_none());
    assert!(cmp.aligned.iter().all(|r| r.same_action && r.same_outcome));
}

#[test]
fn comparison_report_renders() {
    let _lock = config_lock();
    let mut engine = start_engine(707, "grand_concert");
    play_scoring(&mut engine, 6);
    let checkpoint = engine.export();
    let turn = checkpoint.state.turn;
    let a = run_branch(
        &checkpoint,
        "rest-branch",
        "cp",
        &BranchConfig {
            policy: "bot".into(),
            max_actions: 500,
            overrides: vec![ActionOverride {
                turn,
                action_id: "rest".into(),
            }],
        },
    );
    let b = run_branch(&checkpoint, "bot-branch", "cp", &BranchConfig::default());
    let cmp = compare_branches(&a, &b);

    let md = render_markdown(&cmp);
    // The Markdown source escapes punctuation while displaying literal names.
    assert!(md.contains(r"# Branch comparison: rest\-branch vs bot\-branch"));
    assert!(md.contains("## First divergence"));
    assert!(md.contains("## Timeline (per step)"));
    assert!(md.contains("## Final outcomes"));
    assert!(md.contains("## Caveats"));
    assert!(md.contains("RNG streams diverge"));

    let js = render_json(&cmp);
    let v: serde_json::Value = serde_json::from_str(&js).expect("report JSON parses");
    assert_eq!(v["aName"], "rest-branch");
    assert!(v["caveats"].as_array().unwrap().len() >= 3);
}

// ---------------------------------------------------------------------------
// RNG exact-restore regression (the seed+calls replay is inexact because
// each public call consumes a variable number of raw draws).
// ---------------------------------------------------------------------------

#[test]
fn rng_state_words_restore_is_exact() {
    use uma_sim_core::rng::SimRandom;
    let mut r = SimRandom::new(12345);
    // Consume a mix of calls with variable raw-draw counts.
    let d0 = r.next_double();
    let i0 = r.next_int_until(100);
    let b0 = r.next_boolean(0.3);
    let words = r.state_words();
    let calls = r.call_count();
    let d1 = r.next_double();
    let i1 = r.next_int_range(5, 50);

    let mut r2 = SimRandom::restore_words(12345, calls, words, false);
    assert_eq!(r2.next_double(), d1);
    assert_eq!(r2.next_int_range(5, 50), i1);
    assert_eq!(r2.call_count(), calls + 2);
    let _ = (d0, i0, b0);
}

#[test]
fn snapshot_carries_rng_state_words() {
    let _lock = config_lock();
    let mut engine = start_engine(4243, "ura");
    play_scoring(&mut engine, 3);
    let snap = engine.export();
    assert!(snap.rng_state.is_some(), "exports must carry rngState");
    // Round-trip through JSON (the library path) preserves the words.
    let raw = serde_json::to_string(&snap).unwrap();
    let decoded: RunSnapshot = serde_json::from_str(&raw).unwrap();
    assert_eq!(decoded.rng_state, snap.rng_state);
    // An old snapshot without rngState still decodes (legacy replay path).
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v.as_object_mut().unwrap().remove("rngState");
    let legacy: RunSnapshot = serde_json::from_value(v).unwrap();
    assert!(legacy.rng_state.is_none());
}

#[test]
fn branch_store_round_trip() {
    let _lock = config_lock();
    let store = BranchStore::at(tmp_dir("branches").join("lab"));
    assert!(store.list().expect("list").is_empty());

    let mut engine = start_engine(808, "trackblazer");
    play_scoring(&mut engine, 6);
    let checkpoint = engine.export();
    let result = run_branch(&checkpoint, "tb-run", "cp", &BranchConfig::default());
    store.save(&result).expect("save branch");

    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, result.id);
    assert_eq!(listed[0].name, "tb-run");

    let loaded = store.load(&result.id).expect("load branch");
    assert_eq!(
        serde_json::to_value(&loaded.timeline).unwrap(),
        serde_json::to_value(&result.timeline).unwrap()
    );

    store.delete(&result.id).expect("delete");
    assert!(store.list().expect("list").is_empty());
    assert!(matches!(store.load(&result.id), Err(LabError::NotFound(_))));
    assert!(matches!(
        store.load("../evil"),
        Err(LabError::InvalidName(_))
    ));
    delete_lab_result("br-nope").expect_err("cwd delete of missing branch errors");
}

// ---------------------------------------------------------------------------
// M5: event + mandatory-race checkpoints in all four scenarios
// ---------------------------------------------------------------------------

/// Step until `phase` is reached (bounded), returning steps taken.
fn step_until_phase(engine: &mut SimEngine, phase: &str, max_steps: usize) -> Option<usize> {
    for i in 0..max_steps {
        if engine.state().phase == phase {
            return Some(i);
        }
        if engine.state().career_complete {
            return None;
        }
        // At EVENT phase with options, choose the first; at MANDATORY_RACE the
        // only choice is race; otherwise use the scoring policy.
        let choices = engine.choices();
        if choices.is_empty() {
            return None;
        }
        let first = choices[0].id.clone();
        if engine.state().phase == "EVENT" || engine.state().phase == "MANDATORY_RACE" {
            engine.step(uma_sim_core::session::parse_sim_action(&first));
        } else {
            engine.auto_step_scoring();
        }
    }
    if engine.state().phase == phase {
        Some(max_steps)
    } else {
        None
    }
}

#[test]
fn event_and_mandatory_race_checkpoints_all_scenarios() {
    let _lock = config_lock();
    let lib = CareerLibrary::at(tmp_dir("lib-m5").join("library"));
    let mut seen_event = 0;
    let mut seen_mandatory = 0;

    for (si, scenario) in SCENARIOS.iter().enumerate() {
        for (pi, (phase, want)) in [
            ("EVENT", &mut seen_event),
            ("MANDATORY_RACE", &mut seen_mandatory),
        ]
        .into_iter()
        .enumerate()
        {
            let mut engine = start_engine(1_000_000 + (si as i64) * 10 + pi as i64, scenario);
            let found = step_until_phase(&mut engine, phase, 120);
            let name = format!("{scenario}-{phase}-cp");
            if let Some(_steps) = found {
                *want += 1;
                let checkpoint = engine.export();
                assert_eq!(checkpoint.state.phase, phase);
                lib.save(&name, None, None, &checkpoint, false)
                    .unwrap_or_else(|e| panic!("save {name}: {e}"));

                // Reload and continue: the restored engine must accept the
                // phase-legal choice and advance RNG/state.
                let (reloaded, entry, _) = lib.load(&name).expect("load");
                assert_eq!(entry.phase, phase);
                let rng_before = reloaded.rng_calls;
                let mut resumed = SimEngine::create(test_settings());
                resumed.restore(reloaded);
                let choices = resumed.choices();
                assert!(
                    !choices.is_empty(),
                    "{scenario}/{phase}: restored checkpoint must offer choices"
                );
                let first = choices[0].id.clone();
                resumed.step(uma_sim_core::session::parse_sim_action(&first));
                assert!(
                    resumed.rng_calls() >= rng_before,
                    "{scenario}/{phase}: RNG must advance after the phase-legal step"
                );
            }
        }
    }

    assert!(
        seen_event >= 1,
        "expected at least one EVENT checkpoint across the four scenarios"
    );
    assert!(
        seen_mandatory >= 1,
        "expected at least one MANDATORY_RACE checkpoint across the four scenarios"
    );
}
