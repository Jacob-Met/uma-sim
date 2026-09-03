use uma_race_core::runner::{default_virtual_pacer_horse, HorseRunner, StepCtx};
use uma_race_core::{
    get_course, simulate_solo, Aptitude, GroundCondition, HorseInput, PosKeepMode, PrandoRng,
    Strategy,
};

const DT: f64 = 1.0 / 15.0;

fn apt(h: &serde_json::Value, k: &str) -> Aptitude {
    Aptitude::from_str_letter(h[k].as_str().unwrap()).unwrap()
}

fn strat(s: &str) -> Strategy {
    match s {
        "Nige" => Strategy::Nige,
        "Senkou" => Strategy::Senkou,
        "Sasi" => Strategy::Sasi,
        "Oikomi" => Strategy::Oikomi,
        "Oonige" => Strategy::Oonige,
        _ => panic!("{s}"),
    }
}

fn ground(s: &str) -> GroundCondition {
    match s {
        "Good" => GroundCondition::Good,
        "Yielding" => GroundCondition::Yielding,
        "Soft" => GroundCondition::Soft,
        "Heavy" => GroundCondition::Heavy,
        _ => panic!("{s}"),
    }
}

#[test]
fn probe_last_two_procs() {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for id in ["v3_690_10810_modeled_only", "v3_394_10914_modeled_only"] {
        let c = v["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap();
        let h = &c["horse"];
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
            strategy: strat(h["strategy"].as_str().unwrap()),
            distance_apt: apt(h, "distanceAptitude"),
            surface_apt: apt(h, "surfaceAptitude"),
            strategy_apt: apt(h, "strategyAptitude"),
            mood: c["mood"].as_i64().unwrap() as i8,
            skills,
        };
        let course = get_course(c["course_id"].as_u64().unwrap() as u32).unwrap();
        let seed = c["seed_u32"].as_u64().unwrap() as u32;
        let g = ground(c["ground"].as_str().unwrap());
        let pace = c["pace_effects"].as_bool().unwrap();
        let mut focus = HorseRunner::new(course, g, &horse, seed);
        let mut pacer = if pace {
            let ph = default_virtual_pacer_horse(&horse);
            let mut br = PrandoRng::new(seed.wrapping_add(1));
            Some(HorseRunner::new_pacer(course, g, &ph, br.int32()))
        } else {
            None
        };
        let mut prev = focus.debug_used_skills();
        eprintln!("=== {id} ===");
        while !focus.finished() {
            if let Some(ref mut p) = pacer {
                if !p.finished() {
                    p.step(
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
            }
            let pacer_pos = pacer.as_ref().map(|p| p.pos());
            focus.step(
                DT,
                &StepCtx {
                    pacer_pos,
                    second_pos: None,
                    pos_keep_mode: if pace {
                        PosKeepMode::Virtual
                    } else {
                        PosKeepMode::None
                    },
                    am_i_pacer: matches!(horse.strategy, Strategy::Oonige),
                    place: 2,
                    field_size: if pace { 2 } else { 1 },
                },
            );
            let used = focus.debug_used_skills();
            if used != prev {
                for sid in used.iter().filter(|s| !prev.contains(s)) {
                    eprintln!("  PROC {sid} @ {:.1}", focus.pos());
                }
                prev = used;
            }
        }
        eprintln!(
            "  finish={:.6} used={:?}",
            focus.accum_time(),
            focus.debug_used_skills()
        );
        let _ = simulate_solo; // silence if unused
    }
}
