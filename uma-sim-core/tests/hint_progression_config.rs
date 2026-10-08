use uma_sim_core::{HintProgressionConfig, InspirationConfig, SimRandom};

#[test]
fn caps_at_max_level() {
    assert_eq!(HintProgressionConfig::apply_training_hint(4), 5);
    assert_eq!(HintProgressionConfig::apply_training_hint(5), 5);
}

#[test]
fn increments_below_max() {
    assert_eq!(HintProgressionConfig::apply_training_hint(1), 2);
}

#[test]
fn roll_bonus_within_range() {
    let mut rng = SimRandom::new(12345);
    let min = InspirationConfig::stat_bonus_min();
    let max = InspirationConfig::stat_bonus_max();
    for _ in 0..10 {
        let bonus = InspirationConfig::roll_bonus(&mut rng);
        assert!((min..=max).contains(&bonus));
    }
}

#[test]
fn event_options_include_bonus() {
    let opts = InspirationConfig::event_options(15);
    assert!(opts.iter().all(|o| o.contains("+15")));
}
