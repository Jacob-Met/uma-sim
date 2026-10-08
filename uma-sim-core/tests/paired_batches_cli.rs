use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "uma-paired-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join(".uma-sim")).unwrap();
        fs::write(
            dir.join(".uma-sim/session.json"),
            b"authored session sentinel\n",
        )
        .unwrap();
        fs::write(
            dir.join("baseline.jsonl"),
            include_bytes!("../examples/paired_batches/baseline.jsonl"),
        )
        .unwrap();
        fs::write(
            dir.join("input.jsonl"),
            include_bytes!("../examples/paired_batches/input.jsonl"),
        )
        .unwrap();
        Self(dir)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_uma-sim"));
        command
            .current_dir(&self.0)
            .env("UMA_REPO_ROOT", self.0.join("absent-root"))
            .env("UMA_RACE_MODEL", "invalid-unused-setting")
            .arg("compare-batches");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn assert_unchanged(&self) {
        assert_eq!(
            fs::read(self.0.join(".uma-sim/session.json")).unwrap(),
            b"authored session sentinel\n"
        );
        assert_eq!(
            fs::read(self.0.join("baseline.jsonl")).unwrap(),
            include_bytes!("../examples/paired_batches/baseline.jsonl")
        );
        assert_eq!(
            fs::read(self.0.join("input.jsonl")).unwrap(),
            include_bytes!("../examples/paired_batches/input.jsonl")
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const FILE_ARGS: [&str; 2] = ["--input=input.jsonl", "--baseline=baseline.jsonl"];

#[test]
fn real_cli_reports_saved_records_without_session_or_simulator_configuration() {
    let fixture = Fixture::new();
    let output = fixture.run(&[FILE_ARGS[0], FILE_ARGS[1], "--format=json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["count"], 3);
    assert_eq!(report["summary"]["wins"], 1);
    assert_eq!(report["summary"]["ties"], 1);
    assert_eq!(report["summary"]["losses"], 1);
    assert_eq!(report["comparisons"][0]["seed"], -7);
    assert_eq!(report["comparisons"][2]["score_delta"], -20);
    let text = fixture.run(&FILE_ARGS);
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("Score delta: mean -3.333333; median 0.000000; min -20; max 10; population stddev 12.472191\n"));
    assert!(text.contains("-20\t200\t180\t[\"ura\",\"Special Week\",99]\n"));
    fixture.assert_unchanged();
}

#[test]
fn unknown_duplicate_empty_and_missing_flags_refuse_before_input_access() {
    let fixture = Fixture::new();
    for args in [
        vec![],
        vec![FILE_ARGS[0]],
        vec![FILE_ARGS[1]],
        vec![FILE_ARGS[0], FILE_ARGS[1], "--format=xml"],
        vec![FILE_ARGS[0], FILE_ARGS[1], "--format=json", "--format=text"],
        vec![FILE_ARGS[0], FILE_ARGS[0], FILE_ARGS[1]],
        vec![FILE_ARGS[0], FILE_ARGS[1], FILE_ARGS[1]],
        vec!["--input=", FILE_ARGS[1]],
        vec![FILE_ARGS[0], "--baseline="],
        vec![FILE_ARGS[0], FILE_ARGS[1], "--format="],
        vec![FILE_ARGS[0], FILE_ARGS[1], "--count=3"],
        vec!["--input", "input.jsonl", FILE_ARGS[1]],
        vec!["--help", FILE_ARGS[0]],
    ] {
        let output = fixture.run(&args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
    let output = fixture.run(&["--input=missing", "--baseline=missing", "--format=xml"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("--format must be text or json"));
    fixture.assert_unchanged();
}

#[test]
fn missing_late_invalid_and_unmatched_input_produce_no_primary_output() {
    let fixture = Fixture::new();
    let valid = fs::read_to_string(fixture.0.join("input.jsonl")).unwrap();
    for (name, content) in [
        ("late.jsonl", format!("{valid}{{}}\n")),
        ("duplicate.jsonl", format!("{valid}{valid}")),
        (
            "unmatched.jsonl",
            valid.lines().take(2).collect::<Vec<_>>().join("\n"),
        ),
    ] {
        fs::write(fixture.0.join(name), content).unwrap();
        let output = fixture.run(&[&format!("--input={name}"), FILE_ARGS[1], "--format=json"]);
        assert_eq!(output.status.code(), Some(1), "{name}");
        assert!(output.stdout.is_empty(), "{name}");
    }
    let missing = fixture.run(&["--input=missing.jsonl", FILE_ARGS[1]]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    fixture.assert_unchanged();
}

#[test]
fn help_is_available_without_files() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Usage: uma-sim compare-batches --input=FILE --baseline=FILE [--format=text|json]\n"
    );
    fixture.assert_unchanged();
}

#[cfg(unix)]
#[test]
fn actual_unwritable_output_and_diagnostic_sinks_return_failure_without_panicking() {
    let fixture = Fixture::new();
    for format in ["text", "json"] {
        // A read-only file descriptor is a deterministic failing sink on Unix.
        let sink = fs::File::open(fixture.0.join("baseline.jsonl")).unwrap();
        let output = fixture
            .command()
            .args(FILE_ARGS)
            .arg(format!("--format={format}"))
            .stdout(Stdio::from(sink))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("could not deliver comparison output")
        );
    }
    let sink = fs::File::open(fixture.0.join("baseline.jsonl")).unwrap();
    let output = fixture
        .command()
        .arg("--bogus")
        .stderr(Stdio::from(sink))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    fixture.assert_unchanged();
}
