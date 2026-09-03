use std::collections::{HashMap, HashSet};

use super::decision_context::DecisionContext;

pub mod aptitude_ordinal {
    pub const G: i32 = 0;
    pub const F: i32 = 1;
    pub const E: i32 = 2;
    pub const D: i32 = 3;
    pub const C: i32 = 4;
    pub const B: i32 = 5;
    pub const A: i32 = 6;
    pub const S: i32 = 7;
    pub const MIN_GATE: i32 = C;
}

pub use aptitude_ordinal as AptitudeOrdinal;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UmaAptitudeSnapshot {
    pub style_aptitudes: HashMap<i32, i32>,
    pub distance_aptitudes: HashMap<i32, i32>,
    pub surface_aptitudes: HashMap<i32, i32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkillScoreInputs {
    pub community_tier: Option<i32>,
    pub eval_points: i32,
    pub price: i32,
    pub is_recovery: bool,
    pub is_inherited_unique: bool,
    pub is_user_planned: bool,
    pub running_style_ordinal: Option<i32>,
    pub inferred_running_style_ordinals: Vec<i32>,
    pub track_distance_ordinal: Option<i32>,
    pub track_surface_ordinal: Option<i32>,
    pub style_matches_preference: bool,
    pub stamina_heavy_distance: bool,
    pub prioritize_recovery_for_stamina: bool,
    pub is_gold: bool,
}

impl SkillScoreInputs {
    pub fn new() -> Self {
        Self {
            price: 1,
            style_matches_preference: true,
            prioritize_recovery_for_stamina: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillScoreResult {
    pub score: f64,
    pub gate_reason: Option<String>,
}

impl SkillScoreResult {
    pub fn gated(&self) -> bool {
        self.gate_reason.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillDrainCandidate {
    pub name: String,
    pub price: i32,
    pub evaluation_points: i32,
    pub uma_score: f64,
}

pub fn skill_score_per_point(score: f64, price: i32) -> f64 {
    if price > 0 {
        score / price as f64
    } else {
        0.0
    }
}

pub fn is_eval_heavy_skill_profile(objective_profile: &str) -> bool {
    let p = objective_profile.trim().to_lowercase().replace(' ', "_");
    matches!(
        p.as_str(),
        "spark_farming" | "spark" | "sparks" | "career_score" | "rank" | "score"
    )
}

pub fn uma_drain_score_floor(objective_profile: &str) -> f64 {
    if is_eval_heavy_skill_profile(objective_profile) {
        0.0
    } else {
        45.0
    }
}

pub fn aptitude_gate_reason(
    inputs: &SkillScoreInputs,
    aptitudes: &UmaAptitudeSnapshot,
) -> Option<String> {
    if inputs.is_user_planned {
        return None;
    }

    if let Some(style) = inputs.running_style_ordinal {
        let apt = aptitudes
            .style_aptitudes
            .get(&style)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        if apt < AptitudeOrdinal::MIN_GATE {
            return Some(format!("style_{style} apt_{}", apt_label(apt)));
        }
    }

    if let Some(dist) = inputs.track_distance_ordinal {
        let apt = aptitudes
            .distance_aptitudes
            .get(&dist)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        if apt < AptitudeOrdinal::MIN_GATE {
            return Some(format!("distance_{dist} apt_{}", apt_label(apt)));
        }
    }

    if let Some(surf) = inputs.track_surface_ordinal {
        let apt = aptitudes
            .surface_aptitudes
            .get(&surf)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        if apt < AptitudeOrdinal::MIN_GATE {
            return Some(format!("surface_{surf} apt_{}", apt_label(apt)));
        }
    }

    None
}

fn apt_label(ordinal: i32) -> &'static str {
    match ordinal {
        AptitudeOrdinal::S => "S",
        AptitudeOrdinal::A => "A",
        AptitudeOrdinal::B => "B",
        AptitudeOrdinal::C => "C",
        AptitudeOrdinal::D => "D",
        AptitudeOrdinal::E => "E",
        AptitudeOrdinal::F => "F",
        _ => "G",
    }
}

fn aptitude_soft_multiplier(inputs: &SkillScoreInputs, aptitudes: &UmaAptitudeSnapshot) -> f64 {
    let mut mult = 1.0;
    if let Some(style) = inputs.running_style_ordinal {
        let apt = aptitudes
            .style_aptitudes
            .get(&style)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        mult *= ratio_for_apt(apt);
    }
    if let Some(dist) = inputs.track_distance_ordinal {
        let apt = aptitudes
            .distance_aptitudes
            .get(&dist)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        mult *= ratio_for_apt(apt);
    }
    if let Some(surf) = inputs.track_surface_ordinal {
        let apt = aptitudes
            .surface_aptitudes
            .get(&surf)
            .copied()
            .unwrap_or(AptitudeOrdinal::G);
        mult *= ratio_for_apt(apt);
    }
    mult.clamp(0.35, 1.15)
}

fn ratio_for_apt(apt: i32) -> f64 {
    if apt >= AptitudeOrdinal::S {
        1.1
    } else if apt >= AptitudeOrdinal::A {
        1.1
    } else if apt >= AptitudeOrdinal::B {
        0.95
    } else if apt >= AptitudeOrdinal::C {
        0.9
    } else if apt >= AptitudeOrdinal::D {
        0.75
    } else if apt >= AptitudeOrdinal::E {
        0.65
    } else if apt >= AptitudeOrdinal::F {
        0.55
    } else {
        0.45
    }
}

fn tier_band_score(tier: Option<i32>) -> f64 {
    match tier {
        Some(1) => 1000.0,
        Some(2) => 750.0,
        Some(3) => 400.0,
        Some(4) => 200.0,
        _ => 80.0,
    }
}

pub fn score_skill_for_uma(
    ctx: &DecisionContext,
    inputs: &SkillScoreInputs,
    aptitudes: &UmaAptitudeSnapshot,
) -> SkillScoreResult {
    if let Some(gate) = aptitude_gate_reason(inputs, aptitudes) {
        return SkillScoreResult {
            score: 0.0,
            gate_reason: Some(gate),
        };
    }

    let eval_heavy = is_eval_heavy_skill_profile(&ctx.objective_profile);
    let w = ctx.objective_weights().normalized();
    let mut score = 0.0;

    if !eval_heavy {
        score += tier_band_score(inputs.community_tier)
            * (w.pvp_raceability + w.stat_targets + 0.5 * w.scenario_completion);
    }

    let eval_weight = if eval_heavy {
        18.0 * (w.career_score + w.spark_quality + 0.4)
    } else {
        4.0 * (w.career_score + w.spark_quality + 0.25)
    };
    score += inputs.eval_points as f64 * eval_weight;

    score *= aptitude_soft_multiplier(inputs, aptitudes);

    if inputs.is_recovery && inputs.stamina_heavy_distance && inputs.prioritize_recovery_for_stamina
    {
        score *= 1.5;
    }

    if inputs.is_inherited_unique {
        score += if eval_heavy {
            inputs.eval_points as f64 * 0.6
        } else {
            180.0 * w.pvp_raceability + inputs.eval_points as f64 * 0.2
        };
    }

    if inputs.style_matches_preference {
        score *= 1.12;
    }

    if inputs.is_gold {
        score *= 1.05;
    }

    SkillScoreResult {
        score: score.max(0.0),
        gate_reason: None,
    }
}

pub fn calculate_profile_aware_drain_purchases(
    candidates: &[SkillDrainCandidate],
    budget: i32,
    min_uma_score: f64,
    already_planned: &[String],
    blacklist: &[String],
    max_budget: i32,
) -> Vec<(String, i32)> {
    if budget <= 0 || budget > max_budget {
        return Vec::new();
    }

    let planned: HashSet<&str> = already_planned.iter().map(String::as_str).collect();
    let blocked: HashSet<&str> = blacklist.iter().map(String::as_str).collect();
    let affordable: Vec<&SkillDrainCandidate> = candidates
        .iter()
        .filter(|c| {
            !planned.contains(c.name.as_str())
                && !blocked.contains(c.name.as_str())
                && (1..=budget).contains(&c.price)
                && (min_uma_score <= 0.0 || c.uma_score >= min_uma_score || c.uma_score <= 0.0)
        })
        .collect();

    if affordable.is_empty() {
        return Vec::new();
    }

    if affordable.iter().map(|c| c.price).sum::<i32>() <= budget {
        let mut result: Vec<(String, i32)> = affordable
            .iter()
            .map(|c| (c.name.clone(), c.price))
            .collect();
        result.sort_by_key(|(_, price)| *price);
        return result;
    }

    let n = affordable.len();
    let mut best_spend = vec![0i32; (budget + 1) as usize];
    let mut best_uma = vec![0.0f64; (budget + 1) as usize];
    let mut keep = vec![vec![false; (budget + 1) as usize]; n];

    for (index, skill) in affordable.iter().enumerate() {
        for remaining in (skill.price..=budget).rev() {
            let rem = remaining as usize;
            let prev = remaining - skill.price;
            let spend = best_spend[prev as usize] + skill.price;
            let uma_val =
                best_uma[prev as usize] + skill.uma_score.max(skill.evaluation_points as f64);
            if spend > best_spend[rem] || (spend == best_spend[rem] && uma_val > best_uma[rem]) {
                best_spend[rem] = spend;
                best_uma[rem] = uma_val;
                keep[index][rem] = true;
            }
        }
    }

    let mut result = Vec::new();
    let mut remaining = budget;
    for index in (0..n).rev() {
        let rem = remaining as usize;
        if !keep[index][rem] {
            continue;
        }
        let skill = affordable[index];
        result.push((skill.name.clone(), skill.price));
        remaining -= skill.price;
    }
    result.sort_by_key(|(_, price)| *price);
    result
}

pub fn score_grand_live_training_tokens(
    ctx: &DecisionContext,
    token_gains: &HashMap<String, i32>,
) -> f64 {
    if token_gains.is_empty() {
        return 0.0;
    }
    let mut raw = 0.0;
    for (label, gain) in token_gains {
        if *gain <= 0 {
            continue;
        }
        let deficit_weight = if let Some(total) = ctx.token_totals.get(label).copied() {
            if total > 0 {
                1.0 + 100.0 / total.max(1) as f64
            } else {
                1.25
            }
        } else {
            1.25
        };
        raw += *gain as f64 * deficit_weight * 6.0;
    }
    if !ctx.is_hype_maxed && (0..=4).contains(&ctx.days_to_concert) {
        raw *= 1.4;
    } else if !ctx.is_hype_maxed && (5..=8).contains(&ctx.days_to_concert) {
        raw *= 1.15;
    }
    let w = ctx.objective_weights().normalized();
    raw * (w.scenario_completion + 0.35 * w.stat_targets + 0.15 * w.spark_quality)
}

pub fn format_grand_live_token_score_breakdown(
    ctx: &DecisionContext,
    token_gains: &HashMap<String, i32>,
) -> String {
    if token_gains.is_empty() {
        return "tokens=0".to_string();
    }
    let mut parts = Vec::new();
    for (label, gain) in token_gains {
        if *gain <= 0 {
            continue;
        }
        let total = ctx.token_totals.get(label);
        let pool = total
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".to_string());
        parts.push(format!("{label} +{gain} (pool={pool})"));
    }
    let total_score = score_grand_live_training_tokens(ctx, token_gains);
    let concert_note = if !ctx.is_hype_maxed && (0..=4).contains(&ctx.days_to_concert) {
        " concertBoost=1.4x"
    } else {
        ""
    };
    let rounded = (total_score * 10.0).round() / 10.0;
    format!("{} tokenScore={rounded}{concert_note}", parts.join(", "))
}
