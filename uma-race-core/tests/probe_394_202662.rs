use uma_race_core::runner::HorseRunner;
use uma_race_core::{
    get_course, Aptitude, GroundCondition, HorseInput, PosKeepMode, StepCtx, Strategy,
};

const DT: f64 = 1.0 / 15.0;

#[test]
fn probe_394_202662() {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let c = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == "v3_394_10914_modeled_only")
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
        strategy: Strategy::Sasi,
        distance_apt: apt("distanceAptitude"),
        surface_apt: apt("surfaceAptitude"),
        strategy_apt: apt("strategyAptitude"),
        mood: c["mood"].as_i64().unwrap() as i8,
        skills,
    };
    let course = get_course(10914).unwrap();
    let mut focus = HorseRunner::new(
        course,
        GroundCondition::Soft,
        &horse,
        c["seed_u32"].as_u64().unwrap() as u32,
    );
    while !focus.finished() {
        let pos_before = focus.pos();
        focus.step(
            DT,
            &StepCtx {
                pacer_pos: None,
                second_pos: None,
                pos_keep_mode: PosKeepMode::None,
                am_i_pacer: false,
                place: 1,
                field_size: 1,
            },
        );
        if pos_before < 2262.333 && focus.pos() >= 2252.0 {
            eprintln!(
                "t={:.3} pos {:.3}->{:.3} used={:?} pending={:?}",
                focus.accum_time(),
                pos_before,
                focus.pos(),
                focus.debug_used_skills(),
                focus
                    .debug_pending_triggers()
                    .iter()
                    .filter(|p| p.0 == "202662" || p.0 == "200352")
                    .collect::<Vec<_>>()
            );
        }
    }
    eprintln!("final used={:?}", focus.debug_used_skills());
}
