//! Mid-run Inspiration (Classic/Senior Early April) integration.

use uma_sim_core::calendar::TurnCalendar;
use uma_sim_core::mid_run_inheritance::{apply_mid_run_inspiration, is_mid_run_inspiration_date};
use uma_sim_core::rng::SimRandom;
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::SimEngine;

#[test]
fn mid_run_inspiration_applies_blue_when_compat_guarantees_proc() {
    let mut meta = RunMeta::new(7, "ura", "Special Week");
    meta.legacy_factors = vec!["factor:blue:1@3".into()];
    // Odds = 0.90 * (1 + 500/100) → clamped to 1.0
    meta.compatibility_score = 500;

    let mut engine = SimEngine::new(SimSettings {
        race_model: uma_sim_core::race::RaceModel::Stub,
        ..Default::default()
    });
    engine.start(meta);
    let before_speed = engine.state().stats.speed;

    // Advance calendar to Classic Early April without full play (mutate date).
    let mut state = engine.state().clone();
    state.date.year = 2;
    state.date.month = 4;
    state.date.half = 1;
    assert!(is_mid_run_inspiration_date(&state.date));

    let mut rng = SimRandom::with_trace(99, false);
    let (after, lines) = apply_mid_run_inspiration(&state, &mut rng);
    assert!(
        after.stats.speed > before_speed,
        "expected blue speed gain, lines={lines:?}"
    );
    assert_eq!(after.legacy.inspiration_events_done, 1);
    assert!(lines.iter().any(|l| l.contains("Inherited blue")));
}

#[test]
fn career_start_calendar_is_not_april_inspiration() {
    let cal = TurnCalendar::career_start();
    assert!(!is_mid_run_inspiration_date(&cal.date));
}
