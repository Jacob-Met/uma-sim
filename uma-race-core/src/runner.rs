//! Frame-steppable horse runner extracted from the solo integrator.
//! Supports optional Virtual position-keep via [`StepCtx`].

use crate::condition::skill::{
    compile_skills, skill_activation_chance, EffectType, PendingSkill, SkillEffect,
};
use crate::condition::regions::HorseCtx;
use crate::course::Course;
use crate::course::Slope;
use crate::hp::{max_hp, GroundCondition, StatusModifiers, Strategy, Surface};
use crate::physics::{base_speed, phase_at, phase_start, Phase};
use crate::special_conditions::SpecialConditions;
use crate::compete_fight::{
    compete_accel_bonus, compete_speed_bonus, is_top_half, on_final_straight, END_MIN_HP_FRAC,
    START_MIN_HP_FRAC, TARGET_DISTANCE_GAP_M, TARGET_HOLD_S, TRIGGER_SPEED_GAP_MS,
};
use crate::lead_comp::{
    distance_gap_limit, in_entry_window, lead_comp_duration_s, lead_comp_hp_factor,
    lead_comp_speed_bonus, past_hard_end, same_lead_group,
};
use crate::pos_keep::{
    max_threshold, min_threshold, pos_keep_speed_coef, tick_nige_pos_keep, tick_pack_pos_keep,
    PosKeepMode, PosKeepState,
};
use crate::rng::PrandoRng;
use crate::solver::{
    accel_rate, base_target_speed, boot_solver_rng, choose_spurt, downhill_speed_bonus, dynamics_ok,
    hp_consume, last_spurt_speed, min_speed, roll_section_modifiers, start_dash_cap, uphill_penalty,
    ActiveEffect, EXHAUSTED_DECEL, HorseInput, RaceResult, DT, PACE_DOWN_DECEL, PHASE_DECEL,
    START_DASH_ACCEL, START_SPEED,
};
use std::collections::HashSet;

/// Per-frame context for multi-horse / Virtual position-keep.
#[derive(Clone, Copy, Debug)]
pub struct StepCtx {
    /// Designated pacemaker position this frame (after pacer step when available).
    pub pacer_pos: Option<f64>,
    /// Second-place position for lead-gap checks (SpeedUp / Overtake exit).
    pub second_pos: Option<f64>,
    /// True when this runner is the field pacemaker for the frame.
    pub am_i_pacer: bool,
    pub pos_keep_mode: PosKeepMode,
    /// 1-based place estimate for skill `order` dynamics (solo default = 1).
    pub place: usize,
    pub field_size: usize,
}

impl Default for StepCtx {
    fn default() -> Self {
        Self {
            pacer_pos: None,
            second_pos: None,
            am_i_pacer: false,
            pos_keep_mode: PosKeepMode::None,
            place: 1,
            field_size: 1,
        }
    }
}

/// Compare-mode position-keep ends after this many course sections (of 24).
const POS_KEEP_END_SECTIONS_COMPARE: f64 = 10.0;

struct GatePhysics {
    mod_target: f64,
    mod_accel: f64,
    mod_current: f64,
    active: Vec<ActiveEffect>,
}

/// Fire pending skills whose trigger covers pos=0 before HP init — umalator
/// `processSkillActivations` at construction (greens *and* phase-0 accel/etc.).
fn apply_gate_skills(
    pending: &mut Vec<PendingSkill>,
    h: &mut HorseInput,
    start_delay: &mut f64,
    random_lot: u32,
    wisdom_roll_rng: &mut PrandoRng,
    raw_wisdom: f64,
    activate_count: &mut [u32; 3],
    activate_count_heal: &mut u32,
    used_skills: &mut HashSet<String>,
    course_dist: f64,
    gate_roll: u32,
    num_umas: u32,
) -> GatePhysics {
    let mut physics = GatePhysics {
        mod_target: 0.0,
        mod_accel: 0.0,
        mod_current: 0.0,
        active: Vec::new(),
    };
    let scale = course_dist / 1000.0;
    for i in (0..pending.len()).rev() {
        let trig = &pending[i].trigger;
        if !(trig.start <= 0.0 && trig.end > 0.0) {
            continue;
        }
        if !dynamics_ok(
            &pending[i].dynamics,
            0.0,
            false,
            -1.0,
            *start_delay,
            random_lot,
            1.0,
            1,
            1,
            activate_count,
            *activate_count_heal,
            used_skills.contains(&pending[i].skill_id),
            gate_roll,
            num_umas,
            used_skills,
        ) {
            continue;
        }
        let chance = skill_activation_chance(raw_wisdom);
        if !pending[i].skip_wisdom && wisdom_roll_rng.random() > chance {
            pending.remove(i);
            continue;
        }
        let skill_id = pending[i].skill_id.clone();
        let effects = pending[i].effects.clone();
        for ef in &effects {
            match ef.kind {
                EffectType::SpeedUp
                | EffectType::StaminaUp
                | EffectType::PowerUp
                | EffectType::GutsUp
                | EffectType::WisdomUp
                | EffectType::MultiplyStartDelay
                | EffectType::SetStartDelay
                | EffectType::Heal => {
                    apply_stat_or_delay_effect(h, start_delay, activate_count_heal, ef);
                }
                EffectType::TargetSpeed => {
                    physics.mod_target += ef.modifier;
                    physics.active.push(ActiveEffect {
                        kind: ef.kind,
                        modifier: ef.modifier,
                        timer: -(ef.duration * scale),
                    });
                }
                EffectType::Accel => {
                    physics.mod_accel += ef.modifier;
                    physics.active.push(ActiveEffect {
                        kind: ef.kind,
                        modifier: ef.modifier,
                        timer: -(ef.duration * scale),
                    });
                }
                EffectType::CurrentSpeed | EffectType::CurrentSpeedDecel => {
                    physics.mod_current += ef.modifier;
                    physics.active.push(ActiveEffect {
                        kind: ef.kind,
                        modifier: ef.modifier,
                        timer: -(ef.duration * scale),
                    });
                }
                EffectType::LaneMove => {
                    // Type 28 = LaneMovementSpeed: boosts lateral change speed only.
                    // Forward MoveLaneModifier is applied per-frame while lane_change_speed > 0.
                    physics.active.push(ActiveEffect {
                        kind: ef.kind,
                        modifier: ef.modifier,
                        timer: -(ef.duration * scale),
                    });
                }
                EffectType::Other => {}
            }
        }
        activate_count[0] += 1; // opening phase at gate
        used_skills.insert(skill_id);
        pending.remove(i);
    }
    physics
}

