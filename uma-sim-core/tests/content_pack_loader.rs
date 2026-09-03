//! Port of ContentPackLoaderTest.kt

use std::path::PathBuf;
use uma_sim_core::{detect_repo_root, ContentPackLoader};

#[test]
fn example_pack_has_events() {
    let root = detect_repo_root().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    });
    let path = root.join("content_packs/example.json");
    if !path.exists() {
        return;
    }
    let events = ContentPackLoader::load_pack_file(&path);
    assert!(!events.is_empty(), "example.json should contain at least one event");
}
