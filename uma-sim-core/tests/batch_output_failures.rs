//! Exercise batch output through the actual CLI and operating-system writes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: PathBuf,
    retain: bool,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let receiving = std::env::var_os("UMA_BATCH_RECEIVING_ROOT");
        let retain = receiving.is_some();
        let parent = receiving
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = parent.join(format!(
            "batch-output-{name}-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self { root, retain }
    }

    fn command(&self, output: &Path) -> Command {
        let data = std::env::var_os("UMA_BATCH_DATA_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .into()
            });
        let mut command = Command::new(env!("CARGO_BIN_EXE_uma-sim"));
        command
            .current_dir(&self.root)
            .env("UMA_REPO_ROOT", data)
            .args([
                "batch",
                "--seeds=42,43,44",
                "--scenario=ura",
                "--policy=default",
                "--race-model=physics",
            ])
            .arg(format!("--output={}", output.display()));
        command
    }

    fn record(&self, name: &str, result: &Output) {
        std::fs::write(self.root.join(format!("{name}.stdout")), &result.stdout).unwrap();
        std::fs::write(self.root.join(format!("{name}.stderr")), &result.stderr).unwrap();
        std::fs::write(
            self.root.join(format!("{name}.status.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "code": result.status.code(),
                "success": result.status.success(),
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.retain {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

#[test]
fn batch_output_open_failure_is_nonzero_and_preserves_existing_directory() {
    let fixture = Fixture::new("open");
    let target = fixture.root.join("existing output directory");
    std::fs::create_dir(&target).unwrap();
    let sentinel = target.join("keep.txt");
    std::fs::write(&sentinel, b"existing output is not a file\n").unwrap();

    let result = fixture.command(&target).output().unwrap();
    fixture.record("open-failure", &result);

    assert!(target.is_dir());
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"existing output is not a file\n"
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("Failed to open"));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("Batch wrote"));
    assert_eq!(
        result.status.code(),
        Some(1),
        "batch did not return exit 1 after output-open failure: {:?}",
        result
    );
}

#[test]
fn batch_success_writes_every_requested_career_and_returns_zero() {
    let fixture = Fixture::new("success");
    let target = fixture.root.join("nested output").join("careers.jsonl");
    let result = fixture.command(&target).output().unwrap();
    fixture.record("success", &result);

    assert!(result.status.success(), "{:?}", result);
    assert!(String::from_utf8_lossy(&result.stdout).contains("Batch wrote 3/3 records"));
    let text = std::fs::read_to_string(&target).unwrap();
    assert!(text.ends_with('\n'));
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 3);
    for (record, seed) in records.iter().zip([42_i64, 43, 44]) {
        assert_eq!(record["seed"], seed);
        assert_eq!(record["scenario"], "ura");
        assert!(record["score"].is_i64());
        assert!(!record["grade"].as_str().unwrap().is_empty());
    }
}

// The Linux x86-64 native receiving boundary supports a deterministic real
// mid-file error without a mock writer or filesystem/service modification.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn batch_midstream_write_failure_is_nonzero_and_retains_the_written_prefix() {
    use std::os::unix::process::CommandExt;

    #[repr(C)]
    struct RLimit {
        current: u64,
        maximum: u64,
    }
    extern "C" {
        fn setrlimit(resource: i32, limit: *const RLimit) -> i32;
        fn signal(signal: i32, handler: usize) -> usize;
    }
    const RLIMIT_FSIZE: i32 = 1;
    const SIGXFSZ: i32 = 25;
    const SIG_IGN: usize = 1;

    let fixture = Fixture::new("midstream");
    let reference = fixture.root.join("complete.jsonl");
    let complete = fixture.command(&reference).output().unwrap();
    fixture.record("reference", &complete);
    assert!(complete.status.success(), "{:?}", complete);
    let expected = std::fs::read(&reference).unwrap();
    let first_line_end = expected.iter().position(|byte| *byte == b'\n').unwrap() + 1;
    let budget = (first_line_end + 17) as u64;
    assert!(budget < expected.len() as u64);

    let target = fixture.root.join("partial.jsonl");
    let mut command = fixture.command(&target);
    // Only this child receives the file-size limit. Ignoring SIGXFSZ makes
    // write return EFBIG, so the command's own error exit is being tested.
    unsafe {
        command.pre_exec(move || {
            if signal(SIGXFSZ, SIG_IGN) == usize::MAX {
                return Err(std::io::Error::last_os_error());
            }
            let limit = RLimit {
                current: budget,
                maximum: budget,
            };
            if setrlimit(RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let result = command.output().unwrap();
    fixture.record("write-failure", &result);
    let partial = std::fs::read(&target).unwrap();

    assert_eq!(partial, expected[..budget as usize]);
    assert_eq!(partial.iter().filter(|byte| **byte == b'\n').count(), 1);
    let first_record: serde_json::Value =
        serde_json::from_slice(&partial[..first_line_end]).unwrap();
    assert_eq!(first_record["seed"], 42);
    assert!(String::from_utf8_lossy(&result.stderr).contains("write error:"));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("Batch wrote"));
    assert_eq!(
        result.status.code(),
        Some(1),
        "batch did not return exit 1 after a real midstream write failure: {:?}",
        result
    );
}
