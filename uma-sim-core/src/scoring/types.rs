use std::collections::{HashMap, HashSet};

pub use crate::state::StatName;

impl StatName {
    pub fn from_name(value: &str) -> Option<StatName> {
        match value.to_uppercase().as_str() {
            "SPEED" => Some(StatName::Speed),
            "STAMINA" => Some(StatName::Stamina),
            "POWER" => Some(StatName::Power),
            "GUTS" => Some(StatName::Guts),
            "WIT" => Some(StatName::Wit),
            _ => None,
        }
    }
}

/// Backward-compatible alias for [StatName::from_name].
pub fn stat_name_from_str(name: &str) -> Option<StatName> {
    StatName::from_name(name)
}

/// The three career years. Comparable via natural enum ordinal (`Junior < Classic < Senior`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DateYear {
    Junior,
    Classic,
    Senior,
}

impl DateYear {
    pub const ALL: [DateYear; 3] = [DateYear::Junior, DateYear::Classic, DateYear::Senior];

    pub fn long_name(self) -> &'static str {
        match self {
            DateYear::Junior => "JUNIOR YEAR",
            DateYear::Classic => "CLASSIC YEAR",
            DateYear::Senior => "SENIOR YEAR",
        }
    }

    pub fn from_name(value: &str) -> Option<DateYear> {
        match value.to_uppercase().as_str() {
            "JUNIOR" => Some(DateYear::Junior),
            "CLASSIC" => Some(DateYear::Classic),
            "SENIOR" => Some(DateYear::Senior),
            _ => None,
        }
    }

    pub fn from_ordinal(ordinal: i32) -> Option<DateYear> {
        DateYear::ALL.get(ordinal as usize).copied()
    }

    pub fn ordinal(self) -> i32 {
        match self {
            DateYear::Junior => 0,
            DateYear::Classic => 1,
            DateYear::Senior => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameDateSnapshot {
    pub year: DateYear,
    pub day: i32,
    pub b_is_pre_debut: bool,
    pub is_summer: bool,
}

impl Default for GameDateSnapshot {
    fn default() -> Self {
        Self {
            year: DateYear::Junior,
            day: 0,
            b_is_pre_debut: false,
            is_summer: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarFillResult {
    pub dominant_color: String,
    pub fill_percent: f64,
    pub is_trainer_support: bool,
}

impl Default for BarFillResult {
    fn default() -> Self {
        Self {
            dominant_color: String::new(),
            fill_percent: 0.0,
            is_trainer_support: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingOption {
    pub name: StatName,
    pub stat_gains: HashMap<StatName, i32>,
    pub relationship_bars: Vec<BarFillResult>,
    pub num_rainbow: i32,
    pub num_skill_hints: i32,
    pub training_level: Option<i32>,
}

impl Default for TrainingOption {
    fn default() -> Self {
        Self {
            name: StatName::Speed,
            stat_gains: HashMap::new(),
            relationship_bars: Vec::new(),
            num_rainbow: 0,
            num_skill_hints: 0,
            training_level: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingConfig {
    pub current_stats: HashMap<StatName, i32>,
    pub stat_prioritization: Vec<StatName>,
    pub summer_training_stat_priority: Vec<StatName>,
    pub stat_targets: HashMap<StatName, i32>,
    pub current_date: GameDateSnapshot,
    pub scenario: String,
    pub enable_rainbow_training_bonus: bool,
    pub blacklist: Vec<Option<StatName>>,
    pub disable_training_on_maxed_stat: bool,
    pub skill_hints_per_location: HashMap<StatName, i32>,
    pub enable_prioritize_skill_hints: bool,
    pub enable_training_level_weighting: bool,
    pub enable_prioritize_near_max_friendship: bool,
    pub stats_trained_over_buffer: HashSet<StatName>,
    pub scoring: TrainingScoringConstants,
    pub stat_caps: HashMap<StatName, i32>,
    /// Default constructed via [TrainingConfig::new] or explicit field init; see [crate::scoring::skill_scoring::UmaAptitudeSnapshot].
    pub aptitudes: crate::scoring::skill_scoring::UmaAptitudeSnapshot,
}

fn default_skill_hints_per_location() -> HashMap<StatName, i32> {
    StatName::ALL.iter().map(|s| (*s, 0)).collect()
}

impl TrainingConfig {
    pub fn new(
        current_stats: HashMap<StatName, i32>,
        stat_prioritization: Vec<StatName>,
        summer_training_stat_priority: Vec<StatName>,
        stat_targets: HashMap<StatName, i32>,
        current_date: GameDateSnapshot,
        scenario: String,
        enable_rainbow_training_bonus: bool,
    ) -> Self {
        Self {
            current_stats,
            stat_prioritization,
            summer_training_stat_priority,
            stat_targets,
            current_date,
            scenario,
            enable_rainbow_training_bonus,
            blacklist: Vec::new(),
            disable_training_on_maxed_stat: false,
            skill_hints_per_location: default_skill_hints_per_location(),
            enable_prioritize_skill_hints: false,
            enable_training_level_weighting: false,
            enable_prioritize_near_max_friendship: true,
            stats_trained_over_buffer: HashSet::new(),
            scoring: TrainingScoringConstants::default(),
            stat_caps: HashMap::new(),
            aptitudes: crate::scoring::skill_scoring::UmaAptitudeSnapshot::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingScoringConstants {
    pub ratio_breakpoints: Vec<f64>,
    pub ratio_multipliers: Vec<f64>,
    pub priority_coefficient: f64,
    pub level_boost_rank1_factor: f64,
    pub level_boost_rank2_factor: f64,
    pub level_boost_rank3_factor: f64,
    pub main_stat_thresholds: HashMap<StatName, i32>,
    pub main_stat_bonus_magnitude: f64,
    pub relationship_orange_value: f64,
    pub relationship_green_value: f64,
    pub relationship_blue_value: f64,
    pub relationship_diminishing_factor: f64,
    pub relationship_early_game_bonus: f64,
    pub relationship_trainer_support_bonus: f64,
    pub skill_hint_per_hint_score: f64,
    pub skill_hint_override_score: f64,
    pub stat_weight_with_bars: f64,
    pub stat_weight_without_bars: f64,
    pub relationship_weight_with_bars: f64,
    pub misc_weight: f64,
    pub junior_early_game_flat_bonus: f64,
    pub relationship_scale: f64,
    pub rainbow_multiplier_enabled: f64,
    pub rainbow_multiplier_disabled: f64,
    pub rainbow_per_instance_base: f64,
    pub rainbow_per_instance_decay: f64,
    pub anticipatory_min_fill_percent: f64,
    pub anticipatory_coefficient: f64,
    pub anticipatory_cap: f64,
    pub unity_fill_base_bonus: f64,
    pub unity_fill_per_gauge_bonus: f64,
    pub unity_burst_base_bonus: f64,
    pub unity_burst_per_gauge_bonus: f64,
    pub unity_fill_energy_penalty_per_gauge: f64,
    pub unity_burst_energy_penalty_per_gauge: f64,
    pub unity_extreme_burst_base_bonus: f64,
    pub unity_extreme_burst_per_gauge_bonus: f64,
}

impl Default for TrainingScoringConstants {
    fn default() -> Self {
        let constants = Self {
            ratio_breakpoints: vec![15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            ratio_multipliers: vec![5.0, 4.0, 3.0, 2.0, 1.0, 0.5, 0.3],
            priority_coefficient: 0.5,
            level_boost_rank1_factor: 0.75,
            level_boost_rank2_factor: 0.25,
            level_boost_rank3_factor: 0.10,
            main_stat_thresholds: HashMap::from([
                (StatName::Speed, 30),
                (StatName::Stamina, 30),
                (StatName::Power, 30),
                (StatName::Guts, 30),
                (StatName::Wit, 15),
            ]),
            main_stat_bonus_magnitude: 2.0,
            relationship_orange_value: 0.0,
            relationship_green_value: 1.0,
            relationship_blue_value: 2.5,
            relationship_diminishing_factor: 0.5,
            relationship_early_game_bonus: 1.3,
            relationship_trainer_support_bonus: 1.15,
            skill_hint_per_hint_score: 10.0,
            skill_hint_override_score: 10000.0,
            stat_weight_with_bars: 0.6,
            stat_weight_without_bars: 0.7,
            relationship_weight_with_bars: 0.1,
            misc_weight: 0.3,
            junior_early_game_flat_bonus: 100.0,
            relationship_scale: 1.5,
            rainbow_multiplier_enabled: 2.0,
            rainbow_multiplier_disabled: 1.5,
            rainbow_per_instance_base: 200.0,
            rainbow_per_instance_decay: 0.5,
            anticipatory_min_fill_percent: 50.0,
            anticipatory_coefficient: 0.2,
            anticipatory_cap: 0.6,
            unity_fill_base_bonus: 60.0,
            unity_fill_per_gauge_bonus: 40.0,
            unity_burst_base_bonus: 800.0,
            unity_burst_per_gauge_bonus: 400.0,
            unity_fill_energy_penalty_per_gauge: 0.0,
            unity_burst_energy_penalty_per_gauge: 0.0,
            unity_extreme_burst_base_bonus: 2000.0,
            unity_extreme_burst_per_gauge_bonus: 1000.0,
        };
        constants.validate();
        constants
    }
}

impl TrainingScoringConstants {
    pub fn validate(&self) {
        assert_eq!(
            self.ratio_multipliers.len(),
            self.ratio_breakpoints.len() + 1,
            "ratioMultipliers must have exactly one more entry than ratioBreakpoints (got {} multipliers vs {} breakpoints)",
            self.ratio_multipliers.len(),
            self.ratio_breakpoints.len()
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawScoreBreakdown {
    pub stat_score_weighted: f64,
    pub relationship_score_weighted: f64,
    pub misc_score_weighted: f64,
    pub rainbow_multiplier: f64,
    pub anticipatory_multiplier: f64,
    pub total: f64,
}
