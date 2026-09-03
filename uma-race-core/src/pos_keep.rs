//! Position-keep constants and pack state machine.
//! Constants: `research/race_position_keep.json` and KuromiAK.

use crate::hp::Strategy;
use crate::rng::PrandoRng;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PosKeepMode {
    #[default]
    None,
    Approximate,
    Virtual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosKeepState {
    None = 0,
    PaceUp = 1,
    PaceDown = 2,
    SpeedUp = 3,
    Overtake = 4,
}

const BASE_MIN: [f64; 6] = [0.0, 0.0, 3.0, 6.5, 7.5, 0.0]; // idx by Strategy as i32
const BASE_MAX: [f64; 6] = [0.0, 0.0, 5.0, 7.0, 8.0, 0.0];

pub fn course_factor(distance: f64) -> f64 {
    0.0008 * (distance - 1000.0) + 1.0
}

pub fn min_threshold(strategy: Strategy, distance: f64) -> f64 {
    let base = BASE_MIN[strategy as usize];
    if matches!(strategy, Strategy::Senkou) {
        base
    } else {
        base * course_factor(distance)
    }
}

pub fn max_threshold(strategy: Strategy, distance: f64) -> f64 {
    BASE_MAX[strategy as usize] * course_factor(distance)
}

pub fn pos_keep_speed_coef(state: PosKeepState) -> f64 {
    match state {
        PosKeepState::None => 1.0,
        PosKeepState::PaceUp | PosKeepState::SpeedUp => 1.04,
        PosKeepState::PaceDown => 0.915,
        PosKeepState::Overtake => 1.05,
    }
}

pub fn speed_up_overtake_wit_chance(wisdom: f64) -> f64 {
    // research/race_position_keep.json — clamp: wisdom < 10 → log10(0.1*w) < 0.
    (0.2 * (0.1 * wisdom).log10()).clamp(0.0, 1.0)
}

pub fn pace_up_wit_chance(wisdom: f64) -> f64 {
    (0.15 * (0.1 * wisdom).log10()).clamp(0.0, 1.0)
}

/// Lead gap to trigger SpeedUp (meters). Oonige uses a wider gap.
pub fn speed_up_gap_threshold(strategy: Strategy) -> f64 {
    match strategy {
        Strategy::Oonige => 17.5,
        _ => 4.5,
    }
}

/// Lead gap to exit Overtake (meters).
pub fn overtake_exit_gap_threshold(strategy: Strategy) -> f64 {
    match strategy {
        Strategy::Oonige => 27.5,
        _ => 10.0,
    }
}

/// Front-runner (Nige/Oonige) SpeedUp / Overtake state machine.
///
/// `gap_ahead` = own_pos − second_place_pos when `am_i_pacer`, else ignored on enter
/// (Overtake enter only needs a wit check). On Overtake exit while leading, gap uses the
/// same definition.
///
/// Returns `(new_state, exit_pos, exit_dist, next_timer_t)`.
/// `next_timer_t == -1.0` → leave timer unchanged; `-2`/`-3` → cooldown (counts up each frame).
pub fn tick_nige_pos_keep(
    state: PosKeepState,
    strategy: Strategy,
    am_i_pacer: bool,
    gap_ahead: f64,
    pos: f64,
    exit_pos: f64,
    exit_dist: f64,
    timer_ready: bool,
    wisdom: f64,
    rng: &mut PrandoRng,
    section_len: f64,
) -> (PosKeepState, f64, f64, f64) {
    if !matches!(strategy, Strategy::Nige | Strategy::Oonige) {
        return (state, exit_pos, exit_dist, -1.0);
    }
    let speed_th = speed_up_gap_threshold(strategy);
    let overtake_th = overtake_exit_gap_threshold(strategy);

    match state {
        PosKeepState::None => {
            if !timer_ready {
                return (state, exit_pos, exit_dist, -1.0);
            }
            if am_i_pacer {
                if gap_ahead < speed_th && rng.random() < speed_up_overtake_wit_chance(wisdom) {
                    return (
                        PosKeepState::SpeedUp,
                        pos + section_len.floor(),
                        exit_dist,
                        -1.0,
                    );
                }
            } else if rng.random() < speed_up_overtake_wit_chance(wisdom) {
                return (
                    PosKeepState::Overtake,
                    pos + section_len.floor(),
                    exit_dist,
                    -1.0,
                );
            }
            (PosKeepState::None, exit_pos, exit_dist, -2.0)
        }
        PosKeepState::SpeedUp => {
            if pos >= exit_pos || (am_i_pacer && gap_ahead >= speed_th) {
                (PosKeepState::None, exit_pos, exit_dist, -3.0)
            } else {
                (state, exit_pos, exit_dist, -1.0)
            }
        }
        PosKeepState::Overtake => {
            if pos >= exit_pos || (am_i_pacer && gap_ahead >= overtake_th) {
                (PosKeepState::None, exit_pos, exit_dist, -3.0)
            } else {
                (state, exit_pos, exit_dist, -1.0)
            }
        }
        // Pack states should not appear on a front-runner; clear them.
        other => (other, exit_pos, exit_dist, -1.0),
    }
}

/// Advance Virtual/Approximate position-keep for a non-leading pack horse.
/// Returns `(new_state, exit_pos, exit_dist, next_timer_t)`.
/// `next_timer_t == -1.0` means leave the timer unchanged.
pub fn tick_pack_pos_keep(
    state: PosKeepState,
    strategy: Strategy,
    behind: f64,
    min_th: f64,
    max_th: f64,
    pos: f64,
    exit_pos: f64,
    exit_dist: f64,
    timer_ready: bool,
    has_speed_skills: bool,
    wisdom: f64,
    rng: &mut PrandoRng,
    section_len: f64,
) -> (PosKeepState, f64, f64, f64) {
    if matches!(strategy, Strategy::Nige | Strategy::Oonige) {
        return (PosKeepState::None, exit_pos, exit_dist, -1.0);
    }
    match state {
        PosKeepState::None => {
            if !timer_ready {
                return (state, exit_pos, exit_dist, -1.0);
            }
            if behind > max_th {
                if rng.random() < pace_up_wit_chance(wisdom) {
                    let exit_d = rng.random() * (max_th - min_th) + min_th;
                    return (
                        PosKeepState::PaceUp,
                        pos + section_len.floor(),
                        exit_d,
                        -1.0, // cooldown set on exit only
                    );
                }
            } else if behind < min_th && !has_speed_skills {
                let exit_d = rng.random() * (max_th - min_th) + min_th;
                return (
                    PosKeepState::PaceDown,
                    pos + section_len.floor(),
                    exit_d,
                    -1.0,
                );
            }
            (PosKeepState::None, exit_pos, exit_dist, -2.0)
        }
        PosKeepState::PaceUp => {
            if pos >= exit_pos || behind < exit_dist {
                (PosKeepState::None, exit_pos, exit_dist, -3.0)
            } else {
                (state, exit_pos, exit_dist, -1.0)
            }
        }
        PosKeepState::PaceDown => {
            if pos >= exit_pos || behind > exit_dist || has_speed_skills {
                (PosKeepState::None, exit_pos, exit_dist, -3.0)
            } else {
                (state, exit_pos, exit_dist, -1.0)
            }
        }
        other => (other, exit_pos, exit_dist, -1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn course_factor_at_1000_is_one() {
        assert!((course_factor(1000.0) - 1.0).abs() < 1e-12);
        assert!((course_factor(2000.0) - 1.8).abs() < 1e-12);
    }

    #[test]
    fn senkou_min_threshold_ignores_course_factor() {
        assert!((min_threshold(Strategy::Senkou, 2000.0) - 3.0).abs() < 1e-12);
        let oikomi = min_threshold(Strategy::Oikomi, 2000.0);
        assert!((oikomi - 7.5 * 1.8).abs() < 1e-12);
    }

    #[test]
    fn speed_coefs_match_research_json() {
        assert!((pos_keep_speed_coef(PosKeepState::PaceDown) - 0.915).abs() < 1e-12);
        assert!((pos_keep_speed_coef(PosKeepState::Overtake) - 1.05).abs() < 1e-12);
    }
}
