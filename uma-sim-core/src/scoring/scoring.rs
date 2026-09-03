use std::collections::HashMap;

use serde_json::Value;

use super::types::{
    DateYear, RawScoreBreakdown, StatName, TrainingConfig, TrainingOption, TrainingScoringConstants,
};

const FINALE_RACE_STAT_BONUS: i32 = 15;
const DEFAULT_STAT_CAP: i32 = 1200;
const SOFT_CAP_THRESHOLD: i32 = 1200;
const BEYOND_SOFT_CAP_EFFECTIVENESS: f64 = 0.5;

pub fn get_scenario_stat_cap(scenario: &str, stat_name: StatName) -> i32 {
    match scenario {
        "URA Finale" => 1400,
        "Unity Cup" => {
            if stat_name == StatName::Wit {
                1800
            } else {
                1300
            }
        }
        "Trackblazer" => match stat_name {
            StatName::Stamina => 1900,
            StatName::Wit => 1500,
            _ => DEFAULT_STAT_CAP,
        },
        "Grand Live" => match stat_name {
            StatName::Speed => 1600,
            StatName::Guts => 1500,
            _ => 1300,
        },
        _ => DEFAULT_STAT_CAP,
    }
}

pub fn get_current_stat_cap(stat_name: StatName, config: &TrainingConfig) -> i32 {
    config
        .stat_caps
        .get(&stat_name)
        .copied()
        .filter(|cap| *cap > 0)
        .unwrap_or_else(|| get_scenario_stat_cap(&config.scenario, stat_name))
}

pub fn soft_cap_effectiveness_multiplier(
    current_stat: i32,
    stat_gain: i32,
    stat_cap: i32,
) -> f64 {
    if stat_gain <= 0 {
        return 1.0;
    }
    let end = (current_stat + stat_gain).min(stat_cap);
    if end <= current_stat {
        return 0.0;
    }
    let full_portion = (end.min(SOFT_CAP_THRESHOLD) - current_stat).max(0);
    let soft_portion = (end - current_stat.max(SOFT_CAP_THRESHOLD)).max(0);
    let effective_gain =
        full_portion as f64 + soft_portion as f64 * BEYOND_SOFT_CAP_EFFECTIVENESS;
    effective_gain / stat_gain as f64
}

pub fn get_remaining_finale_races(current_day: i32) -> i32 {
    (75 - current_day.max(72)).max(0)
}

pub fn get_finale_stat_bonus(current_day: i32) -> i32 {
    get_remaining_finale_races(current_day) * FINALE_RACE_STAT_BONUS
}

pub fn level_boost_multiplier(
    priority_rank: i32,
    training_level: Option<i32>,
    constants: &TrainingScoringConstants,
) -> f64 {
    let level = training_level.unwrap_or(1);
    if level <= 1 {
        return 1.0;
    }
    let priority_factor = match priority_rank {
        1 => constants.level_boost_rank1_factor,
        2 => constants.level_boost_rank2_factor,
        3 => constants.level_boost_rank3_factor,
        _ => 0.0,
    };
    let level_factor = (level - 1) as f64 / 4.0;
    1.0 + priority_factor * level_factor
}

pub fn calculate_stat_efficiency_score(config: &TrainingConfig, training: &TrainingOption) -> f64 {
    let mut score = 0.0;
    let active_priority = if config.current_date.is_summer {
        &config.summer_training_stat_priority
    } else {
        &config.stat_prioritization
    };

    for stat_name in StatName::ALL {
        let current_stat = config.current_stats.get(&stat_name).copied().unwrap_or(0);
        let target_stat = config.stat_targets.get(&stat_name).copied().unwrap_or(0);
        let stat_gain = training.stat_gains.get(&stat_name).copied().unwrap_or(0);

        if stat_gain > 0 && target_stat > 0 {
            let priority_index = active_priority
                .iter()
                .position(|s| *s == stat_name)
                .map(|i| i as i32)
                .unwrap_or(-1);

            let completion_percent = (current_stat as f64 / target_stat as f64) * 100.0;

            let ratio_multiplier = {
                let breakpoints = &config.scoring.ratio_breakpoints;
                let multipliers = &config.scoring.ratio_multipliers;
                let bucket = breakpoints.iter().position(|bp| completion_percent < *bp);
                match bucket {
                    None => *multipliers.last().unwrap(),
                    Some(idx) => multipliers[idx],
                }
            };

            let priority_multiplier = if priority_index != -1 {
                1.0 + config.scoring.priority_coefficient
                    * (active_priority.len() as f64 - priority_index as f64)
            } else {
                1.0
            };

            let level_multiplier = if config.enable_training_level_weighting
                && stat_name == training.name
                && priority_index != -1
            {
                level_boost_multiplier(priority_index + 1, training.training_level, &config.scoring)
            } else {
                1.0
            };

            let is_main_stat = training.name == stat_name;
            let main_stat_bonus = if is_main_stat {
                let threshold = config
                    .scoring
                    .main_stat_thresholds
                    .get(&stat_name)
                    .copied()
                    .unwrap_or_else(|| panic!("No mainStatThresholds entry for {:?}", stat_name));
                if stat_gain >= threshold {
                    config.scoring.main_stat_bonus_magnitude
                } else {
                    1.0
                }
            } else {
                1.0
            };

            let soft_cap_multiplier = soft_cap_effectiveness_multiplier(
                current_stat,
                stat_gain,
                get_current_stat_cap(stat_name, config),
            );

            let mut stat_score = stat_gain as f64;
            stat_score *= ratio_multiplier;
            stat_score *= soft_cap_multiplier;
            stat_score *= priority_multiplier;
            stat_score *= level_multiplier;
            stat_score *= main_stat_bonus;
            score += stat_score;
        }
    }
    score
}

