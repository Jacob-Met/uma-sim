//! T7/T8 — CLI policy behaviors for uma-sim issue #6.
//!
//! These pin the CLI side of the REST/CLI policy-parity defect as explicit,
//! documented behavior. They invoke the built `uma-sim` binary (not a
//! shell-out from a unit module) and run each case in a fresh temp working
//! directory so the CLI's `.uma-sim/session.json` never lands in the repo.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_uma-sim"))
}

/// Fresh temp working directory per test, so `.uma-sim/session.json` written
/// by the CLI stays out of the repo tree.
fn temp_cwd(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("umasim-cli-policy-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp cwd");
    dir
}

/// T7 — CLI `fast --policy=external` with no policy server configured.
///
/// CURRENT BEHAVIOR (pinned, not blessed): the CLI panics naming the missing
/// env var (exit 101). This is issue #6's second symptom; the loud REST 503
/// is the fixed half.
/// TODO (needs Jacob's decision (c)): convert this panic into a clean
/// non-zero exit (exit 2) with the same message, tracked as a follow-up;
/// when that lands, flip these assertions to exit 2 + no panic.
#[test]
fn cli_policy_external_missing_cmd_names_env() {
    // Precondition: no external policy server is configured.
    std::env::remove_var("UMA_POLICY_CMD");

    let out = Command::new(bin())
        .arg("fast")
        .arg("--policy=external")
        .current_dir(temp_cwd("t7"))
        .output()
        .expect("run uma-sim fast --policy=external");

    let code = out.status.code();
    assert_eq!(
        code,
        Some(101),
        "current behavior: CLI panics (exit 101); got {code:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("UMA_POLICY_CMD"),
        "panic must name the missing config; stderr was: {stderr}"
    );
}

/// T8 — CLI `fast --policy=foo` (unknown policy).
///
/// CURRENT BEHAVIOR (pinned, not blessed): the CLI silently runs the default
/// heuristic and exits 0 — the CLI-side instance of the same silent-downgrade
/// class as issue #6.
/// TODO (follow-up): reject unknown `--policy=` values with a non-zero exit;
/// when that lands, flip these assertions accordingly.
#[test]
fn cli_policy_unknown_runs_default() {
    let out = Command::new(bin())
        .arg("fast")
        .arg("--policy=foo")
        .current_dir(temp_cwd("t8"))
        .output()
        .expect("run uma-sim fast --policy=foo");

    assert!(
        out.status.success(),
        "current behavior: unknown CLI policy exits 0 via default heuristic; got {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("policy=foo"),
        "run should echo the requested policy on completion; stdout was: {stdout}"
    );
}
