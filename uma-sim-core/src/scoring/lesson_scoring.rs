use std::collections::HashMap;

use super::decision_context::DecisionContext;
use super::types::StatName;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LessonScoreInputs {
    pub is_song: bool,
    pub song_known: bool,
    pub song_already_owned: bool,
    pub training_effectiveness_pct: f64,
    pub training_gain_amount: f64,
    pub support_events: bool,
    pub raw_stat_gains: HashMap<StatName, i32>,
    pub skill_hint_amount: f64,
    pub energy_gain: i32,
    pub legacy_rank_score: f64,
}

const EV_BLEND: f64 = 0.65;
const RANK_BLEND: f64 = 0.35;
const TARGET_SONGS: i32 = 18;

pub fn score_lesson_option(ctx: &DecisionContext, inputs: &LessonScoreInputs) -> f64 {
    let w = ctx.objective_weights().normalized();
    let mut ev = 0.0;
    let turns_left = ctx.remaining_turns.max(1) as f64;

    for (stat, amt) in &inputs.raw_stat_gains {
        let discounted = ctx.soft_cap_discount(*stat, *amt);
        let idx = ctx
            .stat_prioritization
            .iter()
            .position(|s| s == stat)
            .map(|i| i as i32)
            .unwrap_or(-1);
        let priority_mult = match idx {
            0 => 1.5,
            1 => 1.3,
            2 => 1.15,
            3 => 1.0,
            _ => 0.85,
        };
        ev += discounted * priority_mult * 3.0 * (w.stat_targets + w.career_score);
    }

    if inputs.training_effectiveness_pct > 0.0 {
        ev += inputs.training_effectiveness_pct
            * turns_left
            * 0.35
            * (w.stat_targets + w.scenario_completion);
    }
    if inputs.training_gain_amount > 0.0 {
        ev += inputs.training_gain_amount * turns_left * 0.5 * (w.stat_targets + w.career_score);
    }
    if inputs.support_events {
        ev += turns_left * 1.2 * (w.scenario_completion + 0.5 * w.spark_quality);
    }
    if inputs.skill_hint_amount > 0.0 {
        ev += inputs.skill_hint_amount * 12.0 * (w.pvp_raceability + w.career_score);
    }

    if inputs.energy_gain > 0 {
        let need = match ctx.energy {
            e if e < 30 => 3.0,
            e if e < 50 => 2.0,
            e if e < 70 => 1.0,
            e if e >= 90 => 0.1,
            _ => 0.5,
        };
        ev += inputs.energy_gain as f64 * need;
    }

    if inputs.is_song {
        let days = ctx.days_to_concert;
        if !ctx.is_hype_maxed && (0..=4).contains(&days) {
            ev += 200.0 * (w.scenario_completion + 0.5 * w.spark_quality);
        } else if !ctx.is_hype_maxed && (5..=8).contains(&days) {
            ev += 80.0 * w.scenario_completion;
        }
        if !inputs.song_already_owned && ctx.songs_learned < TARGET_SONGS {
            let urgency = (TARGET_SONGS - ctx.songs_learned).min(8);
            ev += (60.0 + urgency as f64 * 15.0) * (w.scenario_completion + 0.3);
        }
        ev += 40.0 * (w.scenario_completion + 0.2);
    } else if !ctx.is_hype_maxed && (0..=3).contains(&ctx.days_to_concert) {
        ev *= 0.7;
    }

    EV_BLEND * ev + RANK_BLEND * inputs.legacy_rank_score
}

pub fn should_hold_for_lesson(
    ctx: &DecisionContext,
    locked_is_sought_after: bool,
    locked_is_song: bool,
    locked_song_unowned: bool,
    affordable_learnable_count: i32,
) -> bool {
    if ctx.is_finale_season {
        return false;
    }
    if !locked_is_sought_after
        && !(locked_is_song && locked_song_unowned && ctx.songs_learned < TARGET_SONGS)
    {
        return false;
    }
    let concert_far = ctx.days_to_concert < 0 || ctx.days_to_concert > 4;
    if ctx.is_hype_maxed && concert_far {
        return true;
    }
    if locked_is_song && locked_song_unowned && affordable_learnable_count == 0 {
        return true;
    }
    if locked_is_song && locked_song_unowned && ctx.songs_learned >= TARGET_SONGS - 3 && concert_far
    {
        return true;
    }
    false
}
