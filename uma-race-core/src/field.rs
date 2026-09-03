//! Multi-horse field helpers and re-exports of position-keep APIs.

pub use crate::pos_keep::{
    course_factor, max_threshold, min_threshold, overtake_exit_gap_threshold, pace_up_wit_chance,
    pos_keep_speed_coef, speed_up_gap_threshold, speed_up_overtake_wit_chance, tick_nige_pos_keep,
    tick_pack_pos_keep, PosKeepMode, PosKeepState,
};

use crate::course::Course;
use crate::hp::{GroundCondition, Strategy};
use crate::runner::{
    entrant_seed, second_place_pos, select_pacer_index, simulate_solo,
    simulate_with_default_pacer as sim_pacer, HorseRunner, StepCtx,
};
use crate::solver::{HorseInput, RaceResult, DT};

#[derive(Clone, Debug)]
pub struct Finisher {
    pub index: usize,
    pub finish_time: f64,
    pub frames: u32,
    pub final_pos: f64,
    pub hp_ratio: f64,
}

#[derive(Clone, Debug)]
pub struct FieldResult {
    pub finishers: Vec<Finisher>,
}

/// Multi-horse race with `PosKeepMode::None`: independent solos, sorted by finish time.
pub fn simulate_field_independent(
    course: &Course,
    ground: GroundCondition,
    horses: &[HorseInput],
    seed: u32,
) -> FieldResult {
    let mut finishers: Vec<Finisher> = horses
        .iter()
        .enumerate()
        .map(|(index, h)| {
            let r: RaceResult = simulate_solo(course, ground, h, entrant_seed(seed, index));
            Finisher {
                index,
                finish_time: r.finish_time,
                frames: r.frames,
                final_pos: r.final_pos,
                hp_ratio: r.hp_ratio,
            }
        })
        .collect();
    finishers.sort_by(|a, b| {
        a.finish_time
            .partial_cmp(&b.finish_time)
            .unwrap()
            .then_with(|| a.index.cmp(&b.index))
    });
    FieldResult { finishers }
}

/// Focus horse + default Nige pacer under the given position-keep mode.
pub fn simulate_with_default_pacer(
    course: &Course,
    ground: GroundCondition,
    horse: &HorseInput,
    seed: u32,
    mode: PosKeepMode,
) -> RaceResult {
    sim_pacer(course, ground, horse, seed, mode)
}