pub fn calculate_relationship_score(config: &TrainingConfig, training: &TrainingOption) -> f64 {
    if training.relationship_bars.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;
    let mut max_score = 0.0;

    for bar in &training.relationship_bars {
        let base_value = match bar.dominant_color.as_str() {
            "orange" => config.scoring.relationship_orange_value,
            "green" => config.scoring.relationship_green_value,
            "blue" => config.scoring.relationship_blue_value,
            _ => 0.0,
        };

        if base_value > 0.0 {
            let fill_level = bar.fill_percent / 100.0;
            let diminishing_factor =
                1.0 - (fill_level * config.scoring.relationship_diminishing_factor);
            let early_game_bonus = if config.current_date.year == DateYear::Junior
                || config.current_date.b_is_pre_debut
            {
                config.scoring.relationship_early_game_bonus
            } else {
                1.0
            };
            let trainer_support_bonus = if bar.is_trainer_support {
                config.scoring.relationship_trainer_support_bonus
            } else {
                1.0
            };
            score += base_value * diminishing_factor * early_game_bonus * trainer_support_bonus;
            max_score += config.scoring.relationship_blue_value
                * config.scoring.relationship_early_game_bonus;
        }
    }

    if max_score > 0.0 {
        score / max_score * 100.0
    } else {
        0.0
    }
}

pub fn calculate_misc_score(config: &TrainingConfig, training: &TrainingOption) -> f64 {
    let mut score = 50.0;
    let num_skill_hints = config
        .skill_hints_per_location
        .get(&training.name)
        .copied()
        .unwrap_or(0);
    score += config.scoring.skill_hint_per_hint_score * num_skill_hints as f64;

    if config.enable_prioritize_skill_hints && num_skill_hints > 0 {
        return config.scoring.skill_hint_override_score + score;
    }

    score.clamp(0.0, 100.0)
}

pub fn calculate_raw_training_score(config: &TrainingConfig, training: &TrainingOption) -> f64 {
    raw_training_score_components(config, training).total
}

