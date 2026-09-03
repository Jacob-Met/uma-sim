use uma_race_core::runner::{default_virtual_pacer_horse, HorseRunner, StepCtx};
use uma_race_core::{
    get_course, Aptitude, GroundCondition, HorseInput, PosKeepMode, PrandoRng, Strategy,
};

const DT: f64 = 1.0 / 15.0;

#[test]
fn probe_534_procs() {
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
    let skills: Vec<String> = c["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    let horse = HorseInput {
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
        skills: skills.clone(),
    };
    let course = get_course(c["course_id"].as_u64().unwrap() as u32).unwrap();
    let seed = c["seed_u32"].as_u64().unwrap() as u32;
    let mut focus = HorseRunner::new(course, GroundCondition::Good, &horse, seed);
    let pacer_h = default_virtual_pacer_horse(&horse);
    let mut base_pacer = PrandoRng::new(seed.wrapping_add(1));
    let mut pacer =
        HorseRunner::new_pacer(course, GroundCondition::Good, &pacer_h, base_pacer.int32());
    let mut prev: Vec<String> = focus.debug_used_skills();
    while !focus.finished() {
        if !pacer.finished() {
            pacer.step(
                DT,
                &StepCtx {
                    pacer_pos: None,
                    second_pos: None,
                    pos_keep_mode: PosKeepMode::None,
                    am_i_pacer: false,
                    place: 1,
                    field_size: 2,
                },
            );
        }
        focus.step(
            DT,
            &StepCtx {
                pacer_pos: Some(pacer.pos()),
                second_pos: None,
                pos_keep_mode: PosKeepMode::Virtual,
                am_i_pacer: false,
                place: 2,
                field_size: 2,
            },
        );
        let used = focus.debug_used_skills();
        if used != prev {
            for id in used.iter().filter(|id| !prev.contains(id)) {
                eprintln!(
                    "PROC {id} @ pos={:.3} t={:.3}",
                    focus.pos(),
                    focus.accum_time()
                );
            }
            prev = used;
        }
    }
    eprintln!(
        "finish={:.6} used={:?} spd={} stm={} pow={} guts={} wis={} maxhp={:.1}",
        focus.accum_time(),
        focus.debug_used_skills(),
        focus.debug_stats().0,
        focus.debug_stats().1,
        focus.debug_stats().2,
        focus.debug_stats().3,
        focus.debug_stats().4,
        focus.max_hp(),
    );
}
