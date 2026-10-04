//! T100 — saved-career branch-and-compare laboratory.
//!
//! End-to-end tests through the built `uma-sim` CLI, each in a fresh temp
//! working directory so `.uma-sim/` state never lands in the repo tree.
//! Flow under test: `lab save` → `lab branch --from` → `lab play` (two
//! policies) → `lab compare`, plus the negative cases.

use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_uma-sim"))
}

fn temp_cwd(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("umasim-career-lab-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp cwd");
    dir
}

fn run(cwd: &PathBuf, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("run uma-sim {args:?}: {e}"))
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn assert_ok(out: &Output, ctx: &str) {
    assert!(
        out.status.success(),
        "{ctx}: exit {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        stdout(out),
        stderr(out)
    );
}

/// Build the canonical fixture: base entry + two completed branches.
fn build_two_branches(cwd: &PathBuf) {
    let out = run(cwd, &["start", "--seed=42", "--speed=100"]);
    assert_ok(&out, "start");
    let out = run(cwd, &["lab", "save", "base", "--note=t100-fixture"]);
    assert_ok(&out, "lab save base");
    assert!(
        stdout(&out).contains("Saved 'base'"),
        "save ack: {}",
        stdout(&out)
    );

    let out = run(cwd, &["lab", "branch", "--from=base"]);
    assert_ok(&out, "lab branch base");
    let out = run(cwd, &["lab", "play", "--policy=default", "--speed=100"]);
    assert_ok(&out, "lab play default");
    assert!(
        stdout(&out).contains("Played to completion"),
        "default run completes: {}",
        stdout(&out)
    );
    let out = run(cwd, &["lab", "save", "cont_default", "--policy=default"]);
    assert_ok(&out, "lab save cont_default");

    let out = run(cwd, &["lab", "branch", "--from=base"]);
    assert_ok(&out, "lab branch base (2nd)");
    let out = run(cwd, &["lab", "play", "--policy=bot", "--speed=100"]);
    assert_ok(&out, "lab play bot");
    let out = run(cwd, &["lab", "save", "cont_bot", "--policy=bot"]);
    assert_ok(&out, "lab save cont_bot");
}

#[test]
fn lab_save_branch_play_list_roundtrip() {
    let cwd = temp_cwd("roundtrip");
    let out = run(&cwd, &["start", "--seed=7", "--speed=100"]);
    assert_ok(&out, "start");
    let out = run(
        &cwd,
        &["lab", "save", "fork_point", "--note=before-big-choice"],
    );
    assert_ok(&out, "lab save");
    let saved = stdout(&out);
    assert!(
        saved.contains("Saved 'fork_point'"),
        "ack names entry: {saved}"
    );

    // Library lists the entry with its parent/note-free metadata.
    let out = run(&cwd, &["lab", "list"]);
    assert_ok(&out, "lab list");
    let list = stdout(&out);
    assert!(list.contains("fork_point"), "list shows entry: {list}");
    assert!(list.contains("seed=7"), "list shows seed: {list}");

    // Info renders the recorded note.
    let out = run(&cwd, &["lab", "info", "fork_point"]);
    assert_ok(&out, "lab info");
    let info = stdout(&out);
    assert!(
        info.contains("before-big-choice"),
        "info shows note: {info}"
    );
    assert!(info.contains("turn:"), "info shows turn: {info}");

    // Branching restores the exact turn into the active session.
    let out = run(&cwd, &["lab", "branch", "--from=fork_point"]);
    assert_ok(&out, "lab branch");
    assert!(
        stdout(&out).contains("Branched from 'fork_point'"),
        "branch ack: {}",
        stdout(&out)
    );
    let out = run(&cwd, &["state"]);
    assert_ok(&out, "state after branch");
    assert!(
        stdout(&out).contains("Turn 1 "),
        "branched session resumes at saved turn: {}",
        stdout(&out)
    );
}

#[test]
fn lab_compare_two_policy_branches() {
    let cwd = temp_cwd("compare");
    build_two_branches(&cwd);

    let out = run(&cwd, &["lab", "compare", "cont_default", "cont_bot"]);
    assert_ok(&out, "lab compare");
    let cmp = stdout(&out);
    assert!(
        cmp.contains("origin: shared"),
        "same origin detected: {cmp}"
    );
    assert!(
        cmp.contains("related branches: true"),
        "branch relation: {cmp}"
    );
    assert!(cmp.contains("terminal A:"), "both terminals shown: {cmp}");
    assert!(cmp.contains("delta:"), "outcome delta shown: {cmp}");
    assert!(
        cmp.contains("shared action prefix:"),
        "action prefix reported: {cmp}"
    );
    // Policies must actually diverge somewhere on this fixture, otherwise the
    // lab cannot demonstrate its core purpose.
    assert!(
        cmp.contains("first divergence at step"),
        "policies diverge (lab's core purpose): {cmp}"
    );
}

#[test]
fn lab_compare_entry_with_itself_is_identical() {
    let cwd = temp_cwd("selfcmp");
    build_two_branches(&cwd);

    let out = run(&cwd, &["lab", "compare", "cont_default", "cont_default"]);
    assert_ok(&out, "lab compare self");
    let cmp = stdout(&out);
    assert!(
        cmp.contains("action sequences identical for the compared span"),
        "self-compare has no divergence: {cmp}"
    );
    assert!(cmp.contains("delta:"), "self delta shown: {cmp}");
}

#[test]
fn lab_rejects_unsafe_names_and_missing_entries() {
    let cwd = temp_cwd("negative");
    let out = run(&cwd, &["start", "--seed=42"]);
    assert_ok(&out, "start");

    let out = run(&cwd, &["lab", "save", "../escape"]);
    assert!(!out.status.success(), "path-traversal name rejected");
    assert!(
        stderr(&out).contains("invalid library name"),
        "names the problem: {}",
        stderr(&out)
    );

    let out = run(&cwd, &["lab", "branch", "--from=does-not-exist"]);
    assert!(!out.status.success(), "branch of missing entry fails");
    assert!(
        stderr(&out).contains("no library entry"),
        "names the missing entry: {}",
        stderr(&out)
    );

    let out = run(&cwd, &["lab", "info", "does-not-exist"]);
    assert!(!out.status.success(), "info of missing entry fails");

    let out = run(&cwd, &["lab", "delete", "does-not-exist"]);
    assert!(!out.status.success(), "delete of missing entry fails");
}

#[test]
fn lab_play_rejects_external_policy() {
    let cwd = temp_cwd("extpolicy");
    let out = run(&cwd, &["start", "--seed=42"]);
    assert_ok(&out, "start");
    let out = run(&cwd, &["lab", "play", "--policy=external"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "external policy rejected loudly: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("built-in policies only"),
        "explains the boundary: {}",
        stderr(&out)
    );
}

#[test]
fn lab_delete_removes_entry() {
    let cwd = temp_cwd("delete");
    let out = run(&cwd, &["start", "--seed=42"]);
    assert_ok(&out, "start");
    let out = run(&cwd, &["lab", "save", "temp_entry"]);
    assert_ok(&out, "save");
    let out = run(&cwd, &["lab", "delete", "temp_entry"]);
    assert_ok(&out, "delete");
    assert!(
        stdout(&out).contains("Deleted 'temp_entry'"),
        "delete ack: {}",
        stdout(&out)
    );
    let out = run(&cwd, &["lab", "list"]);
    assert_ok(&out, "list");
    assert!(
        !stdout(&out).contains("temp_entry"),
        "entry gone from list: {}",
        stdout(&out)
    );
}
