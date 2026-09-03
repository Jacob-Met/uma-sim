//! Shared training/event/skill scoring math ported from `uma-android-automation` scoring-shared.

pub mod types;
pub mod skill_scoring;
pub mod decision_context;
pub mod objective_profiles;
pub mod formula_gain;
pub mod support_effects;
#[path = "scoring.rs"]
pub mod scoring;
pub mod event_scoring;
pub mod lesson_scoring;
pub mod rank_estimate;
pub mod terminal_utility;

pub use formula_gain::{apply_training_multipliers, SupportEffectSlice};
pub use scoring::soft_cap_effectiveness_multiplier;
pub use types::{stat_name_from_str, StatName};

pub use decision_context::{DecisionContext, mood_ordinal as MoodOrdinal};
pub use event_scoring::{
    choose_best_event_option, event_reward_branches, owner_match_boost, parse_event_reward_text,
    sample_event_reward, score_event_option, score_event_reading, EventEffectReading,
};
pub use formula_gain::{expected_value_under_failure, mood_adjust_score};
pub use lesson_scoring::{score_lesson_option, should_hold_for_lesson, LessonScoreInputs};
pub use objective_profiles::{combine_objective_score, ObjectiveProfiles, ObjectiveWeights};
pub use rank_estimate::{
    estimate_rank, evaluate_skill_score, rank_label_to_image_index, score_to_rank_label, stat_score,
    unique_bonus, RankAptitudes, RankResult, SkillScoreInput,
};
pub use terminal_utility::{
    evaluate_career_terminal, mean_phi_blue, mean_phi_blue_stats, phi_blue, psi_grade,
    spend_skill_points, terminal_utility, BracketHits, CareerTerminalRecord, TerminalStats,
    GOLD_SKILL_SCORE_PER_SP,
};
pub use scoring::{
    calculate_misc_score, calculate_raw_training_score, calculate_relationship_score,
    calculate_stat_efficiency_score, estimate_failure_chance_from_energy, get_current_stat_cap,
    get_finale_stat_bonus, get_remaining_finale_races, get_scenario_stat_cap, level_boost_multiplier,
    raw_training_score_components, scoring_constants_from_map,
};
pub use skill_scoring::{
    aptitude_gate_reason, calculate_profile_aware_drain_purchases, format_grand_live_token_score_breakdown,
    is_eval_heavy_skill_profile, score_grand_live_training_tokens, score_skill_for_uma,
    skill_score_per_point, uma_drain_score_floor, AptitudeOrdinal, SkillDrainCandidate,
    SkillScoreInputs, SkillScoreResult, UmaAptitudeSnapshot,
};
pub use support_effects::{
    deck_specialty_bias, estimate_facility_slices, mood_adjust_with_deck, stat_name_to_support_type,
    ResolvedDeckCard, SupportLevelBreakpoints,
};
pub use types::{
    BarFillResult, DateYear, GameDateSnapshot, RawScoreBreakdown, TrainingConfig, TrainingOption,
    TrainingScoringConstants,
};

use crate::state::{MoodLevel, SimDate};

/// Display name used by training config / soft-cap tables.
pub fn scenario_display_name(scenario_id: &str) -> String {
    match scenario_id.to_lowercase().replace(' ', "_").as_str() {
        "ura" | "ura_finale" => "URA Finale".to_string(),
        "grand_concert" | "grand_live" | "gl" => "Grand Live".to_string(),
        "unity" | "unity_cup" => "Unity Cup".to_string(),
        "trackblazer" | "tb" => "Trackblazer".to_string(),
        _ => "URA Finale".to_string(),
    }
}

pub fn to_game_date_snapshot(date: &SimDate, turn: i32) -> GameDateSnapshot {
    let year = match date.year {
        1 => DateYear::Junior,
        2 => DateYear::Classic,
        _ => DateYear::Senior,
    };
    GameDateSnapshot {
        year,
        day: turn,
        b_is_pre_debut: turn <= 12,
        is_summer: (7..=8).contains(&date.month),
    }
}

pub fn mood_to_ordinal(m: MoodLevel) -> i32 {
    match m {
        MoodLevel::Awful => MoodOrdinal::AWFUL,
        MoodLevel::Bad => MoodOrdinal::BAD,
        MoodLevel::Normal => MoodOrdinal::NORMAL,
        MoodLevel::Good => MoodOrdinal::GOOD,
        MoodLevel::Great => MoodOrdinal::GREAT,
    }
}
