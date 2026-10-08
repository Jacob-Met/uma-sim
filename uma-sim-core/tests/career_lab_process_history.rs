//! Regression receiving for PR #37's CLI process/history boundary.
//!
//! Run the native binary in disposable directories. New processes restore the
//! snapshot without in-memory telemetry, so the durable continuation history
//! must retain the actions recorded by earlier commands.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

fn temp_cwd(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "uma-lab-process-history-{}-{}-{label}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).expect("create fresh receiving directory");
    dir
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_uma-sim"))
        .args(args)
        .env("UMA_RACE_MODEL", "stub")
        .current_dir(cwd)
        .output()
        .expect("execute native uma-sim");
    assert!(
        out.status.success(),
        "cwd={} args={args:?} exit={:?}\nstdout={}\nstderr={}",
        cwd.display(),
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn history(cwd: &Path, relative: &str) -> Vec<Value> {
    let raw = std::fs::read_to_string(cwd.join(".uma-sim").join(relative))
        .expect("read recorded native telemetry");
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("decode native telemetry record"))
        .collect()
}

fn bytes_under(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(current).expect("read receiving directory") {
            let entry = entry.expect("read receiving entry");
            let path = entry.path();
            if entry.file_type().expect("read file type").is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(path).expect("read durable bytes"),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(dir, dir, &mut files);
    files
}

fn setup(label: &str) -> PathBuf {
    let cwd = temp_cwd(label);
    run(&cwd, &["start", "--seed=42", "--speed=100"]);
    let debut = run(&cwd, &["step", "race"]);
    let choices = String::from_utf8_lossy(&debut.stdout);
    assert!(
        choices.contains("train_speed") && choices.contains("train_stamina"),
        "fixture must complete the mandatory debut before training: {choices}"
    );
    run(&cwd, &["lab", "save", "base"]);
    cwd
}

#[test]
fn manual_steps_keep_the_previous_process_history() {
    let cwd = setup("manual-prefix");
    run(&cwd, &["lab", "branch", "--from=base"]);
    run(&cwd, &["step", "train_speed"]);
    let first = history(&cwd, "session.telemetry.jsonl");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["action"], "Train:speed");

    run(&cwd, &["step", "rest"]);
    let second = history(&cwd, "session.telemetry.jsonl");
    assert!(
        second.starts_with(&first),
        "earlier manual action was lost across CLI processes; cwd={}\nfirst={first:?}\nsecond={second:?}",
        cwd.display()
    );
    assert_eq!(second.len(), 2);
    assert_eq!(second[1]["action"], "Rest");
    run(&cwd, &["lab", "save", "continued"]);
    assert_eq!(
        history(&cwd, "library/continued.telemetry.jsonl"),
        second,
        "save must retain the complete continuation"
    );
}

#[test]
fn manual_branches_report_their_first_different_action() {
    let cwd = setup("manual-compare");
    let library = cwd.join(".uma-sim/library");
    let source_before = bytes_under(&library);
    for (name, action) in [
        ("speed_branch", "train_speed"),
        ("stamina_branch", "train_stamina"),
    ] {
        run(&cwd, &["lab", "branch", "--from=base"]);
        run(&cwd, &["step", action]);
        run(&cwd, &["step", "rest"]);
        run(&cwd, &["lab", "save", name]);
    }
    let source_after = bytes_under(&library);
    for (name, bytes) in source_before {
        assert_eq!(
            source_after.get(&name),
            Some(&bytes),
            "source checkpoint changed"
        );
    }
    let all_before = bytes_under(&cwd.join(".uma-sim"));
    let out = run(&cwd, &["lab", "compare", "speed_branch", "stamina_branch"]);
    let compared = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        bytes_under(&cwd.join(".uma-sim")),
        all_before,
        "compare mutated durable state"
    );
    assert!(
        compared.contains("first divergence at step 0"),
        "different first manual actions must remain visible after a shared Rest; cwd={}\n{compared}",
        cwd.display()
    );
    assert!(compared.contains("Train:speed") && compared.contains("Train:stamina"));
}

#[test]
fn manual_then_play_keeps_the_manual_prefix() {
    let cwd = setup("manual-play");
    run(&cwd, &["lab", "branch", "--from=base"]);
    run(&cwd, &["step", "train_power"]);
    let manual = history(&cwd, "session.telemetry.jsonl");
    assert_eq!(manual.len(), 1);
    assert_eq!(manual[0]["action"], "Train:power");

    run(&cwd, &["lab", "play", "--policy=default", "--speed=100"]);
    let continued = history(&cwd, "session.telemetry.jsonl");
    assert!(
        continued.starts_with(&manual),
        "lab play discarded a prior manual action; cwd={}\nmanual={manual:?}\ncontinued_first={:?}",
        cwd.display(),
        continued.first()
    );
    assert!(continued.len() > manual.len());
    run(&cwd, &["lab", "save", "mixed"]);
    assert_eq!(history(&cwd, "library/mixed.telemetry.jsonl"), continued);
}
