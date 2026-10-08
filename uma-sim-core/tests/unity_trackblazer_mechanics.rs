//! Port of Kotlin `UnityTrackblazerMechanicsTest.kt` (Gate R4).

use std::collections::HashMap;
use std::sync::Mutex;

use uma_sim_core::{
    CareerState, MoodLevel, RunMeta, ScenarioPlugin, ScenarioResources, SimDate,
    TrackblazerMechanics, TrackblazerScenarioPlugin, TraineeStats, TrainingFacility,
    UnityCupMechanics, UnityCupScenarioPlugin,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn career(
    seed: i64,
    scenario: &str,
    turn: i32,
    resources: ScenarioResources,
    stats: TraineeStats,
) -> CareerState {
    CareerState {
        meta: RunMeta::new(seed, scenario, "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn,
        stats,
        energy: 80,
        max_energy: 100,
        mood: MoodLevel::Normal,
        fans: 0,
        skill_points: 0,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: String::new(),
        completed_races: Vec::new(),
        facility_levels: HashMap::new(),
        facility_train_counts: HashMap::new(),
        pending_event_options: Vec::new(),
        hint_levels: HashMap::new(),
        statuses: Vec::new(),
        performance_tokens: HashMap::new(),
        scenario_resources: resources,
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    }
}

#[test]
fn unity_spirit_burst_at_research_threshold() {
    let _g = TEST_LOCK.lock().unwrap();
    UnityCupMechanics::load_research(Some(
        r#"{"spirit_gauge":{"burst_threshold":100,"extreme_burst_threshold":150,"gain_per_training_success":{"base":15,"per_facility_level":3}}}"#,
    ));
    let (res, lines, _) = UnityCupMechanics::apply_training_spirit(
        &ScenarioResources::from_map(HashMap::from([("unity_spirit".into(), 90)])),
        15,
        TrainingFacility::Speed,
    );
    assert_eq!(res.get("unity_burst_ready"), 1);
    assert!(lines.iter().any(|l| l.contains("Spirit Burst")));
}

#[test]
fn trackblazer_shop_opens_on_interval() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop":{"interval_turns":6,"min_coins_to_open":50}}"#,
    ));
    assert!(TrackblazerMechanics::should_open_shop(6, 60));
    assert!(!TrackblazerMechanics::should_open_shop(6, 30));
    assert!(!TrackblazerMechanics::should_open_shop(5, 60));
}

#[test]
fn trackblazer_climax_pays_more_coins() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop":{"coins_per_optional_race":40,"coins_per_climax_race":80}}"#,
    ));
    assert_eq!(TrackblazerMechanics::race_coin_gain("optional"), 40);
    assert_eq!(TrackblazerMechanics::race_coin_gain("climax_1"), 80);
}

#[test]
fn trackblazer_shop_purchase_deducts_coins_and_applies_stats() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop_items":[{"id":"speed_charm","name":"Speed Charm","cost":50,"effect":"Speed +20"}]}"#,
    ));
    let state = career(
        1,
        "trackblazer",
        6,
        ScenarioResources::from_map(HashMap::from([("tb_coins".into(), 60)])),
        TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
    );
    let option = TrackblazerMechanics::format_option(
        &TrackblazerMechanics::shop_catalog()
            .into_iter()
            .find(|it| it.id == "speed_charm")
            .expect("speed_charm"),
    );
    let (after_purchase, lines) = TrackblazerMechanics::apply_purchase(&state, &option);
    assert_eq!(after_purchase.scenario_resources.get("tb_coins"), 10);
    assert!(lines.iter().any(|l| l.contains("Coins")));
}

#[test]
fn trackblazer_megaphone_training_multiplier() {
    let _g = TEST_LOCK.lock().unwrap();
    let res = ScenarioResources::from_map(HashMap::from([
        ("tb_training_bonus_pct".into(), 20),
        ("tb_training_bonus_turns".into(), 2),
    ]));
    assert_eq!(TrackblazerMechanics::training_stat_multiplier(&res), 1.2);
}

