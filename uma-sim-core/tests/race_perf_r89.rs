//! Race physics perf gates (R8.9 / plan §9).

use std::time::Instant;
use uma_race_core::{
    get_course, simulate_field_synced, Aptitude, GroundCondition, HorseInput, PosKeepMode, Strategy,
};
use uma_sim_core::state::{RunMeta, SimSettings};
use uma_sim_core::{RaceModel, SimEngine};

fn horse(speed: f64, strategy: Strategy, i: usize) -> HorseInput {
    HorseInput {
        speed: speed + (i as f64) * 5.0,
        stamina: 900.0,
        power: 900.0,
        guts: 800.0,
        wisdom: 800.0,
        strategy,
        distance_apt: Aptitude::A,
        surface_apt: Aptitude::A,
        strategy_apt: Aptitude::A,
        mood: 1,
        skills: vec![],
    }
}

#[test]
fn eighteen_horse_2000m_under_10ms_p50() {
    // Tokyo 2000m (course 10604) — plan §9 budget race.
    let course = get_course(10604).expect("Tokyo 2000m course 10604");

    let strategies = [
        Strategy::Nige,
        Strategy::Senkou,
        Strategy::Sasi,
        Strategy::Oikomi,
    ];
    let field: Vec<_> = (0..18)
        .map(|i| horse(1000.0, strategies[i % 4], i))
        .collect();

    let mut samples = Vec::with_capacity(21);
    for seed in 0u32..21 {
        let t0 = Instant::now();
        let _ = simulate_field_synced(
            course,
            GroundCondition::Good,
            &field,
            1000 + seed,
            PosKeepMode::Virtual,
        );
        samples.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = samples[10];
    eprintln!(
        "18-horse ~{}m Virtual p50={p50:.3}ms (min={:.3} max={:.3})",
        course.distance, samples[0], samples[20]
    );
    if cfg!(debug_assertions) {
        // Debug builds are ~10–30× slower; plan §9 budget is release.
        eprintln!("debug build: soft-check only (release gate is ≤10ms)");
        assert!(p50 < 150.0, "debug sanity: p50={p50:.3}ms unexpectedly high");
        return;
    }
    assert!(
        p50 <= 10.0,
        "plan §9: ≤10ms p50 for 18-horse ~2000m (release); got {p50:.3}ms"
    );
}

#[test]
fn physics_career_race_budget_under_250ms() {
    // Full career includes training; race portion alone is <<250ms (release re-freeze ~3ms/career).
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 100,
        race_model: RaceModel::Physics,
        ..Default::default()
    });
    let t0 = Instant::now();
    engine.start(RunMeta::new(4242, "ura", "PerfRace"));
    engine.play_to_completion(500);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("physics career wall={ms:.2}ms");
    if cfg!(debug_assertions) {
        eprintln!("debug build: soft-check only (release gate is ≤250ms)");
        assert!(
            ms < 2000.0,
            "debug sanity: career wall={ms:.2}ms unexpectedly high"
        );
        return;
    }
    assert!(
        ms < 250.0,
        "plan §9: ≤250ms race time per career (proxy: full physics career wall); got {ms:.2}ms"
    );
}
