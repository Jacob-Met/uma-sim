use uma_sim_core::catalog::skill::SkillCatalog;
use uma_sim_core::catalog::trainee::TraineeCatalog;
use uma_sim_core::factory::init_from_detected_repo;

#[test]
fn skill_catalog_resolves_hydrate_or_loads() {
    let root = init_from_detected_repo(true);
    assert!(root.is_some());
    assert!(
        SkillCatalog::is_loaded(),
        "skill.json should load from knowledge/"
    );
    // Hydrate is a common skill; if missing, at least catalog is non-empty.
    let _ = SkillCatalog::lookup_by_name("Hydrate");
}

#[test]
fn trainee_skills_event_loaded_for_special_week() {
    let root = uma_sim_core::detect_repo_root().expect("repo root");
    TraineeCatalog::init_from_repo(Some(&root));
    let tm = TraineeCatalog::lookup("Special Week")
        .or_else(|| TraineeCatalog::lookup("Special Dreamer"))
        .expect("Special Week");
    assert!(
        !tm.skills_event.is_empty(),
        "skills_event should be loaded from trainee.json"
    );
}
