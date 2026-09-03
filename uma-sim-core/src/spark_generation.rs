//! End-of-career spark (gene) generation.
//!
//! Uses a **derived** RNG (`career_seed ^ SPARK_STREAM`) so career-loop RNG traces stay stable.
//! Tables: `knowledge/mechanics/parent_farming_utility.md`.

use crate::catalog::trainee::TraineeCatalog;
use crate::rng::SimRandom;
use crate::scoring::terminal_utility::{GRADE_CLIFF_B, GRADE_CLIFF_SS, GRADE_CLIFF_UE};
use crate::state::{CareerState, GeneratedSpark, TraineeStats};

/// Mix into career seed for the spark stream (does not draw on career RNG).
const SPARK_STREAM: i64 = 0x5A_52_4B_53; // "SRKS"

const BLUE_IDS: [(&str, &str); 5] = [
    ("speed", "factor:blue:1"),
    ("stamina", "factor:blue:2"),
    ("power", "factor:blue:3"),
    ("guts", "factor:blue:4"),
    ("wit", "factor:blue:5"),
];

fn spark_rng(career_seed: i64) -> SimRandom {
    SimRandom::with_trace(career_seed ^ SPARK_STREAM, false)
}

fn roll_weighted(rng: &mut SimRandom, weights: &[f64]) -> usize {
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 || weights.is_empty() {
        return 0;
    }
    let mut t = rng.next_double() * sum;
    for (i, w) in weights.iter().enumerate() {
        t -= w;
        if t <= 0.0 {
            return i;
        }
    }
    weights.len() - 1
}

/// Blue ★ quality from selected-stat band (collapsed 3-regime table).
pub fn blue_star_from_stat(stat: i32, rng: &mut SimRandom) -> i32 {
    let weights = if stat < 600 {
        [0.90, 0.10, 0.0]
    } else if stat < 1100 {
        [0.49, 0.45, 0.06]
    } else {
        [0.20, 0.69, 0.107]
    };
    (roll_weighted(rng, &weights) + 1) as i32
}

/// White/green ★ quality from run-grade score cliffs.
pub fn grade_quality_stars(score: i32, rng: &mut SimRandom) -> i32 {
    let weights = if score < GRADE_CLIFF_B {
        [0.90, 0.10, 0.0]
    } else if score < GRADE_CLIFF_SS {
        [0.50, 0.45, 0.05]
    } else if score < GRADE_CLIFF_UE {
        [0.20, 0.70, 0.10]
    } else {
        [0.175, 0.70, 0.125]
    };
    (roll_weighted(rng, &weights) + 1) as i32
}

/// Red ★ at generation: 10% / 70% / 20% for 3★ / 2★ / 1★.
pub fn red_generation_stars(rng: &mut SimRandom) -> i32 {
    let roll = rng.next_double();
    if roll < 0.10 {
        3
    } else if roll < 0.80 {
        2
    } else {
        1
    }
}

fn stat_value(stats: &TraineeStats, key: &str) -> i32 {
    match key {
        "stamina" => stats.stamina,
        "power" => stats.power,
        "guts" => stats.guts,
        "wit" => stats.wit,
        _ => stats.speed,
    }
}

fn aptitude_is_a_plus(letter: &str) -> bool {
    matches!(letter.to_uppercase().as_str(), "A" | "S")
}

/// Generate end-of-career sparks onto `state.generated_sparks`.
pub fn generate_end_of_career_sparks(state: &mut CareerState, career_score: i32) {
    if !state.generated_sparks.is_empty() {
        return;
    }
    let mut rng = spark_rng(state.meta.seed);
    let mut out = Vec::new();

    // Blue: one stat chosen uniformly, then band ★ roll.
    let idx = rng.next_int_until(5) as usize;
    let (stat_key, blue_id) = BLUE_IDS[idx];
    let blue_stars = blue_star_from_stat(stat_value(&state.stats, stat_key), &mut rng);
    out.push(GeneratedSpark {
        color: "blue".into(),
        factor_id: blue_id.into(),
        stars: blue_stars,
        label: format!("{stat_key} blue"),
    });

    // Red: one aptitude among A+ (legacy aptitudes, else trainee base).
    let mut a_plus: Vec<(String, String)> = state
        .legacy
        .aptitudes
        .iter()
        .filter(|(_, letter)| aptitude_is_a_plus(letter))
        .map(|(tag, letter)| (tag.clone(), letter.clone()))
        .collect();
    if a_plus.is_empty() {
        if let Some(tm) = TraineeCatalog::lookup(&state.meta.trainee_name) {
            for (i, key) in crate::catalog::trainee::APTITUDE_KEYS.iter().enumerate() {
                if let Some(letter) = tm.aptitudes.get(i) {
                    if aptitude_is_a_plus(letter) {
                        a_plus.push(((*key).to_string(), letter.clone()));
                    }
                }
            }
        }
    }
    if !a_plus.is_empty() {
        let pick = rng.next_int_until(a_plus.len() as i32) as usize;
        let (tag, _) = &a_plus[pick];
        let stars = red_generation_stars(&mut rng);
        out.push(GeneratedSpark {
            color: "red".into(),
            factor_id: format!("factor:pink:{tag}"),
            stars,
            label: format!("{tag} aptitude"),
        });
    }

    // White: each learned skill rolls appearance (base 20%), then grade quality.
    for skill in &state.learned_skill_ids {
        if !rng.next_boolean(0.20) {
            continue;
        }
        let mut stars = grade_quality_stars(career_score, &mut rng);
        // Flat 5% force 3★ before grade table — approximate by bumping if roll hits.
        if rng.next_boolean(0.05) {
            stars = 3;
        }
        out.push(GeneratedSpark {
            color: "white".into(),
            factor_id: skill.clone(),
            stars,
            label: format!("skill {skill}"),
        });
    }

    // Green: guaranteed when trainee has a unique skill (proxy for 3★+ cards).
    if let Some(tm) = TraineeCatalog::lookup(&state.meta.trainee_name) {
        if let Some(uid) = tm.skills_unique.first() {
            let stars = grade_quality_stars(career_score, &mut rng);
            out.push(GeneratedSpark {
                color: "green".into(),
                factor_id: format!("skill:{uid}"),
                stars,
                label: "unique skill".into(),
            });
        }
    }

    state.generated_sparks = out;
}
