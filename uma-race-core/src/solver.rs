//! Single-horse integrator helpers and types.
//! Formulas from `knowledge/mechanics/race_model.md` and KuromiAK.
//!
//! RNG burn order before `startDelay` matches the reference oracle's SeededRng
//! consumption so differential tests share the same start delay and section rolls
//! (behavioural interface contract; not a transcription of race logic).

use crate::condition::skill::EffectType;
use crate::condition::regions::DynamicPred;
use crate::condition::parser::Op;
use crate::course::{get_course, Course};
use crate::hp::{
    guts_modifier, spurt_accept_threshold, GroundCondition, StatusModifiers, Strategy, Surface,
};
use crate::physics::{base_speed, phase_start, Phase};
use crate::rng::PrandoRng;

pub use crate::hp::Aptitude;

pub(crate) const DT: f64 = 1.0 / 15.0;
pub(crate) const START_SPEED: f64 = 3.0;
pub(crate) const BASE_ACCEL: f64 = 0.0006;
pub(crate) const UPHILL_BASE_ACCEL: f64 = 0.0004;
pub(crate) const START_DASH_ACCEL: f64 = 24.0;
pub(crate) const PHASE_DECEL: [f64; 3] = [-1.2, -0.8, -1.0];
/// Fixed deceleration while HP is empty (overrides phase accel/decel).
pub(crate) const EXHAUSTED_DECEL: f64 = -1.2;
/// Pace-down position-keep uses a fixed deceleration instead of phase decel.
pub(crate) const PACE_DOWN_DECEL: f64 = -0.5;

pub(crate) const DIST_PROF_SPEED: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.6, 0.4, 0.2, 0.1];
pub(crate) const GROUND_PROF_ACCEL: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.7, 0.5, 0.3, 0.1];
pub(crate) const DIST_PROF_ACCEL: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.5, 0.4];

pub(crate) fn speed_phase_coef(strategy: Strategy, phase: Phase) -> f64 {
    let row = match strategy {
        Strategy::Oonige => [1.063, 0.962, 0.95],
        Strategy::Nige => [1.0, 0.98, 0.962],
        Strategy::Senkou => [0.978, 0.991, 0.975],
        Strategy::Sasi => [0.938, 0.998, 0.994],
        Strategy::Oikomi => [0.931, 1.0, 1.0],
    };
    let i = match phase {
        Phase::Opening => 0,
        Phase::Middle => 1,
        Phase::End | Phase::LastSpurt => 2,
    };
    row[i]
}

pub(crate) fn accel_phase_coef(strategy: Strategy, phase: Phase) -> f64 {
    let row = match strategy {
        Strategy::Oonige => [1.17, 0.94, 0.956],
        Strategy::Nige => [1.0, 1.0, 0.996],
        Strategy::Senkou => [0.985, 1.0, 0.996],
        Strategy::Sasi => [0.975, 1.0, 1.0],
        Strategy::Oikomi => [0.945, 1.0, 0.997],
    };
    let i = match phase {
        Phase::Opening => 0,
        Phase::Middle => 1,
        Phase::End | Phase::LastSpurt => 2,
    };
    row[i]
}

#[derive(Clone, Debug)]
pub struct HorseInput {
    pub speed: f64,
    pub stamina: f64,
    pub power: f64,
    pub guts: f64,
    pub wisdom: f64,
    pub strategy: Strategy,
    pub distance_apt: Aptitude,
    pub surface_apt: Aptitude,
    pub strategy_apt: Aptitude,
    pub mood: i8,
    pub skills: Vec<String>,
}

fn course_speed_modifier(course: &Course, speed: f64, stamina: f64, power: f64, guts: f64, wisdom: f64) -> f64 {
    if course.course_set_status.is_empty() {
        return 1.0;
    }
    let stats = [0.0, speed, stamina, power, guts, wisdom];
    let sum: f64 = course
        .course_set_status
        .iter()
        .map(|&s| {
            let v = stats.get(s as usize).copied().unwrap_or(0.0).min(901.0);
            (1.0 + (v / 300.01).floor()) * 0.05
        })
        .sum();
    1.0 + sum / course.course_set_status.len() as f64
}

