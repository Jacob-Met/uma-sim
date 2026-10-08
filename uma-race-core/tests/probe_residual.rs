//! Bare v3_590: dump pos-keep timer / behind around PD exit→reenter.

use uma_race_core::runner::{default_virtual_pacer_horse, HorseRunner, StepCtx};
use uma_race_core::{
    get_course, Aptitude, GroundCondition, HorseInput, PosKeepMode, PrandoRng, Strategy,
};

const DT: f64 = 1.0 / 15.0;

#[test]
fn probe_590_pk() {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "v3_590_10603_modeled_only")
        .unwrap();
    let horse = &case["horse"];
    let apt = |k: &str| Aptitude::from_str_letter(horse[k].as_str().unwrap()).unwrap();
    let seed = case["seed_u32"].as_u64().unwrap() as u32;
    let c = get_course(case["course_id"].as_u64().unwrap() as u32).unwrap();
    let base = HorseInput {
        speed: horse["speed"].as_f64().unwrap(),
        stamina: horse["stamina"].as_f64().unwrap(),
        power: horse["power"].as_f64().unwrap(),
        guts: horse["guts"].as_f64().unwrap(),
        wisdom: horse["wisdom"].as_f64().unwrap(),
        strategy: Strategy::Senkou,
        distance_apt: apt("distanceAptitude"),
        surface_apt: apt("surfaceAptitude"),
        strategy_apt: apt("strategyAptitude"),
        mood: case["mood"].as_i64().unwrap() as i8,
        skills: vec![],
    };
    let mut focus = HorseRunner::new(c, GroundCondition::Heavy, &base, seed);
    let pacer_h = default_virtual_pacer_horse(&base);
    let mut base_pacer = PrandoRng::new(seed.wrapping_add(1));
    let mut pacer = HorseRunner::new_pacer(c, GroundCondition::Heavy, &pacer_h, base_pacer.int32());
    let (mn, mx) = focus.pk_thresholds();
    eprintln!(
        "min_th={mn:.3} max_th={mx:.3} pk_end={}",
        focus.debug_pk_end()
    );
    for i in 0..300 {
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
        let pk_before = focus.pos_keep_state();
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
        let pk = focus.pos_keep_state();
        let gap = pacer.pos() - focus.pos();
        let interesting = pk != pk_before
            || (i >= 105 && i <= 130)
            || ((i >= 60 && i <= 180 && i % 5 == 0) || i < 5);
        if interesting {
            eprintln!(
                "i={i} t={:.2} pos={:.1} gap={:.2} behind={:.3} pk={:?}->{:?} timer={:.3} spd={:.3} spdSkills={} exitPos={:.1} exitDist={:.2}",
                focus.accum_time(),
                focus.pos(),
                gap,
                focus.debug_pk_behind(),
                pk_before,
                pk,
                focus.debug_pk_timer(),
                focus.current_speed(),
                focus.debug_active_speed_skill_count(),
                focus.debug_exit_pos(),
                focus.debug_exit_dist(),
            );
        }
    }
    eprintln!("FINISH t={:.6} expected bare~92.667", focus.accum_time());
}

#[test]
fn probe_590_pos() {
    let raw = std::fs::read_to_string("../research/race_checkpoint_v3_sample_1000.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "v3_590_10603_modeled_only")
        .unwrap();
    let horse = &case["horse"];
    let apt = |k: &str| Aptitude::from_str_letter(horse[k].as_str().unwrap()).unwrap();
    let seed = case["seed_u32"].as_u64().unwrap() as u32;
    let c = get_course(case["course_id"].as_u64().unwrap() as u32).unwrap();
    let base = HorseInput {
        speed: horse["speed"].as_f64().unwrap(),
        stamina: horse["stamina"].as_f64().unwrap(),
        power: horse["power"].as_f64().unwrap(),
        guts: horse["guts"].as_f64().unwrap(),
        wisdom: horse["wisdom"].as_f64().unwrap(),
        strategy: Strategy::Senkou,
        distance_apt: apt("distanceAptitude"),
        surface_apt: apt("surfaceAptitude"),
        strategy_apt: apt("strategyAptitude"),
        mood: case["mood"].as_i64().unwrap() as i8,
        skills: vec![],
    };
    let mut focus = HorseRunner::new(c, GroundCondition::Heavy, &base, seed);
    let pacer_h = default_virtual_pacer_horse(&base);
    let mut base_pacer = PrandoRng::new(seed.wrapping_add(1));
    let mut pacer = HorseRunner::new_pacer(c, GroundCondition::Heavy, &pacer_h, base_pacer.int32());
    eprintln!("FOCUS startDelay {:.15}", focus.start_delay_val());
    eprintln!("PACER startDelay {:.15}", pacer.start_delay_val());
    for i in 0..=120 {
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
        if i < 5
            || i == 30
            || i == 60
            || i == 68
            || (110..=115).contains(&i)
            || [10, 15, 20, 25, 28, 29, 35].contains(&i)
        {
            let (base, pk, sec, tgt) = focus.debug_target_bits();
            eprintln!(
                "i={i} focus={:.4} pacer={:.4} fSpd={:.4} pSpd={:.4} behind={:.4} tgt={:.4} dash={} pk={}",
                focus.pos(),
                pacer.pos(),
                focus.current_speed(),
                pacer.current_speed(),
                pacer.pos() - focus.pos(),
                tgt,
                focus.in_start_dash(),
                pk
            );
            let _ = (base, sec);
        }
    }
}
