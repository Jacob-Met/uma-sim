//! URA retained participation policy. Real public hooks and the production physics adapter.
use uma_sim_core::race::{course_id_for_race, derive_race_seed, run_physics_race};
use uma_sim_core::{
    CareerState, RunMeta, ScenarioPlugin, ScenarioResources, SimDate, SimEngine, SimSettings,
    UraScenarioPlugin,
};

const PREFIX: &str = "ura_finale_";
const VERSION: &str = "ura_finale_history_version";
const COMPLETE: &str = "ura_finale_history_complete";
const FROZEN: &str = "ura_finale_frozen_class";
const SPRINT: &str = "ura_finale_turf_sprint";
const STAGES: [&str; 3] = ["finale_qualifier", "finale_semifinal", "finale_finals"];

fn fresh() -> CareerState {
    let mut engine = SimEngine::new(SimSettings::default());
    engine.start(RunMeta::new(712, "ura", "Synthetic history"));
    engine.state().clone()
}
fn record(mut state: CareerState, race: &str, won: bool, n: usize) -> CareerState {
    let plugin = UraScenarioPlugin::new();
    for _ in 0..n {
        state = plugin.on_race_complete(&state, race, won).0;
    }
    state
}
fn boundary(mut state: CareerState, stage: usize) -> CareerState {
    state.date = SimDate {
        year: 3,
        month: if stage == 2 { 12 } else { 11 },
        half: if stage == 1 { 2 } else { 1 },
    };
    state.turn = 68 + stage as i32;
    state.pending_race_id = None;
    let (state, _) = UraScenarioPlugin::new().on_turn_start(&state);
    assert_eq!(state.pending_race_id.as_deref(), Some(STAGES[stage]));
    state
}
fn observed(state: &CareerState, stage: usize) -> u32 {
    let outcome = run_physics_race(state, STAGES[stage]);
    assert_eq!(
        outcome.seed,
        derive_race_seed(state.meta.seed, state.turn, STAGES[stage])
    );
    outcome.course_id
}
fn assert_legacy(state: &CareerState) {
    for (stage, expected) in [10602, 10606, 10608].into_iter().enumerate() {
        assert_eq!(observed(state, stage), expected);
    }
}

#[test]
fn initialization_is_new_ura_only_and_restore_does_not_reconstruct_history() {
    let state = fresh();
    assert_eq!(state.scenario_resources.get(VERSION), 1);
    assert_eq!(state.scenario_resources.get(COMPLETE), 1);
    assert_eq!(
        state
            .scenario_resources
            .values
            .keys()
            .filter(|k| k.starts_with(PREFIX))
            .count(),
        10
    );
    let mut other = SimEngine::new(SimSettings::default());
    other.start(RunMeta::new(712, "unity", "Control"));
    assert!(!other
        .state()
        .scenario_resources
        .values
        .keys()
        .any(|k| k.starts_with(PREFIX)));
    let mut engine = SimEngine::new(SimSettings::default());
    engine.start(RunMeta::new(712, "ura", "Legacy"));
    let mut save = engine.export();
    save.state.scenario_resources = ScenarioResources::new();
    save.state.completed_races = vec!["debut".into(), "optional".into(), "1001".into()];
    engine.restore(save);
    let legacy = record(engine.state().clone(), "optional", true, 3);
    assert!(!legacy.scenario_resources.values.contains_key(VERSION));
    assert_legacy(&boundary(legacy, 0));
}

#[test]
fn seven_native_joint_bins_use_verified_representatives_and_retain_matching_stages() {
    let cases = [
        ("1002", [10601, 10601, 10601], (1, 1)),
        ("race:1004", [10602, 10602, 10602], (1, 2)),
        ("1003", [10604, 10606, 10604], (1, 3)),
        ("1006", [10607, 10607, 10608], (1, 4)),
        ("1104", [10609, 10609, 10609], (2, 1)),
        ("1001", [10611, 10611, 10611], (2, 2)),
        ("1101", [10612, 10612, 10612], (2, 3)),
    ];
    for (race, expected, class) in cases {
        let state = boundary(record(fresh(), race, false, 1), 0);
        for (stage, course) in expected.into_iter().enumerate() {
            assert_eq!(observed(&state, stage), course, "{race} stage {stage}");
            let actual = uma_race_core::get_course(course).unwrap();
            assert_eq!((actual.surface, actual.distance_type), class);
        }
    }
}

#[test]
fn participation_is_joint_occurrence_count_not_surface_marginal_or_wins() {
    let start = fresh();
    let completion = start.completed_races.clone();
    let state = record(
        record(record(start, "1002", true, 3), "1004", true, 3),
        "1104",
        false,
        5,
    );
    assert_eq!(state.completed_races, completion);
    assert_eq!(observed(&boundary(state, 0), 0), 10609);
    let state = record(record(fresh(), "1002", false, 3), "1001", true, 2);
    assert_eq!(observed(&boundary(state, 0), 0), 10601);
}