#[test]
fn trackblazer_shop_roll_is_deterministic() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop":{"offers_per_shop":2},"shop_items":[
              {"id":"a","name":"Item A","cost":30,"effect":"Speed +10"},
              {"id":"b","name":"Item B","cost":40,"effect":"Stamina +10"},
              {"id":"c","name":"Item C","cost":50,"effect":"Power +10"}
            ]}"#,
    ));
    let state = career(
        99,
        "trackblazer",
        12,
        ScenarioResources::from_map(HashMap::from([("tb_coins".into(), 100)])),
        TraineeStats::default(),
    );
    let first = TrackblazerMechanics::roll_shop_options(&state);
    let second = TrackblazerMechanics::roll_shop_options(&state);
    assert_eq!(first, second);
    assert!(first.last().is_some_and(|s| s.starts_with("Skip")));
}

#[test]
fn unity_team_rank_maps_to_facility_level() {
    let _g = TEST_LOCK.lock().unwrap();
    UnityCupMechanics::load_research(Some(
        r#"{"team_rank_facility_levels":{"min_rank":1,"max_rank":5}}"#,
    ));
    let mut res = UnityCupMechanics::initial_resources();
    assert_eq!(
        UnityCupMechanics::facility_level_for(&res, TrainingFacility::Speed),
        1
    );
    res = res.set("unity_rank_speed", 4);
    assert_eq!(
        UnityCupMechanics::facility_level_for(&res, TrainingFacility::Speed),
        4
    );
    assert_eq!(UnityCupMechanics::rank_label(4), "A");
}

#[test]
fn unity_spirit_burst_bumps_team_rank() {
    let _g = TEST_LOCK.lock().unwrap();
    UnityCupMechanics::load_research(Some(
        r#"{"spirit_gauge":{"burst_threshold":100,"extreme_burst_threshold":150}}"#,
    ));
    let (res, lines, rank_up) = UnityCupMechanics::apply_training_spirit(
        &ScenarioResources::from_map(HashMap::from([
            ("unity_spirit".into(), 90),
            ("unity_rank_speed".into(), 2),
        ])),
        15,
        TrainingFacility::Speed,
    );
    assert!(rank_up);
    assert_eq!(res.get("unity_rank_speed"), 3);
    assert!(lines.iter().any(|l| l.contains("team rank")));
}

#[test]
fn unity_plugin_syncs_facility_levels_from_rank() {
    let _g = TEST_LOCK.lock().unwrap();
    let plugin = UnityCupScenarioPlugin::default();
    let init = plugin.initial_scenario_resources(&RunMeta::new(1, "unity", "Test"));
    let state = CareerState {
        energy: 100,
        max_energy: 100,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        turn: 5,
        ..career(1, "unity", 5, init.clone(), TraineeStats::default())
    };
    let (after_turn, _) = plugin.on_turn_start(&state);
    assert_eq!(after_turn.facility_levels.get("speed").copied(), Some(1));
    assert_eq!(
        plugin.effective_facility_level(
            &state.with_resources(init.set("unity_rank_stamina", 4)),
            TrainingFacility::Stamina,
        ),
        Some(4)
    );
}

#[test]
fn trackblazer_climax_awards_victory_points() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"climax":{"victory_points_per_win":100},"shop":{"coins_per_climax_race":80}}"#,
    ));
    assert_eq!(
        TrackblazerMechanics::climax_victory_points("climax_1", true),
        100
    );
    assert_eq!(
        TrackblazerMechanics::climax_victory_points("optional", true),
        0
    );
    let plugin = TrackblazerScenarioPlugin::default();
    let state = career(
        1,
        "trackblazer",
        69,
        ScenarioResources::from_map(HashMap::from([("tb_coins".into(), 0)])),
        TraineeStats::default(),
    );
    let (after, lines) = plugin.on_race_complete(&state, "climax_1", true);
    assert_eq!(after.scenario_resources.get("tb_victory_points"), 100);
    assert_eq!(after.scenario_resources.get("tb_coins"), 80);
    assert!(lines.iter().any(|l| l.contains("Victory Points")));
}