const STRATEGY_PROF: [f64; 8] = [1.1, 1.0, 0.85, 0.75, 0.6, 0.4, 0.2, 0.1];

impl HorseInput {
    pub(crate) fn adjusted_for_course(&self, course: &Course, ground: GroundCondition) -> Self {
        let m = 1.0 + 0.02 * (self.mood as f64);
        let over = |x: f64| {
            if x > 1200.0 {
                1200.0 + ((x - 1200.0) / 2.0).floor()
            } else {
                x
            }
        };
        let speed = over(self.speed) * m;
        let stamina = over(self.stamina) * m;
        let power = over(self.power) * m;
        let guts = over(self.guts) * m;
        let wisdom = over(self.wisdom) * m;
        let cmod = course_speed_modifier(course, speed, stamina, power, guts, wisdom);
        // Ground modifiers (KuromiAK): Good turf = 0.
        let (spd_g, pow_g) = match (course.surface_enum(), ground) {
            // GroundSpeedModifier / GroundPowerModifier (umalator RaceSolverBuilder):
            // Turf:  speed [0,0,0,-50], power [0,-50,-50,-50] for Good..Heavy
            // Dirt:  speed [0,0,0,-50], power [-100,-50,-100,-100]
            (Surface::Turf, GroundCondition::Heavy) => (-50.0, -50.0),
            (Surface::Turf, GroundCondition::Yielding | GroundCondition::Soft) => (0.0, -50.0),
            (Surface::Dirt, GroundCondition::Good) => (0.0, -100.0),
            (Surface::Dirt, GroundCondition::Yielding) => (0.0, -50.0),
            (Surface::Dirt, GroundCondition::Soft) => (0.0, -100.0),
            (Surface::Dirt, GroundCondition::Heavy) => (-50.0, -100.0),
            _ => (0.0, 0.0),
        };
        Self {
            speed: (speed * cmod + spd_g).max(1.0),
            stamina,
            power: (power + pow_g).max(1.0),
            guts,
            wisdom: wisdom * STRATEGY_PROF[self.strategy_apt as usize],
            strategy: self.strategy,
            distance_apt: self.distance_apt,
            surface_apt: self.surface_apt,
            strategy_apt: self.strategy_apt,
            mood: self.mood,
            skills: self.skills.clone(),
        }
    }