fn apply_stat_or_delay_effect(
    h: &mut HorseInput,
    start_delay: &mut f64,
    activate_count_heal: &mut u32,
    ef: &SkillEffect,
) {
    match ef.kind {
        EffectType::SpeedUp => h.speed = (h.speed + ef.modifier).max(1.0),
        EffectType::StaminaUp => h.stamina = (h.stamina + ef.modifier).max(1.0),
        EffectType::PowerUp => h.power = (h.power + ef.modifier).max(1.0),
        EffectType::GutsUp => h.guts = (h.guts + ef.modifier).max(1.0),
        EffectType::WisdomUp => h.wisdom = (h.wisdom + ef.modifier).max(1.0),
        EffectType::MultiplyStartDelay => *start_delay *= ef.modifier,
        EffectType::SetStartDelay => *start_delay = ef.modifier,
        EffectType::Heal => {
            // Gate heals before max_hp exists — count only; HP filled after init.
            *activate_count_heal += 1;
        }
        _ => {}
    }
}

pub struct HorseRunner {
    h: HorseInput,
    raw_wisdom: f64,
    surface: Surface,
    ground: GroundCondition,
    course_dist: f64,
    section_len: f64,
    slopes: Vec<Slope>,
    infinite_hp: bool,

    pending: Vec<PendingSkill>,
    rushed_rng: PrandoRng,
    wisdom_roll_rng: PrandoRng,
    pos_keep_rng: PrandoRng,
    hp_rng: PrandoRng,
    section_modifiers: Vec<f64>,
    downhill_rngs: Vec<PrandoRng>,
    rushed_section: i32,

    max_hp: f64,
    hp: f64,
    pos: f64,
    speed: f64,
    t: f64,
    frames: u32,
    is_last_spurt: bool,
    last_spurt_v: f64,
    /// Negative ⇒ activate immediately once decided.
    last_spurt_transition: f64,
    spurt_decided: bool,
    in_start_dash: bool,
    downhill_mode: bool,
    downhill_tick: f64,
    hill_idx: i32,
    is_rushed: bool,
    has_been_rushed: bool,
    rushed_timer: f64,
    /// One-shot early-clear roll at 3s (oracle: ~55% clear, else hold to 12s max).
    rushed_clear_checked: bool,
    active: Vec<ActiveEffect>,
    mod_target: f64,
    mod_accel: f64,
    mod_current: f64,
    min_spd: f64,
    /// Phase-bucket skill activation counts [start, middle, end].
    activate_count: [u32; 3],
    activate_count_heal: u32,
    used_skills: HashSet<String>,
    dash_cap: f64,
    /// Initial start delay (umalator startDelay); used by is_badstart.
    start_delay: f64,
    /// Once-per-race random_lot roll (0..99).
    random_lot: u32,
    /// Umalator gateRoll for `post_number` / lane init.
    gate_roll: u32,
    /// Race field size for `post_number` (oracle default 9 when unset).
    num_umas: u32,
    start_delay_acc: f64,
    hill_start_q: Vec<usize>,
    hill_end_q: Vec<usize>,

    // Position-keep
    pos_keep_state: PosKeepState,
    pos_keep_timer: f64,
    pos_keep_exit_pos: f64,
    pos_keep_exit_dist: f64,
    pos_keep_min_th: f64,
    pos_keep_max_th: f64,
    pos_keep_end: f64,
    /// Debug: last pack behind (pacer − pos) seen by pos-keep this frame.
    debug_last_pk_behind: f64,

    // Lead competition (CompeteTop)
    lead_comp_active: bool,
    lead_comp_timer: f64,
    lead_comp_bonus: f64,
    /// After a bout ends, require gap to open before another entry (avoids permanent re-trigger).
    lead_comp_gap_open: bool,
    /// At most one lead-competition bout per race (oracle Virtual Nige default-pacer cases).
    lead_comp_used: bool,

    // Compete-fight / dueling (追い比べ) — final straight; lane gap deferred (=0).
    last_straight_start: f64,
    last_straight_end: f64,
    compete_target_hold: f64,
    compete_active: bool,
    compete_speed_bonus: f64,
    compete_accel_bonus: f64,

    // Lateral lane (clean-room of CourseHelpers + applyLaneMovement).
    horse_lane: f64,
    max_lane_distance: f64,
    move_lane_point: f64,
    lane_change_accel_pf: f64,
    lane_rng: PrandoRng,
    current_lane: f64,
    target_lane: f64,
    lane_change_speed: f64,
    extra_move_lane: f64,
    /// Approximate Markov `blocked_side` / `overtake` (A9 / SpecialConditions).
    special: SpecialConditions,
}

impl HorseRunner {
    /// Focus horse: root seed → skill / solver / hp streams (builder.build path).
    pub fn new(course: &Course, ground: GroundCondition, horse: &HorseInput, seed: u32) -> Self {
        let mut root = PrandoRng::new(seed);
        let mut skill_rng = PrandoRng::new(root.int32());
        let mut solver_rng = PrandoRng::new(root.int32());
        let hp_rng = PrandoRng::new(root.int32());
        Self::build(
            course,
            ground,
            horse,
            &mut skill_rng,
            &mut solver_rng,
            hp_rng,
            false,
            9, // oracle default when RaceParameters.numUmas unset
        )
    }

    /// Field race: `num_umas` drives `post_number` (umalator `extra.numUmas`).
    pub fn new_in_field(
        course: &Course,
        ground: GroundCondition,
        horse: &HorseInput,
        seed: u32,
        num_umas: u32,
    ) -> Self {
        let mut root = PrandoRng::new(seed);
        let mut skill_rng = PrandoRng::new(root.int32());
        let mut solver_rng = PrandoRng::new(root.int32());
        let hp_rng = PrandoRng::new(root.int32());
        Self::build(
            course,
            ground,
            horse,
            &mut skill_rng,
            &mut solver_rng,
            hp_rng,
            false,
            num_umas.max(1),
        )
    }

    /// Default/virtual pacer: `solver_rng` is already the RaceSolver rng (no skill/hp root split).
    /// Infinite HP (NoopHpPolicy). Skills are compiled from a fresh burn on the same rng only
    /// when non-empty; empty skills match oracle `buildPacer` with no pre-solver skill sampling.
    pub fn new_pacer(
        course: &Course,
        ground: GroundCondition,
        horse: &HorseInput,
        solver_seed: u32,
    ) -> Self {
        let mut solver_rng = PrandoRng::new(solver_seed);
        // Empty skill list: no skillRng stream. Non-empty: sample from solver rng first
        // (matches setupPacerSkillTriggers burning pacerRng before RaceSolver ctor).
        let mut skill_rng = if horse.skills.is_empty() {
            PrandoRng::new(0) // unused
        } else {
            PrandoRng::new(solver_rng.int32())
        };
        let hp_rng = PrandoRng::new(0); // unused with infinite HP
        Self::build(
            course,
            ground,
            horse,
            &mut skill_rng,
            &mut solver_rng,
            hp_rng,
            true,
            9,
        )
    }

