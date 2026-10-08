//! Exercise the real CLI in isolated working directories: failed publication must
//! neither acknowledge success nor replace the previously loadable career.
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_uma-sim");
const START: &[&str] = &[
    "start",
    "--seed=42",
    "--scenario=ura",
    "--dialogue=off",
    "--race-model=stub",
    "--deck=10001",
];
const FAST: &[&str] = &[
    "fast",
    "--seed=42",
    "--scenario=ura",
    "--dialogue=off",
    "--race-model=stub",
    "--deck=10001",
    "--policy=default",
];

struct Career(PathBuf);

impl Career {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "uma-session-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn session(&self) -> PathBuf {
        self.0.join(".uma-sim/session.json")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .current_dir(&self.0)
            .env_remove("UMA_POLICY_CMD")
            .env_remove("LLVM_PROFILE_FILE")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn limited(&self, args: &[&str]) -> Output {
        Command::new("bash")
            .args([
                "-c",
                "trap '' XFSZ; ulimit -f 1; exec \"$@\"",
                "session-test",
                BIN,
            ])
            .args(args)
            .current_dir(&self.0)
            .env_remove("UMA_POLICY_CMD")
            .env_remove("LLVM_PROFILE_FILE")
            .output()
            .unwrap()
    }

    fn start(&self) -> Vec<u8> {
        succeeded(self.run(START));
        let bytes = fs::read(self.session()).unwrap();
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        bytes
    }

    fn assert_only_session_and_sentinel(&self) {
        let mut names: Vec<_> = fs::read_dir(self.0.join(".uma-sim"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["session.json", "unrelated.keep"]);
        assert_eq!(
            fs::read(self.0.join(".uma-sim/unrelated.keep")).unwrap(),
            b"keep"
        );
    }

    fn sentinel(&self) {
        fs::write(self.0.join(".uma-sim/unrelated.keep"), b"keep").unwrap();
    }
}

impl Drop for Career {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn succeeded(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn failed_save(output: Output) {
    assert!(!output.status.success(), "failed save returned success");
    assert!(
        output.stdout.is_empty(),
        "failed save printed success output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Failed to save session:"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn healthy_commands_publish_reloadable_sessions() {
    let c = Career::new();
    let original = c.start();
    c.sentinel();
    succeeded(c.run(&["step", "rest"]));
    let stepped = fs::read(c.session()).unwrap();
    assert_ne!(stepped, original);
    let _: serde_json::Value = serde_json::from_slice(&stepped).unwrap();
    succeeded(c.run(&["deck", "place", "10001", "speed"]));
    let placed = fs::read(c.session()).unwrap();
    assert_ne!(placed, stepped);
    let _: serde_json::Value = serde_json::from_slice(&placed).unwrap();
    // Starting the same seed replaces the saved career with its original state.
    succeeded(c.run(START));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(c.session()).unwrap()).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&original).unwrap()
    );
    c.assert_only_session_and_sentinel();
}

#[test]
fn start_reports_obstructed_parent_without_success_output() {
    let c = Career::new();
    fs::write(c.0.join(".uma-sim"), b"keep").unwrap();
    failed_save(c.run(START));
    assert_eq!(fs::read(c.0.join(".uma-sim")).unwrap(), b"keep");
}

#[test]
fn fast_reports_obstructed_parent_without_success_output() {
    let c = Career::new();
    fs::write(c.0.join(".uma-sim"), b"keep").unwrap();
    failed_save(c.run(FAST));
    assert_eq!(fs::read(c.0.join(".uma-sim")).unwrap(), b"keep");
}

#[cfg(unix)]
fn failed_replacement_preserves_career(args: &[&str]) {
    let c = Career::new();
    let prior = c.start();
    assert!(prior.len() > 1024);
    c.sentinel();
    failed_save(c.limited(args));
    assert_eq!(fs::read(c.session()).unwrap(), prior);
    c.assert_only_session_and_sentinel();
    // A subsequent normal action must still resume the saved career.
    succeeded(c.run(&["step", "rest"]));
    let _: serde_json::Value = serde_json::from_slice(&fs::read(c.session()).unwrap()).unwrap();
}

#[cfg(unix)]
#[test]
fn start_write_failure_preserves_prior_career() {
    failed_replacement_preserves_career(START);
}

#[cfg(unix)]
#[test]
fn fast_write_failure_preserves_prior_career() {
    failed_replacement_preserves_career(FAST);
}

#[cfg(unix)]
#[test]
fn step_write_failure_preserves_prior_career() {
    failed_replacement_preserves_career(&["step", "rest"]);
}

#[cfg(unix)]
#[test]
fn deck_write_failure_preserves_prior_career() {
    failed_replacement_preserves_career(&["deck", "place", "10001", "speed"]);
}

#[cfg(unix)]
#[test]
fn replacing_session_retains_existing_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let c = Career::new();
    c.start();
    fs::set_permissions(c.session(), fs::Permissions::from_mode(0o640)).unwrap();
    succeeded(c.run(&["step", "rest"]));
    assert_eq!(
        fs::metadata(c.session()).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[cfg(unix)]
#[test]
fn existing_session_symlink_still_publishes_to_its_target() {
    use std::os::unix::fs::symlink;
    let c = Career::new();
    let original = c.start();
    let target = c.0.join("career.json");
    fs::rename(c.session(), &target).unwrap();
    symlink("../career.json", c.session()).unwrap();
    succeeded(c.run(&["step", "rest"]));
    assert!(fs::symlink_metadata(c.session())
        .unwrap()
        .file_type()
        .is_symlink());
    assert_ne!(fs::read(&target).unwrap(), original);
    assert_eq!(fs::read(c.session()).unwrap(), fs::read(target).unwrap());
}

#[cfg(unix)]
#[test]
fn dangling_session_symlink_can_start_a_career() {
    use std::os::unix::fs::symlink;
    let c = Career::new();
    fs::create_dir(c.0.join(".uma-sim")).unwrap();
    symlink("../career.json", c.session()).unwrap();
    c.start();
    assert!(fs::symlink_metadata(c.session())
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read(c.session()).unwrap(),
        fs::read(c.0.join("career.json")).unwrap()
    );
}
