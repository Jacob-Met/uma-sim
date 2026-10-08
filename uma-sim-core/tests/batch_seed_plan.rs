//! Actual-CLI receiving at the batch output and saved-career boundaries.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const SENTINEL: &[u8] = b"prior reviewed batch output must remain exact\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "uma-batch-seed-plan-{}-{stamp}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_uma-sim"))
            .args(args)
            .args(["--race-model=stub", "--dialogue=off", "--speed=100"])
            .current_dir(&self.0)
            .env(
                "UMA_REPO_ROOT",
                Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
            )
            .env_remove("UMA_POLICY_CMD")
            .env_remove("UMA_RACE_MODEL")
            .output()
            .expect("launch actual CLI")
    }

    fn saved_career(&self) -> Vec<u8> {
        let started = self.run(&["start", "--seed=17"]);
        assert!(
            started.status.success(),
            "{}",
            String::from_utf8_lossy(&started.stderr)
        );
        fs::read(self.0.join(".uma-sim/session.json")).unwrap()
    }

    fn batch(&self, flags: &[&str], output: &Path, fail_if_work_starts: bool) -> Output {
        let output_flag = format!("--output={}", output.display());
        let mut args = vec!["batch"];
        args.extend_from_slice(flags);
        args.push(&output_flag);
        if fail_if_work_starts {
            // A broken admission path must fail promptly instead of running
            // the unintended default100 careers. A valid preflight refusal
            // occurs before this unavailable provider can be consulted.
            args.push("--policy=external");
        }
        self.run(&args)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_refused(flags: &[&str], diagnostic: &str) {
    let work = Workspace::new();
    let saved = work.saved_career();
    let output = work.0.join("authored.jsonl");
    fs::write(&output, SENTINEL).unwrap();
    let result = work.batch(flags, &output, true);
    let stderr = String::from_utf8_lossy(&result.stderr);
    let output_preserved = fs::read(&output).unwrap() == SENTINEL;
    let career_preserved = fs::read(work.0.join(".uma-sim/session.json")).unwrap() == saved;
    assert!(
        result.status.code() == Some(2)
            && result.stdout.is_empty()
            && stderr.contains(diagnostic)
            && output_preserved
            && career_preserved,
        "flags={flags:?}, code={:?}, output_preserved={output_preserved}, career_preserved={career_preserved}\nstdout={}\nstderr={stderr}",
        result.status.code(), String::from_utf8_lossy(&result.stdout)
    );
}

fn assert_seeds(flags: &[&str], expected: &[i64]) {
    let work = Workspace::new();
    let saved = work.saved_career();
    let output = work.0.join("careers.jsonl");
    let result = work.batch(flags, &output, false);
    assert!(
        result.status.success(),
        "flags={flags:?}\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = fs::read_to_string(output).unwrap();
    let seeds = text
        .lines()
        .map(|line| {
            let terminal: serde_json::Value =
                serde_json::from_str(line).expect("native terminal JSON");
            terminal["seed"]
                .as_i64()
                .expect("exact signed terminal seed")
        })
        .collect::<Vec<_>>();
    assert_eq!(seeds, expected);
    assert_eq!(
        fs::read(work.0.join(".uma-sim/session.json")).unwrap(),
        saved
    );
}

#[test]
fn invalid_effective_counts_preserve_output_and_saved_career() {
    for flag in [
        "--count=",
        "--count=oops",
        "--count=1.5",
        "--count=0",
        "--count=-3",
        "--count=9223372036854775808",
    ] {
        assert_refused(&[flag], "Error: invalid batch count");
    }
}

#[test]
fn overflowing_explicit_and_default_ranges_preserve_output() {
    assert_refused(
        &["--seed=9223372036854775807", "--count=2"],
        "seed range overflows",
    );
    assert_refused(&["--seed=9223372036854775807"], "seed range overflows");
}

#[test]
fn refusal_happens_before_creating_output_parents_or_session() {
    for flags in [
        vec!["--count=0"],
        vec!["--count=not-a-count"],
        vec!["--seed=9223372036854775807", "--count=2"],
    ] {
        let work = Workspace::new();
        let parent = work.0.join("never-created");
        let output = parent.join("batch.jsonl");
        let result = work.batch(&flags, &output, true);
        assert_eq!(result.status.code(), Some(2), "flags={flags:?}");
        assert!(result.stdout.is_empty());
        assert!(!parent.exists());
        assert!(!work.0.join(".uma-sim").exists());
    }
}

#[test]
fn first_count_precedence_is_preserved() {
    assert_seeds(&["--seed=7", "--count=2", "--count=oops"], &[7, 8]);
    assert_refused(&["--count=oops", "--count=2"], "invalid batch count");
}

#[test]
fn explicit_list_retains_order_duplicates_and_ignored_count() {
    assert_seeds(
        &[
            "--seed=9223372036854775807",
            "--count=oops",
            "--seeds=10, ,5,,10,",
            "--seeds=99",
        ],
        &[10, 5, 10],
    );
}

#[test]
fn consecutive_signed_boundary_seeds_are_exact() {
    assert_seeds(&["--seed=-1", "--count=3"], &[-1, 0, 1]);
    assert_seeds(
        &["--seed=9223372036854775806", "--count=2"],
        &[i64::MAX - 1, i64::MAX],
    );
    assert_seeds(&["--seed=-9223372036854775808", "--count=1"], &[i64::MIN]);
}

#[test]
fn malformed_seed_remains_the_first_refusal() {
    assert_refused(&["--seed=oops", "--count=0"], "Error: invalid seed 'oops'");
    assert_refused(
        &["--seed=oops", "--seeds=7", "--count=oops"],
        "Error: invalid seed 'oops'",
    );
}

#[test]
fn explicit_list_refusal_keeps_original_status_and_output() {
    let work = Workspace::new();
    let output = work.0.join("prior.jsonl");
    fs::write(&output, SENTINEL).unwrap();
    let result = work.batch(&["--seeds=7,nope", "--count=oops"], &output, true);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--seeds needs a comma-separated list")
    );
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read(output).unwrap(), SENTINEL);
}