    fn build(
        course: &Course,
        ground: GroundCondition,
        horse: &HorseInput,
        skill_rng: &mut PrandoRng,
        solver_rng: &mut PrandoRng,
        hp_rng: PrandoRng,
        infinite_hp: bool,
        num_umas: u32,
    ) -> Self {
        let raw_wisdom = horse.raw_wisdom();
        let mut h = horse.adjusted_for_course(course, ground);
        let surface = course.surface_enum();
        let course_dist = course.distance;
        let section_len = course_dist / 24.0;
        let bs = base_speed(course_dist);

        let mut pending = compile_skills(
            &h.skills,
            course,
            HorseCtx {
                strategy: h.strategy,
                distance_apt: h.distance_apt,
                surface_apt: h.surface_apt,
                ground,
                mood: h.mood,
                speed: h.speed,
                stamina: h.stamina,
                power: h.power,
                guts: h.guts,
                wisdom: h.wisdom,
                weather: 1,
                season: 1,
                time: 2,
                grade: 100,
            },
            skill_rng,
        );

        let boot = boot_solver_rng(solver_rng, h.wisdom, bs, course.slopes.len());
        let mut start_delay_acc = boot.start_delay;
        // Gate skills: fire at pos=0 before HP init (umalator processSkillActivations),
        // including phase-0 accel / target-speed (e.g. 200531).
        let mut wisdom_roll_rng = boot.wisdom_roll_rng;
        let mut activate_count = [0u32; 3];
        let mut activate_count_heal = 0u32;
        let mut used_skills = HashSet::new();
        let gate = apply_gate_skills(
            &mut pending,
            &mut h,
            &mut start_delay_acc,
            boot.random_lot,
            &mut wisdom_roll_rng,
            raw_wisdom,
            &mut activate_count,
            &mut activate_count_heal,
            &mut used_skills,
            course_dist,
            boot.gate_roll,
            num_umas,
        );

        // After gate greens (umalator: sectionModifier uses post-green wisdom).
        let section_modifiers = roll_section_modifiers(h.wisdom, bs, &mut wisdom_roll_rng);

        let max_hp_v = max_hp(h.strategy, h.stamina, course_dist);
        let min_spd = min_speed(course_dist, h.guts);
        let dash_cap = start_dash_cap(course_dist);

        let mut by_start: Vec<usize> = (0..course.slopes.len()).collect();
        by_start.sort_by(|&a, &b| {
            course.slopes[a]
                .start
                .partial_cmp(&course.slopes[b].start)
                .unwrap()
        });
        let mut hill_start_q: Vec<usize> = by_start.iter().copied().rev().collect();
        let hill_end_q: Vec<usize> = by_start.iter().copied().rev().collect();

        let mut hill_idx: i32 = -1;
        let mut downhill_mode = false;
        let mut downhill_tick = 0.0;
        let mut downhill_rngs = boot.downhill_rngs;

        if let Some(&idx) = hill_start_q.last() {
            if course.slopes[idx].start == 0.0 {
                hill_idx = idx as i32;
                hill_start_q.pop();
                downhill_tick = 0.0;
                let slope = course.slopes[idx].slope;
                if slope < 0.0 {
                    let roll = downhill_rngs[idx].random();
                    if roll < h.wisdom * 0.0004 {
                        downhill_mode = true;
                    }
                }
            }
        }

        Self {
            pos_keep_min_th: min_threshold(h.strategy, course_dist),
            pos_keep_max_th: max_threshold(h.strategy, course_dist),
            pos_keep_end: section_len * POS_KEEP_END_SECTIONS_COMPARE,
            debug_last_pk_behind: 0.0,
            h,
            raw_wisdom,
            surface,
            ground,
            course_dist,
            section_len,
            slopes: course.slopes.clone(),
            infinite_hp,
            pending,
            rushed_rng: boot.rushed_rng,
            wisdom_roll_rng,
            pos_keep_rng: boot.pos_keep_rng,
            hp_rng,
            section_modifiers,
            downhill_rngs,
            rushed_section: boot.rushed_section,
            max_hp: max_hp_v,
            hp: max_hp_v,
            pos: 0.0,
            speed: START_SPEED,
            t: 0.0,
            frames: 0,
            is_last_spurt: false,
            last_spurt_v: 0.0,
            last_spurt_transition: -1.0,
            spurt_decided: false,
            in_start_dash: true,
            downhill_mode,
            downhill_tick,
            hill_idx,
            is_rushed: false,
            has_been_rushed: false,
            rushed_timer: 0.0,
            rushed_clear_checked: false,
            active: gate.active,
            mod_target: gate.mod_target,
            mod_accel: gate.mod_accel,
            mod_current: gate.mod_current,
            min_spd,
            activate_count,
            activate_count_heal,
            used_skills,
            dash_cap,
            start_delay: start_delay_acc,
            random_lot: boot.random_lot,
            gate_roll: boot.gate_roll,
            num_umas,
            start_delay_acc,
            hill_start_q,
            hill_end_q,
            pos_keep_state: PosKeepState::None,
            pos_keep_timer: 0.0,
            pos_keep_exit_pos: 0.0,
            pos_keep_exit_dist: 0.0,
            lead_comp_active: false,
            lead_comp_timer: 0.0,
            lead_comp_bonus: 0.0,
            lead_comp_gap_open: true,
            lead_comp_used: false,
            last_straight_start: course
                .straights
                .last()
                .map(|s| s.start)
                .unwrap_or(course_dist),
            last_straight_end: course
                .straights
                .last()
                .map(|s| s.end)
                .unwrap_or(course_dist),
            compete_target_hold: 0.0,
            compete_active: false,
            compete_speed_bonus: 0.0,
            compete_accel_bonus: 0.0,
            horse_lane: course.horse_lane(),
            max_lane_distance: course.max_lane_distance(),
            move_lane_point: course.move_lane_point(),
            lane_change_accel_pf: course.lane_change_accel_per_frame(),
            lane_rng: boot.lane_rng,
            // Default post ≈ gate % 9 (oracle `numUmas || 9` for post_number).
            current_lane: (boot.gate_roll % 9) as f64 * course.horse_lane(),
            target_lane: (boot.gate_roll % 9) as f64 * course.horse_lane(),
            lane_change_speed: 0.0,
            extra_move_lane: -1.0,
            special: SpecialConditions::new(boot.special_condition_rng),
        }
    }

    pub fn pos(&self) -> f64 {
        self.pos
    }

    pub fn pos_keep_state(&self) -> PosKeepState {
        self.pos_keep_state
    }

    pub fn accum_time(&self) -> f64 {
        self.t
    }

    pub fn adjusted_speed(&self) -> f64 {
        self.h.speed
    }

    pub fn current_speed(&self) -> f64 {
        self.speed
    }

    pub fn start_delay_val(&self) -> f64 {
        self.start_delay
    }