pub fn raw_training_score_components(
    config: &TrainingConfig,
    training: &TrainingOption,
) -> RawScoreBreakdown {
    let zero = RawScoreBreakdown {
        stat_score_weighted: 0.0,
        relationship_score_weighted: 0.0,
        misc_score_weighted: 0.0,
        rainbow_multiplier: 1.0,
        anticipatory_multiplier: 1.0,
        total: 0.0,
    };

    if config.blacklist.iter().any(|b| *b == Some(training.name)) {
        return zero;
    }

    let current_stat = config
        .current_stats
        .get(&training.name)
        .copied()
        .unwrap_or(0);
    let potential_stat = current_stat
        + training
            .stat_gains
            .get(&training.name)
            .copied()
            .unwrap_or(0);
    let stat_cap = get_current_stat_cap(training.name, config);
    let finale_bonus = get_finale_stat_bonus(config.current_date.day);
    let effective_stat_cap = stat_cap - 100 - finale_bonus;

    if current_stat >= stat_cap {
        return zero;
    }

    if config.disable_training_on_maxed_stat && current_stat >= effective_stat_cap {
        let can_use_allowance = training.num_rainbow > 0
            && !config.stats_trained_over_buffer.contains(&training.name);
        if !can_use_allowance {
            return zero;
        }
    }

    if potential_stat >= effective_stat_cap {
        let can_use_allowance = training.num_rainbow > 0
            && !config.stats_trained_over_buffer.contains(&training.name);
        if !can_use_allowance {
            return zero;
        }
    }

    let stat_score = calculate_stat_efficiency_score(config, training);
    let relationship_score = calculate_relationship_score(config, training);
    let misc_score = calculate_misc_score(config, training);

    let stat_weight = if !training.relationship_bars.is_empty() {
        config.scoring.stat_weight_with_bars
    } else {
        config.scoring.stat_weight_without_bars
    };
    let relationship_weight = if !training.relationship_bars.is_empty() {
        config.scoring.relationship_weight_with_bars
    } else {
        0.0
    };
    let misc_weight = config.scoring.misc_weight;

    let stat_score_weighted = stat_score * stat_weight;
    let relationship_score_weighted = relationship_score * relationship_weight;
    let misc_score_weighted = misc_score * misc_weight;
    let mut total_score =
        stat_score_weighted + relationship_score_weighted + misc_score_weighted;

    let rainbow_multiplier = if training.num_rainbow > 0 && config.current_date.year > DateYear::Junior
    {
        if config.enable_rainbow_training_bonus {
            config.scoring.rainbow_multiplier_enabled
        } else {
            config.scoring.rainbow_multiplier_disabled
        }
    } else {
        1.0
    };
    total_score *= rainbow_multiplier;

    let mut anticipatory_multiplier = 1.0;
    if config.enable_prioritize_near_max_friendship
        && config.current_date.year > DateYear::Junior
        && training.num_rainbow == 0
        && !training.relationship_bars.is_empty()
    {
        let mut contributions = 0.0;
        let mut qualifying_bars = 0;
        for bar in &training.relationship_bars {
            if (bar.dominant_color == "green" || bar.dominant_color == "blue")
                && bar.fill_percent > config.scoring.anticipatory_min_fill_percent
            {
                contributions += bar.fill_percent / 100.0;
                qualifying_bars += 1;
            }
        }
        if qualifying_bars > 0 {
            anticipatory_multiplier = 1.0
                + config
                    .scoring
                    .anticipatory_cap
                    .min(config.scoring.anticipatory_coefficient * contributions);
            total_score *= anticipatory_multiplier;
        }
    }

    RawScoreBreakdown {
        stat_score_weighted,
        relationship_score_weighted,
        misc_score_weighted,
        rainbow_multiplier,
        anticipatory_multiplier,
        total: total_score.max(0.0),
    }
}

pub fn estimate_failure_chance_from_energy(current_energy: i32, stat_name: Option<StatName>) -> i32 {
    let energy = current_energy.clamp(0, 100);
    let estimated = if stat_name == Some(StatName::Wit) {
        let raw = 161.4 * 0.9793_f64.powi(energy) - 81.4;
        raw as i32
    } else if energy >= 50 {
        0
    } else {
        (50 - energy) * 2
    };
    estimated.clamp(0, 100)
}

fn d(settings: &HashMap<String, Value>, key: &str, fallback: f64) -> f64 {
    settings
        .get(key)
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
        .unwrap_or(fallback)
}

fn i(settings: &HashMap<String, Value>, key: &str, fallback: i32) -> i32 {
    settings
        .get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(fallback)
}

