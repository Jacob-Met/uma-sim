//! Parent-farming terminal utility (world-model evaluation of career outcomes).
//! Mirrors Kotlin `TerminalUtility.kt` — keep the cliff constants in lockstep.

use crate::state::StatName;
use std::collections::HashMap;

pub const BLUE_CLIFF_600: i32 = 600;
pub const BLUE_CLIFF_1100: i32 = 1100;
pub const GRADE_CLIFF_B: i32 = 6_500;
pub const GRADE_CLIFF_SS: i32 = 17_500;
pub const GRADE_CLIFF_UE: i32 = 28_800;

pub const PHI_BELOW_600: f64 = 4.5;
pub const PHI_MID: f64 = 6.97;
pub const PHI_AT_1100: f64 = 8.72;

pub const PSI_BELOW_B: f64 = 4.5;
pub const PSI_MID: f64 = 6.85;
pub const PSI_SS: f64 = 8.70;
pub const PSI_UE: f64 = 9.0;

/// Approximate gold-skill score yield used by the minimal end-of-career skill shop.
pub const GOLD_SKILL_SCORE_PER_SP: f64 = 3.3;

pub fn phi_blue(stat_value: i32) -> f64 {
    if stat_value < BLUE_CLIFF_600 {
        PHI_BELOW_600
    } else if stat_value < BLUE_CLIFF_1100 {
        PHI_MID
    } else {
        PHI_AT_1100
    }
}

pub fn mean_phi_blue(stats: &HashMap<StatName, i32>) -> f64 {
    let sum: f64 = StatName::ALL
        .iter()
        .map(|s| phi_blue(*stats.get(s).unwrap_or(&0)))
        .sum();
    sum / StatName::ALL.len() as f64
}

pub fn mean_phi_blue_stats(speed: i32, stamina: i32, power: i32, guts: i32, wit: i32) -> f64 {
    mean_phi_blue(&HashMap::from([
        (StatName::Speed, speed),
        (StatName::Stamina, stamina),
        (StatName::Power, power),
        (StatName::Guts, guts),
        (StatName::Wit, wit),
    ]))
}

pub fn psi_grade(score: i32) -> f64 {
    if score < GRADE_CLIFF_B {
        PSI_BELOW_B
    } else if score < GRADE_CLIFF_SS {
        PSI_MID
    } else if score < GRADE_CLIFF_UE {
        PSI_SS
    } else {
        PSI_UE
    }
}

pub fn terminal_utility(
    stats: &HashMap<StatName, i32>,
    career_score: i32,
    appearance_probs: &[f64],
) -> f64 {
    mean_phi_blue(stats) + psi_grade(career_score) + appearance_probs.iter().sum::<f64>()
}

/// Minimal skill shop: convert remaining SP into career score at a flat gold-skill rate.
/// Returns `(score_gained, sp_spent)`.
pub fn spend_skill_points(sp: i32, pts_per_sp: f64) -> (i32, i32) {
    if sp <= 0 || pts_per_sp <= 0.0 {
        return (0, 0);
    }
    let gained = (sp as f64 * pts_per_sp).round() as i32;
    (gained, sp)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CareerTerminalRecord {
    pub seed: i64,
    pub scenario: String,
    pub trainee: String,
    pub u: f64,
    pub phi_blue: f64,
    pub psi_grade: f64,
    pub grade: String,
    pub score: i32,
    pub score_before_shop: i32,
    pub sp_spent: i32,
    pub sp_remaining: i32,
    pub brackets: BracketHits,
    pub stats: TerminalStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BracketHits {
    pub at_or_above_600: i32,
    pub at_or_above_1100: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalStats {
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
}

/// Evaluate terminal parent-farming utility after a minimal SP→score skill shop.
pub fn evaluate_career_terminal(
    seed: i64,
    scenario: &str,
    trainee: &str,
    speed: i32,
    stamina: i32,
    power: i32,
    guts: i32,
    wit: i32,
    skill_points: i32,
    appearance_probs: &[f64],
) -> CareerTerminalRecord {
    use super::rank_estimate::{estimate_rank, score_to_rank_label, RankAptitudes};

    let stats_map = HashMap::from([
        (StatName::Speed, speed),
        (StatName::Stamina, stamina),
        (StatName::Power, power),
        (StatName::Guts, guts),
        (StatName::Wit, wit),
    ]);
    let apt = RankAptitudes {
        turf: "A".into(),
        dirt: "A".into(),
        sprint: "A".into(),
        mile: "A".into(),
        medium: "A".into(),
        long: "A".into(),
        front: "A".into(),
        pace: "A".into(),
        late: "A".into(),
        end: "A".into(),
    };
    let before = estimate_rank(speed, stamina, power, guts, wit, &[], &apt, 0);
    let (shop_score, sp_spent) = spend_skill_points(skill_points, GOLD_SKILL_SCORE_PER_SP);
    let score = before.total_score + shop_score;
    let grade = score_to_rank_label(score).to_string();
    let phi = mean_phi_blue(&stats_map);
    let psi = psi_grade(score);
    let u = phi + psi + appearance_probs.iter().sum::<f64>();
    let at_600 = StatName::ALL
        .iter()
        .filter(|s| *stats_map.get(s).unwrap_or(&0) >= BLUE_CLIFF_600)
        .count() as i32;
    let at_1100 = StatName::ALL
        .iter()
        .filter(|s| *stats_map.get(s).unwrap_or(&0) >= BLUE_CLIFF_1100)
        .count() as i32;
    CareerTerminalRecord {
        seed,
        scenario: scenario.to_string(),
        trainee: trainee.to_string(),
        u,
        phi_blue: phi,
        psi_grade: psi,
        grade,
        score,
        score_before_shop: before.total_score,
        sp_spent,
        sp_remaining: 0,
        brackets: BracketHits {
            at_or_above_600: at_600,
            at_or_above_1100: at_1100,
        },
        stats: TerminalStats {
            speed,
            stamina,
            power,
            guts,
            wit,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_matches_condor_allocations() {
        // Condor baseline: 1246/614/749/671/842 → 7.32
        let condor = mean_phi_blue_stats(1246, 614, 749, 671, 842);
        assert!((condor - 7.32).abs() < 1e-9, "condor={condor}");

        // 2×1100 + 3×600 at same 4122 budget → 7.67
        let two = mean_phi_blue_stats(1100, 1100, 600, 600, 722);
        assert!((two - 7.67).abs() < 1e-9, "two={two}");

        // 3×1100 at 4122 budget → 7.03 (1122+1100+1100+400+400)
        let three = mean_phi_blue_stats(1122, 1100, 1100, 400, 400);
        assert!((three - 7.03).abs() < 0.01, "three={three}");
    }

    #[test]
    fn skill_shop_converts_sp() {
        let (gained, spent) = spend_skill_points(2056, GOLD_SKILL_SCORE_PER_SP);
        assert_eq!(spent, 2056);
        assert_eq!(gained, (2056.0_f64 * 3.3).round() as i32);
    }

    #[test]
    fn evaluate_produces_nonzero_u() {
        let rec = evaluate_career_terminal(
            1,
            "ura",
            "Test",
            1100,
            1100,
            600,
            600,
            722,
            2000,
            &[],
        );
        assert!(rec.u > 0.0, "u={}", rec.u);
        assert!(rec.sp_spent > 0);
        assert!(rec.score > rec.score_before_shop);
    }
}
