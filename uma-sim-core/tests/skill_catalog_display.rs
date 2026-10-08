use serde_json::json;
use uma_sim_core::catalog::skill::SkillCatalog;

#[test]
fn display_projection_preserves_name_lookup_and_failed_load_semantics() {
    SkillCatalog::clear_for_test();
    let fixture = json!([
        {
            "id": "skill:30",
            "name_ja": "日本語の名前",
            "payload": { "skill_id": 30, "desc_en": null }
        },
        {
            "id": "skill:10",
            "name_en_official": "  Official name  ",
            "name_en_fan": "Fan name",
            "aliases": ["Existing alias"],
            "payload": { "skill_id": 10, "desc_en": "  Catalog text <literal>.  " }
        },
        {
            "id": "skill:20",
            "name_en_official": " ",
            "name_en_fan": "Fan fallback",
            "payload": {}
        },
        { "id": "skill:40", "payload": {} },
        {
            "id": "skill:10",
            "name_en_official": "Later duplicate",
            "payload": { "skill_id": 10, "desc_en": "Do not replace first." }
        },
        null,
        { "id": "not-a-skill", "name_en_official": "Ignored" }
    ]);
    assert!(SkillCatalog::load_json(&fixture.to_string()));
    let rows = SkillCatalog::list_all();
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["skill:10", "skill:20", "skill:30", "skill:40"]
    );
    assert_eq!(rows[0].name.as_deref(), Some("Official name"));
    assert_eq!(
        rows[0].description.as_deref(),
        Some("Catalog text <literal>.")
    );
    assert_eq!(rows[1].name.as_deref(), Some("Fan fallback"));
    assert_eq!(rows[2].name.as_deref(), Some("日本語の名前"));
    assert_eq!(rows[3].name, None);
    assert_eq!(rows[3].description, None);
    assert_eq!(SkillCatalog::lookup_by_name("Existing alias"), Some(10));
    assert_eq!(SkillCatalog::lookup_by_name("official NAME ○"), Some(10));
    assert_eq!(SkillCatalog::lookup_by_name("Later duplicate"), Some(10));

    let before = serde_json::to_value(&rows).unwrap();
    assert!(!SkillCatalog::load_json("malformed JSON"));
    assert!(!SkillCatalog::load_json("{}"));
    assert_eq!(
        serde_json::to_value(SkillCatalog::list_all()).unwrap(),
        before
    );
    assert_eq!(SkillCatalog::lookup_by_name("Existing alias"), Some(10));

    let mut caller_copy = SkillCatalog::list_all();
    caller_copy[0].name = Some("Caller edit".into());
    assert_eq!(
        SkillCatalog::list_all()[0].name.as_deref(),
        Some("Official name")
    );
    SkillCatalog::clear_for_test();
    assert!(SkillCatalog::list_all().is_empty());
    assert_eq!(SkillCatalog::lookup_by_name("Existing alias"), None);
}
