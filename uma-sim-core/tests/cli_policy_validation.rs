use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

static NEXT: AtomicUsize = AtomicUsize::new(0);

type FileSnapshot = Vec<(PathBuf, Vec<u8>, SystemTime)>;

fn snapshot(root: &Path) -> FileSnapshot {
    fn visit(root: &Path, dir: &Path, rows: &mut FileSnapshot) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                    fs::metadata(path).unwrap().modified().unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

struct Fixture {
    root: PathBuf,
    before: FileSnapshot,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "uma-policy-validation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join(".uma-sim")).unwrap();
        fs::create_dir(root.join("output-directory")).unwrap();
        fs::write(
            root.join(".uma-sim/session.json"),
            b"existing session sentinel\n",
        )
        .unwrap();
        fs::write(root.join("output.jsonl"), b"existing output sentinel\n").unwrap();
        fs::write(
            root.join("output-directory/keep.txt"),
            b"directory sentinel\n",
        )
        .unwrap();
        let before = snapshot(&root);
        Self { root, before }
    }

    fn run(&self, command: &str, policies: &[&str], output: &Path) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_uma-sim"));
        child.current_dir(&self.root);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("UMA_") {
                child.env_remove(key);
            }
        }
        child
            .env("UMA_REPO_ROOT", &self.root)
            .arg(command)
            .args(["--seed=42", "--count=1", "--race-model=physics"])
            .args(policies)
            .arg(format!("--output={}", output.display()))
            .output()
            .unwrap()
    }

    fn assert_preserved(&self) {
        assert_eq!(snapshot(&self.root), self.before);
        assert!(!self.root.join("missing-parent").exists());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn assert_refusal(output: Output, value: &str) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!("Error: unknown policy '{value}'; expected one of: default, bot, external\n")
    );
}

#[test]
fn every_career_command_refuses_before_touching_session_or_existing_output() {
    let fixture = Fixture::new();
    for command in ["start", "fast", "batch", "export-telemetry"] {
        assert_refusal(
            fixture.run(
                command,
                &["--policy=bto"],
                &fixture.root.join("output.jsonl"),
            ),
            "bto",
        );
        fixture.assert_preserved();
    }
}

#[test]
fn refusal_creates_neither_output_file_nor_parent_directory() {
    let fixture = Fixture::new();
    for command in ["start", "fast", "batch", "export-telemetry"] {
        assert_refusal(
            fixture.run(
                command,
                &["--policy=bto"],
                &fixture.root.join("missing-parent/output.jsonl"),
            ),
            "bto",
        );
        fixture.assert_preserved();
    }
}

#[test]
fn policy_values_are_exact_and_every_repeated_value_is_validated() {
    let fixture = Fixture::new();
    let output = fixture.root.join("output-directory");
    for value in [
        "",
        "bto",
        "BOT",
        "Default",
        " default",
        "bot ",
        "external\t",
    ] {
        let arg = format!("--policy={value}");
        assert_refusal(fixture.run("batch", &[&arg], &output), value);
    }
    assert_refusal(
        fixture.run("batch", &["--policy=bto", "--policy=default"], &output),
        "bto",
    );
    assert_refusal(
        fixture.run("batch", &["--policy=default", "--policy=bto"], &output),
        "bto",
    );
    fixture.assert_preserved();
}

#[test]
fn known_omitted_and_repeated_valid_policies_reach_existing_output_admission() {
    let fixture = Fixture::new();
    let output_path = fixture.root.join("output-directory");
    // Creating a file at an existing directory fails after flag parsing and
    // before SimEngine::create. No policy, including external, executes here.
    for args in [
        vec![],
        vec!["--policy=default"],
        vec!["--policy=bot"],
        vec!["--policy=external"],
        vec!["--policy=bot", "--policy=default"],
        vec!["--policy=default", "--policy=bot"],
        vec!["--policy=external", "--policy=default"],
        vec!["--unrelated-argument=unchanged", "--policy=default"],
    ] {
        let output = fixture.run("batch", &args, &output_path);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .starts_with(&format!("Failed to open {}: ", output_path.display())),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture.assert_preserved();
    }
}
