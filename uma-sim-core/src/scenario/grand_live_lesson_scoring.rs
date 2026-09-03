//! Grand Live lesson scoring via shared bot scoring.

use crate::bot::BotDecisionAdapter;
use crate::events::sample_event_reward;
use crate::scenario::grand_live::GrandLiveConcertBonus;
use crate::scenario::grand_live_catalog::GrandLiveCatalog;
use crate::scenario::ScenarioPlugin;
use crate::scoring::{score_lesson_option, DecisionContext, LessonScoreInputs};
use crate::state::{CareerState, SimChoice};

pub struct GrandLiveLessonScoring;

impl GrandLiveLessonScoring {
    pub fn choose_best_lesson(
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
        choices: &[SimChoice],
    ) -> Option<SimChoice> {
        let lessons: Vec<_> = choices
            .iter()
            .filter(|c| c.id.starts_with("gl_song_") || c.id.starts_with("gl_tech_"))
            .collect();
        if lessons.is_empty() {
            return None;
        }
        let ctx = BotDecisionAdapter::to_decision_context(state, plugin);
        lessons
            .iter()
            .max_by(|a, b| {
                Self::score_choice(&ctx, &a.id)
                    .partial_cmp(&Self::score_choice(&ctx, &b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| (*c).clone())
    }

    pub fn score_choice(ctx: &DecisionContext, choice_id: &str) -> f64 {
        inputs_for_choice(choice_id)
            .map(|inputs| score_lesson_option(ctx, &inputs))
            .unwrap_or(f64::NEG_INFINITY)
    }
}

fn inputs_for_choice(choice_id: &str) -> Option<LessonScoreInputs> {
    if let Some(rest) = choice_id.strip_prefix("gl_song_") {
        return inputs_for_song(rest);
    }
    if let Some(rest) = choice_id.strip_prefix("gl_tech_") {
        return inputs_for_technique(rest);
    }
    None
}

fn inputs_for_song(song_key: &str) -> Option<LessonScoreInputs> {
    let song = GrandLiveCatalog::find_song(song_key)?;
    let reading = sample_event_reward(&song.purchase_bonus_text, 0.5, 0.5);
    let bonus = song.concert_bonus.as_ref();
    Some(LessonScoreInputs {
        is_song: true,
        song_already_owned: false,
        training_effectiveness_pct: friendship_pct(bonus),
        training_gain_amount: training_gain_amount(&song.purchase_bonus_text),
        support_events: bonus
            .map(|b| b.effect.to_lowercase().contains("support chain"))
            .unwrap_or(false),
        raw_stat_gains: reading.stats,
        skill_hint_amount: reading.skill_pts.max(0) as f64,
        energy_gain: reading.energy_delta.max(0),
        ..Default::default()
    })
}

fn inputs_for_technique(tech_key: &str) -> Option<LessonScoreInputs> {
    let tech = GrandLiveCatalog::find_technique(tech_key)
        .or_else(|| GrandLiveCatalog::find_technique(&format!("lesson:{tech_key}")))?;
    let reading = sample_event_reward(&tech.effect_text, 0.5, 0.5);
    Some(LessonScoreInputs {
        is_song: false,
        raw_stat_gains: if tech.category == "stat" {
            reading.stats
        } else {
            std::collections::HashMap::new()
        },
        skill_hint_amount: if tech.category == "skill_hint" {
            1.0
        } else {
            0.0
        },
        energy_gain: if tech.category == "recovery" {
            reading.energy_delta.max(0)
        } else {
            0
        },
        ..Default::default()
    })
}

fn friendship_pct(bonus: Option<&GrandLiveConcertBonus>) -> f64 {
    bonus
        .filter(|b| b.effect.to_lowercase().contains("friendship"))
        .map(|b| b.value as f64)
        .unwrap_or(0.0)
}

fn training_gain_amount(text: &str) -> f64 {
    let lower = text.to_lowercase();
    if lower.contains("training speed gain")
        || lower.contains("training skill pt gain")
        || (lower.contains("training") && lower.contains("gain"))
    {
        regex_amount(text).unwrap_or(if lower.contains("gain") { 1.0 } else { 0.0 })
    } else {
        0.0
    }
}

fn regex_amount(text: &str) -> Option<f64> {
    text.split('+')
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
}
