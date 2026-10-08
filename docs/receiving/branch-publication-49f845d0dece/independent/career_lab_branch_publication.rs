//! Independent receiving for issue76: branch publication is creation, not update.
//! Uses the public domain API and real independent processes with disposable stores.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};
use uma_sim_core::career_lab::{BranchStore, LabError, LabResult};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "uma-branch-receiving-{}-{}-{}-{}",
            label, std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::SeqCst),
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn store(&self) -> BranchStore { BranchStore::at(self.0.join("lab")) }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

fn result(id: &str, name: &str, marker: u64) -> LabResult {
    serde_json::from_value(json!({
        "id": id, "name": name, "checkpointName": "preserved-checkpoint",
        "checkpointTurn": 11, "seed": 42, "scenarioId": "ura", "traineeName": "Special Week",
        "config": {"policy": if marker % 2 == 0 {"default"} else {"bot"}, "maxActions": marker + 1, "overrides": []},
        "startedAt": "2026-10-08T09:33:51Z", "timeline": [],
        "outcome": {
            "steps": marker + 1, "finalTurn": 12 + marker, "careerComplete": false,
            "completedRaces": [], "stats": {"speed": 100, "stamina": 101, "power": 102, "guts": 103, "wit": 104},
            "energy": 70, "mood": "NORMAL", "fans": 100 + marker, "skillPoints": 20,
            "learnedSkills": [], "sparks": [], "scenarioResources": {},
            "totalRngCalls": marker + 7, "telemetryRecords": marker + 1
        }
    })).expect("authored complete LabResult fixture")
}

fn value(result: &LabResult) -> Value { serde_json::to_value(result).unwrap() }
fn bytes(result: &LabResult) -> Vec<u8> { serde_json::to_vec_pretty(result).unwrap() }

#[test]
fn changed_payload_cannot_replace_an_acknowledged_identity() {
    let fixture = Fixture::new("changed");
    let store = fixture.store();
    let first = result("br-1791452031-0000", "first-acknowledged", 0);
    let path = store.save(&first).expect("first creation");
    let original = fs::read(&path).unwrap();
    let second = result(&first.id, "later-distinct-outcome", 1);
    let attempted = store.save(&second);
    assert!(matches!(attempted, Err(LabError::AlreadyExists(ref id)) if id == &first.id),
        "a second creation must report a conflict, got {attempted:?}");
    assert_eq!(fs::read(&path).unwrap(), original, "acknowledged bytes changed");
    assert_eq!(value(&fixture.store().load(&first.id).unwrap()), value(&first));
}

#[test]
fn identical_payload_repeated_save_is_still_a_creation_conflict() {
    let fixture = Fixture::new("identical");
    let store = fixture.store();
    let original = result("br-same-identity", "same-payload", 0);
    let path = store.save(&original).unwrap();
    let first_bytes = fs::read(&path).unwrap();
    let attempted = store.save(&original);
    assert!(matches!(attempted, Err(LabError::AlreadyExists(_))),
        "save does not expose an update/idempotent-upsert contract: {attempted:?}");
    assert_eq!(fs::read(&path).unwrap(), first_bytes);
}

#[test]
fn distinct_results_reopen_without_changing_checkpoint_session_or_sibling() {
    let fixture = Fixture::new("distinct");
    let checkpoint = fixture.0.join("checkpoint.snapshot.json");
    let session = fixture.0.join("session.json");
    fs::write(&checkpoint, b"checkpoint bytes outside the branch store").unwrap();
    fs::write(&session, b"session bytes outside the branch store").unwrap();
    let store = fixture.store();
    let sibling = result("br-earlier-sibling", "earlier", 2);
    let sibling_path = store.save(&sibling).unwrap();
    let protected: Vec<_> = [&checkpoint, &session, &sibling_path].into_iter()
        .map(|path| (path.to_path_buf(), fs::read(path).unwrap())).collect();
    let first = result("br-1791452031-0000", "legacy-shaped-id", 0);
    let second = result("br-opaque-new-identity", "new-identity", 1);
    store.save(&first).unwrap();
    store.save(&second).unwrap();
    drop(store);
    let reopened = fixture.store();
    assert_eq!(value(&reopened.load(&first.id).unwrap()), value(&first));
    assert_eq!(value(&reopened.load(&second.id).unwrap()), value(&second));
    assert_eq!(reopened.list().unwrap().len(), 3);
    for (path, before) in protected { assert_eq!(fs::read(path).unwrap(), before); }
}

struct ManagedChild(Option<Child>);
impl ManagedChild {
    fn wait(&mut self) -> std::process::Output {
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn child(root: &Path, member: &str, operation: &str) -> ManagedChild {
    ManagedChild(Some(Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "subprocess_branch_store_worker", "--nocapture"])
        .env("UMA_BRANCH_RECEIVING_ROOT", root)
        .env("UMA_BRANCH_RECEIVING_MEMBER", member)
        .env("UMA_BRANCH_RECEIVING_OPERATION", operation)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("spawn independent domain receiver")))
}

