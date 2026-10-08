//! Phase geometry and base speed (community-documented constants).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Opening = 0,
    Middle = 1,
    End = 2,
    LastSpurt = 3,
}

/// `20 - (distance - 2000) / 1000`
pub fn base_speed(distance_m: f64) -> f64 {
    20.0 - (distance_m - 2000.0) / 1000.0
}

pub fn phase_start(distance_m: f64, phase: Phase) -> f64 {
    match phase {
        Phase::Opening => 0.0,
        Phase::Middle => distance_m / 6.0,
        Phase::End => distance_m * 2.0 / 3.0,
        Phase::LastSpurt => distance_m * 5.0 / 6.0,
    }
}

pub fn phase_end(distance_m: f64, phase: Phase) -> f64 {
    match phase {
        Phase::Opening => distance_m / 6.0,
        Phase::Middle => distance_m * 2.0 / 3.0,
        Phase::End => distance_m * 5.0 / 6.0,
        Phase::LastSpurt => distance_m,
    }
}

pub fn phase_at(distance_m: f64, pos: f64) -> Phase {
    if pos < phase_start(distance_m, Phase::Middle) {
        Phase::Opening
    } else if pos < phase_start(distance_m, Phase::End) {
        Phase::Middle
    } else if pos < phase_start(distance_m, Phase::LastSpurt) {
        Phase::End
    } else {
        Phase::LastSpurt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn constants() -> Value {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../research/race_model_constants.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn base_speed_at_2000_is_20() {
        assert!((base_speed(2000.0) - 20.0).abs() < 1e-12);
        assert!((base_speed(1000.0) - 21.0).abs() < 1e-12);
        assert!((base_speed(3000.0) - 19.0).abs() < 1e-12);
    }

    #[test]
    fn phase_fractions_match_constants_file() {
        let c = constants();
        let d = 2400.0;
        assert!(
            (phase_start(d, Phase::Middle) / d - c["phases"]["phase0_end_frac"].as_f64().unwrap())
                .abs()
                < 1e-12
        );
        assert!(
            (phase_start(d, Phase::End) / d - c["phases"]["phase1_end_frac"].as_f64().unwrap())
                .abs()
                < 1e-12
        );
        assert!(
            (phase_start(d, Phase::LastSpurt) / d
                - c["phases"]["phase2_end_frac"].as_f64().unwrap())
            .abs()
                < 1e-12
        );
    }
}
