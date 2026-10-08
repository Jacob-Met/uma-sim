//! Real-CLI admission checks for issue #87.
//!
//! Every child uses its own working directory and command-local environment.
//! Malformed seeds must be refused before saved careers or batch output change.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const OUTPUT_SENTINEL: &[u8] = b"authored batch output: keep these bytes\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "uma-seed-admission-{}-{stamp}-{serial}-{label}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create exclusive test workspace");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_uma-sim"))
            .args(args)
            .args(["--race-model=stub", "--dialogue=off", "--speed=100"])
            .current_dir(&self.0)
            .env(
                "UMA_REPO_ROOT",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("workspace root"),
            )
            .env_remove("UMA_POLICY_CMD")
            .env_remove("UMA_RACE_MODEL")
            .output()
            .expect("launch actual uma-sim CLI")
    }

    fn session_path(&self) -> PathBuf {
        self.0.join(".uma-sim/session.json")
    }

    fn start(&self, seed_flags: &[&str]) -> Vec<u8> {
        let mut args = vec!["start"];
        args.extend_from_slice(seed_flags);
        let out = self.run(&args);
        assert!(
            out.status.success(),
            "valid start failed: code={:?}\nstdout={}\nstderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        fs::read(self.session_path()).expect("actual start saved a career")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn saved_seed(bytes: &[u8]) -> Option<i64> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    value.get("meta")?.get("seed")?.as_i64()
}

fn check_refused_start(label: &str, seed_flags: &[&str], invalid_value: &str) {
    let work = Workspace::new(label);
    let before = work.start(&["--seed=7"]);
    assert_eq!(saved_seed(&before), Some(7), "authored career precondition");

    let mut args = vec!["start"];
    args.extend_from_slice(seed_flags);
    let out = work.run(&args);
    // Capture all consequential observations before any failing assertion.
    let after = fs::read(work.session_path()).ok();
    let unchanged = after.as_deref() == Some(before.as_slice());
    let after_seed = after.as_deref().and_then(saved_seed);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let diagnostic = format!("Error: invalid seed '{invalid_value}'");
    assert!(
        out.status.code() == Some(2)
            && unchanged
            && out.stdout.is_empty()
            && stderr.contains(&diagnostic)
            && stderr.contains("signed 64-bit integer"),
        "seed admission failed: args={args:?}, code={:?}, bytes_unchanged={unchanged}, \
         before_len={}, after_len={:?}, before_seed=7, after_seed={after_seed:?}\n\
         stdout={}\nstderr={stderr}",
        out.status.code(),
        before.len(),
        after.as_ref().map(Vec::len),
        String::from_utf8_lossy(&out.stdout)
    );
}

fn check_refused_batch(label: &str, seed_flags: &[&str], invalid_value: &str) {
    let work = Workspace::new(label);
    let career_before = work.start(&["--seed=7"]);
    let output_path = work.0.join("authored.jsonl");
    fs::write(&output_path, OUTPUT_SENTINEL).expect("author existing batch output");
    let output_flag = format!("--output={}", output_path.display());
    let mut args = vec!["batch", "--count=0"];
    args.extend_from_slice(seed_flags);
    args.push(&output_flag);
    let out = work.run(&args);
    // The original CLI truncates this file even when zero careers are requested.
    let output_after = fs::read(&output_path).ok();
    let career_after = fs::read(work.session_path()).ok();
    let output_unchanged = output_after.as_deref() == Some(OUTPUT_SENTINEL);
    let career_unchanged = career_after.as_deref() == Some(career_before.as_slice());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let diagnostic = format!("Error: invalid seed '{invalid_value}'");
    assert!(
        out.status.code() == Some(2)
            && output_unchanged
            && career_unchanged
            && out.stdout.is_empty()
            && stderr.contains(&diagnostic)
            && stderr.contains("signed 64-bit integer"),
        "batch admission failed: args={args:?}, code={:?}, \
         output_unchanged={output_unchanged}, before_output_len={}, after_output_len={:?}, \
         career_unchanged={career_unchanged}, after_seed={:?}\nstdout={}\nstderr={stderr}",
        out.status.code(),
        OUTPUT_SENTINEL.len(),
        output_after.as_ref().map(Vec::len),
        career_after.as_deref().and_then(saved_seed),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn malformed_start_preserves_existing_career() {
    check_refused_start("malformed-start", &["--seed=not-a-number"], "not-a-number");
}

#[test]
fn empty_seed_preserves_existing_career() {
    check_refused_start("empty", &["--seed="], "");
}

#[test]
fn fractional_seed_preserves_existing_career() {
    check_refused_start("fractional", &["--seed=1.5"], "1.5");
}

#[test]
fn seed_above_i64_max_preserves_existing_career() {
    check_refused_start(
        "above-max",
        &["--seed=9223372036854775808"],
        "9223372036854775808",
    );
}

#[test]
fn seed_below_i64_min_preserves_existing_career() {
    check_refused_start(
        "below-min",
        &["--seed=-9223372036854775809"],
        "-9223372036854775809",
    );
}

#[test]
fn malformed_batch_preserves_output_before_zero_career_run() {
    check_refused_batch("malformed-batch", &["--seed=not-a-number"], "not-a-number");
}

#[test]
fn malformed_seed_before_valid_duplicate_is_refused() {
    check_refused_start("invalid-first", &["--seed=oops", "--seed=99"], "oops");
}

#[test]
fn malformed_seed_after_valid_duplicate_is_refused() {
    check_refused_start("invalid-last", &["--seed=99", "--seed=oops"], "oops");
}

#[test]
fn malformed_seed_is_refused_even_when_explicit_list_would_override_it() {
    check_refused_batch(
        "invalid-with-list",
        &["--seed=oops", "--seeds=10,5"],
        "oops",
    );
}

#[test]
fn malformed_seed_does_not_create_a_first_career() {
    let work = Workspace::new("no-prior-career");
    let out = work.run(&["start", "--seed=oops"]);
    let session_after = fs::read(work.session_path()).ok();
    let directory_created = work.0.join(".uma-sim").exists();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.code() == Some(2)
            && session_after.is_none()
            && !directory_created
            && out.stdout.is_empty()
            && stderr.contains("Error: invalid seed 'oops'"),
        "first-career admission failed: code={:?}, directory_created={directory_created}, \
         saved_bytes={:?}, saved_seed={:?}\nstdout={}\nstderr={stderr}",
        out.status.code(),
        session_after.as_ref().map(Vec::len),
        session_after.as_deref().and_then(saved_seed),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn omitted_seed_still_starts_seed_42() {
    let work = Workspace::new("default");
    let saved = work.start(&[]);
    assert_eq!(saved_seed(&saved), Some(42));
}

#[test]
fn valid_signed_seeds_are_saved_exactly() {
    for (flag, expected) in [("--seed=+7", 7), ("--seed=-7", -7), ("--seed=0", 0)] {
        let work = Workspace::new("signed");
        let saved = work.start(&[flag]);
        assert_eq!(saved_seed(&saved), Some(expected), "{flag}");
    }
}

#[test]
fn last_valid_duplicate_seed_still_wins() {
    let work = Workspace::new("valid-duplicates");
    let saved = work.start(&["--seed=7", "--seed=-11"]);
    assert_eq!(saved_seed(&saved), Some(-11));
}

#[test]
fn signed_i64_boundaries_are_admitted_in_one_record_batches() {
    for seed in [
        "-9223372036854775808",
        "9223372036854775807",
        "+9223372036854775807",
        "-0",
        "+0",
    ] {
        let work = Workspace::new("boundary");
        let output_path = work.0.join("boundary.jsonl");
        let output_flag = format!("--output={}", output_path.display());
        let seed_flag = format!("--seed={seed}");
        let out = work.run(&["batch", "--count=1", &seed_flag, &output_flag]);
        let output_after = fs::read(&output_path).ok();
        let terminal_seed = output_after.as_deref().and_then(|bytes| {
            let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
            value.get("seed")?.as_i64()
        });
        assert!(
            out.status.success()
                && terminal_seed == seed.parse::<i64>().ok()
                && !work.session_path().exists(),
            "valid boundary seed={seed}: code={:?}, output_len={:?}\nstdout={}\nstderr={}",
            out.status.code(),
            output_after.as_ref().map(Vec::len),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn explicit_seed_list_still_overrides_valid_seed_and_count() {
    let work = Workspace::new("valid-list");
    let output_path = work.0.join("careers.jsonl");
    let output_flag = format!("--output={}", output_path.display());
    let out = work.run(&[
        "batch",
        "--seed=77",
        "--count=0",
        "--seeds=10,5,7",
        &output_flag,
    ]);
    let output_after = fs::read_to_string(&output_path).ok();
    let seeds = output_after.as_ref().map(|text| {
        text.lines()
            .map(|line| {
                let value: serde_json::Value =
                    serde_json::from_str(line).expect("actual batch JSONL");
                value["seed"].as_i64().expect("integer terminal seed")
            })
            .collect::<Vec<_>>()
    });
    assert!(
        out.status.success() && seeds == Some(vec![10, 5, 7]) && !work.session_path().exists(),
        "valid list priority failed: code={:?}, seeds={seeds:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
