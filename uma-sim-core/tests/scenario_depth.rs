//! Port of Kotlin `ScenarioDepthTest.kt` (Gate R4).

use std::collections::HashMap;
use std::sync::Mutex;

use uma_sim_core::{
    CareerState, GrandConcertScenarioPlugin, MoodLevel, RunMeta, ScenarioPlugin, ScenarioResources,
    SimDate, TraineeStats, TrainingFacility, TurnPhase, UnityCupScenarioPlugin, UraMechanics,
    UraScenarioPlugin,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn gl_state(turn: i32, date: SimDate, resources: ScenarioResources) -> CareerState {
    CareerState {
        meta: RunMeta::new(1, "grand_concert", "Test"),
        date,
        turn,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 100,
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
fn grand_concert_promo_mandatory_at_turn_36() {
    let plugin = GrandConcertScenarioPlugin;
    let state = gl_state(
        36,
        SimDate {
            year: 2,
            month: 12,
            half: 1,
        },
        ScenarioResources::new(),
    );
    let (next, _) = plugin.on_turn_start(&state);
    assert_eq!(next.phase, TurnPhase::MandatoryRace.as_str());
    assert_eq!(next.pending_race_id.as_deref(), Some("promo_2"));
}

#[test]
fn happy_meek_badge_rolls_on_turn_start() {
    let _g = TEST_LOCK.lock().unwrap();
    UraMechanics::load_research(Some(
        r#"{"happy_meek":{"spawn_chance_per_turn":1.0,"min_turn":1}}"#,
    ));
    let plugin = UraScenarioPlugin::new();
    let state = CareerState {
        meta: RunMeta::new(42, "ura", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 10,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 100,
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
        scenario_resources: ScenarioResources::new(),
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    };
    let (next, lines) = plugin.on_turn_start(&state);
    let badge = next.scenario_resources.get("happy_meek_badge");
    assert!((1..=5).contains(&badge));
    assert!(lines.iter().any(|l| l.contains("duel badge")));
}

#[test]
fn unity_extreme_burst_consumes_ready_flag() {
    let plugin = UnityCupScenarioPlugin::default();
    let state = CareerState {
        meta: RunMeta::new(1, "unity", "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 5,
        stats: TraineeStats {
            speed: 100,
            stamina: 100,
            power: 100,
            guts: 100,
            wit: 100,
        },
        energy: 100,
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
        scenario_resources: ScenarioResources::from_map(HashMap::from([(
            "unity_extreme_ready".into(),
            1,
        )])),
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    };
    let (after, lines) = plugin.on_training_complete(&state, TrainingFacility::Speed, true);
    assert!(lines.iter().any(|l| l.contains("Extreme")));
    assert_eq!(after.scenario_resources.get("unity_extreme_ready"), 0);
}

#[test]
fn grand_concert_perfect_when_18_songs_and_great_success() {
    let plugin = GrandConcertScenarioPlugin;
    let mut values: HashMap<String, i32> =
        (1..=18).map(|i| (format!("song_owned:{i}"), 1)).collect();
    values.insert("songs_learned".into(), 18);
    values.insert("hype".into(), 3);
    values.insert("great_success_required".into(), 3);
    let state = gl_state(
        72,
        SimDate {
            year: 3,
            month: 12,
            half: 2,
        },
        ScenarioResources::from_map(values),
    );
    let (after, lines) = plugin.on_race_complete(&state, "grand_concert", true);
    assert!(lines
        .iter()
        .any(|l| l.contains("Great Success") || l.contains("I Wanna Win")));
    assert_eq!(after.scenario_resources.get("grand_live_perfect"), 1);
}
