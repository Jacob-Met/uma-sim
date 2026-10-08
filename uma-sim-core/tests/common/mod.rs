//! Shared helpers for Kotlin → Rust test ports.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use uma_sim_core::state::{
    default_facility_levels, CareerState, DeckState, LegacyState, MoodLevel, RunMeta,
    ScenarioResources, SimDate, TraineeStats, TurnPhase,
};

/// Serialize tests that mutate process-global research/config/lookup state.
pub fn config_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub const SCENARIOS: [&str; 4] = ["ura", "grand_concert", "unity", "trackblazer"];

pub const KT_RES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/kotlin");

pub fn stats(v: i32) -> TraineeStats {
    TraineeStats {
        speed: v,
        stamina: v,
        power: v,
        guts: v,
        wit: v,
    }
}

pub fn stats5(speed: i32, stamina: i32, power: i32, guts: i32, wit: i32) -> TraineeStats {
    TraineeStats {
        speed,
        stamina,
        power,
        guts,
        wit,
    }
}

pub fn base_state(scenario: &str, turn: i32) -> CareerState {
    CareerState {
        meta: RunMeta::new(1, scenario, "Test"),
        date: SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn,
        stats: stats(100),
        energy: 100,
        max_energy: 100,
        mood: MoodLevel::Normal,
        fans: 0,
        skill_points: 0,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: TurnPhase::Free.as_str().to_string(),
        completed_races: Vec::new(),
        facility_levels: default_facility_levels(),
        facility_train_counts: HashMap::new(),
        pending_event_options: Vec::new(),
        hint_levels: HashMap::new(),
        statuses: Vec::new(),
        performance_tokens: HashMap::new(),
        scenario_resources: ScenarioResources::new(),
        legacy: LegacyState::default(),
        learned_skill_ids: Vec::new(),
        deck: DeckState::default(),
        log: Vec::new(),
        generated_sparks: Vec::new(),
        base_aptitudes: Default::default(),
        preferred_running_style: None,
    }
}

pub fn with_resources(mut state: CareerState, values: HashMap<String, i32>) -> CareerState {
    state.scenario_resources = ScenarioResources::from_map(values);
    state
}

pub fn with_deck(mut state: CareerState, deck: DeckState) -> CareerState {
    state.deck = deck;
    state
}

pub fn with_fans(mut state: CareerState, fans: i32) -> CareerState {
    state.fans = fans;
    state
}

pub fn with_phase(mut state: CareerState, phase: TurnPhase) -> CareerState {
    state.phase = phase.as_str().to_string();
    state
}

pub fn settings_fast(speed: i32) -> uma_sim_core::SimSettings {
    uma_sim_core::SimSettings {
        speed_multiplier: speed,
        ..Default::default()
    }
}