    pub(crate) fn raw_wisdom(&self) -> f64 {
        let m = 1.0 + 0.02 * (self.mood as f64);
        let over = |x: f64| {
            if x > 1200.0 {
                1200.0 + ((x - 1200.0) / 2.0).floor()
            } else {
                x
            }
        };
        over(self.wisdom) * m
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveEffect {
    pub kind: EffectType,
    pub modifier: f64,
    /// Counts up from −duration; expires when ≥ 0 (oracle timer convention).
    pub timer: f64,
}

#[derive(Clone, Debug)]
pub struct RaceResult {
    pub finish_time: f64,
    pub frames: u32,
    pub final_pos: f64,
    pub hp_ratio: f64,
}

pub(crate) fn base_target_speed(h: &HorseInput, course_dist: f64, phase: Phase) -> f64 {
    let bs = base_speed(course_dist);
    let coef = speed_phase_coef(h.strategy, phase);
    let mut v = bs * coef;
    if matches!(phase, Phase::End | Phase::LastSpurt) {
        v += (500.0 * h.speed).sqrt() * DIST_PROF_SPEED[h.distance_apt as usize] * 0.002;
    }
    v
}

pub(crate) fn last_spurt_speed(h: &HorseInput, course_dist: f64) -> f64 {
    let bs = base_speed(course_dist);
    let base2 = base_target_speed(h, course_dist, Phase::End);
    let dist = DIST_PROF_SPEED[h.distance_apt as usize];
    (base2 + 0.01 * bs) * 1.05
        + (500.0 * h.speed).sqrt() * dist * 0.002
        + (450.0 * h.guts).powf(0.597) * 0.0001
}

pub(crate) fn accel_rate(h: &HorseInput, phase: Phase, uphill: bool) -> f64 {
    let base = if uphill {
        UPHILL_BASE_ACCEL
    } else {
        BASE_ACCEL
    };
    base * (500.0 * h.power).sqrt()
        * accel_phase_coef(h.strategy, phase)
        * GROUND_PROF_ACCEL[h.surface_apt as usize]
        * DIST_PROF_ACCEL[h.distance_apt as usize]
}

pub(crate) fn uphill_penalty(slope: f64, power: f64) -> f64 {
    if slope <= 0.0 || power <= 0.0 {
        return 0.0;
    }
    (slope / 10_000.0) * 200.0 / power
}

pub(crate) fn downhill_speed_bonus(slope: f64) -> f64 {
    0.3 + slope / 100_000.0
}

pub(crate) fn min_speed(course_dist: f64, guts: f64) -> f64 {
    0.85 * base_speed(course_dist) + (200.0 * guts).sqrt() * 0.001
}

pub(crate) fn start_dash_cap(course_dist: f64) -> f64 {
    0.85 * base_speed(course_dist)
}

pub(crate) fn hp_consume(
    velocity: f64,
    course_dist: f64,
    phase: Phase,
    guts: f64,
    surface: Surface,
    ground: GroundCondition,
    status: StatusModifiers,
) -> f64 {
    let bs = base_speed(course_dist);
    let guts_mod = if (phase as u8) >= (Phase::End as u8) {
        guts_modifier(guts)
    } else {
        1.0
    };
    let ground_mod = match (surface, ground) {
        (Surface::Turf, GroundCondition::Good | GroundCondition::Yielding) => 1.0,
        (Surface::Turf, _) => 1.02,
        (Surface::Dirt, GroundCondition::Good | GroundCondition::Yielding) => 1.0,
        (Surface::Dirt, GroundCondition::Soft) => 1.01,
        (Surface::Dirt, GroundCondition::Heavy) => 1.02,
    };
    20.0 * (velocity - bs + 12.0).powi(2) / 144.0 * status.factor() * ground_mod * guts_mod
}

/// Returns `(transition_pos, spurt_speed)`. `transition_pos < 0` means activate immediately.
pub(crate) fn choose_spurt(
    h: &HorseInput,
    course_dist: f64,
    pos: f64,
    hp: f64,
    surface: Surface,
    ground: GroundCondition,
    hp_rng: &mut PrandoRng,
) -> (f64, f64) {
    let max_v = last_spurt_speed(h, course_dist);
    let base2 = base_target_speed(h, course_dist, Phase::End);
    // Max-spurt HP check uses the full late-race leg from phase-2 start (not current remain).
    let max_dist = course_dist - phase_start(course_dist, Phase::End);
    let s_full = ((max_dist - 60.0) / max_v).max(0.0);
    let need_full = hp_consume(
        max_v,
        course_dist,
        Phase::End,
        h.guts,
        surface,
        ground,
        StatusModifiers::default(),
    ) * s_full;
    if hp >= need_full {
        return (-1.0, max_v);
    }

    let remain = (course_dist - 60.0 - pos).max(0.0);
    let thr = spurt_accept_threshold(h.wisdom);
    let mut candidates: Vec<(f64, f64)> = Vec::new();
    let mut speed = max_v - 0.1;
    while speed >= base2 - 1e-9 {
        let hp_at_speed = hp_consume(
            speed,
            course_dist,
            Phase::End,
            h.guts,
            surface,
            ground,
            StatusModifiers::default(),
        );
        let hp_at_base = hp_consume(
            base2,
            course_dist,
            Phase::End,
            h.guts,
            surface,
            ground,
            StatusModifiers::default(),
        );
        let denom = base2 * hp_at_speed - hp_at_base * speed;
        let spurt_duration = if denom.abs() < 1e-12 {
            0.0
        } else {
            ((base2 * hp - hp_at_base * remain) / denom).max(0.0)
        }
        .min(remain / speed);
        let spurt_distance = spurt_duration * speed;
        let transition = course_dist - spurt_distance - 60.0;
        candidates.push((transition, speed));
        speed -= 0.1;
    }
    if candidates.is_empty() {
        return (-1.0, base2);
    }
    candidates.sort_by(|a, b| {
        let ta = (a.0 - pos) / base2 + (course_dist - a.0) / a.1;
        let tb = (b.0 - pos) / base2 + (course_dist - b.0) / b.1;
        ta.partial_cmp(&tb).unwrap()
    });
    for &(transition, v) in &candidates {
        if hp_rng.uniform(100_000) <= thr {
            return (transition, v);
        }
    }
    candidates[candidates.len() - 1]
}

pub(crate) fn rushed_chance(wisdom: f64) -> f64 {
    let denom = (0.1 * wisdom + 1.0).log10();
    if denom <= 0.0 {
        return 0.0;
    }
    (6.5 / denom).powi(2) / 100.0
}

pub(crate) struct Boot {
    pub rushed_rng: PrandoRng,
    pub wisdom_roll_rng: PrandoRng,
    pub pos_keep_rng: PrandoRng,
    pub lane_rng: PrandoRng,
    /// Approximate `blocked_side` / `overtake` Markov (umalator `specialConditionRng`).
    pub special_condition_rng: PrandoRng,
    pub gate_roll: u32,
    /// Once-per-race `random_lot` roll in 0..99 (umalator `randomLot`).
    pub random_lot: u32,
    pub start_delay: f64,
    pub downhill_rngs: Vec<PrandoRng>,
    pub rushed_section: i32,
}

pub(crate) fn boot_solver_rng(solver_rng: &mut PrandoRng, wisdom: f64, base_spd: f64, n_slopes: usize) -> Boot {
    let _ = base_spd; // section mods rolled after gate skills (post-green wisdom)
    let _sync = PrandoRng::new(solver_rng.int32());
    let _gorosi = PrandoRng::new(solver_rng.int32());
    let mut rushed_rng = PrandoRng::new(solver_rng.int32());
    let wisdom_roll_rng = PrandoRng::new(solver_rng.int32());
    let pos_keep_rng = PrandoRng::new(solver_rng.int32());
    let lane_rng = PrandoRng::new(solver_rng.int32());
    let special_condition_rng = PrandoRng::new(solver_rng.int32());
    let _compete = PrandoRng::new(solver_rng.int32());
    let gate_roll = solver_rng.uniform(12_252_240);
    let random_lot = solver_rng.uniform(100);

    let mut rushed_section: i32 = -1;
    if rushed_rng.random() < rushed_chance(wisdom) {
        rushed_section = 2 + rushed_rng.uniform(8) as i32;
    }

    let start_delay = 0.1 * solver_rng.random();

    // Section modifiers intentionally deferred: umalator rolls them after gate greens
    // so wisdom-stat greens affect the Wiz distribution. See `roll_section_modifiers`.

    // initHills allocates one downhill RNG per slope (original order in course data).
    let downhill_rngs: Vec<PrandoRng> = (0..n_slopes)
        .map(|_| PrandoRng::new(solver_rng.int32()))
        .collect();

    Boot {
        rushed_rng,
        wisdom_roll_rng,
        pos_keep_rng,
        lane_rng,
        special_condition_rng,
        gate_roll,
        random_lot,
        start_delay,
        downhill_rngs,
        rushed_section,
    }
}

/// umalator constructor: after `processSkillActivations()` so green wisdom applies.
pub(crate) fn roll_section_modifiers(
    wisdom: f64,
    base_spd: f64,
    wisdom_roll_rng: &mut PrandoRng,
) -> Vec<f64> {
    (0..24)
        .map(|_| {
            let max = wisdom / 5500.0 * (wisdom * 0.1).log10();
            let factor = (max - 0.65 + wisdom_roll_rng.random() * 0.65) / 100.0;
            base_spd * factor
        })
        .collect()
}

/// Umalator `gateBlock` for `post_number` (ActivationConditions.ts).
pub(crate) fn gate_block(gate_roll: u32, num_umas: u32) -> u32 {
    let n = num_umas.max(1);
    let gate_number = gate_roll % n;
    if gate_number < 9 {
        gate_number
    } else {
        1 + (24 - gate_number) % 8
    }
}

pub(crate) fn dynamics_ok(
    dynamics: &[DynamicPred],
    t: f64,
    is_last_spurt: bool,
    last_spurt_transition: f64,
    start_delay: f64,
    random_lot: u32,
    hp_ratio: f64,
    place: usize,
    field_size: usize,
    activate_count: &[u32; 3],
    activate_count_heal: u32,
    this_skill_already_used: bool,
    gate_roll: u32,
    num_umas: u32,
    used_skills: &std::collections::HashSet<String>,
) -> bool {
    let n = field_size.max(1) as i64;
    let cmp = |op: &Op, lhs: i64, rhs: i64| -> bool {
        match op {
            Op::Ge => lhs >= rhs,
            Op::Gt => lhs > rhs,
            Op::Le => lhs <= rhs,
            Op::Lt => lhs < rhs,
            Op::Eq => lhs == rhs,
            Op::Ne => lhs != rhs,
        }
    };
    for d in dynamics {
        match d {
            DynamicPred::Always => {}
            DynamicPred::PostNumber { op, value } => {
                let post = gate_block(gate_roll, num_umas) as i64;
                if !cmp(op, post, *value) {
                    return false;
                }
            }
            DynamicPred::UsedSkillId { skill_id } => {
                if !used_skills.contains(skill_id) {
                    return false;
                }
            }
            DynamicPred::OrderRate { op, value } => {
                // Umalator compare fixtures leave orderRange unset → order_rate is a no-op.
                // Enforce only in real multi-horse fields (career NPCs, n≥3).
                if field_size < 3 {
                    continue;
                }
                // order_rate R → place threshold round(numUmas * R/100) (GameTora).
                let thresh = ((n as f64) * (*value as f64) / 100.0).round() as i64;
                if !cmp(op, place as i64, thresh) {
                    return false;
                }
            }
            DynamicPred::Order { op, value } => {
                // Umalator: `order` is a compile-time orderRange filter; unset → no-op.
                // Solo/pacer compare (n≤2) must not reject order>=N. Enforce for n≥3 fields.
                if field_size < 3 {
                    continue;
                }
                if !cmp(op, place as i64, *value) {
                    return false;
                }
            }
            DynamicPred::IsLastSpurt { eq } => {
                if is_last_spurt != *eq {
                    return false;
                }
            }
            DynamicPred::LastSpurtCase { case } => {
                let ok = match case {
                    1 => is_last_spurt && last_spurt_transition >= 0.0,
                    2 => is_last_spurt && last_spurt_transition < 0.0,
                    3 => !is_last_spurt,
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
            DynamicPred::AccumulateTime { op, value } => {
                let v = *value as f64;
                let ok = match op {
                    Op::Ge => t >= v,
                    Op::Gt => t > v,
                    Op::Le => t <= v,
                    Op::Lt => t < v,
                    Op::Eq => (t - v).abs() < 1e-6,
                    Op::Ne => (t - v).abs() >= 1e-6,
                };
                if !ok {
                    return false;
                }
            }
            DynamicPred::ActivateCountHeal { op, value } => {
                if !cmp(op, activate_count_heal as i64, *value) {
                    return false;
                }
            }
            DynamicPred::ActivateCountPhase { phase, op, value } => {
                let idx = (*phase as usize).min(2);
                if !cmp(op, activate_count[idx] as i64, *value) {
                    return false;
                }
            }
            DynamicPred::ActivateCountAll { op, value } => {
                let sum = activate_count.iter().sum::<u32>() as i64;
                if !cmp(op, sum, *value) {
                    return false;
                }
            }
            DynamicPred::IsActivateOtherSkillDetail { eq } => {
                if this_skill_already_used != *eq {
                    return false;
                }
            }
            DynamicPred::IsBadStart { want_bad } => {
                let is_bad = start_delay > 0.08;
                if is_bad != *want_bad {
                    return false;
                }
            }
            DynamicPred::RandomLot { max_exclusive } => {
                // umalator: randomLot < lot (uniform 0..99).
                if (random_lot as i64) >= *max_exclusive {
                    return false;
                }
            }
            DynamicPred::HpPer { op, value } => {
                let thresh = (*value as f64) / 100.0;
                let ok = match op {
                    Op::Ge => hp_ratio >= thresh,
                    Op::Gt => hp_ratio > thresh,
                    Op::Le => hp_ratio <= thresh,
                    Op::Lt => hp_ratio < thresh,
                    Op::Eq => (hp_ratio - thresh).abs() < 1e-6,
                    Op::Ne => (hp_ratio - thresh).abs() >= 1e-6,
                };
                if !ok {
                    return false;
                }
            }
        }
    }
    true
}

pub fn simulate_solo(
    course: &Course,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
) -> RaceResult {
    crate::runner::simulate_solo(course, ground, horse, seed)
}

pub fn simulate_solo_by_id(
    course_id: u32,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
) -> Option<RaceResult> {
    let c = get_course(course_id)?;
    Some(simulate_solo(c, ground, horse, seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case1_horse() -> HorseInput {
        HorseInput {
            speed: 1200.0,
            stamina: 1000.0,
            power: 1000.0,
            guts: 900.0,
            wisdom: 900.0,
            strategy: Strategy::Senkou,
            distance_apt: Aptitude::A,
            surface_apt: Aptitude::A,
            strategy_apt: Aptitude::A,
            mood: 2,
            skills: vec![],
        }
    }

    fn case3_horse() -> HorseInput {
        HorseInput {
            speed: 1000.0,
            stamina: 900.0,
            power: 1100.0,
            guts: 1000.0,
            wisdom: 1000.0,
            strategy: Strategy::Oikomi,
            distance_apt: Aptitude::A,
            surface_apt: Aptitude::A,
            strategy_apt: Aptitude::A,
            mood: 2,
            // 200701: mile late-race accel (effect type 31). 200501 is type 28 lane-move (no forward effect).
            skills: vec!["200701".into()],
        }
    }

    #[test]
    fn case1_finish_within_one_frame_of_oracle() {
        let r = simulate_solo_by_id(10601, GroundCondition::Good, &case1_horse(), 2615953739)
            .expect("course 10601");
        let oracle = 66.46666666666802;
        let delta = (r.finish_time - oracle).abs();
        eprintln!(
            "case1 finish={:.6} oracle={:.6} Δ={:.6}s frames={} hp={}",
            r.finish_time, oracle, delta, r.frames, r.hp_ratio
        );
        assert!(
            delta <= DT + 1e-6,
            "finish {:.6} vs oracle {:.6} (Δ={:.6}); need ≤1 frame ({DT})",
            r.finish_time,
            oracle,
            delta
        );
    }

    #[test]
    fn case3_skill_finish_within_one_frame_of_oracle() {
        let r = simulate_solo_by_id(10611, GroundCondition::Good, &case3_horse(), 2615953739)
            .expect("course 10611");
        let oracle = 76.13333333333414;
        let delta = (r.finish_time - oracle).abs();
        eprintln!(
            "case3 finish={:.6} oracle={:.6} Δ={:.6}s frames={} hp={}",
            r.finish_time, oracle, delta, r.frames, r.hp_ratio
        );
        assert!(
            delta <= DT + 1e-6,
            "finish {:.6} vs oracle {:.6} (Δ={:.6}); need ≤1 frame ({DT})",
            r.finish_time,
            oracle,
            delta
        );
    }

    #[test]
    fn case3_accel_skill_beats_no_skill() {
        let mut no = case3_horse();
        no.skills.clear();
        let a = simulate_solo_by_id(10611, GroundCondition::Good, &no, 2615953739).unwrap();
        let b = simulate_solo_by_id(10611, GroundCondition::Good, &case3_horse(), 2615953739).unwrap();
        assert!(
            b.finish_time + 1e-9 < a.finish_time,
            "accel skill should finish sooner: with={:.6} without={:.6}",
            b.finish_time,
            a.finish_time
        );
    }

    #[test]
    fn uphill_penalty_uses_power_not_sqrt500() {
        let p = uphill_penalty(20_000.0, 1040.0);
        assert!((p - 400.0 / 1040.0).abs() < 1e-12, "penalty={p}");
    }

    #[test]
    fn section_random_band_matches_kuromiak_examples() {
        let wiz = 400.0_f64;
        let max = (wiz / 5500.0) * (wiz * 0.1).log10();
        let min = max - 0.65;
        assert!((max - 0.117).abs() < 0.002, "max={max}");
        assert!((min - -0.533).abs() < 0.002, "min={min}");
    }

    #[test]
    fn order_rate_enforced_only_for_multi_horse_fields() {
        use crate::condition::parser::Op;
        use crate::condition::regions::DynamicPred;
        let dyns = vec![DynamicPred::OrderRate {
            op: Op::Gt,
            value: 50,
        }];
        // Solo / pacer compare: noop (matches umalator without orderRange).
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 2, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        // 9-horse field: order_rate>50 → place > round(4.5)=5.
        assert!(!dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(!dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 5, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 6, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
    }

    #[test]
    fn order_ge_noop_in_solo_enforced_in_pack() {
        use crate::condition::parser::Op;
        use crate::condition::regions::DynamicPred;
        let dyns = vec![DynamicPred::Order {
            op: Op::Ge,
            value: 3,
        }];
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 2, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(!dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(!dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 2, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 3, 9, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
    }

    #[test]
    fn activate_count_heal_gates_until_heal() {
        use crate::condition::parser::Op;
        use crate::condition::regions::DynamicPred;
        let dyns = vec![DynamicPred::ActivateCountHeal {
            op: Op::Ge,
            value: 1,
        }];
        assert!(!dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 0, false, 0, 9, &std::collections::HashSet::new()));
        assert!(dynamics_ok(&dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 1, false, 0, 9, &std::collections::HashSet::new()));
    }

    #[test]
    fn post_number_uses_gate_block() {
        use crate::condition::parser::Op;
        use crate::condition::regions::DynamicPred;
        let dyns = vec![DynamicPred::PostNumber {
            op: Op::Eq,
            value: 7,
        }];
        // gate_roll % 9 == 7 → ok
        let empty = std::collections::HashSet::new();
        assert!(dynamics_ok(
            &dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 0, false, 7, 9, &empty
        ));
        assert!(!dynamics_ok(
            &dyns, 0.0, false, -1.0, 0.0, 0, 1.0, 1, 1, &[0, 0, 0], 0, false, 6, 9, &empty
        ));
        assert_eq!(gate_block(7, 9), 7);
        assert_eq!(gate_block(16, 18), 1 + (24 - 16) % 8);
    }
}
