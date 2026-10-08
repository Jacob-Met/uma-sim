//! Maps sim state to scoring types for decisions.
//!
//! Training / event / rest action selection for `--policy=external` goes through
//! [`crate::policy_external`]. This adapter remains for DecisionContext construction
//! (lesson board, telemetry) and for offline helpers that call into `scoring/`.

use crate::calendar::CAREER_TURNS;
use crate::config::TrainingFailureConfig;
use crate::policy::default_auto_policy;
use crate::scenario::ura::UraMechanics;
use crate::scenario::{
    grand_live::GrandLiveMechanics, grand_live_lesson_scoring::GrandLiveLessonScoring,
    ScenarioPlugin,
};
use crate::scoring::{
    calculate_raw_training_score, choose_best_event_option, mood_to_ordinal, DateYear,
    DecisionContext,
};
use crate::state::{CareerState, SimAction, SimActionKind, SimChoice, StatName, TrainingFacility};
use crate::training::{TrainingPreview, TrainingResolver};

pub struct BotDecisionAdapter;

impl BotDecisionAdapter {
    pub fn to_decision_context(
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
    ) -> DecisionContext {
        let is_grand = state.meta.scenario_id.to_lowercase().contains("grand");
        let year = match state.date.year {
            1 => DateYear::Junior,
            2 => DateYear::Classic,
            _ => DateYear::Senior,
        };
        DecisionContext {
            trainee_name: state.meta.trainee_name.clone(),
            energy: state.energy,
            mood_ordinal: mood_to_ordinal(state.mood),
            stats: stat_map(state),
            stat_caps: stat_caps_from_legacy(state, plugin),
            stat_prioritization: StatName::ALL.to_vec(),
            event_choice_stat_priority: StatName::ALL.to_vec(),
            day: state.turn,
            year,
            is_finale_season: state.turn >= 72,
            remaining_turns: (CAREER_TURNS - state.turn).max(0),
            objective_profile: if is_grand {
                "scenario_clear_grand_concert".to_string()
            } else {
                state.meta.objective_profile.clone()
            },
            prioritize_energy: state.energy < 40,
            dating_schedule_enabled: false,
            dating_chain_complete: false,
            preferred_skill_hints: Default::default(),
            deck_support_names: Vec::new(),
            token_totals: if is_grand {
                GrandLiveMechanics::token_totals_for_bot(&state.scenario_resources)
            } else {
                state
                    .scenario_resources
                    .values
                    .iter()
                    .map(|(k, v)| (k.replace("perf_", ""), *v))
                    .collect()
            },
            days_to_concert: if is_grand {
                GrandLiveMechanics::days_to_concert(state.turn)
            } else {
                -1
            },
            songs_learned: plugin.songs_learned(state),
            is_hype_maxed: if is_grand {
                GrandLiveMechanics::is_hype_maxed(&state.scenario_resources)
            } else {
                false
            },
        }
    }

    pub fn choose_event_option(state: &CareerState, plugin: &dyn ScenarioPlugin) -> i32 {
        if state.pending_event_options.is_empty() {
            return 0;
        }
        if state
            .pending_event_title
            .as_deref()
            .map(|t| t.to_lowercase().contains("happy meek"))
            .unwrap_or(false)
        {
            return UraMechanics::choose_duel_contest_index(
                &state.pending_event_options,
                &StatName::ALL,
            );
        }
        let ctx = Self::to_decision_context(state, plugin);
        choose_best_event_option(&ctx, &state.pending_event_options).0 as i32
    }

    pub fn choose_training_facility(
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
        resolver: &TrainingResolver,
    ) -> TrainingFacility {
        let caps = stat_caps_from_legacy(state, plugin);
        let config = TrainingPreview::to_training_config(state, caps);
        let previews = TrainingPreview::build_options(state, resolver);
        let mut best = TrainingFacility::Speed;
        let mut best_score = f64::NEG_INFINITY;
        for facility in TrainingFacility::ALL {
            let Some(option) = previews.get(&facility) else {
                continue;
            };
            let mut score = calculate_raw_training_score(&config, option);
            let key = facility.key();
            let level = state.facility_levels.get(key).copied().unwrap_or(1);
            let fail_pct = TrainingFailureConfig::failure_chance_pct(
                state.energy.max(0),
                state.max_energy,
                state.mood,
                level,
            );
            score = UraMechanics::apply_duel_training_bias(
                score,
                facility,
                &state.scenario_resources,
                fail_pct,
                30,
            );
            if score > best_score {
                best_score = score;
                best = facility;
            }
        }
        best
    }

