//! Port of KnowledgeValidateTest.kt

use std::process::Command;
use uma_sim_core::detect_repo_root;

#[test]
fn canonical_knowledge_base_passes_validate_script() {
    let root = detect_repo_root().expect("repo root required for KB validate");
    let script = root.join("knowledge/validate/validate.py");
    assert!(script.exists(), "validate.py missing");
    // Prefer `python3`: most Linux/macOS machines (and this repo's CI) only
    // provide a python3 binary; fall back to bare `python` for Windows/venv
    // setups that ship only that name.
    let output = ["python3", "python"]
        .iter()
        .find_map(|bin| {
            Command::new(bin)
                .arg(&script)
                .current_dir(&root)
                .output()
                .ok()
        })
        .expect("failed to run validate.py: no python3/python interpreter found on PATH");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(output.status.success(), "KB validate failed:\n{combined}");
    assert!(combined.contains("OK"), "{combined}");
}
