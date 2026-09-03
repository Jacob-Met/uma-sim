//! Reduce parsed condition trees to course regions + sample policy.
//! Keyword semantics from GameTora skill_condition reference + KuromiAK.

use crate::condition::parser::{parse_condition, Atom, Expr, Op};
use crate::condition::region::{Region, RegionList};
use crate::condition::sample::SamplePolicy;
use crate::course::Course;
use crate::hp::{Aptitude, Strategy};
use crate::physics::{phase_end, phase_start, Phase};

#[derive(Clone, Debug)]
pub struct ReducedCondition {
    pub regions: RegionList,
    pub policy: SamplePolicy,
    /// Dynamic predicates checked at trigger time (order, is_lastspurt, etc.).
    pub dynamics: Vec<DynamicPred>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicPred {
    Always,
    /// Enforced only when `field_size ≥ 3` (umalator orderRange unset → no-op).
    Order {
        op: Op,
        value: i64,
    },
    OrderRate {
        op: Op,
        value: i64,
    },
    IsLastSpurt {
        eq: bool,
    },
    /// `lastspurt==1|2|3` (umalator): 1=spurt with planned transition, 2=spurt
    /// without transition, 3=not spurting.
    LastSpurtCase {
        case: i64,
    },
    AccumulateTime {
        op: Op,
        value: i64,
    },
    /// Heal skill activations so far (`activate_count_heal`).
    ActivateCountHeal {
        op: Op,
        value: i64,
    },
    /// Phase-bucketed activations: 0=start, 1=middle, 2=end_after.
    ActivateCountPhase {
        phase: u8,
        op: Op,
        value: i64,
    },
    /// Sum of all phase activation counts.
    ActivateCountAll {
        op: Op,
        value: i64,
    },
    /// Second trigger of dual-effect skills (`is_activate_other_skill_detail==1`).
    IsActivateOtherSkillDetail {
        eq: bool,
    },
    /// `is_badstart==0|1`: initial start delay ≷ 0.08s (umalator).
    IsBadStart {
        want_bad: bool,
    },
    /// `random_lot==N`: once-per-race roll `random_lot < N`.
    RandomLot {
        max_exclusive: i64,
    },
    /// `hp_per` ratio gate (0–100 scale in conditions → fraction).
    HpPer {
        op: Op,
        value: i64,
    },
    /// `post_number` vs umalator `gateBlock(gateRoll, numUmas)` (default numUmas=9).
    PostNumber {
        op: Op,
        value: i64,
    },
    /// `is_used_skill_id==N`: true once skill N has activated this race.
    UsedSkillId {
        skill_id: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct HorseCtx {
    pub strategy: Strategy,
    pub distance_apt: Aptitude,
    pub surface_apt: Aptitude,
    pub ground: crate::hp::GroundCondition,
    /// Mood on −2..=+2; `motivation` condition uses mood+3 (1..=5).
    pub mood: i8,
    /// Pre-green stats (mood applied for `base_*` filters, matching umalator).
    pub speed: f64,
    pub stamina: f64,
    pub power: f64,
    pub guts: f64,
    pub wisdom: f64,
    /// Race-env defaults match `RaceSolverBuilder` (Sunny/Spring/Midday/G1).
    pub weather: i64,
    pub season: i64,
    pub time: i64,
    pub grade: i64,
}

impl HorseCtx {
    /// Umalator builder defaults when checkpoint/career do not set race env.
    pub fn with_default_race_env(mut self) -> Self {
        self.weather = 1; // Sunny
        self.season = 1; // Spring
        self.time = 2; // Midday
        self.grade = 100; // G1
        self
    }
}

fn displayed_stat(stat: f64, mood: i8) -> f64 {
    stat * (1.0 + 0.02 * f64::from(mood))
}

fn cmp_i64(op: &Op, lhs: i64, rhs: i64) -> bool {
    match op {
        Op::Eq => lhs == rhs,
        Op::Ne => lhs != rhs,
        Op::Ge => lhs >= rhs,
        Op::Gt => lhs > rhs,
        Op::Le => lhs <= rhs,
        Op::Lt => lhs < rhs,
    }
}

fn value_filter_regions(
    regions: &RegionList,
    op: &Op,
    lhs: i64,
    rhs: i64,
) -> (RegionList, SamplePolicy, Vec<DynamicPred>) {
    if cmp_i64(op, lhs, rhs) {
        (regions.clone(), SamplePolicy::Immediate, vec![])
    } else {
        (RegionList::default(), SamplePolicy::Immediate, vec![])
    }
}

fn phase_from_i(v: i64) -> Option<Phase> {
    match v {
        0 => Some(Phase::Opening),
        1 => Some(Phase::Middle),
        2 => Some(Phase::End),
        3 => Some(Phase::LastSpurt),
        _ => None,
    }
}

fn slope_regions(course: &Course, slope_type: i64) -> Result<Vec<Region>, String> {
    if !(0..=2).contains(&slope_type) {
        return Err(format!("bad slope type {slope_type}"));
    }
    // Boundary slopes: uphills for type≠2, downhills for type≠1 (type 0 uses both).
    let slopes: Vec<&crate::course::Slope> = course
        .slopes
        .iter()
        .filter(|s| (slope_type != 2 && s.slope > 0.0) || (slope_type != 1 && s.slope < 0.0))
        .collect();
    if slope_type == 0 {
        let mut last_end = 0.0;
        let mut out = Vec::new();
        for s in &slopes {
            out.push(Region::new(last_end, s.start));
            last_end = s.start + s.length;
        }
        if (last_end - course.distance).abs() > 1e-9 {
            out.push(Region::new(last_end, course.distance));
        }
        Ok(out)
    } else {
        Ok(slopes
            .iter()
            .map(|s| Region::new(s.start, s.start + s.length))
            .collect())
    }
}

fn distance_type(course: &Course) -> i64 {
    course.distance_type as i64
}

fn ground_type(course: &Course) -> i64 {
    course.surface as i64
}

fn running_style(strategy: Strategy) -> i64 {
    strategy as i64
}

/// `running_style==Nige` also matches Oonige (and vice versa).
fn running_style_matches(horse: Strategy, required: i64) -> bool {
    let h = running_style(horse);
    h == required
        || (h == Strategy::Nige as i64 && required == Strategy::Oonige as i64)
        || (h == Strategy::Oonige as i64 && required == Strategy::Nige as i64)
}

fn filter_atom(
    regions: &RegionList,
    atom: &Atom,
    course: &Course,
    horse: HorseCtx,
) -> Result<(RegionList, SamplePolicy, Vec<DynamicPred>), String> {
    let d = course.distance;
    match atom.name.as_str() {
        "always" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::Always],
        )),
        "phase" => {
            let ph = phase_from_i(atom.value).ok_or_else(|| format!("bad phase {}", atom.value))?;
            let bounds = match atom.op {
                Op::Eq => Region::new(phase_start(d, ph), phase_end(d, ph)),
                Op::Ge => Region::new(phase_start(d, ph), d),
                Op::Gt => Region::new(phase_end(d, ph), d),
                Op::Le => Region::new(0.0, phase_end(d, ph)),
                Op::Lt => Region::new(0.0, phase_start(d, ph)),
                Op::Ne => {
                    // whole course minus phase — approximate as two sides
                    let mut out = RegionList::default();
                    out.push(Region::new(0.0, phase_start(d, ph)).intersect(Region::new(0.0, d)));
                    out.push(Region::new(phase_end(d, ph), d));
                    return Ok((out, SamplePolicy::Immediate, vec![]));
                }
            };
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "phase_random" => {
            let ph = phase_from_i(atom.value)
                .ok_or_else(|| format!("bad phase_random {}", atom.value))?;
            if !matches!(atom.op, Op::Eq) {
                return Err("phase_random only supports ==".into());
            }
            let bounds = Region::new(phase_start(d, ph), phase_end(d, ph));
            Ok((regions.map_intersect(bounds), SamplePolicy::Random, vec![]))
        }
        "phase_firsthalf"
        | "phase_firsthalf_random"
        | "phase_firstquarter"
        | "phase_firstquarter_random" => {
            if !matches!(atom.op, Op::Eq) {
                return Err(format!("{} only supports ==", atom.name));
            }
            let ph = phase_from_i(atom.value).ok_or("bad phase")?;
            let start = phase_start(d, ph);
            let end = phase_end(d, ph);
            let frac = if atom.name.contains("quarter") {
                0.25
            } else {
                0.5
            };
            let bounds = Region::new(start, start + (end - start) * frac);
            let pol = if atom.name.contains("random") {
                SamplePolicy::Random
            } else {
                SamplePolicy::Immediate
            };
            Ok((regions.map_intersect(bounds), pol, vec![]))
        }
        "phase_laterhalf" | "phase_laterhalf_random" => {
            if !matches!(atom.op, Op::Eq) {
                return Err(format!("{} only supports ==", atom.name));
            }
            let ph = phase_from_i(atom.value).ok_or("bad phase")?;
            let start = phase_start(d, ph);
            let end = phase_end(d, ph);
            let bounds = Region::new((start + end) / 2.0, end);
            let pol = if atom.name.contains("random") {
                SamplePolicy::Random
            } else {
                SamplePolicy::Immediate
            };
            Ok((regions.map_intersect(bounds), pol, vec![]))
        }
        // Random trigger at/after a distance-rate threshold (e.g. ==50 → second half).
        "distance_rate_after_random" => {
            if !matches!(atom.op, Op::Eq | Op::Ge) {
                return Err("distance_rate_after_random only supports ==/>=".into());
            }
            let start = d * atom.value as f64 / 100.0;
            let bounds = Region::new(start.min(d), d);
            Ok((regions.map_intersect(bounds), SamplePolicy::Random, vec![]))
        }
        "distance_type" => {
            let ok = match atom.op {
                Op::Eq => distance_type(course) == atom.value,
                Op::Ne => distance_type(course) != atom.value,
                _ => return Err("distance_type: unsupported op".into()),
            };
            Ok((
                if ok {
                    regions.clone()
                } else {
                    RegionList::default()
                },
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "ground_type" => {
            let ok = match atom.op {
                Op::Eq => ground_type(course) == atom.value,
                Op::Ne => ground_type(course) != atom.value,
                _ => return Err("ground_type: unsupported op".into()),
            };
            Ok((
                if ok {
                    regions.clone()
                } else {
                    RegionList::default()
                },
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "running_style" => {
            let ok = match atom.op {
                Op::Eq => running_style_matches(horse.strategy, atom.value),
                Op::Ne => !running_style_matches(horse.strategy, atom.value),
                _ => return Err("running_style: unsupported op".into()),
            };
            Ok((
                if ok {
                    regions.clone()
                } else {
                    RegionList::default()
                },
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "course_distance" => {
            let cd = d as i64;
            let ok = match atom.op {
                Op::Eq => cd == atom.value,
                Op::Ne => cd != atom.value,
                Op::Ge => cd >= atom.value,
                Op::Gt => cd > atom.value,
                Op::Le => cd <= atom.value,
                Op::Lt => cd < atom.value,
            };
            Ok((
                if ok {
                    regions.clone()
                } else {
                    RegionList::default()
                },
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        // GameTora: core distance iff course length is divisible by 400.
        "is_basis_distance" => {
            let is_core = if (d as i64) % 400 == 0 { 1 } else { 0 };
            let ok = match atom.op {
                Op::Eq => is_core == atom.value,
                Op::Ne => is_core != atom.value,
                Op::Ge => is_core >= atom.value,
                Op::Gt => is_core > atom.value,
                Op::Le => is_core <= atom.value,
                Op::Lt => is_core < atom.value,
            };
            Ok((
                if ok {
                    regions.clone()
                } else {
                    RegionList::default()
                },
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "distance_rate" => {
            // Restrict to positions where distance_rate (pos/distance*100) matches.
            let bounds = match atom.op {
                Op::Ge => Region::new(d * atom.value as f64 / 100.0, d),
                Op::Gt => Region::new(d * (atom.value as f64 + 1e-9) / 100.0, d),
                Op::Le => Region::new(0.0, d * atom.value as f64 / 100.0),
                Op::Lt => Region::new(0.0, d * atom.value as f64 / 100.0),
                Op::Eq => {
                    let p = d * atom.value as f64 / 100.0;
                    Region::new(p, (p + 10.0).min(d))
                }
                Op::Ne => Region::new(0.0, d), // weak
            };
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "remain_distance" => {
            // remain = distance - pos
            let bounds = match atom.op {
                Op::Ge => Region::new(0.0, d - atom.value as f64),
                Op::Le => Region::new((d - atom.value as f64).max(0.0), d),
                Op::Gt => Region::new(0.0, d - atom.value as f64),
                Op::Lt => Region::new((d - atom.value as f64).max(0.0), d),
                Op::Eq => {
                    let p = d - atom.value as f64;
                    Region::new(p, (p + 10.0).min(d))
                }
                Op::Ne => Region::new(0.0, d),
            };
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "accumulatetime" => {
            // Static clip: ~0.85 * baseSpeed * t so Immediate triggers aren't stuck
            // before the time gate can ever pass.
            let mut clipped = regions.clone();
            if matches!(atom.op, Op::Ge | Op::Gt) {
                let base_speed = 20.0 - (d - 2000.0) / 1000.0;
                let t = atom.value as f64;
                let min_pos = 0.85 * base_speed * t;
                clipped = clipped.map_intersect(Region::new(min_pos, d));
            }
            Ok((
                clipped,
                SamplePolicy::Immediate,
                vec![DynamicPred::AccumulateTime {
                    op: atom.op.clone(),
                    value: atom.value,
                }],
            ))
        }
        "is_lastspurt" => {
            // umalator: clip to phase≥2 bounds + dynamic `s.isLastSpurt`.
            if !matches!(atom.op, Op::Eq) || !(0..=1).contains(&atom.value) {
                return Err("is_lastspurt only supports ==0|1".into());
            }
            let want = atom.value != 0;
            let clipped = if want {
                regions.map_intersect(Region::new(phase_start(d, Phase::End), d))
            } else {
                // is_lastspurt==0: before final third (weak approx; rare).
                regions.map_intersect(Region::new(0.0, phase_start(d, Phase::End)))
            };
            Ok((
                clipped,
                SamplePolicy::Immediate,
                vec![DynamicPred::IsLastSpurt { eq: want }],
            ))
        }
        "lastspurt" => {
            // Cases 1–3; restrict to final third like umalator.
            if !matches!(atom.op, Op::Eq) || !(1..=3).contains(&atom.value) {
                return Err("lastspurt only supports ==1|2|3".into());
            }
            let bounds = Region::new(phase_start(d, Phase::End), d);
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Immediate,
                vec![DynamicPred::LastSpurtCase { case: atom.value }],
            ))
        }
        "rotation" => {
            let turn = course.turn as i64;
            let ok = match atom.op {
                Op::Eq => turn == atom.value,
                Op::Ne => turn != atom.value,
                Op::Ge => turn >= atom.value,
                Op::Gt => turn > atom.value,
                Op::Le => turn <= atom.value,
                Op::Lt => turn < atom.value,
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "motivation" => {
            // Mood −2..=+2 → motivation 1..=5.
            let mot = (horse.mood as i64) + 3;
            let ok = match atom.op {
                Op::Eq => mot == atom.value,
                Op::Ne => mot != atom.value,
                Op::Ge => mot >= atom.value,
                Op::Gt => mot > atom.value,
                Op::Le => mot <= atom.value,
                Op::Lt => mot < atom.value,
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "straight_front_type" => {
            if !matches!(atom.op, Op::Eq) || !(1..=2).contains(&atom.value) {
                return Err("straight_front_type only supports ==1|2".into());
            }
            let pieces: Vec<Region> = course
                .straights
                .iter()
                .filter(|s| s.front_type as i64 == atom.value)
                .map(|s| Region::new(s.start, s.end))
                .collect();
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        "is_dirtgrade" => {
            // JP dirt tracks that share the "dirt grade" flag set.
            const DIRT_GRADE: &[u32] = &[10101, 10103, 10104, 10105];
            let is = DIRT_GRADE.contains(&course.race_track_id);
            let ok = match atom.op {
                Op::Eq if atom.value == 1 => is,
                Op::Ne if atom.value == 1 => !is,
                Op::Eq if atom.value == 0 => !is,
                Op::Ne if atom.value == 0 => is,
                _ => return Err("is_dirtgrade only supports ==1 / !=1".into()),
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "order" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::Order {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "order_rate" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::OrderRate {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        // Corner / straight geometry
        "corner" => {
            // GameTora: 0 = not in a corner; 1..4 = oval corner number (may match
            // multiple geometry segments when the course loops). Formula matches
            // course.corners.length + n - 5, stepping by −4.
            let corners = &course.corners;
            match atom.op {
                Op::Eq if atom.value == 0 => {
                    let mut pieces = Vec::new();
                    let mut last = 0.0;
                    for c in corners {
                        if c.start > last {
                            pieces.push(Region::new(last, c.start));
                        }
                        last = c.start + c.length;
                    }
                    if last < d {
                        pieces.push(Region::new(last, d));
                    }
                    Ok((
                        regions.map_intersect_all(&pieces),
                        SamplePolicy::Immediate,
                        vec![],
                    ))
                }
                Op::Eq => {
                    let n = atom.value;
                    if !(1..=4).contains(&n) {
                        return Ok((RegionList::default(), SamplePolicy::Immediate, vec![]));
                    }
                    if (corners.len() as i64) + n < 5 {
                        return Ok((RegionList::default(), SamplePolicy::Immediate, vec![]));
                    }
                    let mut pieces = Vec::new();
                    let mut idx = corners.len() as i64 + n - 5;
                    while idx >= 0 {
                        let c = &corners[idx as usize];
                        pieces.push(Region::new(c.start, c.start + c.length));
                        idx -= 4;
                    }
                    pieces.reverse();
                    Ok((
                        regions.map_intersect_all(&pieces),
                        SamplePolicy::Immediate,
                        vec![],
                    ))
                }
                Op::Ne if atom.value == 0 => {
                    let pieces: Vec<Region> = corners
                        .iter()
                        .map(|c| Region::new(c.start, c.start + c.length))
                        .collect();
                    Ok((
                        regions.map_intersect_all(&pieces),
                        SamplePolicy::Immediate,
                        vec![],
                    ))
                }
                Op::Ne => Ok((regions.clone(), SamplePolicy::Immediate, vec![])),
                _ => Err("corner: unsupported op".into()),
            }
        }
        "straight_random" => {
            if !matches!(atom.op, Op::Eq) || atom.value != 1 {
                return Err("straight_random only supports ==1".into());
            }
            let pieces: Vec<Region> = course
                .straights
                .iter()
                .map(|s| Region::new(s.start, s.end))
                .collect();
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::StraightRandom,
                vec![],
            ))
        }
        "phase_straight_random" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("phase_straight_random only supports ==".into());
            }
            let ph = phase_from_i(atom.value).ok_or_else(|| format!("bad phase {}", atom.value))?;
            let phase_bounds = Region::new(phase_start(d, ph), phase_end(d, ph));
            let pieces: Vec<Region> = course
                .straights
                .iter()
                .map(|s| Region::new(s.start, s.end).intersect(phase_bounds))
                .filter(|r| !r.is_empty())
                .collect();
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::StraightRandom,
                vec![],
            ))
        }
        "compete_fight_count" => {
            if let Some(s) = course.straights.last() {
                Ok((
                    regions.map_intersect(Region::new(s.start, s.end)),
                    SamplePolicy::DistUniform,
                    vec![DynamicPred::Always],
                ))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "all_corner_random" => {
            if !matches!(atom.op, Op::Eq) || atom.value != 1 {
                return Err("all_corner_random only supports ==1".into());
            }
            let pieces: Vec<Region> = course
                .corners
                .iter()
                .map(|c| Region::new(c.start, c.start + c.length))
                .collect();
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::AllCornerRandom,
                vec![],
            ))
        }
        "phase_corner_random" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("phase_corner_random only supports ==".into());
            }
            let ph = phase_from_i(atom.value).ok_or_else(|| format!("bad phase {}", atom.value))?;
            let phase_bounds = Region::new(phase_start(d, ph), phase_end(d, ph));
            let pieces: Vec<Region> = course
                .corners
                .iter()
                .map(|c| Region::new(c.start, c.start + c.length).intersect(phase_bounds))
                .filter(|r| !r.is_empty())
                .collect();
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::Random,
                vec![],
            ))
        }
        "is_finalcorner"
        | "is_last_straight"
        | "is_finalcorner_random"
        | "is_finalcorner_laterhalf" => {
            if atom.name == "is_finalcorner"
                || atom.name == "is_finalcorner_random"
                || atom.name == "is_finalcorner_laterhalf"
            {
                if !matches!(atom.op, Op::Eq) || !(0..=1).contains(&atom.value) {
                    return Err(format!("{} only supports ==0|1", atom.name));
                }
                // umalator: empty corners → no final-corner region (e.g. course 10301).
                let Some(c) = course.corners.last() else {
                    return Ok((RegionList::default(), SamplePolicy::Immediate, vec![]));
                };
                let (start, end) = if atom.name == "is_finalcorner_laterhalf" {
                    if atom.value != 1 {
                        return Ok((RegionList::default(), SamplePolicy::Immediate, vec![]));
                    }
                    let mid = (c.start + c.start + c.length) / 2.0;
                    (mid, c.start + c.length)
                } else if atom.name == "is_finalcorner" {
                    // umalator: flag 1 → [finalCornerStart, distance]; flag 0 → [0, finalCornerStart]
                    if atom.value == 1 {
                        (c.start, d)
                    } else {
                        (0.0, c.start)
                    }
                } else {
                    // is_finalcorner_random: sample within the final corner arc
                    if atom.value != 1 {
                        return Ok((RegionList::default(), SamplePolicy::Immediate, vec![]));
                    }
                    (c.start, c.start + c.length)
                };
                let bounds = Region::new(start, end);
                let pol = if atom.name.contains("random") {
                    SamplePolicy::Random
                } else {
                    SamplePolicy::Immediate
                };
                return Ok((regions.map_intersect(bounds), pol, vec![]));
            }
            if atom.name == "is_last_straight" {
                if let Some(s) = course.straights.last() {
                    let bounds = Region::new(s.start, s.end);
                    return Ok((
                        regions.map_intersect(bounds),
                        SamplePolicy::Immediate,
                        vec![],
                    ));
                }
            }
            Ok((
                regions.clone(),
                SamplePolicy::Immediate,
                vec![DynamicPred::Always],
            ))
        }
        "is_last_straight_onetime" => {
            // 10m trigger window at the start of the final straight (umalator).
            if !matches!(atom.op, Op::Eq) || atom.value != 1 {
                return Err("is_last_straight_onetime only supports ==1".into());
            }
            if let Some(s) = course.straights.last() {
                let bounds = Region::new(s.start, s.start + 10.0);
                Ok((
                    regions.map_intersect(bounds),
                    SamplePolicy::Immediate,
                    vec![],
                ))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        // Other-uma / order-change conditions: Erlang-timed activation (umalator noopErlang).
        "change_order_onetime"
        | "overtake_target_time"
        | "overtake_target_no_order_up_time"
        | "blocked_side"
        | "blocked_front"
        | "blocked_front_continuetime"
        | "blocked_side_continuetime"
        | "bashin_diff_behind"
        | "bashin_diff_infront"
        | "behind_near_lane_time"
        | "infront_near_lane_time"
        | "is_surrounded" => Ok((
            regions.clone(),
            SamplePolicy::Erlang { k: 3, lambda: 2 },
            vec![DynamicPred::Always],
        )),
        "is_overtake" => Ok((
            regions.clone(),
            SamplePolicy::Erlang { k: 1, lambda: 2 },
            vec![DynamicPred::Always],
        )),
        "near_count" => Ok((
            regions.clone(),
            SamplePolicy::Erlang { k: 2, lambda: 2 },
            vec![DynamicPred::Always],
        )),
        "is_move_lane" => Ok((
            regions.clone(),
            SamplePolicy::Erlang { k: 5, lambda: 1 },
            vec![DynamicPred::Always],
        )),
        // change_order_up_* : Erlang within phase/corner bounds (umalator erlangRandom filters).
        "change_order_up_end_after" => {
            let bounds = Region::new(phase_start(d, Phase::End), d);
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Erlang { k: 3, lambda: 2 },
                vec![DynamicPred::Always],
            ))
        }
        "change_order_up_finalcorner_after" => {
            if let Some(c) = course.corners.last() {
                let bounds = Region::new(c.start, d);
                Ok((
                    regions.map_intersect(bounds),
                    SamplePolicy::Erlang { k: 3, lambda: 2 },
                    vec![DynamicPred::Always],
                ))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "change_order_up_middle" => {
            let bounds = Region::new(phase_start(d, Phase::Middle), phase_end(d, Phase::Middle));
            Ok((
                regions.map_intersect(bounds),
                SamplePolicy::Erlang { k: 3, lambda: 2 },
                vec![DynamicPred::Always],
            ))
        }
        "activate_count_heal" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::ActivateCountHeal {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "activate_count_start" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::ActivateCountPhase {
                phase: 0,
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "activate_count_middle" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::ActivateCountPhase {
                phase: 1,
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "activate_count_end_after" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::ActivateCountPhase {
                phase: 2,
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "activate_count_all" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::ActivateCountAll {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "down_slope_random" => {
            if !matches!(atom.op, Op::Eq) || atom.value != 1 {
                return Err("down_slope_random only supports ==1".into());
            }
            let downs: Vec<Region> = course
                .slopes
                .iter()
                .filter(|s| s.slope < 0.0)
                .map(|s| Region::new(s.start, s.start + s.length))
                .collect();
            Ok((
                regions.map_intersect_all(&downs),
                SamplePolicy::Random,
                vec![],
            ))
        }
        "up_slope_random" => {
            if !matches!(atom.op, Op::Eq) || atom.value != 1 {
                return Err("up_slope_random only supports ==1".into());
            }
            let ups: Vec<Region> = course
                .slopes
                .iter()
                .filter(|s| s.slope > 0.0)
                .map(|s| Region::new(s.start, s.start + s.length))
                .collect();
            Ok((
                regions.map_intersect_all(&ups),
                SamplePolicy::Random,
                vec![],
            ))
        }
        "is_activate_other_skill_detail" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("is_activate_other_skill_detail only supports ==".into());
            }
            Ok((
                regions.clone(),
                SamplePolicy::Immediate,
                vec![DynamicPred::IsActivateOtherSkillDetail {
                    eq: atom.value != 0,
                }],
            ))
        }
        "corner_random" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("corner_random only supports ==".into());
            }
            // Oval corner index: corners.len + cornerNum - 5 (umalator ActivationConditions).
            let n = atom.value;
            let corners = &course.corners;
            if corners.len() as i64 + n >= 5 {
                let idx = (corners.len() as i64 + n - 5) as usize;
                if idx < corners.len() {
                    let c = &corners[idx];
                    let bounds = Region::new(c.start, c.start + c.length);
                    return Ok((regions.map_intersect(bounds), SamplePolicy::Random, vec![]));
                }
            }
            Ok((RegionList::default(), SamplePolicy::Random, vec![]))
        }
        "track_id" => {
            let tid = course.race_track_id as i64;
            let ok = match atom.op {
                Op::Eq => tid == atom.value,
                Op::Ne => tid != atom.value,
                _ => return Err("track_id: unsupported op".into()),
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "ground_condition" => {
            let g = horse.ground as i64;
            let ok = match atom.op {
                Op::Eq => g == atom.value,
                Op::Ne => g != atom.value,
                Op::Ge => g >= atom.value,
                Op::Gt => g > atom.value,
                Op::Le => g <= atom.value,
                Op::Lt => g < atom.value,
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "is_used_skill_id" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("is_used_skill_id only supports ==".into());
            }
            Ok((
                regions.clone(),
                SamplePolicy::Immediate,
                vec![DynamicPred::UsedSkillId {
                    skill_id: atom.value.to_string(),
                }],
            ))
        }
        "post_number" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::PostNumber {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        // Solo-safe defaults: assume true for niche flags so skills still place.
        "temptation_count"
        | "is_hp_empty_onetime"
        | "popularity"
        | "lane_type"
        | "is_used_skill"
        | "distance_diff_top"
        | "distance_diff_top_float"
        | "distance_diff_rate"
        | "order_rate_in20_continue"
        | "order_rate_in40_continue"
        | "order_rate_in80_continue"
        | "order_rate_out20_continue"
        | "order_rate_out40_continue"
        | "order_rate_out50_continue"
        | "order_rate_out70_continue"
        | "running_style_equal_popularity_one"
        | "visiblehorse"
        | "same_skill_horse_count"
        | "is_own_course"
        | "running_style_count_same"
        | "running_style_count_same_rate"
        | "is_downslope"
        | "is_up_slope"
        | "is_behind_in" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::Always],
        )),
        "random_lot" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("random_lot only supports ==".into());
            }
            Ok((
                regions.clone(),
                SamplePolicy::Immediate,
                vec![DynamicPred::RandomLot {
                    max_exclusive: atom.value,
                }],
            ))
        }
        "hp_per" => Ok((
            regions.clone(),
            SamplePolicy::Immediate,
            vec![DynamicPred::HpPer {
                op: atom.op.clone(),
                value: atom.value,
            }],
        )),
        "weather" => Ok(value_filter_regions(
            regions,
            &atom.op,
            horse.weather,
            atom.value,
        )),
        "season" => Ok(value_filter_regions(
            regions,
            &atom.op,
            horse.season,
            atom.value,
        )),
        "time" => Ok(value_filter_regions(
            regions, &atom.op, horse.time, atom.value,
        )),
        "grade" => Ok(value_filter_regions(
            regions,
            &atom.op,
            horse.grade,
            atom.value,
        )),
        "is_badstart" => {
            if !matches!(atom.op, Op::Eq) || !(0..=1).contains(&atom.value) {
                return Err("is_badstart only supports ==0|1".into());
            }
            Ok((
                regions.clone(),
                SamplePolicy::Immediate,
                vec![DynamicPred::IsBadStart {
                    want_bad: atom.value == 1,
                }],
            ))
        }
        // Compile-time base-stat gates on mood-adjusted displayed stats (pre-green).
        "base_guts" | "base_wiz" | "base_speed" | "base_stamina" | "base_power" => {
            let base = match atom.name.as_str() {
                "base_speed" => displayed_stat(horse.speed, horse.mood),
                "base_stamina" => displayed_stat(horse.stamina, horse.mood),
                "base_power" => displayed_stat(horse.power, horse.mood),
                "base_guts" => displayed_stat(horse.guts, horse.mood),
                "base_wiz" => displayed_stat(horse.wisdom, horse.mood),
                _ => unreachable!(),
            };
            let thresh = atom.value as f64;
            let ok = match atom.op {
                Op::Eq => (base - thresh).abs() < 1e-6,
                Op::Ne => (base - thresh).abs() >= 1e-6,
                Op::Ge => base >= thresh,
                Op::Gt => base > thresh,
                Op::Le => base <= thresh,
                Op::Lt => base < thresh,
            };
            if ok {
                Ok((regions.clone(), SamplePolicy::Immediate, vec![]))
            } else {
                Ok((RegionList::default(), SamplePolicy::Immediate, vec![]))
            }
        }
        "slope" => {
            if !matches!(atom.op, Op::Eq) {
                return Err("slope only supports ==".into());
            }
            let pieces = slope_regions(course, atom.value)?;
            Ok((
                regions.map_intersect_all(&pieces),
                SamplePolicy::Immediate,
                vec![],
            ))
        }
        other => Err(format!("unsupported condition keyword: {other}")),
    }
}

fn reduce_expr(
    expr: &Expr,
    regions: &RegionList,
    course: &Course,
    horse: HorseCtx,
) -> Result<ReducedCondition, String> {
    match expr {
        Expr::Atom(a) => {
            let (regions, policy, dynamics) = filter_atom(regions, a, course, horse)?;
            Ok(ReducedCondition {
                regions,
                policy,
                dynamics,
            })
        }
        Expr::And(a, b) => {
            let left = reduce_expr(a, regions, course, horse)?;
            let right = reduce_expr(b, &left.regions, course, horse)?;
            let mut dynamics = left.dynamics;
            dynamics.extend(right.dynamics);
            Ok(ReducedCondition {
                regions: right.regions,
                policy: left.policy.reconcile(right.policy),
                dynamics,
            })
        }
        Expr::Or(a, b) => {
            let left = reduce_expr(a, regions, course, horse)?;
            let right = reduce_expr(b, regions, course, horse)?;
            let mut dynamics = left.dynamics;
            dynamics.extend(right.dynamics);
            Ok(ReducedCondition {
                regions: left.regions.union(&right.regions),
                policy: left.policy.reconcile(right.policy),
                dynamics,
            })
        }
    }
}

pub fn reduce_condition_str(
    condition: &str,
    course: &Course,
    horse: HorseCtx,
) -> Result<ReducedCondition, String> {
    let expr = parse_condition(condition)
        .map_err(|e| e.to_string())?
        .unwrap_or(Expr::Atom(Atom {
            name: "always".into(),
            op: Op::Eq,
            value: 1,
        }));
    let whole = RegionList::whole_course(course.distance);
    reduce_expr(&expr, &whole, course, horse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::get_course;
    use crate::hp::Aptitude;
    use crate::hp::Strategy;

    fn ctx() -> HorseCtx {
        HorseCtx {
            strategy: Strategy::Oikomi,
            distance_apt: Aptitude::A,
            surface_apt: Aptitude::A,
            ground: crate::hp::GroundCondition::Good,
            mood: 0,
            speed: 1000.0,
            stamina: 1000.0,
            power: 1000.0,
            guts: 1000.0,
            wisdom: 1000.0,
            weather: 1,
            season: 1,
            time: 2,
            grade: 100,
        }
    }

    #[test]
    fn phase_random_2_on_1600_is_final_third() {
        let c = get_course(10611).unwrap();
        let r = reduce_condition_str("phase_random==2", c, ctx()).unwrap();
        assert!(!r.regions.is_empty());
        assert_eq!(r.policy, SamplePolicy::Random);
        let reg = r.regions.regions[0];
        assert!((reg.start - 1600.0 * 2.0 / 3.0).abs() < 1e-6);
        assert!((reg.end - 1600.0 * 5.0 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn all_corner_random_intersects_corners_not_always() {
        let c = get_course(10611).unwrap();
        let r = reduce_condition_str("all_corner_random==1", c, ctx()).unwrap();
        assert_eq!(r.policy, SamplePolicy::AllCornerRandom);
        assert!(!r.regions.is_empty());
        assert_eq!(r.regions.regions.len(), c.corners.len());
        for (reg, corner) in r.regions.regions.iter().zip(c.corners.iter()) {
            assert!((reg.start - corner.start).abs() < 1e-9);
            assert!((reg.end - (corner.start + corner.length)).abs() < 1e-9);
        }
    }

    #[test]
    fn base_power_gate_uses_mood_adjusted_stats() {
        let c = get_course(10611).unwrap();
        let mut weak = ctx();
        weak.power = 9.0;
        weak.mood = 2;
        let miss = reduce_condition_str("base_power>=1000", c, weak).unwrap();
        assert!(
            miss.regions.is_empty(),
            "power 9 should fail base_power>=1000"
        );
        let mut strong = ctx();
        strong.power = 1200.0;
        let hit = reduce_condition_str("base_power>=1000", c, strong).unwrap();
        assert!(!hit.regions.is_empty());
    }

    #[test]
    fn finalcorner_empty_on_straight_course_10301() {
        let c = get_course(10301).unwrap();
        assert!(c.corners.is_empty());
        let r = reduce_condition_str("is_finalcorner==1", c, ctx()).unwrap();
        assert!(
            r.regions.is_empty(),
            "empty corners must reject is_finalcorner"
        );
    }

    #[test]
    fn season_spring_default_rejects_autumn_winter_greens() {
        let c = get_course(10611).unwrap();
        // Builder default season=Spring(1); season==3|4 must empty.
        let r = reduce_condition_str("season==3@season==4", c, ctx()).unwrap();
        assert!(r.regions.is_empty());
        let spring = reduce_condition_str("season==1", c, ctx()).unwrap();
        assert!(!spring.regions.is_empty());
    }
}
