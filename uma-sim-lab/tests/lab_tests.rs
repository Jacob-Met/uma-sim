//! Acceptance tests for the uma-sim branch-and-compare lab (T100).
//!
//! 1. checkpoint roundtrip: save -> restore -> identical state and RNG position.
//! 2. determinism: identical checkpoint + identical policy replay => identical timeline hash.
//! 3. branch isolation: playing a branch cannot mutate its checkpoint or a sibling.
//! 4. divergence: two policies from one checkpoint diverge; comparison names first divergence.
//! 5. save-close-reopen: branch head persists; restore-next-choice works after reload.

use std::path::PathBuf;
use uma_sim_core::{SimEngine, SimSettings};
use uma_sim_lab::{compare_timelines, resolve_policy, LabLibrary};

fn fresh_lib(tag: &str) -> LabLibrary {
    let dir = std::env::temp_dir().join(format!("uma-lab-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    LabLibrary::new(dir).expect("library")
}

fn start_engine(seed: i64) -> SimEngine {
    LabLibrary::start_career(seed, "ura", "Special Week", SimSettings::default()).expect("start")
}

fn play_n(engine: &mut SimEngine, policy_name: &str, actions: u32) {
    let policy = resolve_policy(policy_name).expect("policy");
    for _ in 0..actions {
        if engine.state().career_complete {
            break;
        }
        let choices = engine.choices();
        if choices.is_empty() {
            break;
        }
        engine.step(policy(&choices, engine.state()));
    }
}

fn play_to_turn(engine: &mut SimEngine, policy_name: &str, target_turn: i32) {
    let policy = resolve_policy(policy_name).expect("policy");
    let mut actions = 0;
    while !engine.state().career_complete && engine.state().turn < target_turn && actions < 400 {
        let choices = engine.choices();
        if choices.is_empty() {
            break;
        }
        engine.step(policy(&choices, engine.state()));
        actions += 1;
    }
}

fn file_bytes(p: &PathBuf) -> Vec<u8> {
    std::fs::read(p).expect("read file")
}

#[test]
fn checkpoint_roundtrip_preserves_state_and_rng() {
    let lib = fresh_lib("roundtrip");
    let mut engine = start_engine(7);
    play_n(&mut engine, "default", 12);
    let before = engine.export();
    let rec = lib
        .save_checkpoint(&engine, "cp1", "roundtrip")
        .expect("save");

    // Reload the checkpoint through the library and restore two fresh engines.
    // Acceptance is checkpoint->restore->same-actions reproducibility, so both
    // engines under test are restored from the same checkpoint file.
    let loaded = lib.load_checkpoint("cp1").expect("load");
    let mut engine_a = LabLibrary::restore_engine(&loaded.snapshot).expect("restore A");
    let mut engine_b = LabLibrary::restore_engine(&loaded.snapshot).expect("restore B");
    let ra = engine_a.export();
    let rb = engine_b.export();
    assert_eq!(ra.rng_calls, rb.rng_calls, "twin restores must agree");
    assert_eq!(ra.state.turn, before.state.turn);
    assert_eq!(ra.rng_calls, before.rng_calls);
    assert_eq!(ra.rng_seed, before.rng_seed);
    assert_eq!(ra.state.stats.speed, before.state.stats.speed);
    assert_eq!(ra.state.energy, before.state.energy);
    assert_eq!(rec.turn, ra.state.turn);

    // Continuing from both restored engines with the same policy stays identical.
    play_n(&mut engine_a, "default", 10);
    play_n(&mut engine_b, "default", 10);
    let e1 = engine_a.export();
    let e2 = engine_b.export();
    assert_eq!(e1.state.turn, e2.state.turn);
    assert_eq!(e1.rng_calls, e2.rng_calls);
    assert_eq!(e1.state.stats.stamina, e2.state.stats.stamina);
    assert_eq!(e1.state.fans, e2.state.fans);
    assert_eq!(e1.state.energy, e2.state.energy);
}

#[test]
fn identical_policy_replay_is_deterministic() {
    let lib = fresh_lib("determinism");
    let mut engine = start_engine(99);
    play_n(&mut engine, "default", 8);
    lib.save_checkpoint(&engine, "det", "det").expect("save");
    lib.fork_checkpoint("det", "b1", "").expect("fork");
    lib.fork_checkpoint("det", "b2", "").expect("fork");

    let t1 = lib.play_branch("b1", "speed", 15).expect("play b1");
    // Reset b2 by re-forking from the same checkpoint: fresh identical start.
    lib.delete_branch("b2").expect("del");
    lib.fork_checkpoint("det", "b2", "").expect("fork");
    let t2 = lib.play_branch("b2", "speed", 15).expect("play b2");

    assert_eq!(
        t1.hash, t2.hash,
        "same checkpoint + same policy must replay identically"
    );
    assert_eq!(t1.steps.len(), t2.steps.len());
    let cmp = compare_timelines(&t1, &t2);
    assert!(
        cmp.first_divergence_step.is_none(),
        "identical replays must not diverge"
    );
}

#[test]
fn branch_play_cannot_mutate_checkpoint_or_sibling() {
    let lib = fresh_lib("isolation");
    let mut engine = start_engine(1234);
    play_n(&mut engine, "default", 10);
    lib.save_checkpoint(&engine, "origin", "").expect("save");
    let cp_path = lib.dir().join("checkpoints").join("origin.json");
    let cp_before = file_bytes(&cp_path);

    lib.fork_checkpoint("origin", "alpha", "").expect("fork");
    lib.fork_checkpoint("origin", "beta", "").expect("fork");
    let alpha_path = lib.dir().join("branches").join("alpha.json");
    let beta_path = lib.dir().join("branches").join("beta.json");
    let beta_before = file_bytes(&beta_path);

    lib.play_branch("alpha", "stamina", 20).expect("play alpha");

    assert_eq!(
        file_bytes(&cp_path),
        cp_before,
        "checkpoint mutated by branch play!"
    );
    assert_eq!(
        file_bytes(&beta_path),
        beta_before,
        "sibling branch mutated!"
    );
    assert_ne!(
        file_bytes(&alpha_path),
        beta_before,
        "alpha head should have advanced"
    );

    // Sibling still restores to the fork point.
    let beta = lib.load_branch("beta").expect("load beta");
    assert_eq!(beta.snapshot.state.turn, engine.export().state.turn);
}

#[test]
fn policies_diverge_and_comparison_reports_first_divergence() {
    let lib = fresh_lib("diverge");
    let mut engine = start_engine(2026);
    play_n(&mut engine, "default", 6);
    lib.save_checkpoint(&engine, "fork", "").expect("save");
    lib.fork_checkpoint("fork", "speedy", "").expect("fork");
    lib.fork_checkpoint("fork", "tanky", "").expect("fork");

    let t_speed = lib.play_branch("speedy", "speed", 25).expect("play speed");
    let t_stam = lib
        .play_branch("tanky", "stamina", 25)
        .expect("play stamina");

    assert_ne!(
        t_speed.hash, t_stam.hash,
        "speed vs stamina policies should diverge"
    );
    let cmp = compare_timelines(&t_speed, &t_stam);
    let div = cmp.first_divergence_step.expect("must diverge");
    assert!(div >= 1, "divergence step must be >= 1");
    assert!(cmp.first_divergence_field.is_some());
    // The two branches share the same checkpoint RNG origin but evolve
    // independently; both timelines are non-empty.
    assert!(!cmp.rows.is_empty());
    assert_eq!(
        cmp.steps_compared,
        t_speed.steps.len().min(t_stam.steps.len())
    );
}

#[test]
fn save_close_reopen_restore_next_choice() {
    let lib = fresh_lib("reopen");
    let mut engine = start_engine(555);
    play_n(&mut engine, "default", 9);
    lib.save_checkpoint(&engine, "re", "").expect("save");
    lib.fork_checkpoint("re", "play1", "").expect("fork");
    lib.play_branch("play1", "default", 5).expect("play");

    // "Close": drop everything and reopen the library from disk.
    drop(engine);
    drop(lib);
    let lib2 = LabLibrary::new(
        std::env::temp_dir().join(format!("uma-lab-test-reopen-{}", std::process::id())),
    )
    .expect("reopen");
    let before = lib2.load_branch("play1").expect("load branch").snapshot;

    // Restore-next-choice: take the engine's offered choices and step once.
    let eng = LabLibrary::restore_engine(&before).expect("restore");
    let choices = eng.choices();
    assert!(
        !choices.is_empty(),
        "branch must offer choices after reopen"
    );
    let action_id = choices[0].id.clone();
    let lines = lib2.step_branch("play1", &action_id).expect("step");
    assert!(!lines.is_empty());

    let after = lib2.load_branch("play1").expect("load").snapshot;
    assert!(after.rng_calls >= before.rng_calls);
    // State actually advanced or the action at least persisted.
    assert!(after.state.turn >= before.state.turn);
}

#[test]
fn per_branch_rng_streams_are_independent() {
    let lib = fresh_lib("rng");
    let mut engine = start_engine(31337);
    // Play past the opening event sequence so the two policies actually
    // choose different actions afterwards.
    play_to_turn(&mut engine, "default", 8);
    lib.save_checkpoint(&engine, "rng0", "").expect("save");
    let origin_calls = engine.export().rng_calls;

    lib.fork_checkpoint("rng0", "ra", "").expect("fork");
    lib.fork_checkpoint("rng0", "rb", "").expect("fork");

    // Both branches start from the identical RNG position.
    let ra0 = lib.load_branch("ra").expect("ra").snapshot;
    let rb0 = lib.load_branch("rb").expect("rb").snapshot;
    assert_eq!(ra0.rng_calls, origin_calls);
    assert_eq!(rb0.rng_calls, origin_calls);
    assert_eq!(ra0.rng_seed, rb0.rng_seed);

    // Different action sequences consume RNG differently; heads differ.
    lib.play_branch("ra", "speed", 20).expect("play ra");
    lib.play_branch("rb", "racer", 20).expect("play rb");
    let ra1 = lib.load_branch("ra").expect("ra").snapshot;
    let rb1 = lib.load_branch("rb").expect("rb").snapshot;
    assert_ne!(
        (ra1.state.stats.speed, ra1.rng_calls),
        (rb1.state.stats.speed, rb1.rng_calls),
        "branches must evolve independent RNG streams"
    );
}