pub fn scoring_constants_from_map(
    settings: &HashMap<String, Value>,
    defaults: &TrainingScoringConstants,
) -> TrainingScoringConstants {
    let constants = TrainingScoringConstants {
        ratio_breakpoints: defaults.ratio_breakpoints.clone(),
        ratio_multipliers: vec![
            d(settings, "ratioMultiplier1", defaults.ratio_multipliers[0]),
            d(settings, "ratioMultiplier2", defaults.ratio_multipliers[1]),
            d(settings, "ratioMultiplier3", defaults.ratio_multipliers[2]),
            d(settings, "ratioMultiplier4", defaults.ratio_multipliers[3]),
            d(settings, "ratioMultiplier5", defaults.ratio_multipliers[4]),
            d(settings, "ratioMultiplier6", defaults.ratio_multipliers[5]),
            d(settings, "ratioMultiplier7", defaults.ratio_multipliers[6]),
        ],
        priority_coefficient: d(settings, "priorityCoefficient", defaults.priority_coefficient),
        level_boost_rank1_factor: d(
            settings,
            "levelBoostRank1Factor",
            defaults.level_boost_rank1_factor,
        ),
        level_boost_rank2_factor: d(
            settings,
            "levelBoostRank2Factor",
            defaults.level_boost_rank2_factor,
        ),
        level_boost_rank3_factor: d(
            settings,
            "levelBoostRank3Factor",
            defaults.level_boost_rank3_factor,
        ),
        main_stat_thresholds: HashMap::from([
            (
                StatName::Speed,
                i(
                    settings,
                    "mainStatThresholdSpeed",
                    defaults.main_stat_thresholds[&StatName::Speed],
                ),
            ),
            (
                StatName::Stamina,
                i(
                    settings,
                    "mainStatThresholdStamina",
                    defaults.main_stat_thresholds[&StatName::Stamina],
                ),
            ),
            (
                StatName::Power,
                i(
                    settings,
                    "mainStatThresholdPower",
                    defaults.main_stat_thresholds[&StatName::Power],
                ),
            ),
            (
                StatName::Guts,
                i(
                    settings,
                    "mainStatThresholdGuts",
                    defaults.main_stat_thresholds[&StatName::Guts],
                ),
            ),
            (
                StatName::Wit,
                i(
                    settings,
                    "mainStatThresholdWit",
                    defaults.main_stat_thresholds[&StatName::Wit],
                ),
            ),
        ]),
        main_stat_bonus_magnitude: d(
            settings,
            "mainStatBonusMagnitude",
            defaults.main_stat_bonus_magnitude,
        ),
        relationship_orange_value: d(
            settings,
            "relationshipOrangeValue",
            defaults.relationship_orange_value,
        ),
        relationship_green_value: d(
            settings,
            "relationshipGreenValue",
            defaults.relationship_green_value,
        ),
        relationship_blue_value: d(
            settings,
            "relationshipBlueValue",
            defaults.relationship_blue_value,
        ),
        relationship_diminishing_factor: d(
            settings,
            "relationshipDiminishingFactor",
            defaults.relationship_diminishing_factor,
        ),
        relationship_early_game_bonus: d(
            settings,
            "relationshipEarlyGameBonus",
            defaults.relationship_early_game_bonus,
        ),
        relationship_trainer_support_bonus: d(
            settings,
            "relationshipTrainerSupportBonus",
            defaults.relationship_trainer_support_bonus,
        ),
        skill_hint_per_hint_score: d(
            settings,
            "skillHintPerHintScore",
            defaults.skill_hint_per_hint_score,
        ),
        skill_hint_override_score: d(
            settings,
            "skillHintOverrideScore",
            defaults.skill_hint_override_score,
        ),
        stat_weight_with_bars: d(settings, "statWeightWithBars", defaults.stat_weight_with_bars),
        stat_weight_without_bars: d(
            settings,
            "statWeightWithoutBars",
            defaults.stat_weight_without_bars,
        ),
        relationship_weight_with_bars: d(
            settings,
            "relationshipWeightWithBars",
            defaults.relationship_weight_with_bars,
        ),
        misc_weight: d(settings, "miscWeight", defaults.misc_weight),
        junior_early_game_flat_bonus: d(
            settings,
            "juniorEarlyGameFlatBonus",
            defaults.junior_early_game_flat_bonus,
        ),
        relationship_scale: d(settings, "relationshipScale", defaults.relationship_scale),
        rainbow_multiplier_enabled: d(
            settings,
            "rainbowMultiplierEnabled",
            defaults.rainbow_multiplier_enabled,
        ),
        rainbow_multiplier_disabled: d(
            settings,
            "rainbowMultiplierDisabled",
            defaults.rainbow_multiplier_disabled,
        ),
        rainbow_per_instance_base: d(
            settings,
            "rainbowPerInstanceBase",
            defaults.rainbow_per_instance_base,
        ),
        rainbow_per_instance_decay: d(
            settings,
            "rainbowPerInstanceDecay",
            defaults.rainbow_per_instance_decay,
        ),
        anticipatory_min_fill_percent: d(
            settings,
            "anticipatoryMinFillPercent",
            defaults.anticipatory_min_fill_percent,
        ),
        anticipatory_coefficient: d(
            settings,
            "anticipatoryCoefficient",
            defaults.anticipatory_coefficient,
        ),
        anticipatory_cap: d(settings, "anticipatoryCap", defaults.anticipatory_cap),
        unity_fill_base_bonus: d(settings, "unityFillBaseBonus", defaults.unity_fill_base_bonus),
        unity_fill_per_gauge_bonus: d(
            settings,
            "unityFillPerGaugeBonus",
            defaults.unity_fill_per_gauge_bonus,
        ),
        unity_burst_base_bonus: d(settings, "unityBurstBaseBonus", defaults.unity_burst_base_bonus),
        unity_burst_per_gauge_bonus: d(
            settings,
            "unityBurstPerGaugeBonus",
            defaults.unity_burst_per_gauge_bonus,
        ),
        unity_fill_energy_penalty_per_gauge: d(
            settings,
            "unityFillEnergyPenaltyPerGauge",
            defaults.unity_fill_energy_penalty_per_gauge,
        ),
        unity_burst_energy_penalty_per_gauge: d(
            settings,
            "unityBurstEnergyPenaltyPerGauge",
            defaults.unity_burst_energy_penalty_per_gauge,
        ),
        unity_extreme_burst_base_bonus: d(
            settings,
            "unityExtremeBurstBaseBonus",
            defaults.unity_extreme_burst_base_bonus,
        ),
        unity_extreme_burst_per_gauge_bonus: d(
            settings,
            "unityExtremeBurstPerGaugeBonus",
            defaults.unity_extreme_burst_per_gauge_bonus,
        ),
    };
    constants.validate();
    constants
}
