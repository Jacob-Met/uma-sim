//! Port of SupportCatalogTest.kt

use uma_sim_core::{detect_repo_root, SupportCatalog};

#[test]
fn loads_special_week_from_kb_when_available() {
    let Some(root) = detect_repo_root() else {
        return;
    };
    SupportCatalog::init_from_repo(Some(&root));
    let card =
        SupportCatalog::lookup("support:10001").or_else(|| SupportCatalog::lookup("Special Week"));
    if let Some(card) = card {
        assert!(card.friendship_bonus_pct > 0.0);
        assert!(!card.card_type.is_empty());
    }
}