    pub fn choose_action(
        choices: &[SimChoice],
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
        resolver: &TrainingResolver,
    ) -> SimAction {
        // Prefer the JVM policy when UMA_POLICY_CMD is set so `--policy=bot` and tests share one source.
        if std::env::var_os("UMA_POLICY_CMD").is_some() {
            return crate::policy_external::external_auto_policy(choices, state, resolver, plugin);
        }
        Self::choose_action_local(choices, state, plugin, resolver)
    }

    /// Offline fallback used only when the JVM policy server binary is missing.
    pub fn choose_action_local(
        choices: &[SimChoice],
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
        resolver: &TrainingResolver,
    ) -> SimAction {
        if choices
            .iter()
            .any(|c| c.id == "race" && c.label.to_lowercase().contains("mandatory"))
        {
            return SimAction {
                kind: SimActionKind::Race,
                payload: None,
            };
        }
        if choices.iter().any(|c| c.id.starts_with("event_")) {
            return SimAction {
                kind: SimActionKind::Choose,
                payload: Some(Self::choose_event_option(state, plugin).to_string()),
            };
        }
        if choices
            .iter()
            .any(|c| c.id.starts_with("gl_song_") || c.id.starts_with("gl_tech_"))
        {
            if let Some(lesson) = GrandLiveLessonScoring::choose_best_lesson(state, plugin, choices)
            {
                return SimAction {
                    kind: SimActionKind::Lesson,
                    payload: Some(lesson.id),
                };
            }
            return default_auto_policy(choices);
        }
        if choices.iter().any(|c| c.id == "race")
            && crate::race::RaceScheduler::should_run_optional_race(state)
        {
            return SimAction {
                kind: SimActionKind::Race,
                payload: None,
            };
        }
        if choices.iter().any(|c| c.id == "rest") && (state.energy < 40 || state.is_injured()) {
            return SimAction {
                kind: SimActionKind::Rest,
                payload: None,
            };
        }
        if choices.iter().any(|c| c.id.starts_with("train_")) {
            if state.is_injured() {
                return SimAction {
                    kind: SimActionKind::Rest,
                    payload: None,
                };
            }
            let facility = Self::choose_training_facility(state, plugin, resolver);
            return SimAction {
                kind: SimActionKind::Train,
                payload: Some(facility.key().to_string()),
            };
        }
        if choices.iter().any(|c| c.id == "rest") && state.energy < 40 {
            return SimAction {
                kind: SimActionKind::Rest,
                payload: None,
            };
        }
        if choices.iter().any(|c| c.id == "race") && choices.len() == 1 {
            return SimAction {
                kind: SimActionKind::Race,
                payload: None,
            };
        }
        default_auto_policy(choices)
    }
}

pub fn scoring_auto_policy(
    choices: &[SimChoice],
    state: &CareerState,
    resolver: &TrainingResolver,
    plugin: &dyn ScenarioPlugin,
) -> SimAction {
    BotDecisionAdapter::choose_action(choices, state, plugin, resolver)
}

fn stat_map(state: &CareerState) -> std::collections::HashMap<StatName, i32> {
    std::collections::HashMap::from([
        (StatName::Speed, state.stats.speed),
        (StatName::Stamina, state.stats.stamina),
        (StatName::Power, state.stats.power),
        (StatName::Guts, state.stats.guts),
        (StatName::Wit, state.stats.wit),
    ])
}

fn stat_caps_from_legacy(
    state: &CareerState,
    plugin: &dyn ScenarioPlugin,
) -> std::collections::HashMap<StatName, i32> {
    let caps = plugin.stat_caps();
    StatName::ALL
        .into_iter()
        .map(|stat| {
            let key = match stat {
                StatName::Speed => "speed",
                StatName::Stamina => "stamina",
                StatName::Power => "power",
                StatName::Guts => "guts",
                StatName::Wit => "wit",
            };
            let base = caps.get(key).copied().unwrap_or(1400);
            let cap = crate::legacy::LegacyApplicator::effective_stat_cap(base, key, &state.legacy)
                + UraMechanics::cap_bonus(&state.scenario_resources, key);
            (stat, cap)
        })
        .collect()
}