#[test]
fn trackblazer_shop_sale_reduces_purchase_cost() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop":{"interval_turns":6,"sale_discount_pct":20,"sale_every_n_shops":1},"shop_items":[{"id":"speed_charm","name":"Speed Charm","cost":50,"effect":"Speed +20"}]}"#,
    ));
    assert!(TrackblazerMechanics::is_sale_turn(12));
    assert_eq!(TrackblazerMechanics::effective_cost(50, 12), 40);
    let state = career(
        1,
        "trackblazer",
        12,
        ScenarioResources::from_map(HashMap::from([("tb_coins".into(), 50)])),
        TraineeStats {
            speed: 100,
            ..TraineeStats::default()
        },
    );
    let option = TrackblazerMechanics::format_option(
        &TrackblazerMechanics::shop_catalog()
            .into_iter()
            .find(|it| it.id == "speed_charm")
            .expect("speed_charm"),
    );
    let (after, lines) = TrackblazerMechanics::apply_purchase(&state, &option);
    assert_eq!(after.scenario_resources.get("tb_coins"), 10);
    assert!(lines.iter().any(|l| l.contains("sale")));
}

#[test]
fn trackblazer_speed_charm_effects_raise_speed() {
    let _g = TEST_LOCK.lock().unwrap();
    TrackblazerMechanics::load_research(Some(
        r#"{"shop_items":[{"id":"speed_charm","name":"Speed Charm","cost":50,"effect":"Speed +20"}]}"#,
    ));
    let state = career(
        1,
        "trackblazer",
        6,
        ScenarioResources::from_map(HashMap::from([("tb_coins".into(), 60)])),
        TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
    );
    let option = TrackblazerMechanics::format_option(
        &TrackblazerMechanics::shop_catalog()
            .into_iter()
            .find(|it| it.id == "speed_charm")
            .expect("speed_charm"),
    );
    let (after_buy, _) = TrackblazerMechanics::apply_purchase(&state, &option);
    let mut rng = uma_sim_core::SimRandom::new(1);
    let (after_fx, _) = TrackblazerMechanics::apply_item_effects(&after_buy, &option, &mut rng);
    assert_eq!(after_fx.stats.speed, 120);
}

#[test]
fn unity_team_race_win_counts_five_legs_and_zenith() {
    let _g = TEST_LOCK.lock().unwrap();
    let plugin = UnityCupScenarioPlugin::default();
    let mut state = career(
        1,
        "unity",
        24,
        UnityCupMechanics::initial_resources(),
        TraineeStats::default(),
    );
    for race in [
        "unity_preseason",
        "unity_team_2",
        "unity_team_3",
        "unity_team_4",
    ] {
        let (after, lines) = plugin.on_race_complete(&state, race, true);
        state = after;
        assert!(lines.iter().any(|l| l.contains("legs")));
    }
    assert_eq!(state.scenario_resources.get("unity_legs_won"), 20);
    assert_eq!(state.scenario_resources.get("unity_zenith_upgraded"), 1);
}

#[test]
fn trackblazer_rival_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/trackblazer.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let nm = v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap();
    assert!(nm.iter().any(|x| x.as_str() == Some("rival encounters")));
}

#[test]
fn trackblazer_epithet_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/trackblazer.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str() == Some("epithet system")));
}

#[test]
fn trackblazer_inventory_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/trackblazer.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str() == Some("inventory quick-use")));
}

#[test]
fn trackblazer_climax2_gap_documented() {
    let plugin = TrackblazerScenarioPlugin::default();
    let ids: Vec<_> = plugin
        .mandatory_races()
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert!(ids.contains(&"climax_1"));
    assert!(ids.contains(&"climax_3"));
    assert!(!ids.contains(&"climax_2"));
}

#[test]
fn unity_extreme_burst_grants_ignited_spirit_hint() {
    let _g = TEST_LOCK.lock().unwrap();
    let state = career(
        1,
        "unity",
        20,
        ScenarioResources::from_map(HashMap::from([
            ("unity_extreme_ready".into(), 1),
            ("unity_burst_count".into(), 3),
        ])),
        TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
    );
    let (after, lines) = UnityCupMechanics::consume_extreme_burst(&state, TrainingFacility::Speed);
    assert!(lines.iter().any(|l| l.contains("Ignited Spirit")));
    assert_eq!(
        after.hint_levels.get("ignited_spirit_speed").copied(),
        Some(1)
    );
}

#[test]
fn unity_per_leg_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/unity_cup.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str() == Some("per-leg team race simulation")));
}

#[test]
fn unity_teammate_share_gap_documented() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../research/unity_cup.json"
    ))
    .expect("research");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(v["sim_implementation_status"]["not_modeled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.as_str() == Some("teammate stat sharing")));
}