#[test]
fn ties_prefer_turf_before_shorter_within_each_surface() {
    let cross_surface = record(record(fresh(), "1006", false, 1), "1104", true, 1);
    assert_eq!(observed(&boundary(cross_surface, 0), 0), 10607);
    let turf = record(record(fresh(), "1003", false, 1), "1004", true, 1);
    assert_eq!(observed(&boundary(turf, 0), 0), 10602);
    let dirt = record(record(fresh(), "1101", false, 1), "1001", true, 1);
    assert_eq!(observed(&boundary(dirt, 0), 0), 10611);
}

#[test]
fn symbolic_optional_and_debut_count_as_actual_interim_courses() {
    let state = record(record(fresh(), "optional", false, 2), "debut", true, 1);
    assert_eq!(state.scenario_resources.get(SPRINT), 3);
    assert_eq!(observed(&boundary(state, 0), 0), 10601);
}

#[test]
fn freeze_excludes_finals_and_all_later_callbacks_including_unknowns() {
    let frozen = boundary(record(fresh(), "1001", false, 1), 0);
    let before = frozen.scenario_resources.clone();
    let mut after = frozen.clone();
    for id in [
        "finale_qualifier",
        "finale_semifinal",
        "finale_finals",
        "optional",
        "absent",
    ] {
        after = record(after, id, true, 2);
    }
    assert_eq!(after.scenario_resources, before);
    assert_eq!(observed(&after, 2), 10611);
    let repeated = boundary(after, 0);
    assert_eq!(
        repeated.scenario_resources.get(FROZEN),
        frozen.scenario_resources.get(FROZEN)
    );
}

#[test]
fn unavailable_empty_unknown_and_later_final_states_keep_legacy_courses() {
    assert_legacy(&boundary(fresh(), 0));
    for id in ["absent", "race:race:1001", "1001-extra", ""] {
        let state = record(record(fresh(), "1001", false, 2), id, true, 1);
        let state = record(state, "1001", true, 3);
        assert_legacy(&boundary(state, 0));
    }
    let late = boundary(record(fresh(), "1001", false, 2), 1);
    assert_legacy(&late);
    assert_legacy(&boundary(late, 0));
    let without_boundary = record(
        record(fresh(), "1001", false, 2),
        "finale_qualifier",
        false,
        1,
    );
    assert_legacy(&boundary(without_boundary, 0));
}

#[test]
fn malformed_partial_negative_and_overflow_history_never_selects_partial_majority() {
    let full = record(fresh(), "optional", false, 1);
    let mut mutations = Vec::new();
    let mut missing = full.clone();
    missing
        .scenario_resources
        .values
        .remove("ura_finale_dirt_medium");
    mutations.push(missing);
    for (key, value) in [
        (VERSION, 2),
        (COMPLETE, 0),
        (COMPLETE, 2),
        (SPRINT, -1),
        (FROZEN, 8),
        (FROZEN, 7),
    ] {
        let mut state = full.clone();
        state.scenario_resources = state.scenario_resources.set(key, value);
        mutations.push(state);
    }
    let mut overflow = full.clone();
    overflow.scenario_resources = overflow.scenario_resources.set(SPRINT, i32::MAX);
    mutations.push(record(overflow, "optional", false, 1));
    for state in mutations {
        assert_legacy(&boundary(record(state, "1001", true, 1), 0));
    }
}

#[test]
fn callbacks_change_only_owned_resources_and_preserve_legacy_completion_vector() {
    let mut state = fresh();
    state.completed_races = vec!["debut".into(), "debut".into()];
    state.scenario_resources = state.scenario_resources.set("unrelated_owner", 93);
    let mut after = record(state.clone(), "optional", false, 1);
    assert_eq!(after.scenario_resources.get("unrelated_owner"), 93);
    after.scenario_resources = state.scenario_resources.clone();
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(state).unwrap()
    );
}

#[test]
fn serialized_frozen_state_reopens_without_reselection() {
    let frozen = boundary(record(fresh(), "1003", false, 2), 0);
    let raw = serde_json::to_string(&frozen).unwrap();
    let reopened: CareerState = serde_json::from_str(&raw).unwrap();
    assert_eq!(reopened.scenario_resources, frozen.scenario_resources);
    assert_eq!(observed(&record(reopened, "1001", true, 10), 1), 10606);
}

#[test]
fn context_free_resolver_nonfinal_races_and_other_scenarios_are_unchanged() {
    let state = boundary(record(fresh(), "1001", false, 2), 0);
    for (id, expected) in [
        ("finale_qualifier", 10602),
        ("finale_semifinal", 10606),
        ("finale_finals", 10608),
    ] {
        assert_eq!(course_id_for_race(id), expected);
    }
    for id in ["debut", "optional", "absent", "1001"] {
        assert_eq!(
            run_physics_race(&state, id).course_id,
            course_id_for_race(id)
        );
    }
    let mut other = state;
    other.meta.scenario_id = "unity".into();
    assert_legacy(&other);
    assert_eq!(
        record(other.clone(), "optional", true, 1).scenario_resources,
        other.scenario_resources
    );
}
