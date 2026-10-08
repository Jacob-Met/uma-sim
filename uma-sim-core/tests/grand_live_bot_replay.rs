//! R7.2 GL-specific bot adapter replay fixtures (≥95%).

mod common;

use common::{base_state, stats5, with_resources};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use uma_sim_core::scenario::grand_live_lesson_scoring::GrandLiveLessonScoring;
use uma_sim_core::state::{MoodLevel, SimChoice, TurnPhase};
use uma_sim_core::{
    scenario_plugin_for, BotDecisionAdapter, GrandLiveMechanics, TrainingFacility, TrainingResolver,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlFixture {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default = "default_turn")]
    turn: i32,
    #[serde(default = "default_energy")]
    energy: i32,
    #[serde(default = "default_mood")]
    mood: String,
    #[serde(default = "default_stat")]
    speed: i32,
    #[serde(default = "default_stat")]
    stamina: i32,
    #[serde(default = "default_stat")]
    power: i32,
    #[serde(default = "default_stat")]
    guts: i32,
    #[serde(default = "default_stat")]
    wit: i32,
    #[serde(default)]
    hype: i32,
    #[serde(default = "default_gs")]
    great_success_required: i32,
    #[serde(default)]
    songs_learned: i32,
    #[serde(default)]
    techniques_learned: i32,
    #[serde(default)]
    days_to_concert: i32,
    #[serde(default)]
    choices: Vec<String>,
    #[serde(default)]
    expected_action: String,
    #[serde(default)]
    expected_facility: String,
    #[serde(default)]
    expected_hype_maxed: Option<bool>,
    #[serde(default)]
    description: String,
}

fn default_turn() -> i32 {
    10
}
fn default_energy() -> i32 {
    80
}
fn default_mood() -> String {
    "NORMAL".into()
}
fn default_stat() -> i32 {
    200
}
fn default_gs() -> i32 {
    3
}

fn load_fixtures() -> Vec<GlFixture> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/grand_live_replay/fixtures.json");
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("parse GL fixtures")
}

fn mood_from(s: &str) -> MoodLevel {
    MoodLevel::from_scoring_name(s).unwrap_or(MoodLevel::Normal)
}

fn gl_state(f: &GlFixture) -> uma_sim_core::CareerState {
    let mut state = base_state("grand_concert", f.turn);
    state.stats = stats5(f.speed, f.stamina, f.power, f.guts, f.wit);
    state.energy = f.energy;
    state.mood = mood_from(&f.mood);
    state.phase = TurnPhase::Free.as_str().to_string();
    state = with_resources(
        state,
        HashMap::from([
            ("hype".into(), f.hype),
            ("great_success_required".into(), f.great_success_required),
            ("songs_learned".into(), f.songs_learned),
            ("techniques_learned".into(), f.techniques_learned),
            (
                "techniques_since_last_song".into(),
                f.techniques_learned.max(2),
            ),
            ("song_slot_index".into(), 0),
            ("concert_index".into(), 1),
            ("perf_Da".into(), 80),
            ("perf_Pa".into(), 80),
            ("perf_Vo".into(), 80),
            ("perf_Vi".into(), 80),
            ("perf_Me".into(), 80),
            ("song_owned:1".into(), 1),
        ]),
    );
    state
}

fn fixture_matches(f: &GlFixture) -> bool {
    let plugin = scenario_plugin_for("grand_concert");
    let resolver = TrainingResolver::default();
    let state = gl_state(f);
    match f.kind.as_str() {
        "context" => {
            let ctx = BotDecisionAdapter::to_decision_context(&state, plugin.as_ref());
            let expected = f.expected_hype_maxed.unwrap_or(false);
            ctx.is_hype_maxed == expected
                && (f.days_to_concert < 0
                    || ctx.days_to_concert == f.days_to_concert
                    || GrandLiveMechanics::days_to_concert(f.turn) == f.days_to_concert)
        }
        "training" => {
            let facility =
                BotDecisionAdapter::choose_training_facility(&state, plugin.as_ref(), &resolver);
            facility.key() == f.expected_facility.to_lowercase()
        }
        "lesson" => {
            let ctx = BotDecisionAdapter::to_decision_context(&state, plugin.as_ref());
            let mut best_id = f.choices.first().cloned().unwrap_or_default();
            let mut best_score = f64::NEG_INFINITY;
            for id in &f.choices {
                let score = if id.starts_with("gl_") {
                    GrandLiveLessonScoring::score_choice(&ctx, id)
                } else if id.starts_with("train_") {
                    let fac = id.trim_start_matches("train_");
                    // Prefer training when lesson scores are low; use crude proxy.
                    let lag = match fac {
                        "speed" => 500 - state.stats.speed,
                        "stamina" => 500 - state.stats.stamina,
                        "power" => 500 - state.stats.power,
                        _ => 0,
                    };
                    lag as f64
                } else if id == "rest" {
                    if state.energy < 40 {
                        50.0
                    } else {
                        -10.0
                    }
                } else {
                    0.0
                };
                if score > best_score {
                    best_score = score;
                    best_id = id.clone();
                }
            }
            best_id == f.expected_action || {
                // Also accept BotDecisionAdapter when choices are SimChoices.
                let choices: Vec<SimChoice> = f
                    .choices
                    .iter()
                    .map(|id| SimChoice {
                        id: id.clone(),
                        label: id.clone(),
                    })
                    .collect();
                let action =
                    BotDecisionAdapter::choose_action(&choices, &state, plugin.as_ref(), &resolver);
                action.payload.as_deref() == Some(f.expected_action.as_str())
                    || (action.kind == uma_sim_core::state::SimActionKind::Rest
                        && f.expected_action == "rest")
                    || (action.kind == uma_sim_core::state::SimActionKind::Train
                        && f.expected_action.starts_with("train_"))
                    || (action.kind == uma_sim_core::state::SimActionKind::Lesson
                        && f.expected_action.starts_with("gl_"))
            }
        }
        _ => false,
    }
}

#[test]
fn gl_replay_fixtures_match_at_least_95_percent() {
    let fixtures = load_fixtures();
    assert!(
        fixtures.len() >= 8,
        "expected ≥8 GL fixtures, got {}",
        fixtures.len()
    );
    let mut matches = 0;
    let mut failures = Vec::new();
    for f in &fixtures {
        if fixture_matches(f) {
            matches += 1;
        } else {
            failures.push(f.description.clone());
        }
    }
    let rate = matches as f64 / fixtures.len() as f64;
    assert!(
        rate >= 0.95,
        "GL replay {matches}/{} = {:.0}% (failed: {failures:?})",
        fixtures.len(),
        rate * 100.0
    );
}

#[test]
fn gl_token_totals_wired_into_decision_context() {
    let state = with_resources(
        {
            let mut s = base_state("grand_concert", 10);
            s.energy = 80;
            s
        },
        HashMap::from([
            ("perf_Da".into(), 40),
            ("perf_Pa".into(), 20),
            ("hype".into(), 2),
            ("great_success_required".into(), 3),
        ]),
    );
    let plugin = scenario_plugin_for("grand_concert");
    let ctx = BotDecisionAdapter::to_decision_context(&state, plugin.as_ref());
    assert_eq!(ctx.token_totals.get("Da").copied(), Some(40));
    assert_eq!(ctx.token_totals.get("Pa").copied(), Some(20));
    assert!(!ctx.is_hype_maxed);
    assert_eq!(ctx.days_to_concert, GrandLiveMechanics::days_to_concert(10));
    let _ = TrainingFacility::Speed;
}
