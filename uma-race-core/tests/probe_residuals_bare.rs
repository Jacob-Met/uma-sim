//! Compare bare vs full finish for remaining fails.

use uma_race_core::{
    get_course, simulate_solo, simulate_with_default_pacer, Aptitude, GroundCondition, HorseInput,
    PosKeepMode, Strategy,
};

fn case(id: &str) -> (HorseInput, u32, u32, GroundCondition, bool, Vec<String>, f64) {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let c = v["cases"].as_array().unwrap().iter().find(|x| x["id"] == id).unwrap();
    let h = &c["horse"];
    let apt = |k: &str| Aptitude::from_str_letter(h[k].as_str().unwrap()).unwrap();
    let strat = match h["strategy"].as_str().unwrap() {
        "Nige" => Strategy::Nige,
        "Senkou" => Strategy::Senkou,
        "Sasi" => Strategy::Sasi,
        "Oikomi" => Strategy::Oikomi,
        "Oonige" => Strategy::Oonige,
        _ => panic!(),
    };
    let ground = match c["ground"].as_str().unwrap() {
        "Good" => GroundCondition::Good,
        "Yielding" => GroundCondition::Yielding,
        "Soft" => GroundCondition::Soft,
        "Heavy" => GroundCondition::Heavy,
        _ => panic!(),
    };
    let skills: Vec<String> = c["skills"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
    let horse = HorseInput {
        speed: h["speed"].as_f64().unwrap(),
        stamina: h["stamina"].as_f64().unwrap(),
        power: h["power"].as_f64().unwrap(),
        guts: h["guts"].as_f64().unwrap(),
        wisdom: h["wisdom"].as_f64().unwrap(),
        strategy: strat,
        distance_apt: apt("distanceAptitude"),
        surface_apt: apt("surfaceAptitude"),
        strategy_apt: apt("strategyAptitude"),
        mood: c["mood"].as_i64().unwrap() as i8,
        skills: skills.clone(),
    };
    (
        horse,
        c["seed_u32"].as_u64().unwrap() as u32,
        c["course_id"].as_u64().unwrap() as u32,
        ground,
        c["pace_effects"].as_bool().unwrap(),
        skills,
        c["expected_finish"].as_f64().unwrap(),
    )
}

fn run(id: &str) {
    let (mut horse, seed, cid, ground, pace, skills, exp) = case(id);
    let c = get_course(cid).unwrap();
    let full = if pace {
        simulate_with_default_pacer(c, ground, &horse, seed, PosKeepMode::Virtual)
    } else {
        simulate_solo(c, ground, &horse, seed)
    };
    horse.skills.clear();
    let bare = if pace {
        simulate_with_default_pacer(c, ground, &horse, seed, PosKeepMode::Virtual)
    } else {
        simulate_solo(c, ground, &horse, seed)
    };
    eprintln!(
        "{id} pace={pace} nskills={} exp={exp:.4} full={:.4} bare={:.4} dFull={:.4} skills={:?}",
        skills.len(),
        full.finish_time,
        bare.finish_time,
        full.finish_time - exp,
        skills
    );
}

#[test]
fn probe_residuals_bare() {
    for id in [
        "v3_586_10310_modeled_only",
        "v3_394_10914_modeled_only",
        "v3_534_10311_modeled_only",
        "v3_690_10810_modeled_only",
        "v3_753_10908_modeled_only",
    ] {
        run(id);
    }
}