    pub fn pk_thresholds(&self) -> (f64, f64) {
        (self.pos_keep_min_th, self.pos_keep_max_th)
    }

    pub fn debug_pk_timer(&self) -> f64 {
        self.pos_keep_timer
    }

    pub fn debug_pk_end(&self) -> f64 {
        self.pos_keep_end
    }

    pub fn debug_active_speed_skill_count(&self) -> usize {
        self.active
            .iter()
            .filter(|ef| {
                matches!(
                    ef.kind,
                    EffectType::TargetSpeed
                        | EffectType::CurrentSpeed
                        | EffectType::CurrentSpeedDecel
                )
            })
            .count()
    }

    pub fn debug_exit_dist(&self) -> f64 {
        self.pos_keep_exit_dist
    }

    pub fn debug_exit_pos(&self) -> f64 {
        self.pos_keep_exit_pos
    }

    /// Behind gap used for the last pack pos-keep decision this frame (pacer − self pre-move).
    pub fn debug_pk_behind(&self) -> f64 {
        self.debug_last_pk_behind
    }

    pub fn debug_used_skills(&self) -> Vec<String> {
        let mut v: Vec<String> = self.used_skills.iter().cloned().collect();
        v.sort();
        v
    }

    pub fn debug_stats(&self) -> (f64, f64, f64, f64, f64) {
        (
            self.h.speed,
            self.h.stamina,
            self.h.power,
            self.h.guts,
            self.h.wisdom,
        )
    }

    pub fn debug_pending_triggers(&self) -> Vec<(String, f64, f64)> {
        self.pending
            .iter()
            .map(|p| (p.skill_id.clone(), p.trigger.start, p.trigger.end))
            .collect()
    }

    pub fn debug_target_bits(&self) -> (f64, f64, f64, f64) {
        use crate::physics::phase_at;
        use crate::solver::base_target_speed;
        use crate::pos_keep::pos_keep_speed_coef;
        let phase = phase_at(self.course_dist, self.pos);
        let section = ((self.pos / self.section_len).floor() as usize).min(23);
        let base = base_target_speed(&self.h, self.course_dist, phase);
        let pk = pos_keep_speed_coef(self.pos_keep_state);
        let sec = self.section_modifiers.get(section).copied().unwrap_or(0.0);
        (base, pk, sec, base * pk + sec)
    }

    pub fn wisdom_adj(&self) -> f64 {
        self.h.wisdom
    }

    pub fn in_start_dash(&self) -> bool {
        self.in_start_dash
    }

    pub fn dash_cap_val(&self) -> f64 {
        self.dash_cap
    }

    pub fn debug_mod_target(&self) -> f64 {
        self.mod_target
    }

    /// Update lead competition vs another front-runner / oonige (lane model deferred → lane gap 0).
    pub fn update_lead_competition(&mut self, other_pos: f64, other_strategy: Strategy) {
        if past_hard_end(self.pos, self.section_len) {
            self.lead_comp_active = false;
            self.lead_comp_bonus = 0.0;
            return;
        }
        let gap = (self.pos - other_pos).abs();
        let limit = distance_gap_limit(self.h.strategy);
        if gap >= limit {
            self.lead_comp_gap_open = true;
        }
        if self.lead_comp_active {
            self.lead_comp_timer += DT;
            if self.lead_comp_timer >= lead_comp_duration_s(self.h.guts) {
                self.lead_comp_active = false;
                self.lead_comp_bonus = 0.0;
                self.lead_comp_gap_open = false;
            }
            return;
        }
        if self.lead_comp_used {
            return;
        }
        if !matches!(self.h.strategy, Strategy::Nige | Strategy::Oonige) {
            return;
        }
        if !same_lead_group(self.h.strategy, other_strategy) {
            return;
        }
        if !in_entry_window(self.pos, self.section_len) {
            return;
        }
        if self.lead_comp_gap_open && gap < limit {
            self.lead_comp_active = true;
            self.lead_comp_used = true;
            self.lead_comp_timer = 0.0;
            self.lead_comp_bonus = lead_comp_speed_bonus(self.h.guts);
            self.lead_comp_gap_open = false;
        }
    }

