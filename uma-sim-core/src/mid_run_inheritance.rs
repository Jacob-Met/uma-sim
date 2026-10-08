//! Mid-run inheritance (Classic / Senior Early April).
//!
//! Odds: `base_odds(stars) × (1 + compat/100)` from `parent_farming_utility.md`.
//! Blue mid-run rolls: 1★ 1–10, 2★ 1–16, 3★ 1–28 (Crazyfellow gene table).
//! Pink/red hidden value: +1…+5 aptitude steps (mid-run may reach S).

use crate::legacy::{
    parse_entry_public, raise_aptitude_letter_uncapped, LegacyApplicator, LegacyFactorContext,
};
use crate::rng::SimRandom;
use crate::state::{CareerState, SimDate};

/// Overall compatibility display grade (◎ / 〇 / △).
pub fn compatibility_grade(score: i32) -> &'static str {
    if score > 150 {
        "◎"
    } else if score >= 51 {
        "〇"
    } else {
        "△"
    }
}

/// Classic Early April (year 2) and Senior Early April (year 3).
pub fn is_mid_run_inspiration_date(date: &SimDate) -> bool {
    (date.year == 2 || date.year == 3) && date.month == 4 && date.half == 1
}

pub fn base_proc_rate(category: &str, stars: i32) -> f64 {
    let s = stars.clamp(1, 3) as usize;
    let rates: [f64; 3] = match category {
        "blue" => [0.70, 0.80, 0.90],
        "green" | "scenario" => [0.05, 0.10, 0.15],
        "skill" | "white" | "other" => [0.03, 0.06, 0.09],
        "race" => [0.01, 0.02, 0.03],
        "pink" | "red" => [0.01, 0.03, 0.05],
        _ => [0.03, 0.06, 0.09],
    };
    rates[s - 1]
}

pub fn inheritance_odds(category: &str, stars: i32, compatibility: i32) -> f64 {
    let base = base_proc_rate(category, stars);
    (base * (1.0 + compatibility as f64 / 100.0)).clamp(0.0, 1.0)
}

pub fn blue_mid_run_stat_range(stars: i32) -> (i32, i32) {
    match stars.clamp(1, 3) {
        1 => (1, 10),
        2 => (1, 16),
        _ => (1, 28),
    }
}

fn roll_uniform_inclusive(rng: &mut SimRandom, lo: i32, hi: i32) -> i32 {
    if hi <= lo {
        return lo;
    }
    lo + rng.next_int_until(hi - lo + 1)
}

