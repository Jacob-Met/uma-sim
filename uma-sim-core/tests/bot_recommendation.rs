use std::collections::BTreeSet;
use uma_sim_core::{RunMeta, SimEngine, SimSettings};

/// Exercise the real decision loop, including forced race, event, rest and
/// training phases, rather than inventing artificial choice arrays.
#[test]
fn local_bot_advice_matches_actual_bot_and_never_changes_career() {
    // The external JVM policy is not the local bot being explained.
    assert!(std::env::var_os("UMA_POLICY_CMD").is_none());
    for scenario in ["ura", "grand_concert"] {
        let mut engine = SimEngine::create(SimSettings::default());
        engine.start(RunMeta::new(42, scenario, "Special Week"));
        let mut seen = BTreeSet::new();
        for step in 0..500 {
            if engine.state().career_complete {
                break;
            }
            let choices = engine.choices();
            let original_state = format!("{:?}", engine.state());
            let original_rng_calls = engine.rng_calls();
            let recommendation = engine
                .local_bot_recommendation()
                .expect("choice before end");
            let id = recommendation.action_id();
            assert!(
                choices.iter().any(|choice| choice.id == id),
                "{scenario} turn {}: unavailable bot recommendation {id}",
                engine.state().turn
            );
            assert_eq!(
                Some(recommendation.action.clone()),
                engine.scoring_action(),
                "{scenario} step {step}: explanation differs from scoring bot"
            );
            assert_eq!(original_rng_calls, engine.rng_calls());
            assert_eq!(original_state, format!("{:?}", engine.state()));
            seen.insert(format!("{:?}", recommendation.action.kind));
            if id.starts_with("train_") {
                assert_eq!(recommendation.training_scores.len(), 5);
            } else {
                assert!(recommendation.training_scores.is_empty());
            }
            engine.step(recommendation.action);
        }
        assert!(
            engine.state().career_complete,
            "{scenario}: incomplete after 500 steps"
        );
        assert!(
            seen.contains("Race"),
            "{scenario}: did not encounter a race"
        );
        assert!(
            seen.contains("Train"),
            "{scenario}: did not encounter training"
        );
        println!("{scenario} decision kinds: {seen:?}");
    }
}

#[test]
fn local_bot_values_larger_native_gains_instead_of_facility_order() {
    let mut engine = SimEngine::create(SimSettings::default());
    engine.start(RunMeta::new(11, "unity", "Special Week"));
    engine.step(uma_sim_core::session::parse_sim_action("race"));
    assert!(engine.choices().iter().any(|choice| choice.id == "train_stamina"));

    // An upgraded facility should compete on what it actually offers. With
    // an empty target map every stat gain had zero value, leaving a 15-point
    // tie that always selected the first (Speed) facility in an empty deck.
    let mut snapshot = engine.export();
    snapshot.state.facility_levels.insert("stamina".into(), 5);
    engine.restore(snapshot);
    let rows = engine.training_inspection().rows;
    let total = |id: &str| {
        let gains = &rows.iter().find(|row| row.action_id == id).unwrap().stat_gains;
        gains.speed + gains.stamina + gains.power + gains.guts + gains.wit
    };
    assert!(total("train_stamina") > total("train_speed"));
    let recommendation = engine.local_bot_recommendation().unwrap();
    assert_eq!(recommendation.action_id(), "train_stamina");

    // A large nominal gain must still lose its value once its stat is capped.
    let mut snapshot = engine.export();
    snapshot.state.stats.stamina = 1300; // Unity's native Stamina cap.
    engine.restore(snapshot);
    assert_ne!(engine.local_bot_recommendation().unwrap().action_id(), "train_stamina");
}
