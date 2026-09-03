//! HP model from `knowledge/mechanics/race_model.md` /
//! `research/race_model_constants.json`.

use crate::physics::Phase;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aptitude {
    S = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    F = 6,
    G = 7,
}

impl Aptitude {
    pub fn from_str_letter(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "S" => Some(Self::S),
            "A" => Some(Self::A),
            "B" => Some(Self::B),
            "C" => Some(Self::C),
            "D" => Some(Self::D),
            "E" => Some(Self::E),
            "F" => Some(Self::F),
            "G" => Some(Self::G),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    Nige = 1,
    Senkou = 2,
    Sasi = 3,
    Oikomi = 4,
    Oonige = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundCondition {
    Good = 1,
    Yielding = 2,
    Soft = 3,
    Heavy = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Turf = 1,
    Dirt = 2,
}

pub fn strategy_coef(strategy: Strategy) -> f64 {
    match strategy {
        Strategy::Nige => 0.95,
        Strategy::Senkou => 0.89,
        Strategy::Sasi => 1.0,
        Strategy::Oikomi => 0.995,
        Strategy::Oonige => 0.86,
    }
}

/// `MaxHP = 0.8 × StrategyCoef × Stamina + CourseDistance`
pub fn max_hp(strategy: Strategy, stamina: f64, distance_m: f64) -> f64 {
    0.8 * strategy_coef(strategy) * stamina + distance_m
}

pub fn guts_modifier(guts: f64) -> f64 {
    1.0 + 200.0 / (600.0 * guts).sqrt()
}

pub fn ground_modifier(surface: Surface, ground: GroundCondition) -> f64 {
    let row: [f64; 5] = match surface {
        Surface::Turf => [0.0, 1.0, 1.0, 1.02, 1.02],
        Surface::Dirt => [0.0, 1.0, 1.0, 1.01, 1.02],
    };
    row[ground as usize]
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StatusModifiers {
    pub rushed: bool,
    pub pace_down: bool,
    pub downhill: bool,
}

impl StatusModifiers {
    pub fn factor(self) -> f64 {
        let mut m = 1.0;
        if self.downhill {
            m *= 0.4;
        }
        if self.rushed {
            m *= 1.6;
        }
        if self.pace_down {
            m *= 0.6;
        }
        m
    }
}

/// HP consumed per second at `velocity`.
pub fn hp_per_second(
    velocity: f64,
    base_speed: f64,
    phase: Phase,
    guts: f64,
    surface: Surface,
    ground: GroundCondition,
    status: StatusModifiers,
) -> f64 {
    let guts_mod = if (phase as u8) >= (Phase::End as u8) {
        guts_modifier(guts)
    } else {
        1.0
    };
    20.0 * (velocity - base_speed + 12.0).powi(2) / 144.0
        * status.factor()
        * ground_modifier(surface, ground)
        * guts_mod
}

/// Threshold for `uniform(100000) <= threshold` spurt candidate accept.
/// `round((15 + 0.05 * wisdom) * 1000)`.
pub fn spurt_accept_threshold(wisdom: f64) -> u32 {
    ((15.0 + 0.05 * wisdom) * 1000.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::base_speed;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn constants() -> Value {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../research/race_model_constants.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn strategy_coefs_match_constants_file() {
        let c = &constants()["max_hp"]["strategy_coef"];
        assert_eq!(strategy_coef(Strategy::Nige), c["nige"].as_f64().unwrap());
        assert_eq!(strategy_coef(Strategy::Senkou), c["senkou"].as_f64().unwrap());
        assert_eq!(strategy_coef(Strategy::Sasi), c["sasi"].as_f64().unwrap());
        assert_eq!(strategy_coef(Strategy::Oikomi), c["oikomi"].as_f64().unwrap());
        assert_eq!(strategy_coef(Strategy::Oonige), c["oonige"].as_f64().unwrap());
    }

    #[test]
    fn max_hp_formula() {
        // 0.8 * 0.89 * 1000 + 1400 = 712 + 1400 = 2112
        let hp = max_hp(Strategy::Senkou, 1000.0, 1400.0);
        assert!((hp - 2112.0).abs() < 1e-9);
    }

    #[test]
    fn guts_modifier_example() {
        let g = guts_modifier(900.0);
        let expected = 1.0 + 200.0 / (600.0 * 900.0_f64).sqrt();
        assert!((g - expected).abs() < 1e-12);
    }

    #[test]
    fn hp_per_second_at_base_speed_is_20() {
        let d = 2000.0;
        let bs = base_speed(d);
        let rate = hp_per_second(
            bs,
            bs,
            Phase::Opening,
            900.0,
            Surface::Turf,
            GroundCondition::Good,
            StatusModifiers::default(),
        );
        // 20 * 12^2 / 144 = 20
        assert!((rate - 20.0).abs() < 1e-9);
    }

    #[test]
    fn spurt_accept_threshold_matches_doc() {
        // wisdom 900 → (15 + 45) * 1000 = 60000
        assert_eq!(spurt_accept_threshold(900.0), 60_000);
        assert_eq!(spurt_accept_threshold(0.0), 15_000);
    }

    #[test]
    fn status_modifiers_from_constants() {
        let c = &constants()["hp_per_second"]["status_modifiers"];
        assert_eq!(
            StatusModifiers {
                rushed: true,
                ..Default::default()
            }
            .factor(),
            c["rushed"].as_f64().unwrap()
        );
        assert_eq!(
            StatusModifiers {
                pace_down: true,
                ..Default::default()
            }
            .factor(),
            c["pace_down"].as_f64().unwrap()
        );
        assert_eq!(
            StatusModifiers {
                downhill: true,
                ..Default::default()
            }
            .factor(),
            c["downhill"].as_f64().unwrap()
        );
    }
}