/// Frame-synced N-horse race with optional Virtual position-keep.
///
/// **Entrant seeds:** [`entrant_seed`] — index 0 uses `seed`; later indices burn `index`
/// × `int32` from `PrandoRng::new(seed)` then take the next `int32`.
///
/// **Step order** (oracle `pacer.step` then focus, generalized):
/// 1. Snapshot → [`select_pacer_index`] (furthest Oonige → Nige → any).
/// 2. Step pacemaker first (if unfinished).
/// 3. Step remaining unfinished horses by descending start-of-frame position
///    (stable by index on ties).
///
/// Pack horses get PaceUp/PaceDown; Nige/Oonige get SpeedUp/Overtake via
/// [`tick_nige_pos_keep`]. Lucky-pace strategy mutation is not modeled.
pub fn simulate_field_synced(
    course: &Course,
    ground: GroundCondition,
    horses: &[HorseInput],
    seed: u32,
    mode: PosKeepMode,
) -> FieldResult {
    if horses.is_empty() {
        return FieldResult {
            finishers: Vec::new(),
        };
    }

    let n = horses.len() as u32;
    let mut runners: Vec<HorseRunner> = horses
        .iter()
        .enumerate()
        .map(|(i, h)| HorseRunner::new_in_field(course, ground, h, entrant_seed(seed, i), n))
        .collect();

    let course_dist = course.distance;
    let mut finish_times = vec![None::<f64>; runners.len()];
    let max_frames = (15.0 * 600.0) as u32;
    let mut frames = 0u32;

    while finish_times.iter().any(|t| t.is_none()) && frames < max_frames {
        frames += 1;

        let pacer_idx = select_pacer_index(&runners);
        let second_for_pacer = second_place_pos(&runners, Some(pacer_idx));

        let mut order: Vec<usize> = (0..runners.len()).collect();
        let snap_pos: Vec<f64> = runners.iter().map(|r| r.pos()).collect();
        order.sort_by(|&a, &b| {
            let a_first = a == pacer_idx;
            let b_first = b == pacer_idx;
            match (a_first, b_first) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => snap_pos[b]
                    .partial_cmp(&snap_pos[a])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.cmp(&b)),
            }
        });

        for &i in &order {
            if finish_times[i].is_some() {
                continue;
            }
            if runners[i].finished() {
                finish_times[i] = Some(runners[i].t());
                continue;
            }

            if matches!(mode, PosKeepMode::Virtual | PosKeepMode::Approximate) {
                let pos_i = runners[i].pos();
                let mut nearest: Option<(f64, Strategy)> = None;
                let mut nearest_compete: Option<(f64, f64)> = None;
                for (j, other) in runners.iter().enumerate() {
                    if j == i {
                        continue;
                    }
                    let gap = (pos_i - other.pos()).abs();
                    let better = nearest.map(|(g, _)| gap < g).unwrap_or(true);
                    if better {
                        nearest = Some((other.pos(), other.strategy()));
                        nearest_compete = Some((other.pos(), other.speed()));
                    }
                }
                if let Some((npos, nstrat)) = nearest {
                    runners[i].update_lead_competition(npos, nstrat);
                }
                if let Some((npos, nspd)) = nearest_compete {
                    // Dueling needs a real pack; skip 1–2 horse Virtual+pacer regressions.
                    if runners.len() >= 3 {
                        let field_size = runners.len();
                        let place = snap_pos
                            .iter()
                            .enumerate()
                            .filter(|&(j, _)| j != i)
                            .filter(|&(_, &p)| p > pos_i)
                            .count();
                        runners[i].update_compete_fight(npos, nspd, place, field_size);
                    }
                }
            }

            let pacer_pos = runners[pacer_idx].pos();
            let second_pos = if i == pacer_idx {
                second_for_pacer
            } else {
                second_place_pos(&runners, Some(i))
            };

            let place = snap_pos
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .filter(|&(_, &p)| p > snap_pos[i])
                .count()
                + 1;
            let ctx = StepCtx {
                pacer_pos: if matches!(mode, PosKeepMode::None) {
                    None
                } else {
                    Some(pacer_pos)
                },
                second_pos,
                am_i_pacer: i == pacer_idx,
                pos_keep_mode: mode,
                place,
                field_size: runners.len(),
            };
            runners[i].step(DT, &ctx);
            if runners[i].pos() >= course_dist {
                finish_times[i] = Some(runners[i].t());
            }
        }
    }

    let mut finishers: Vec<Finisher> = runners
        .iter()
        .enumerate()
        .map(|(index, r)| {
            let res = r.result();
            Finisher {
                index,
                finish_time: finish_times[index].unwrap_or(res.finish_time),
                frames: r.frames(),
                final_pos: r.pos(),
                hp_ratio: res.hp_ratio,
            }
        })
        .collect();
    finishers.sort_by(|a, b| {
        a.finish_time
            .partial_cmp(&b.finish_time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.index.cmp(&b.index))
    });
    FieldResult { finishers }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::get_course;
    use crate::hp::Strategy;
    use crate::rng::PrandoRng;
    use crate::solver::{Aptitude, HorseInput};

    fn horse(
        speed: f64,
        stamina: f64,
        strategy: Strategy,
        wisdom: f64,
    ) -> HorseInput {
        HorseInput {
            speed,
            stamina,
            power: speed - 50.0,
            guts: speed - 100.0,
            wisdom,
            strategy,
            distance_apt: Aptitude::A,
            surface_apt: Aptitude::A,
            strategy_apt: Aptitude::A,
            mood: 2,
            skills: vec![],
        }
    }

    #[test]
    fn independent_field_orders_faster_horse_first() {
        let c = get_course(10601).unwrap();
        let slow = horse(800.0, 800.0, Strategy::Oikomi, 800.0);
        let mut fast = slow.clone();
        fast.speed = 1200.0;
        fast.strategy = Strategy::Senkou;
        let r = simulate_field_independent(c, GroundCondition::Good, &[slow, fast], 2615953739);
        assert_eq!(r.finishers[0].index, 1, "faster senkou should finish first");
        assert!(r.finishers[0].finish_time < r.finishers[1].finish_time);
    }

    #[test]
    fn synced_none_matches_independent_finish_order() {
        let c = get_course(10601).unwrap();
        let field = [
            horse(1100.0, 1000.0, Strategy::Senkou, 900.0),
            horse(1000.0, 1100.0, Strategy::Nige, 850.0),
            horse(950.0, 1050.0, Strategy::Oikomi, 1000.0),
        ];
        let seed = 2615953739u32;
        let indep = simulate_field_independent(c, GroundCondition::Good, &field, seed);
        let synced =
            simulate_field_synced(c, GroundCondition::Good, &field, seed, PosKeepMode::None);
        let oi: Vec<_> = indep.finishers.iter().map(|f| f.index).collect();
        let os: Vec<_> = synced.finishers.iter().map(|f| f.index).collect();
        assert_eq!(oi, os, "None synced should match independent order");
        for (a, b) in indep.finishers.iter().zip(synced.finishers.iter()) {
            assert!(
                (a.finish_time - b.finish_time).abs() < 1e-9,
                "index {} time {} vs {}",
                a.index,
                a.finish_time,
                b.finish_time
            );
        }
    }

    #[test]
    fn synced_virtual_order_deterministic() {
        let c = get_course(10601).unwrap();
        let field = [
            horse(1050.0, 1000.0, Strategy::Senkou, 900.0),
            horse(1000.0, 1100.0, Strategy::Nige, 800.0),
            horse(1150.0, 950.0, Strategy::Oikomi, 1000.0),
        ];
        let seed = 2615953739u32;
        let a = simulate_field_synced(c, GroundCondition::Good, &field, seed, PosKeepMode::Virtual);
        let b = simulate_field_synced(c, GroundCondition::Good, &field, seed, PosKeepMode::Virtual);
        let oa: Vec<_> = a.finishers.iter().map(|f| f.index).collect();
        let ob: Vec<_> = b.finishers.iter().map(|f| f.index).collect();
        assert_eq!(oa, ob);
        eprintln!(
            "virtual order={:?} times={:?}",
            oa,
            a.finishers
                .iter()
                .map(|f| format!("{}:{:.3}", f.index, f.finish_time))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn synced_virtual_can_change_order_vs_none() {
        let c = get_course(10601).unwrap();
        // Close field: Nige paces; Senkou/Oikomi pack keep should perturb times/order.
        let field = [
            horse(1080.0, 1000.0, Strategy::Senkou, 950.0),
            horse(1070.0, 1050.0, Strategy::Nige, 700.0),
            horse(1090.0, 980.0, Strategy::Oikomi, 900.0),
        ];
        let seed = 42u32;
        let none = simulate_field_synced(c, GroundCondition::Good, &field, seed, PosKeepMode::None);
        let virt =
            simulate_field_synced(c, GroundCondition::Good, &field, seed, PosKeepMode::Virtual);
        let on: Vec<_> = none.finishers.iter().map(|f| f.index).collect();
        let ov: Vec<_> = virt.finishers.iter().map(|f| f.index).collect();
        let mut max_dt = 0.0_f64;
        for nf in &none.finishers {
            if let Some(vf) = virt.finishers.iter().find(|vf| vf.index == nf.index) {
                let d = (vf.finish_time - nf.finish_time).abs();
                max_dt = max_dt.max(d);
                eprintln!(
                    "idx {} none={:.6} virt={:.6} Δ={:.6}",
                    nf.index, nf.finish_time, vf.finish_time, d
                );
            }
        }
        eprintln!("none order={on:?} virt order={ov:?} max_Δ={max_dt:.6}");
        assert!(
            on != ov || max_dt > DT,
            "Virtual should couple the field (order change or >1 frame time Δ); none={on:?} virt={ov:?} max_Δ={max_dt}"
        );
    }

    #[test]
    fn entrant_seed_index_zero_is_root() {
        assert_eq!(entrant_seed(2615953739, 0), 2615953739);
        let s1 = entrant_seed(2615953739, 1);
        assert_ne!(s1, 2615953739);
        let mut root = PrandoRng::new(2615953739);
        let _ = root.int32();
        assert_eq!(s1, root.int32());
    }
}
