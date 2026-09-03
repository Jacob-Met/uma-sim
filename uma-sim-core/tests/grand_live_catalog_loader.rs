//! Port of JVM `GrandLiveCatalogLoaderTest.kt`.

use uma_sim_core::{detect_repo_root, GrandLiveCatalog, GrandLiveCatalogLoader};

#[test]
fn loads_songs_from_knowledge_base() {
    let Some(root) = detect_repo_root() else {
        return;
    };
    if !root
        .join("knowledge/canonical/by_kind/song.json")
        .is_file()
    {
        return;
    }
    GrandLiveCatalogLoader::init_from_repo(Some(&root));
    assert!(GrandLiveCatalog::loaded());
    assert!(
        GrandLiveCatalog::all_songs().len() >= 20,
        "Expected 20+ purchasable songs from song.json"
    );
    assert!(
        GrandLiveCatalog::all_techniques().len() >= 100,
        "Expected 100+ techniques from lesson.json"
    );
}
