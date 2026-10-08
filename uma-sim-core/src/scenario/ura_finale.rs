//! Retained participation classes for the simulator's existing URA finale schedule.
//!
//! This is a documented, community-grounded modeling policy, not a full game course
//! or calendar implementation. Missing/invalid legacy history keeps legacy courses.

use crate::race::{course_id_for_race, known_course_id_for_history};
use crate::state::{CareerState, ScenarioResources};
use uma_race_core::get_course;

const VERSION: &str = "ura_finale_history_version";
const COMPLETE: &str = "ura_finale_history_complete";
const FROZEN: &str = "ura_finale_frozen_class";
const COUNTS: [&str; 7] = [
    "ura_finale_turf_sprint",
    "ura_finale_turf_mile",
    "ura_finale_turf_medium",
    "ura_finale_turf_long",
    "ura_finale_dirt_sprint",
    "ura_finale_dirt_mile",
    "ura_finale_dirt_medium",
];

// Array order is the explicit tie policy: turf first, then shorter within surface.
const CLASSES: [(u8, u8, u32, &str); 7] = [
    (1, 1, 10601, "turf sprint"),
    (1, 2, 10602, "turf mile"),
    (1, 3, 10604, "turf medium"),
    (1, 4, 10607, "turf long"),
    (2, 1, 10609, "dirt sprint"),
    (2, 2, 10611, "dirt mile"),
    (2, 3, 10612, "dirt medium"),
];

struct Tracker {
    counts: [i32; 7],
    frozen: i32,
}

fn winner(counts: &[i32; 7]) -> Option<usize> {
    let mut selected = None;
    for (i, count) in counts.iter().enumerate() {
        if *count > 0 && selected.is_none_or(|old| *count > counts[old]) {
            selected = Some(i);
        }
    }
    selected
}

fn tracker(resources: &ScenarioResources) -> Option<Tracker> {
    if resources.values.get(VERSION) != Some(&1) || resources.values.get(COMPLETE) != Some(&1) {
        return None;
    }
    let frozen = *resources.values.get(FROZEN)?;
    if !(-1..=7).contains(&frozen) {
        return None;
    }
    let mut counts = [0; 7];
    for (i, key) in COUNTS.iter().enumerate() {
        counts[i] = *resources.values.get(*key)?;
        if counts[i] < 0 {
            return None;
        }
    }
    // A restored selection must agree with its retained, frozen source counts.
    if frozen > 0 && winner(&counts) != Some((frozen - 1) as usize) {
        return None;
    }
    Some(Tracker { counts, frozen })
}

pub(crate) fn initial_resources() -> ScenarioResources {
    let mut resources = ScenarioResources::new();
    resources.values.insert(VERSION.into(), 1);
    resources.values.insert(COMPLETE.into(), 1);
    resources.values.insert(FROZEN.into(), 0);
    for key in COUNTS {
        resources.values.insert(key.into(), 0);
    }
    resources
}

fn is_finale(race_id: &str) -> bool {
    matches!(
        race_id,
        "finale_qualifier" | "finale_semifinal" | "finale_finals"
    )
}

fn class_for_course(course_id: u32) -> Option<usize> {
    let course = get_course(course_id)?;
    CLASSES.iter().position(|(surface, distance, _, _)| {
        course.surface == *surface && course.distance_type == *distance
    })
}

pub(crate) fn record_race(state: &CareerState, race_id: &str) -> ScenarioResources {
    let resources = &state.scenario_resources;
    if state.meta.scenario_id != "ura" {
        return resources.clone();
    }
    let Some(history) = tracker(resources) else {
        // Never initialize, infer or repair a legacy/partial history here.
        return resources.clone();
    };
    if history.frozen != 0 {
        return resources.clone();
    }
    if is_finale(race_id) {
        // A restored mid-finals state cannot retroactively establish a selection.
        return resources.set(FROZEN, -1);
    }
    let Some(class) = known_course_id_for_history(race_id).and_then(class_for_course) else {
        return resources.set(COMPLETE, 0);
    };
    let Some(count) = history.counts[class].checked_add(1) else {
        return resources.set(COMPLETE, 0);
    };
    resources.set(COUNTS[class], count)
}

/// Called after the existing mandatory-race decision, before any finale physics.
pub(crate) fn freeze(state: &CareerState) -> (ScenarioResources, Vec<String>) {
    let resources = &state.scenario_resources;
    let Some(race_id) = state.pending_race_id.as_deref() else {
        return (resources.clone(), Vec::new());
    };
    if state.meta.scenario_id != "ura" || !is_finale(race_id) {
        return (resources.clone(), Vec::new());
    }
    if let Some(history) = tracker(resources) {
        if history.frozen != 0 {
            return (resources.clone(), Vec::new());
        }
        if race_id == "finale_qualifier" {
            if let Some(class) = winner(&history.counts) {
                return (
                    resources.set(FROZEN, (class + 1) as i32),
                    vec![format!(
                        "URA finale class fixed from recorded participation: {} (simulator policy)",
                        CLASSES[class].3
                    )],
                );
            }
        }
    }
    (
        resources.set(FROZEN, -1),
        vec!["URA finale history unavailable; retaining legacy stage courses".into()],
    )
}

/// Only a valid, previously frozen URA history can override a finale course.
pub(crate) fn course_id(state: &CareerState, race_id: &str) -> Option<u32> {
    if state.meta.scenario_id != "ura" || !is_finale(race_id) {
        return None;
    }
    let history = tracker(&state.scenario_resources)?;
    if history.frozen <= 0 {
        return None;
    }
    let class = (history.frozen - 1) as usize;
    let legacy = course_id_for_race(race_id);
    if class_for_course(legacy) == Some(class) {
        return Some(legacy);
    }
    let representative = CLASSES[class].2;
    // Course data is authoritative; a missing/mismatched representative falls back.
    (class_for_course(representative) == Some(class)).then_some(representative)
}
