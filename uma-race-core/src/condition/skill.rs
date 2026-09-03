//! Skill load + compile to pending triggers.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::condition::region::Region;
use crate::condition::regions::{reduce_condition_str, DynamicPred, HorseCtx};
use crate::course::Course;
use crate::rng::PrandoRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectType {
    SpeedUp = 1,
    StaminaUp = 2,
    PowerUp = 3,
    GutsUp = 4,
    WisdomUp = 5,
    Heal = 9,
    MultiplyStartDelay = 10,
    SetStartDelay = 14,
    CurrentSpeed = 21,
    CurrentSpeedDecel = 22,
    TargetSpeed = 27,
    /// Type 28: LaneMovementSpeed — lateral change boost; forward MoveLaneModifier
    /// only while `lane_change_speed > 0` (umalator).
    LaneMove = 28,
    Accel = 31,
    /// Unmodeled / specialty — ignored for forward physics.
    Other,
}

impl EffectType {
    pub fn from_u32(t: u32) -> Self {
        match t {
            1 => Self::SpeedUp,
            2 => Self::StaminaUp,
            3 => Self::PowerUp,
            4 => Self::GutsUp,
            5 => Self::WisdomUp,
            9 => Self::Heal,
            10 => Self::MultiplyStartDelay,
            14 => Self::SetStartDelay,
            21 => Self::CurrentSpeed,
            22 => Self::CurrentSpeedDecel,
            27 => Self::TargetSpeed,
            28 => Self::LaneMove,
            31 => Self::Accel,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkillEffect {
    pub kind: EffectType,
    /// Already scaled (modifier/10000).
    pub modifier: f64,
    /// Seconds.
    pub duration: f64,
}

#[derive(Clone, Debug)]
pub struct PendingSkill {
    pub skill_id: String,
    pub trigger: Region,
    pub effects: Vec<SkillEffect>,
    pub dynamics: Vec<DynamicPred>,
    pub activated: bool,
    /// Skip wisdom roll (green 1–5, unique/evolution).
    pub skip_wisdom: bool,
}

#[derive(Deserialize)]
struct SkillRow {
    payload: SkillPayload,
}

#[derive(Deserialize)]
struct SkillPayload {
    skill_id: serde_json::Value,
    #[serde(default)]
    rarity: Option<u32>,
    #[serde(default)]
    condition_groups: Vec<ConditionGroup>,
    /// Inherited (pink→white) form nested under parent unique in GameTora.
    #[serde(default)]
    gene_version: Option<GeneVersion>,
}

/// Inherited skill payload under `gene_version` (e.g. 910391 under 110391).
#[derive(Deserialize)]
struct GeneVersion {
    id: serde_json::Value,
    #[serde(default)]
    rarity: Option<u32>,
    #[serde(default)]
    condition_groups: Vec<ConditionGroup>,
}

#[derive(Deserialize)]
struct ConditionGroup {
    #[serde(default)]
    base_time: Option<i64>,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    precondition: Option<String>,
    #[serde(default)]
    effects: Vec<RawEffect>,
}

#[derive(Deserialize)]
struct RawEffect {
    #[serde(rename = "type")]
    type_: Option<u32>,
    value: Option<i64>,
    /// 1 = self (default); 9+ = other umas / AoE — skip for solo self-apply.
    #[serde(default)]
    target: Option<i64>,
}

fn skill_catalog() -> &'static HashMap<String, SkillPayload> {
    static CAT: OnceLock<HashMap<String, SkillPayload>> = OnceLock::new();
    CAT.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../knowledge/canonical/by_kind/skill.json");
        let raw = std::fs::read_to_string(&path).expect("skill.json");
        let rows: Vec<SkillRow> = serde_json::from_str(&raw).expect("parse skill.json");
        let mut map = HashMap::new();
        for row in rows {
            let mut payload = row.payload;
            let id = match &payload.skill_id {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => continue,
            };
            // Index inherited gene_version under its own id (GameTora nest, not GPL).
            if let Some(gene) = payload.gene_version.take() {
                let gid = match &gene.id {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::new(),
                };
                if !gid.is_empty() {
                    map.insert(
                        gid,
                        SkillPayload {
                            skill_id: gene.id,
                            rarity: gene.rarity,
                            condition_groups: gene.condition_groups,
                            gene_version: None,
                        },
                    );
                }
            }
            map.insert(id, payload);
        }
        map
    })
}

