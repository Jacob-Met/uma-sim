use uma_race_core::runner::HorseRunner;
use uma_race_core::{get_course, Aptitude, GroundCondition, HorseInput, Strategy};

#[test]
fn probe_534_stats() {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let c = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == "v3_534_10311_modeled_only")
        .unwrap();
    let h = &c["horse"];
    let apt = |k: &str| Aptitude::from_str_letter(h[k].as_str().unwrap()).unwrap();
    let mk = |skills: Vec<String>| HorseInput {
        speed: h["speed"].as_f64().unwrap(),
        stamina: h["stamina"].as_f64().unwrap(),
        power: h["power"].as_f64().unwrap(),
        guts: h["guts"].as_f64().unwrap(),
        wisdom: h["wisdom"].as_f64().unwrap(),
        strategy: Strategy::Senkou,
        distance_apt: apt("distanceAptitude"),
        surface_apt: apt("surfaceAptitude"),
        strategy_apt: apt("strategyAptitude"),
        mood: c["mood"].as_i64().unwrap() as i8,
        skills,
    };
    let course = get_course(10311).unwrap();
    let seed = c["seed_u32"].as_u64().unwrap() as u32;
    for (label, skills) in [
        ("bare", vec![]),
        (
            "full",
            c["skills"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect(),
        ),
    ] {
        let horse = mk(skills);
        let r = HorseRunner::new(course, GroundCondition::Good, &horse, seed);
        eprintln!(
            "{label} stats={:?} maxhp={:.2} used={:?}",
            r.debug_stats(),
            r.max_hp(),
            r.debug_used_skills()
        );
    }
}
