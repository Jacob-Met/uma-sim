//! Event pool gating: support by deck, scenario by active scenario.

use uma_sim_core::catalog::event::{
    event_eligible, scenario_event_owner, EventCatalog, FileEventCatalog,
};
use uma_sim_core::detect_repo_root;
use uma_sim_core::events::SimEventEntry;
use uma_sim_core::rng::SimRandom;

fn sample(kind: &str, name: &str) -> SimEventEntry {
    SimEventEntry {
        id: format!("event:{kind}:{name}:x"),
        title: "Sample".into(),
        owner_kind: kind.into(),
        owner_name: name.into(),
        options: vec!["Energy +1".into()],
    }
}

#[test]
fn scenario_owner_maps_engine_ids() {
    assert_eq!(scenario_event_owner("ura"), Some("URA Finale"));
    assert_eq!(scenario_event_owner("grand_concert"), Some("Grand Live"));
    assert_eq!(scenario_event_owner("trackblazer"), Some("Trackblazer"));
    assert_eq!(scenario_event_owner("unity"), None);
}

#[test]
fn support_events_require_deck_membership() {
    let ev = sample("support", "Admire Vega");
    assert!(!event_eligible(&ev, "Special Week", "ura", &[]));
    assert!(event_eligible(
        &ev,
        "Special Week",
        "ura",
        &["Admire Vega".into()]
    ));
}

#[test]
fn scenario_events_match_active_scenario_only() {
    let ura = sample("scenario", "URA Finale");
    let gl = sample("scenario", "Grand Live");
    assert!(event_eligible(&ura, "Special Week", "ura", &[]));
    assert!(!event_eligible(&ura, "Special Week", "grand_concert", &[]));
    assert!(event_eligible(&gl, "Special Week", "grand_concert", &[]));
    assert!(!event_eligible(&gl, "Special Week", "ura", &[]));
}

#[test]
fn file_catalog_ura_pool_excludes_other_scenarios_and_undecked_supports() {
    let root = detect_repo_root().expect("repo root");
    let cat = FileEventCatalog::load(&FileEventCatalog::default_path(&root));
    assert!(cat.event_count() > 100);
    let mut rng = SimRandom::with_trace(1, false);
    for _ in 0..40 {
        let ev = cat
            .pick_random("Special Week", "ura", &[], 10, &mut rng)
            .expect("pool non-empty");
        assert_ne!(ev.owner_kind.to_lowercase(), "support");
        if ev.owner_kind.eq_ignore_ascii_case("scenario") {
            assert!(ev.owner_name.eq_ignore_ascii_case("URA Finale"));
        }
    }
    let mut rng = SimRandom::with_trace(2, false);
    let with_deck = cat
        .pick_random("Special Week", "ura", &["Admire Vega".into()], 10, &mut rng)
        .expect("pool");
    // With deck present, support events are eligible; may or may not be picked.
    let _ = with_deck;
}