    /// Compete-fight (追い比べ) vs nearest rival on the final straight.
    /// Lane gap deferred (=0 always satisfied). `place` is 0-based (0 = first).
    pub fn update_compete_fight(
        &mut self,
        other_pos: f64,
        other_speed: f64,
        place: usize,
        field_size: usize,
    ) {
        let on_fs = on_final_straight(self.pos, self.last_straight_start, self.last_straight_end);
        let hp_frac = if self.max_hp > 0.0 {
            self.hp / self.max_hp
        } else {
            1.0
        };

        if self.compete_active {
            if !on_fs || (!self.infinite_hp && hp_frac < END_MIN_HP_FRAC) {
                self.compete_active = false;
                self.compete_speed_bonus = 0.0;
                self.compete_accel_bonus = 0.0;
                self.compete_target_hold = 0.0;
            }
            return;
        }

        if !on_fs {
            self.compete_target_hold = 0.0;
            return;
        }
        if !self.infinite_hp && hp_frac < START_MIN_HP_FRAC {
            self.compete_target_hold = 0.0;
            return;
        }

        let gap = (self.pos - other_pos).abs();
        if gap < TARGET_DISTANCE_GAP_M {
            self.compete_target_hold += DT;
        } else {
            self.compete_target_hold = 0.0;
            return;
        }

        if self.compete_target_hold < TARGET_HOLD_S {
            return;
        }
        if !is_top_half(place, field_size) {
            return;
        }
        let speed_gap = (self.speed - other_speed).abs();
        if speed_gap >= TRIGGER_SPEED_GAP_MS {
            return;
        }

        self.compete_active = true;
        self.compete_speed_bonus = compete_speed_bonus(self.h.guts);
        self.compete_accel_bonus = compete_accel_bonus(self.h.guts);
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn hp(&self) -> f64 {
        self.hp
    }

    pub fn max_hp(&self) -> f64 {
        self.max_hp
    }

    pub fn t(&self) -> f64 {
        self.t
    }

    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn strategy(&self) -> Strategy {
        self.h.strategy
    }

    pub fn finished(&self) -> bool {
        self.pos >= self.course_dist
    }

    pub fn result(&self) -> RaceResult {
        RaceResult {
            finish_time: self.t,
            frames: self.frames,
            final_pos: self.pos,
            hp_ratio: if self.infinite_hp {
                1.0
            } else {
                (self.hp / self.max_hp).max(0.0)
            },
        }
    }

    pub fn step(&mut self, dt_frame: f64, ctx: &StepCtx) {
        if self.finished() {
            return;
        }

        self.frames += 1;
        let dt = dt_frame;
        let mut dt_pos = dt_frame;
        self.t += dt_frame;

        // Advance timers every frame (including start-delay frames).
        for ef in &mut self.active {
            ef.timer += dt;
        }
        self.pos_keep_timer += dt;

        // Approximate blocked_side / overtake: 1 Hz Markov (umalator conditionTimer).
        // Phase for rates uses current self.phase; updated later in the frame after hills.
        let phase_for_sc = phase_at(self.course_dist, self.pos);
        self.special.on_frame(
            dt,
            phase_for_sc,
            self.h.strategy,
            self.pos,
            self.section_len,
            self.current_lane,
            self.horse_lane,
        );

        self.active.retain(|ef| {
            if ef.timer >= 0.0 {
                match ef.kind {
                    EffectType::TargetSpeed => self.mod_target -= ef.modifier,
                    EffectType::Accel => self.mod_accel -= ef.modifier,
                    EffectType::CurrentSpeed | EffectType::CurrentSpeedDecel => {
                        // Type 22's oneFrameAccel transfer is unused in the fork oracle;
                        // match CurrentSpeed drop for checkpoint parity.
                        self.mod_current -= ef.modifier
                    }
                    EffectType::LaneMove => {}
                    _ => {}
                }
                false
            } else {
                true
            }
        });

        if self.start_delay_acc > 0.0 {
            self.start_delay_acc -= dt;
            if self.start_delay_acc > 0.0 {
                return;
            }
            // Oracle: speed/HP integrate with full frame dt; only position uses the residual.
            dt_pos = (-self.start_delay_acc).abs();
            self.start_delay_acc = 0.0;
        }

        let phase = phase_at(self.course_dist, self.pos);
        let section = ((self.pos / self.section_len).floor() as usize).min(23);
        let section_bonus = if self.is_last_spurt {
            0.0
        } else {
            self.section_modifiers[section]
        };

        // Enter / leave hills
        if self.hill_idx < 0 {
            if let Some(&idx) = self.hill_start_q.last() {
                if self.pos >= self.slopes[idx].start {
                    self.hill_idx = idx as i32;
                    self.hill_start_q.pop();
                    self.downhill_tick = 0.0;
                    let slope = self.slopes[idx].slope;
                    if slope < 0.0 {
                        let roll = self.downhill_rngs[idx].random();
                        if roll < self.h.wisdom * 0.0004 {
                            self.downhill_mode = true;
                        }
                    }
                }
            }
        } else if let Some(&idx) = self.hill_end_q.last() {
            let end = self.slopes[idx].start + self.slopes[idx].length;
            if self.pos > end {
                self.hill_idx = -1;
                self.hill_end_q.pop();
                self.downhill_mode = false;
            }
        }

        let slope_per = if self.hill_idx >= 0 {
            self.slopes[self.hill_idx as usize].slope
        } else {
            0.0
        };
        let uphill = slope_per > 0.0;

        if self.hill_idx >= 0 {
            self.downhill_tick += dt;
            if self.downhill_tick >= 1.0 {
                self.downhill_tick = 0.0;
                let i = self.hill_idx as usize;
                let roll = self.downhill_rngs[i].random();
                if self.downhill_mode && roll > 0.8 {
                    self.downhill_mode = false;
                } else if !self.downhill_mode && slope_per < 0.0 && roll < self.h.wisdom * 0.0004
                {
                    self.downhill_mode = true;
                }
            }
        }

        if self.rushed_section >= 0
            && !self.is_rushed
            && !self.has_been_rushed
            && self.pos >= self.section_len * self.rushed_section as f64
        {
            self.is_rushed = true;
            self.has_been_rushed = true;
            self.rushed_timer = 0.0;
        }
        if self.is_rushed {
            self.rushed_timer += dt;
            // Oracle behaviour (black-box): single clear check at 3s with ≈55% chance;
            // failure holds until the 12s hard max. (A every-3s×55% model incorrectly
            // cleared cp_7/cp_8 at 6s — those fixtures fail the one 3s roll and ride to 12s.)
            if !self.rushed_clear_checked && self.rushed_timer >= 3.0 {
                self.rushed_clear_checked = true;
                if self.rushed_rng.random() < 0.55 {
                    self.is_rushed = false;
                }
            }
            if self.rushed_timer > 12.0 {
                self.is_rushed = false;
            }
        }

        if !self.spurt_decided && self.pos >= phase_start(self.course_dist, Phase::End) {
            self.spurt_decided = true;
            if self.infinite_hp {
                self.last_spurt_v = last_spurt_speed(&self.h, self.course_dist);
                self.last_spurt_transition = -1.0;
            } else {
                let (transition, v) = choose_spurt(
                    &self.h,
                    self.course_dist,
                    self.pos,
                    self.hp,
                    self.surface,
                    self.ground,
                    &mut self.hp_rng,
                );
                self.last_spurt_transition = transition;
                self.last_spurt_v = v;
            }
        }
        if self.spurt_decided
            && !self.is_last_spurt
            && (self.last_spurt_transition < 0.0 || self.pos >= self.last_spurt_transition)
        {
            self.is_last_spurt = true;
        }

        // Skill activations
        for i in (0..self.pending.len()).rev() {
            if self.pos >= self.pending[i].trigger.end {
                self.pending.remove(i);
                continue;
            }
            let skill_id = self.pending[i].skill_id.clone();
            let already = self.used_skills.contains(&skill_id);
            let can_fire = self.pos >= self.pending[i].trigger.start
                && dynamics_ok(
                    &self.pending[i].dynamics,
                    self.t,
                    self.is_last_spurt,
                    self.last_spurt_transition,
                    self.start_delay,
                    self.random_lot,
                    if self.infinite_hp { 1.0 } else { (self.hp / self.max_hp).max(0.0) },
                    ctx.place,
                    ctx.field_size,
                    &self.activate_count,
                    self.activate_count_heal,
                    already,
                    self.gate_roll,
                    self.num_umas,
                    &self.used_skills,
                );
            if !can_fire {
                continue;
            }
            let chance = skill_activation_chance(self.raw_wisdom);
            if !self.pending[i].skip_wisdom && self.wisdom_roll_rng.random() > chance {
                self.pending.remove(i);
                continue;
            }
            let scale = self.course_dist / 1000.0;
            let effects = self.pending[i].effects.clone();
            for ef in &effects {
                match ef.kind {
                    EffectType::Heal => {
                        self.activate_count_heal += 1;
                        if !self.infinite_hp {
                            self.hp = (self.hp + self.max_hp * ef.modifier).min(self.max_hp);
                        }
                        // umalator activateSkill Recovery: re-evaluate last spurt when
                        // already in phase≥2 and not yet spurting (heal after HP death).
                        let ph = phase_at(self.course_dist, self.pos);
                        if matches!(ph, Phase::End | Phase::LastSpurt) && !self.is_last_spurt {
                            self.spurt_decided = true;
                            if self.infinite_hp {
                                self.last_spurt_v = last_spurt_speed(&self.h, self.course_dist);
                                self.last_spurt_transition = -1.0;
                            } else {
                                let (transition, v) = choose_spurt(
                                    &self.h,
                                    self.course_dist,
                                    self.pos,
                                    self.hp,
                                    self.surface,
                                    self.ground,
                                    &mut self.hp_rng,
                                );
                                self.last_spurt_transition = transition;
                                self.last_spurt_v = v;
                            }
                            if self.last_spurt_transition < 0.0
                                || self.pos >= self.last_spurt_transition
                            {
                                self.is_last_spurt = true;
                            }
                        }
                    }
                    EffectType::SpeedUp => {
                        self.h.speed = (self.h.speed + ef.modifier).max(1.0);
                    }
                    EffectType::StaminaUp => {
                        self.h.stamina = (self.h.stamina + ef.modifier).max(1.0);
                        // Mid-race greens: grow max HP; keep current ratio.
                        if !self.infinite_hp {
                            let ratio = if self.max_hp > 0.0 {
                                self.hp / self.max_hp
                            } else {
                                1.0
                            };
                            self.max_hp = max_hp(self.h.strategy, self.h.stamina, self.course_dist);
                            self.hp = (self.max_hp * ratio).min(self.max_hp);
                        }
                    }
                    EffectType::PowerUp => {
                        self.h.power = (self.h.power + ef.modifier).max(1.0);
                    }
                    EffectType::GutsUp => {
                        self.h.guts = (self.h.guts + ef.modifier).max(1.0);
                        self.min_spd = min_speed(self.course_dist, self.h.guts);
                    }
                    EffectType::WisdomUp => {
                        self.h.wisdom = (self.h.wisdom + ef.modifier).max(1.0);
                    }
                    EffectType::MultiplyStartDelay => {
                        self.start_delay *= ef.modifier;
                        self.start_delay_acc *= ef.modifier;
                    }
                    EffectType::SetStartDelay => {
                        self.start_delay = ef.modifier;
                        self.start_delay_acc = ef.modifier;
                    }
                    EffectType::TargetSpeed => {
                        self.mod_target += ef.modifier;
                        self.active.push(ActiveEffect {
                            kind: ef.kind,
                            modifier: ef.modifier,
                            timer: -(ef.duration * scale),
                        });
                    }
                    EffectType::Accel => {
                        self.mod_accel += ef.modifier;
                        self.active.push(ActiveEffect {
                            kind: ef.kind,
                            modifier: ef.modifier,
                            timer: -(ef.duration * scale),
                        });
                    }
                    EffectType::CurrentSpeed | EffectType::CurrentSpeedDecel => {
                        self.mod_current += ef.modifier;
                        self.active.push(ActiveEffect {
                            kind: ef.kind,
                            modifier: ef.modifier,
                            timer: -(ef.duration * scale),
                        });
                    }
                    EffectType::LaneMove => {
                        // Type 28 LaneMovementSpeed — lateral only; see target-speed gate.
                        self.active.push(ActiveEffect {
                            kind: ef.kind,
                            modifier: ef.modifier,
                            timer: -(ef.duration * scale),
                        });
                    }
                    EffectType::Other => {}
                }
            }
            let phase_idx = match phase_at(self.course_dist, self.pos) {
                Phase::Opening => 0,
                Phase::Middle => 1,
                Phase::End | Phase::LastSpurt => 2,
            };
            self.activate_count[phase_idx] += 1;
            self.used_skills.insert(skill_id);
            self.pending.remove(i);
        }

        // Virtual / Approximate position-keep (pack PaceUp/Down + Nige SpeedUp/Overtake).
        // Approximate with a pacer matches Virtual for default-pacer fixtures (oracle-equal).
        let mut pk_coef = 1.0;
        if matches!(
            ctx.pos_keep_mode,
            PosKeepMode::Virtual | PosKeepMode::Approximate
        ) {
            if self.pos >= self.pos_keep_end {
                self.pos_keep_state = PosKeepState::None;
            } else if let Some(pacer_pos) = ctx.pacer_pos {
                let has_speed_skills = self.active.iter().any(|ef| {
                    matches!(
                        ef.kind,
                        EffectType::TargetSpeed
                            | EffectType::CurrentSpeed
                            | EffectType::CurrentSpeedDecel
                    )
                });
                let timer_ready = self.pos_keep_timer >= 0.0;
                let gap_ahead = ctx
                    .second_pos
                    .map(|s| self.pos - s)
                    .unwrap_or(f64::INFINITY);
                let (st, ex_pos, ex_dist, next_t) =
                    if matches!(self.h.strategy, Strategy::Nige | Strategy::Oonige) {
                        tick_nige_pos_keep(
                            self.pos_keep_state,
                            self.h.strategy,
                            ctx.am_i_pacer,
                            gap_ahead,
                            self.pos,
                            self.pos_keep_exit_pos,
                            self.pos_keep_exit_dist,
                            timer_ready,
                            self.h.wisdom,
                            &mut self.pos_keep_rng,
                            self.section_len,
                        )
                    } else {
                        let behind = pacer_pos - self.pos;
                        self.debug_last_pk_behind = behind;
                        tick_pack_pos_keep(
                            self.pos_keep_state,
                            self.h.strategy,
                            behind,
                            self.pos_keep_min_th,
                            self.pos_keep_max_th,
                            self.pos,
                            self.pos_keep_exit_pos,
                            self.pos_keep_exit_dist,
                            timer_ready,
                            has_speed_skills,
                            self.h.wisdom,
                            &mut self.pos_keep_rng,
                            self.section_len,
                        )
                    };
                self.pos_keep_state = st;
                self.pos_keep_exit_pos = ex_pos;
                self.pos_keep_exit_dist = ex_dist;
                // Sentinel −1.0 = leave timer alone; −2/−3 are cooldowns (count up each frame).
                if (next_t + 1.0).abs() > 1e-9 {
                    self.pos_keep_timer = next_t;
                }
            }
            pk_coef = pos_keep_speed_coef(self.pos_keep_state);
        }

        // Empty HP overrides last-spurt; slope mods apply for all targets (oracle order).
        let exhausted = !self.infinite_hp && self.hp <= 0.0;
        let mut target = if exhausted {
            self.min_spd
        } else if self.is_last_spurt {
            self.last_spurt_v
        } else {
            base_target_speed(&self.h, self.course_dist, phase) * pk_coef + section_bonus
        };
        target += self.mod_target;
        // umalator: MoveLaneModifier only while changing lanes with an active type-28 skill.
        let has_lane_move = self
            .active
            .iter()
            .any(|ef| matches!(ef.kind, EffectType::LaneMove));
        if self.lane_change_speed > 0.0 && has_lane_move {
            target += (0.0002 * self.h.power).max(0.0).sqrt();
        }
        if self.lead_comp_active {
            target += self.lead_comp_bonus;
        }
        if self.compete_active {
            target += self.compete_speed_bonus;
        }
        if self.downhill_mode {
            target += downhill_speed_bonus(slope_per);
        } else if uphill {
            target -= uphill_penalty(slope_per, self.h.power);
            target = target.max(self.min_spd);
        }
        // Do not clamp target up to min_spd — PaceDown may sit below minSpeed while
        // startDash is still active (umalator updateTargetSpeed). Cap only the ceiling.
        target = target.min(30.0);

        // Oracle applyForces: decelerating early-returns (no start-dash / skill accel).
        let a = if exhausted {
            EXHAUSTED_DECEL
        } else if self.speed > target {
            if self.pos_keep_state == PosKeepState::PaceDown {
                PACE_DOWN_DECEL
            } else {
                PHASE_DECEL[match phase {
                    Phase::Opening => 0,
                    Phase::Middle => 1,
                    Phase::End | Phase::LastSpurt => 2,
                }]
            }
        } else if self.speed < target {
            let mut accel = accel_rate(&self.h, phase, uphill);
            accel += self.mod_accel;
            if self.compete_active {
                accel += self.compete_accel_bonus;
            }
            if self.in_start_dash {
                accel += START_DASH_ACCEL;
            }
            accel
        } else {
            0.0
        };

        // Speed integration matches oracle RaceSolver.step.
        let old_speed = self.speed;
        let mut new_speed = if old_speed <= target {
            (old_speed + a * dt).min(target)
        } else {
            (old_speed + a * dt).max(target)
        };
        new_speed = new_speed.max(0.0);
        // Oracle getMaxStartDashSpeed = min(targetSpeed, 0.85*baseSpeed).
        if self.in_start_dash {
            let cap = target.min(self.dash_cap);
            if new_speed > cap {
                new_speed = cap;
            }
        }
        self.speed = new_speed;
        // Oracle order: minSpeed floor uses pre-frame startDash, then end startDash.
        // Ending dash before the floor incorrectly snaps to min_spd on the cap frame.
        if !self.in_start_dash && old_speed < self.min_spd {
            self.speed = self.min_spd;
        }
        if self.in_start_dash && self.speed >= self.dash_cap {
            self.in_start_dash = false;
        }

        let status = StatusModifiers {
            // When lead competition is active, rushed is folded into lead_comp_hp_factor
            // (Nige+rushed = 3.6× total, not 1.6× × 3.6×).
            rushed: self.is_rushed && !self.lead_comp_active,
            downhill: self.downhill_mode,
            pace_down: self.pos_keep_state == PosKeepState::PaceDown,
            ..Default::default()
        };
        if !self.infinite_hp && self.hp > 0.0 {
            let mut drain = hp_consume(
                self.speed,
                self.course_dist,
                phase,
                self.h.guts,
                self.surface,
                self.ground,
                status,
            ) * dt;
            if self.lead_comp_active {
                drain *= lead_comp_hp_factor(self.h.strategy, self.is_rushed);
            }
            self.hp -= drain;
            if self.hp < 0.0 {
                self.hp = 0.0;
            }
        }

        self.pos += (self.speed + self.mod_current) * dt_pos;
        self.apply_lane_movement(phase);
    }

    /// Lateral lane update (umalator `applyLaneMovement`). Type 28 boosts lateral
    /// `actualSpeed`; forward MoveLaneModifier is gated in target-speed when
    /// `lane_change_speed > 0`. Type 35 ChangeLane (outer rail) is not modeled yet.
    fn apply_lane_movement(&mut self, phase: Phase) {
        let current = self.current_lane;
        let side_blocked = self.special.blocked_side.active();
        let overtake = self.special.overtake.active();
        let lane_skill_mod: f64 = self
            .active
            .iter()
            .filter(|ef| matches!(ef.kind, EffectType::LaneMove))
            .map(|ef| ef.modifier)
            .sum();

        if self.extra_move_lane < 0.0 && self.pos >= self.last_straight_start {
            self.extra_move_lane = (current / 0.1)
                .min(self.max_lane_distance)
                * 0.5
                + self.lane_rng.random() * 0.1;
        }

        if overtake {
            // umalator: Math.max(targetLane, horseLane, extraMoveLane); -1 extra is ignored by max.
            self.target_lane = self
                .target_lane
                .max(self.horse_lane)
                .max(self.extra_move_lane);
        } else if !self.infinite_hp && self.hp <= 0.0 {
            self.target_lane = current;
        } else if self.pos_keep_state == PosKeepState::PaceDown {
            self.target_lane = 0.18;
        } else if self.extra_move_lane > current {
            self.target_lane = self.extra_move_lane;
        } else if matches!(phase, Phase::Opening | Phase::Middle) && !side_blocked {
            self.target_lane = (current - 0.05).max(0.0);
        } else {
            self.target_lane = current;
        }

        if (side_blocked && self.target_lane < current)
            || (self.target_lane - current).abs() < 1e-5
        {
            self.lane_change_speed = 0.0;
            return;
        }

        let mut tgt_spd = 0.02 * (0.3 + 0.001 * self.h.power);
        if self.pos < self.move_lane_point && self.max_lane_distance > 0.0 {
            tgt_spd *= 1.0 + current / self.max_lane_distance * 0.05;
        }
        self.lane_change_speed =
            (self.lane_change_speed + self.lane_change_accel_pf).min(tgt_spd);
        let actual = (self.lane_change_speed + lane_skill_mod).min(0.6);

        if self.target_lane > current {
            self.current_lane = (current + actual).min(self.target_lane);
        } else {
            self.current_lane = (current - actual * (1.0 + current)).max(self.target_lane);
        }
    }

    pub fn current_lane(&self) -> f64 {
        self.current_lane
    }
}

pub fn simulate_solo(
    course: &Course,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
) -> RaceResult {
    let mut runner = HorseRunner::new(course, ground, horse, seed);
    let ctx = StepCtx::default();
    while !runner.finished() {
        runner.step(DT, &ctx);
    }
    runner.result()
}

/// Debug: per-frame (t, pos, speed, hp_ratio) for first-divergence vs oracle traces.
pub fn simulate_solo_trace(
    course: &Course,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
) -> Vec<(f64, f64, f64, f64)> {
    let mut runner = HorseRunner::new(course, ground, horse, seed);
    let ctx = StepCtx::default();
    let mut out = Vec::new();
    while !runner.finished() {
        runner.step(DT, &ctx);
        out.push((
            runner.t(),
            runner.pos(),
            runner.speed(),
            if runner.max_hp() > 0.0 {
                runner.hp() / runner.max_hp()
            } else {
                0.0
            },
        ));
    }
    out
}

/// Oracle `useDefaultPacer(false)` for Virtual mode.
///
/// Copies the focus horse with strategy forced to Nige and skills cleared.
/// `HorseRunner::build` applies mood, ground, and strategy-prof wisdom as usual.
pub fn default_virtual_pacer_horse(focus: &HorseInput) -> HorseInput {
    HorseInput {
        speed: focus.speed,
        stamina: focus.stamina,
        power: focus.power,
        guts: focus.guts,
        wisdom: focus.wisdom,
        strategy: Strategy::Nige,
        distance_apt: focus.distance_apt,
        surface_apt: focus.surface_apt,
        strategy_apt: focus.strategy_apt,
        mood: focus.mood,
        skills: vec![],
    }
}

/// Two-horse Virtual race: focus horse + default Nige pacer (infinite HP).
/// Pacer solver seed: `PrandoRng::new(seed+1).int32()` (oracle basePacerRng → int32).
///
/// Matches oracle `initUmas([other, self])` + `pacer.getPacer()` designation:
/// - Oonige focus is always the designated pacemaker (checked before Nige).
/// - Nige focus is designated when at/ahead of the virtual Nige pacer.
/// - Pack strategies (Senkou/Sasi/Oikomi) use the virtual Nige as pacemaker.
pub fn simulate_with_default_pacer(
    course: &Course,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
    mode: PosKeepMode,
) -> RaceResult {
    let mut focus = HorseRunner::new(course, ground, horse, seed);

    let pacer_horse = default_virtual_pacer_horse(horse);
    let mut base_pacer = PrandoRng::new(seed.wrapping_add(1));
    let pacer_seed = base_pacer.int32();
    let mut pacer = HorseRunner::new_pacer(course, ground, &pacer_horse, pacer_seed);

    while !focus.finished() {
        // Designate before steps (oracle getPacer on current positions).
        let focus_is_designated = match focus.strategy() {
            Strategy::Oonige => true,
            Strategy::Nige => focus.pos() >= pacer.pos(),
            _ => false,
        };

        if matches!(mode, PosKeepMode::Virtual | PosKeepMode::Approximate) {
            let fp = focus.pos();
            let pp = pacer.pos();
            let fs = focus.strategy();
            let ps = pacer.strategy();
            focus.update_lead_competition(pp, ps);
            pacer.update_lead_competition(fp, fs);
            // Compete-fight is deferred on the 2-horse Virtual+default-pacer path used by
            // R8.5 checkpoints (oracle sample-0 rarely activates; deterministic wiki trigger
            // over-sped cp_11). Wired in simulate_field_synced for n≥3 career fields.
        }

        if !pacer.finished() {
            // Virtual pacer does not run position-keep (oracle never updatePacer's it).
            let p_place = if pacer.pos() >= focus.pos() { 1 } else { 2 };
            let ctx_pacer = StepCtx {
                pacer_pos: None,
                second_pos: None,
                pos_keep_mode: PosKeepMode::None,
                am_i_pacer: false,
                place: p_place,
                field_size: 2,
            };
            pacer.step(DT, &ctx_pacer);
        }

        let f_place = if focus.pos() >= pacer.pos() { 1 } else { 2 };
        let ctx = if matches!(mode, PosKeepMode::Virtual | PosKeepMode::Approximate) {
            if focus_is_designated {
                // Front-runner SpeedUp/Overtake: gap vs the other horse (virtual pacer).
                StepCtx {
                    pacer_pos: Some(focus.pos()),
                    second_pos: Some(pacer.pos()),
                    pos_keep_mode: mode,
                    am_i_pacer: true,
                    place: f_place,
                    field_size: 2,
                }
            } else {
                // Pack PaceUp/Down relative to designated virtual Nige.
                StepCtx {
                    pacer_pos: Some(pacer.pos()),
                    second_pos: None,
                    pos_keep_mode: mode,
                    am_i_pacer: false,
                    place: f_place,
                    field_size: 2,
                }
            }
        } else {
            StepCtx {
                pacer_pos: None,
                second_pos: None,
                pos_keep_mode: mode,
                am_i_pacer: false,
                place: f_place,
                field_size: 2,
            }
        };
        focus.step(DT, &ctx);
    }
    focus.result()
}

/// Per-entrant root seed (shared by independent + synced fields).
///
/// Scheme (interim; matches prior `simulate_field_independent`):
/// - index `0` → `seed`
/// - index `i > 0` → `PrandoRng::new(seed)`, burn `i` × `int32()`, then take `int32()`
///
/// Oracle multi-entrant builder burns skill/solver/hp per sample from one builder rng;
/// a true multi-horse oracle field is not exposed yet, so this stays the in-repo contract.
pub fn entrant_seed(seed: u32, index: usize) -> u32 {
    if index == 0 {
        return seed;
    }
    let mut root = PrandoRng::new(seed);
    for _ in 0..index {
        let _ = root.int32();
    }
    root.int32()
}

/// Select pacemaker index: furthest Oonige, else furthest Nige, else furthest horse.
/// Uses positions at the start of the frame (oracle `getPacer` front-runner preference,
/// without lucky-pace strategy mutation).
pub fn select_pacer_index(runners: &[HorseRunner]) -> usize {
    let mut best_oonige: Option<(usize, f64)> = None;
    let mut best_nige: Option<(usize, f64)> = None;
    let mut best_any: Option<(usize, f64)> = None;
    for (i, r) in runners.iter().enumerate() {
        let p = r.pos();
        best_any = Some(match best_any {
            Some((bi, bp)) if bp >= p => (bi, bp),
            _ => (i, p),
        });
        match r.strategy() {
            Strategy::Oonige => {
                best_oonige = Some(match best_oonige {
                    Some((bi, bp)) if bp >= p => (bi, bp),
                    _ => (i, p),
                });
            }
            Strategy::Nige => {
                best_nige = Some(match best_nige {
                    Some((bi, bp)) if bp >= p => (bi, bp),
                    _ => (i, p),
                });
            }
            _ => {}
        }
    }
    best_oonige
        .or(best_nige)
        .or(best_any)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Best position among runners other than `exclude` (second place when exclude is the leader).
pub fn second_place_pos(runners: &[HorseRunner], exclude: Option<usize>) -> Option<f64> {
    let mut best: Option<f64> = None;
    for (i, r) in runners.iter().enumerate() {
        if Some(i) == exclude {
            continue;
        }
        let p = r.pos();
        best = Some(match best {
            Some(b) if b >= p => b,
            _ => p,
        });
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::get_course;
    use crate::solver::Aptitude;

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

    #[test]
    fn virtual_default_pacer_case1_within_one_frame() {
        let c = get_course(10601).expect("course 10601");
        let r = simulate_with_default_pacer(
            c,
            GroundCondition::Good,
            &case1_horse(),
            2615953739,
            PosKeepMode::Virtual,
        );
        let oracle = 66.60000000000134;
        let delta = (r.finish_time - oracle).abs();
        eprintln!(
            "virtual case1 finish={:.6} oracle={:.6} Δ={:.6}s frames={} hp={}",
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
}
