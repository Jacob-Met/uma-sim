//! Lead competition (位置取り争い / CompeteTop).
//! Constants from umamusu.wiki / community mechanics notes (not GPL source).

use crate::hp::Strategy;

/// Front-runner (Nige) distance gap to enter lead competition.
pub const NIGE_DISTANCE_GAP_M: f64 = 3.75;
/// Oonige distance gap.
pub const OONIGE_DISTANCE_GAP_M: f64 = 5.0;

/// Lead competition window: after this many meters from the start.
pub const START_MIN_M: f64 = 150.0;
/// Window ends at the start of this section (1-based section index 6 → index 5 in 0-based).
pub const WINDOW_END_SECTION: f64 = 6.0;
/// Hard stop at start of section 9.
pub const HARD_END_SECTION: f64 = 9.0;

pub fn lead_comp_speed_bonus(guts: f64) -> f64 {
    (500.0 * guts).powf(0.6) * 0.0001
}

pub fn lead_comp_duration_s(guts: f64) -> f64 {
    (700.0 * guts).sqrt() * 0.012
}

pub fn lead_comp_hp_factor(strategy: Strategy, rushed: bool) -> f64 {
    match (strategy, rushed) {
        (Strategy::Nige, false) => 1.4,
        (Strategy::Nige, true) => 3.6,
        (Strategy::Oonige, false) => 3.5,
        (Strategy::Oonige, true) => 7.7,
        _ => 1.0,
    }
}

pub fn same_lead_group(a: Strategy, b: Strategy) -> bool {
    matches!(
        (a, b),
        (Strategy::Nige, Strategy::Nige) | (Strategy::Oonige, Strategy::Oonige)
    )
}

pub fn distance_gap_limit(strategy: Strategy) -> f64 {
    match strategy {
        Strategy::Oonige => OONIGE_DISTANCE_GAP_M,
        _ => NIGE_DISTANCE_GAP_M,
    }
}

/// True when `pos` is inside the entry window [150m, section 6).
pub fn in_entry_window(pos: f64, section_len: f64) -> bool {
    pos >= START_MIN_M && pos < section_len * WINDOW_END_SECTION
}

pub fn past_hard_end(pos: f64, section_len: f64) -> bool {
    pos >= section_len * HARD_END_SECTION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guts_782_bonus_and_duration_positive() {
        let b = lead_comp_speed_bonus(750.72);
        let d = lead_comp_duration_s(750.72);
        assert!(b > 0.05 && b < 0.5, "bonus={b}");
        assert!(d > 1.0 && d < 20.0, "duration={d}");
    }
}
