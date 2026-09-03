//! Bot policy tests retargeted at `--policy=external` (JVM scoring-shared).

mod common;

use common::{base_state, settings_fast, stats, with_resources, SCENARIOS};
use std::collections::HashMap;
use uma_sim_core::bot::BotDecisionAdapter;
use uma_sim_core::calendar::CAREER_TURNS;
use uma_sim_core::policy_external::external_auto_policy;
use uma_sim_core::scenario::grand_live_lesson_scoring::GrandLiveLessonScoring;
use uma_sim_core::scenario::{scenario_plugin_for, ScenarioPlugin, UraScenarioPlugin};
use uma_sim_core::state::{
    MoodLevel, RunMeta, SimActionKind, SimChoice, TrainingFacility, TurnPhase,
};
use uma_sim_core::{SimEngine, TrainingResolver};

/// Returns false (and the caller should skip) when no external policy binary is configured.
fn ensure_policy_env() -> bool {
    std::env::var_os("UMA_POLICY_CMD").is_some()
}

fn state_with_event(energy: i32, options: Vec<String>) -> uma_sim_core::state::CareerState {
    let mut state = base_state("ura", 5);
    state.stats = stats(200);
    state.energy = energy;
    state.max_energy = 100;
    state.phase = TurnPhase::Event.as_str().to_string();
    state.pending_event_title = Some("Test Event".into());
    state.pending_event_options = options;
    state
}

#[test]
fn event_low_energy_prefers_energy_option() {
    if !ensure_policy_env() {
        return;
    }
    let state = state_with_event(25, vec!["Speed +10".into(), "Energy +30".into()]);
    let plugin = scenario_plugin_for("ura");
    let resolver = TrainingResolver::default();
    let choices = vec![
        SimChoice {
            id: "event_0".into(),
            label: "Speed +10".into(),
        },
        SimChoice {
            id: "event_1".into(),
            label: "Energy +30".into(),
        },
    ];
    let action = external_auto_policy(&choices, &state, &resolver, plugin.as_ref());
    assert_eq!(action.kind, SimActionKind::Choose);
    assert_eq!(action.payload.as_deref(), Some("1"));
}

#[test]
fn training_picks_speed_when_prioritized() {
    if !ensure_policy_env() {
        return;
    }
    let mut state = base_state("ura", 10);
    state.stats = uma_sim_core::state::TraineeStats {
        speed: 100,
        stamina: 400,
        power: 400,
        guts: 400,
        wit: 400,
    };
    state.energy = 80;
    state.mood = MoodLevel::Great;
    state.fans = 1000;
    let plugin = scenario_plugin_for("ura");
    let resolver = TrainingResolver::default();
    let choices: Vec<SimChoice> = TrainingFacility::ALL
        .iter()
        .map(|f| SimChoice {
            id: format!("train_{}", f.key()),
            label: format!("Train {}", f.key()),
        })
        .collect();
    let action = external_auto_policy(&choices, &state, &resolver, plugin.as_ref());
    assert_eq!(action.kind, SimActionKind::Train);
    assert_eq!(action.payload.as_deref(), Some("speed"));
}

#[test]
fn external_policy_drives_career_self_consistent() {
    if !ensure_policy_env() {
        return;
    }
    let mut engine = SimEngine::new(uma_sim_core::state::SimSettings {
        speed_multiplier: 10,
        trace_telemetry: true,
        ..Default::default()
    });
    let mut meta = RunMeta::new(777, "ura", "Parity");
    meta.legacy_factors = vec!["factor:blue:1@3".into()];
    engine.start(meta);
    let resolver = TrainingResolver::default();
    let mut total = 0;
    let mut matches = 0;
    for _ in 0..72 {
        if engine.state().career_complete {
            break;
        }
        let choices = engine.choices();
        if choices.is_empty() {
            continue;
        }
        let plugin = scenario_plugin_for(&engine.state().meta.scenario_id);
        let a = external_auto_policy(&choices, engine.state(), &resolver, plugin.as_ref());
        let b = external_auto_policy(&choices, engine.state(), &resolver, plugin.as_ref());
        total += 1;
        if a.kind == b.kind && a.payload == b.payload {
            matches += 1;
        }
        engine.step(a);
    }
    assert!(engine.state().career_complete || engine.state().turn >= CAREER_TURNS);
    let rate = if total == 0 {
        1.0
    } else {
        matches as f64 / total as f64
    };
    assert!(
        rate >= 0.90,
        "External policy self-consistency {matches}/{total} = {:.1}%",
        rate * 100.0
    );
}

#[test]
fn lesson_scoring_prefers_song_near_concert_when_hype_not_ready() {
    let state = with_resources(
        {
            let mut s = base_state("grand_concert", 22);
            s.date = uma_sim_core::state::SimDate {
                year: 2,
                month: 6,
                half: 1,
            };
            s.stats = stats(200);
            s.energy = 80;
            s
        },
        HashMap::from([
            ("hype".into(), 1),
            ("great_success_required".into(), 3),
            ("songs_learned".into(), 2),
            ("techniques_learned".into(), 4),
        ]),
    );
    let plugin = scenario_plugin_for("grand_concert");
    let ctx = BotDecisionAdapter::to_decision_context(&state, plugin.as_ref());
    let song_score = GrandLiveLessonScoring::score_choice(&ctx, "gl_song_3");
    let tech_score = GrandLiveLessonScoring::score_choice(&ctx, "gl_tech_11001");
    assert!(
        song_score > tech_score,
        "song={song_score} tech={tech_score}"
    );
}

#[test]
fn external_policy_all_scenarios() {
    if !ensure_policy_env() {
        return;
    }
    for scenario in SCENARIOS {
        let mut engine = SimEngine::new(settings_fast(20));
        engine.start(RunMeta::new(777, scenario, "Parity"));
        let resolver = TrainingResolver::default();
        let mut total = 0;
        let mut matches = 0;
        for _ in 0..CAREER_TURNS {
            if engine.state().career_complete {
                break;
            }
            let choices = engine.choices();
            if choices.is_empty() {
                continue;
            }
            let plugin = scenario_plugin_for(&engine.state().meta.scenario_id);
            let a = external_auto_policy(&choices, engine.state(), &resolver, plugin.as_ref());
            let b = external_auto_policy(&choices, engine.state(), &resolver, plugin.as_ref());
            total += 1;
            if a.kind == b.kind && a.payload == b.payload {
                matches += 1;
            }
            engine.step(a);
        }
        let rate = if total == 0 {
            1.0
        } else {
            matches as f64 / total as f64
        };
        assert!(
            rate >= 0.90,
            "{scenario} external parity {matches}/{total} = {:.1}%",
            rate * 100.0
        );
    }
}

#[test]
fn inheritance_event_at_turn_13() {
    let plugin = UraScenarioPlugin::default();
    let mut state = base_state("ura", 13);
    state.meta.legacy_factors = vec!["factor:blue:1@3".into()];
    state.legacy.inheritance_complete = false;
    let (after, lines) = plugin.on_turn_start(&state);
    assert!(
        after
            .pending_event_title
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("inheritance"),
        "expected inheritance event, got {:?} lines={lines:?}",
        after.pending_event_title
    );
    assert_eq!(after.pending_event_options.len(), 2);
}