/// Resolve one Mid-run Inspiration turn against the run's legacy factors.
pub fn apply_mid_run_inspiration(
    state: &CareerState,
    rng: &mut SimRandom,
) -> (CareerState, Vec<String>) {
    let mut next = state.clone();
    let mut lines = Vec::new();
    let label = if state.date.year == 2 {
        "Classic Early April"
    } else {
        "Senior Early April"
    };
    lines.push(format!("Mid-run Inspiration ({label})!"));

    let compat = state.meta.compatibility_score;
    let factors = state.meta.effective_legacy_factors();
    if factors.is_empty() {
        lines.push("No legacy factors to inherit.".into());
        next.legacy.inspiration_events_done += 1;
        return (next, lines);
    }

    for entry in &factors {
        let (id, stars) = parse_entry_public(entry);
        let meta = LegacyFactorContext::lookup(&id);
        let category = meta
            .as_ref()
            .map(|m| m.category.as_str())
            .unwrap_or_else(|| {
                if LegacyApplicator::blue_stat_public(&id).is_some() {
                    "blue"
                } else if id.contains(":pink:") || id.contains(":red:") {
                    "pink"
                } else if id.contains(":green:") || id.contains(":scenario:") {
                    "green"
                } else if id.contains(":race:") {
                    "race"
                } else if id.contains(":skill:") {
                    "skill"
                } else {
                    "other"
                }
            });
        let odds = inheritance_odds(category, stars, compat);
        if !rng.next_boolean(odds) {
            continue;
        }
        match category {
            "blue" => {
                let stat = meta
                    .as_ref()
                    .and_then(|m| m.stat_key.clone())
                    .or_else(|| LegacyApplicator::blue_stat_public(&id))
                    .unwrap_or_else(|| "speed".into());
                let (lo, hi) = blue_mid_run_stat_range(stars);
                let gain = roll_uniform_inclusive(rng, lo, hi);
                next.stats = match stat.as_str() {
                    "stamina" => next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Stamina, gain),
                    "power" => next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Power, gain),
                    "guts" => next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Guts, gain),
                    "wit" => next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Wit, gain),
                    _ => next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Speed, gain),
                };
                lines.push(format!("Inherited blue {stat} +{gain} ({stars}★)"));
            }
            "pink" | "red" => {
                let tag = meta
                    .as_ref()
                    .and_then(|m| m.pink_tag.clone())
                    .unwrap_or_else(|| "turf".into());
                let hidden = roll_uniform_inclusive(rng, 1, 5);
                let cur = next
                    .legacy
                    .aptitudes
                    .get(&tag)
                    .cloned()
                    .unwrap_or_else(|| "G".into());
                let raised = raise_aptitude_letter_uncapped(&cur, hidden);
                next.legacy.aptitudes.insert(tag.clone(), raised.clone());
                lines.push(format!(
                    "Inherited pink {tag} +{hidden} hidden ({cur}→{raised}, {stars}★)"
                ));
            }
            "skill" | "white" => {
                if let Some(skill) = meta.as_ref().and_then(|m| m.skill_id.clone()) {
                    let hint = roll_uniform_inclusive(rng, 1, 5);
                    let key = if skill.starts_with("skill:") {
                        skill.clone()
                    } else {
                        format!("skill:{skill}")
                    };
                    let cur = next.hint_levels.get(&key).copied().unwrap_or(0);
                    next.hint_levels.insert(key.clone(), cur + hint);
                    lines.push(format!("Inherited white hint {key} +{hint} ({stars}★)"));
                }
            }
            "green" | "scenario" => {
                if let Some(skill) = meta.as_ref().and_then(|m| m.skill_id.clone()) {
                    let hint = roll_uniform_inclusive(rng, 1, 5);
                    let key = if skill.starts_with("skill:") {
                        skill.clone()
                    } else {
                        format!("skill:{skill}")
                    };
                    let cur = next.hint_levels.get(&key).copied().unwrap_or(0);
                    next.hint_levels.insert(key.clone(), cur + hint);
                    lines.push(format!("Inherited green hint {key} +{hint} ({stars}★)"));
                } else {
                    // Scenario genes often grant small dual-stat rolls (10/20/30).
                    let a = [10, 20, 30][rng.next_int_until(3) as usize];
                    let b = [10, 20, 30][rng.next_int_until(3) as usize];
                    next.stats = next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Speed, a);
                    next.stats = next
                        .stats
                        .with_delta(crate::state::TrainingFacility::Stamina, b);
                    lines.push(format!("Inherited scenario gene Speed +{a} / Stamina +{b}"));
                }
            }
            "race" => {
                let sp = 3 * stars.clamp(1, 3);
                next.skill_points += sp;
                lines.push(format!("Inherited race gene SP +{sp} ({stars}★)"));
            }
            _ => {}
        }
    }

    next.legacy.inspiration_events_done += 1;
    (next, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn april_dates_match_classic_and_senior() {
        assert!(is_mid_run_inspiration_date(&SimDate {
            year: 2,
            month: 4,
            half: 1
        }));
        assert!(is_mid_run_inspiration_date(&SimDate {
            year: 3,
            month: 4,
            half: 1
        }));
        assert!(!is_mid_run_inspiration_date(&SimDate {
            year: 2,
            month: 4,
            half: 2
        }));
        assert!(!is_mid_run_inspiration_date(&SimDate {
            year: 1,
            month: 4,
            half: 1
        }));
    }

    #[test]
    fn odds_scale_with_compatibility() {
        let base = inheritance_odds("blue", 3, 0);
        let boosted = inheritance_odds("blue", 3, 100);
        assert!((base - 0.90).abs() < 1e-9);
        assert!((boosted - 1.0).abs() < 1e-9 || boosted >= 0.99);
    }

    #[test]
    fn blue_ranges_match_crazyfellow_table() {
        assert_eq!(blue_mid_run_stat_range(1), (1, 10));
        assert_eq!(blue_mid_run_stat_range(2), (1, 16));
        assert_eq!(blue_mid_run_stat_range(3), (1, 28));
    }

    #[test]
    fn compatibility_grade_bands() {
        assert_eq!(compatibility_grade(0), "△");
        assert_eq!(compatibility_grade(50), "△");
        assert_eq!(compatibility_grade(51), "〇");
        assert_eq!(compatibility_grade(150), "〇");
        assert_eq!(compatibility_grade(151), "◎");
        assert_eq!(compatibility_grade(500), "◎");
    }
}
