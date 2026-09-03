//! Port of RaceSchedulerTest.kt

mod common;

use common::{base_state, with_fans, with_phase};
use uma_sim_core::race::RaceScheduler;
use uma_sim_core::state::TurnPhase;

#[test]
fn suggests_race_when_fans_low() {
    let state = with_fans(base_state("ura", 20), 1000);
    assert!(RaceScheduler::should_run_optional_race(&state));
}

#[test]
fn skips_when_fans_high() {
    let state = with_fans(base_state("ura", 20), 5000);
    assert!(!RaceScheduler::should_run_optional_race(&state));
}

#[test]
fn skips_early_turns() {
    let state = with_fans(base_state("ura", 3), 1000);
    assert!(!RaceScheduler::should_run_optional_race(&state));
}

#[test]
fn skips_non_free_phase() {
    let state = with_phase(with_fans(base_state("ura", 20), 1000), TurnPhase::Event);
    assert!(!RaceScheduler::should_run_optional_race(&state));
}
