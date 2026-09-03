//! Clean-room race physics for mid-run career simulation.
//!
//! Constants are sourced from `research/race_model_constants.json` and
//! `knowledge/mechanics/race_model.md`. The quarantined umalator oracle verifies
//! behaviour; it does not supply implementation.

pub mod compete_fight;
pub mod condition;
pub mod course;
pub mod field;
pub mod hp;
pub mod lead_comp;
pub mod physics;
pub mod pos_keep;
pub mod rng;
pub mod runner;
pub mod solver;
pub mod special_conditions;

pub use condition::{parse_condition, Atom, Expr, Op};
pub use course::{get_course, Course};
pub use field::{
    course_factor, max_threshold, min_threshold, pos_keep_speed_coef, simulate_field_independent,
    simulate_field_synced, simulate_with_default_pacer, FieldResult, Finisher, PosKeepMode,
    PosKeepState,
};
pub use hp::{
    guts_modifier, hp_per_second, max_hp, spurt_accept_threshold, Aptitude, GroundCondition,
    Strategy,
};
pub use physics::{base_speed, phase_end, phase_start, Phase};
pub use rng::PrandoRng;
pub use runner::{simulate_solo_trace, entrant_seed, HorseRunner, StepCtx};
pub use solver::{simulate_solo, simulate_solo_by_id, HorseInput, RaceResult};