#[test]
fn competing_processes_acknowledge_exactly_one_complete_result() {
    let fixture = Fixture::new("processes");
    let mut children: Vec<_> = (0..4).map(|i| child(&fixture.0, &i.to_string(), "publish")).collect();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !(0..4).all(|i| fixture.0.join(format!("ready-{i}")).exists()) {
        assert!(Instant::now() < deadline, "children did not reach the publication barrier");
        thread::sleep(Duration::from_millis(2));
    }
    fs::write(fixture.0.join("release"), b"release all independent publishers").unwrap();
    let mut reports = Vec::new();
    for (i, process) in children.iter_mut().enumerate() {
        let output = process.wait();
        assert!(output.status.success(), "child failed: {}", String::from_utf8_lossy(&output.stderr));
        let report: Value = serde_json::from_slice(&fs::read(fixture.0.join(format!("outcome-{i}.json"))).unwrap()).unwrap();
        reports.push(report);
    }
    let winners: Vec<_> = reports.iter().enumerate().filter(|(_, r)| r["result"] == "created").collect();
    assert_eq!(winners.len(), 1, "one destination can acknowledge only one creator: {reports:?}");
    assert_eq!(reports.iter().filter(|r| r["result"] == "conflict").count(), 3);
    let winner = result("br-forced-process-collision", &format!("publisher-{}", winners[0].0), winners[0].0 as u64);
    let path = fixture.0.join("lab").join(format!("{}.json", winner.id));
    assert_eq!(fs::read(&path).unwrap(), bytes(&winner), "winner was partially written or replaced");
    assert_eq!(value(&fixture.store().load(&winner.id).unwrap()), value(&winner));
    assert_eq!(fs::read_dir(fixture.0.join("lab")).unwrap().count(), 1, "publication left another final or temporary file");

    let output = child(&fixture.0, "restart", "read").wait();
    assert!(output.status.success(), "fresh reader failed: {}", String::from_utf8_lossy(&output.stderr));
    let restarted: Value = serde_json::from_slice(&fs::read(fixture.0.join("restart-readback.json")).unwrap()).unwrap();
    assert_eq!(restarted["branch"], value(&winner), "fresh process did not read the acknowledged winner");
    assert_ne!(restarted["pid"], json!(std::process::id()));
    println!("RECEIVING_PROCESSES {}", json!({"publishers":reports,"readerPid":restarted["pid"],"winner":winners[0].0}));
}

#[cfg(unix)]
#[test]
fn dangling_destination_is_an_existing_identity_and_is_preserved() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new("dangling");
    let store = fixture.store();
    fs::create_dir_all(store.dir()).unwrap();
    let missing = fixture.0.join("absent-target");
    let destination = store.dir().join("br-dangling.json");
    symlink(&missing, &destination).unwrap();
    assert!(!destination.exists(), "fixture must expose exists-versus-directory-entry distinction");
    let attempted = store.save(&result("br-dangling", "must-not-replace-link", 0));
    assert!(matches!(attempted, Err(LabError::AlreadyExists(_))),
        "the destination entry must atomically prevent creation: {attempted:?}");
    assert!(fs::symlink_metadata(&destination).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_link(&destination).unwrap(), missing);
    assert!(!missing.exists());
    assert_eq!(fs::read_dir(store.dir()).unwrap().count(), 1);
}

#[test]
#[ignore = "invoked only by the bounded independent-process receiver"]
fn subprocess_branch_store_worker() {
    let root = PathBuf::from(std::env::var_os("UMA_BRANCH_RECEIVING_ROOT").expect("receiver root"));
    let member = std::env::var("UMA_BRANCH_RECEIVING_MEMBER").unwrap();
    let operation = std::env::var("UMA_BRANCH_RECEIVING_OPERATION").unwrap();
    let store = BranchStore::at(root.join("lab"));
    if operation == "read" {
        let loaded = store.load("br-forced-process-collision").expect("fresh process loads the result");
        fs::write(root.join("restart-readback.json"), serde_json::to_vec(&json!({
            "pid":std::process::id(),"branch":value(&loaded)
        })).unwrap()).unwrap();
        return;
    }
    assert_eq!(operation, "publish");
    let marker: u64 = member.parse().unwrap();
    let publication = result("br-forced-process-collision", &format!("publisher-{member}"), marker);
    fs::write(root.join(format!("ready-{member}")), b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join("release").exists() {
        assert!(Instant::now() < deadline, "parent never released the publication barrier");
        thread::sleep(Duration::from_millis(2));
    }
    let status = match store.save(&publication) {
        Ok(_) => "created",
        Err(LabError::AlreadyExists(_)) => "conflict",
        Err(error) => panic!("unexpected publication error: {error}"),
    };
    fs::write(root.join(format!("outcome-{member}.json")), serde_json::to_vec(&json!({
        "pid":std::process::id(),"member":member,"result":status
    })).unwrap()).unwrap();
}