/// Wisdom skill-activation chance: `max(100 − 9000/wiz, 20) / 100`.
pub fn skill_activation_chance(wisdom: f64) -> f64 {
    if wisdom <= 0.0 {
        return 0.2;
    }
    ((100.0 - 9000.0 / wisdom).max(20.0) * 0.01).clamp(0.0, 1.0)
}

/// umalator `make_skill_data.pl` `patch_modifier`: scenario skills ×1.2.
fn scenario_skill_modifier_scale(skill_id: &str) -> f64 {
    const SCENARIO: &[&str] = &[
        "210011", "210012", "210021", "210022", "210031", "210032", "210041", "210042", "210051",
        "210052", "210061", "210062", "210071", "210072", "210081", "210082", "210261", "210262",
        "210271", "210272", "210281", "210282", "210291",
    ];
    if SCENARIO.iter().any(|s| *s == skill_id) {
        1.2
    } else {
        1.0
    }
}

pub fn compile_skills(
    skill_ids: &[String],
    course: &Course,
    horse: HorseCtx,
    skill_rng: &mut PrandoRng,
) -> Vec<PendingSkill> {
    let cat = skill_catalog();
    let mut out = Vec::new();
    for id in skill_ids {
        let Some(payload) = cat.get(id) else {
            continue;
        };
        let mut placed = 0usize;
        for g in &payload.condition_groups {
            let cond = g.condition.as_deref().unwrap_or("");
            let pre = g.precondition.as_deref().unwrap_or("");
            // After the first placed alternative, only keep follow-up triggers that
            // explicitly chain via is_activate_other_skill_detail / is_used_skill_id
            // (umalator RaceSolverBuilder.buildSkillData).
            if placed > 0
                && !cond.contains("is_activate_other_skill_detail")
                && !cond.contains("is_used_skill_id")
            {
                continue;
            }
            let mut extra_dynamics = Vec::new();
            // Precondition: existence gate; clip condition regions from first pre
            // start through course end (umalator buildSkillData). Carry dynamics.
            let mut pre_clip_start: Option<f64> = None;
            if !pre.is_empty() {
                if let Ok(pre_r) = reduce_condition_str(pre, course, horse) {
                    if pre_r.regions.is_empty() {
                        continue;
                    }
                    pre_clip_start = pre_r
                        .regions
                        .regions
                        .iter()
                        .map(|r| r.start)
                        .fold(None, |a, s| Some(a.map_or(s, |x: f64| x.min(s))));
                    extra_dynamics.extend(pre_r.dynamics);
                }
            }
            let Ok(mut reduced) = reduce_condition_str(cond, course, horse) else {
                continue;
            };
            if let Some(start) = pre_clip_start {
                reduced.regions =
                    reduced
                        .regions
                        .map_intersect(crate::condition::region::Region::new(
                            start,
                            course.distance,
                        ));
            }
            if reduced.regions.is_empty() {
                continue;
            }
            reduced.dynamics.extend(extra_dynamics);
            let Some(trigger) = reduced.policy.sample(&reduced.regions, skill_rng) else {
                continue;
            };
            let duration = g.base_time.unwrap_or(0) as f64 / 10_000.0;
            let effects: Vec<SkillEffect> = g
                .effects
                .iter()
                .filter_map(|e| {
                    let t = e.type_?;
                    let v = e.value? as f64;
                    // Non-self targets (AheadOfSelf=9, BehindSelf=10, …) are Noop for the
                    // focus horse (umalator isTarget + Perspective.Self).
                    if let Some(tgt) = e.target {
                        if tgt != 1 && tgt != 2 {
                            return None;
                        }
                    }
                    Some(SkillEffect {
                        kind: EffectType::from_u32(t),
                        modifier: (v * scenario_skill_modifier_scale(id)) / 10_000.0,
                        duration,
                    })
                })
                .filter(|e| e.kind != EffectType::Other)
                .collect();
            if effects.is_empty() {
                continue;
            }
            // umalator RaceSolver.shouldSkipWisdomCheck:
            // - Green skills: first effect type in 1..=5 (Speed/Stamina/Power/Guts/Wisdom)
            // - Unique: master rarity 3/4/5 remapped to SkillRarity.Unique
            // Evolution (6) still rolls.
            let green_skip = matches!(
                effects[0].kind,
                EffectType::SpeedUp
                    | EffectType::StaminaUp
                    | EffectType::PowerUp
                    | EffectType::GutsUp
                    | EffectType::WisdomUp
            );
            let skip_wisdom = green_skip || matches!(payload.rarity.unwrap_or(1), 3 | 4 | 5);
            out.push(PendingSkill {
                skill_id: id.clone(),
                trigger,
                effects,
                dynamics: reduced.dynamics,
                activated: false,
                skip_wisdom,
            });
            placed += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::get_course;
    use crate::hp::Aptitude;
    use crate::hp::Strategy;

    #[test]
    fn compiles_200701_phase_random_accel() {
        let c = get_course(10611).unwrap();
        let mut rng = PrandoRng::new(1);
        let pending = compile_skills(
            &["200701".into()],
            c,
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
            },
            &mut rng,
        );
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].effects[0].kind, EffectType::Accel);
        assert!((pending[0].effects[0].modifier - 0.4).abs() < 1e-9);
        assert!((pending[0].effects[0].duration - 3.0).abs() < 1e-9);
        assert!(pending[0].trigger.start >= 1600.0 * 2.0 / 3.0 - 1e-6);
        assert!(pending[0].trigger.start < 1600.0 * 5.0 / 6.0);
    }

    #[test]
    fn evolution_does_not_skip_wisdom_unique_does() {
        let c = get_course(10612).unwrap();
        let mut rng = PrandoRng::new(1);
        let evo = compile_skills(
            &["102701111".into()],
            c,
            HorseCtx {
                strategy: Strategy::Nige,
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
            },
            &mut rng,
        );
        assert_eq!(evo.len(), 1);
        assert!(!evo[0].skip_wisdom, "Evolution (6) must roll wisdom");

        let mut rng = PrandoRng::new(1);
        // 110391 is Unique (5)
        let uniq = compile_skills(
            &["110391".into()],
            get_course(10905).unwrap(),
            HorseCtx {
                strategy: Strategy::Sasi,
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
            },
            &mut rng,
        );
        assert!(!uniq.is_empty());
        assert!(uniq[0].skip_wisdom, "Unique (5) skips wisdom");
    }

    #[test]
    fn gene_version_910391_is_catalogued() {
        let c = get_course(10905).unwrap();
        let mut rng = PrandoRng::new(1);
        let pending = compile_skills(
            &["910391".into()],
            c,
            HorseCtx {
                strategy: Strategy::Sasi,
                distance_apt: Aptitude::C,
                surface_apt: Aptitude::G,
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
            },
            &mut rng,
        );
        assert_eq!(pending.len(), 1);
        assert!((pending[0].effects[0].modifier - 0.15).abs() < 1e-9);
        assert!((pending[0].effects[0].duration - 3.0).abs() < 1e-9);
        assert!(!pending[0].skip_wisdom);
    }

    #[test]
    fn activation_chance_kuromiak_examples() {
        assert!((skill_activation_chance(300.0) - 0.70).abs() < 1e-9);
        assert!((skill_activation_chance(600.0) - 0.85).abs() < 1e-9);
        assert!((skill_activation_chance(900.0) - 0.90).abs() < 1e-9);
        assert!((skill_activation_chance(1200.0) - 0.925).abs() < 1e-9);
        // Floor at 20% for very low wisdom.
        assert!((skill_activation_chance(50.0) - 0.20).abs() < 1e-9);
    }
}
