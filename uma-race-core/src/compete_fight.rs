//! Compete-fight / dueling (追い比べ).
//! Constants from umamusu.wiki parameter notes (less RE-confirmed than lead competition).

/// Distance gap to count as a competition target (m).
pub const TARGET_DISTANCE_GAP_M: f64 = 3.0;
/// Speed gap required to trigger after sustained targeting (m/s).
pub const TRIGGER_SPEED_GAP_MS: f64 = 0.6;
/// How long a target must remain valid before compete-fight can start (s).
pub const TARGET_HOLD_S: f64 = 2.0;
/// Cannot start when remaining HP fraction is below this.
pub const START_MIN_HP_FRAC: f64 = 0.15;
/// Ends when remaining HP fraction drops below this.
pub const END_MIN_HP_FRAC: f64 = 0.05;

pub fn compete_speed_bonus(guts: f64) -> f64 {
    (200.0 * guts).powf(0.708) * 0.0001
}

pub fn compete_accel_bonus(guts: f64) -> f64 {
    (160.0 * guts).powf(0.59) * 0.0001
}

/// True when `pos` is on the course's last straight (inclusive start, exclusive end).
pub fn on_final_straight(pos: f64, last_start: f64, last_end: f64) -> bool {
    pos >= last_start && pos < last_end
}

/// Top 50% placement: place index 0 = first. For `n` horses, place < ceil(n/2).
pub fn is_top_half(place: usize, field_size: usize) -> bool {
    if field_size == 0 {
        return false;
    }
    let half = (field_size + 1) / 2; // ceil(n/2)
    place < half
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guts_bonuses_positive() {
        let s = compete_speed_bonus(800.0);
        let a = compete_accel_bonus(800.0);
        assert!(s > 0.01 && s < 2.0, "speed={s}");
        assert!(a > 0.001 && a < 1.0, "accel={a}");
    }

    #[test]
    fn top_half_two_horse() {
        assert!(is_top_half(0, 2));
        assert!(!is_top_half(1, 2));
    }
}
