use crate::state::{CareerState, LegacyState, TraineeStats};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct LegacyFactorMeta {
    pub id: String,
    pub category: String,
    pub stat_key: Option<String>,
    pub skill_id: Option<String>,
    pub pink_tag: Option<String>,
    pub race_name: Option<String>,
}

type FactorLookupFn = fn(&str) -> Option<LegacyFactorMeta>;

static FACTOR_LOOKUP: LazyLock<Mutex<Option<FactorLookupFn>>> = LazyLock::new(|| Mutex::new(None));

pub struct LegacyFactorContext;

impl LegacyFactorContext {
    pub fn set_lookup(lookup: Option<FactorLookupFn>) {
        *FACTOR_LOOKUP.lock().unwrap() = lookup;
    }

    pub fn lookup(id: &str) -> Option<LegacyFactorMeta> {
        FACTOR_LOOKUP.lock().unwrap().and_then(|f| f(id))
    }
}

pub struct LegacyDeckConfig {
    per_star_cap_bonus: i32,
    max_spark_bonus: i32,
    inherited_skill_slots: i32,
}

impl Default for LegacyDeckConfig {
    fn default() -> Self {
        Self {
            per_star_cap_bonus: 20,
            max_spark_bonus: 400,
            inherited_skill_slots: 2,
        }
    }
}

static LEGACY_CONFIG: LazyLock<Mutex<LegacyDeckConfig>> =
    LazyLock::new(|| Mutex::new(LegacyDeckConfig::default()));

impl LegacyDeckConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let mut cfg = LEGACY_CONFIG.lock().unwrap();
        if let Some(bonus) = root.get("spark_stat_cap_bonus").and_then(|v| v.as_object()) {
            if let Some(v) = bonus.get("per_star").and_then(|v| v.as_i64()) {
                cfg.per_star_cap_bonus = v as i32;
            }
            if let Some(v) = bonus.get("max_bonus").and_then(|v| v.as_i64()) {
                cfg.max_spark_bonus = v as i32;
            }
        }
        if let Some(v) = root.get("inherited_skill_slots").and_then(|v| v.as_i64()) {
            cfg.inherited_skill_slots = (v as i32).max(0);
        }
    }

    pub fn per_star_cap_bonus() -> i32 {
        LEGACY_CONFIG.lock().unwrap().per_star_cap_bonus
    }

    pub fn max_spark_bonus() -> i32 {
        LEGACY_CONFIG.lock().unwrap().max_spark_bonus
    }

    pub fn inherited_skill_slots() -> i32 {
        LEGACY_CONFIG.lock().unwrap().inherited_skill_slots
    }
}

static BLUE_STAT_MAP: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| {
    Mutex::new(HashMap::from([
        ("factor:blue:1".into(), "speed".into()),
        ("factor:blue:2".into(), "stamina".into()),
        ("factor:blue:3".into(), "power".into()),
        ("factor:blue:4".into(), "guts".into()),
        ("factor:blue:5".into(), "wit".into()),
    ]))
});

pub struct LegacyApplicator;

impl LegacyApplicator {
    pub fn set_blue_stat_map(map: HashMap<String, String>) {
        *BLUE_STAT_MAP.lock().unwrap() = map;
    }

    fn blue_stat(key: &str) -> Option<String> {
        BLUE_STAT_MAP.lock().unwrap().get(key).cloned()
    }
    pub const PINK_WIT_PER_FACTOR: i32 = 2;
    pub const RACE_SP_BONUS_PER_FACTOR: i32 = 5;

    pub fn build_legacy(factor_entries: &[String], parent_names: Vec<String>) -> LegacyState {
        let mut factor_ids = Vec::new();
        let mut spark_caps = HashMap::new();
        let mut inherited_skills = Vec::new();
        let mut pink_factors = Vec::new();
        let mut pink_aptitude_tags = Vec::new();
        let mut race_factors = Vec::new();
        for entry in factor_entries {
            let (id, stars) = parse_entry(entry);
            factor_ids.push(id.clone());
            let meta = LegacyFactorContext::lookup(&id);
            match meta.as_ref().map(|m| m.category.as_str()) {
                Some("blue") => {
                    let stat = meta
                        .as_ref()
                        .and_then(|m| m.stat_key.clone())
                        .or_else(|| Self::blue_stat(&id))
                        .unwrap_or_default();
                    if !stat.is_empty() {
                        let bonus = (stars * LegacyDeckConfig::per_star_cap_bonus())
                            .min(LegacyDeckConfig::max_spark_bonus());
                        *spark_caps.entry(stat).or_insert(0) += bonus;
                    }
                }
                Some("skill") => {
                    if let Some(skill) = meta.as_ref().and_then(|m| m.skill_id.clone()) {
                        if inherited_skills.len()
                            < LegacyDeckConfig::inherited_skill_slots() as usize
                        {
                            inherited_skills.push(skill);
                        }
                    }
                }
                Some("pink") => {
                    pink_factors.push(id.clone());
                    if let Some(tag) = meta.as_ref().and_then(|m| m.pink_tag.clone()) {
                        if !pink_aptitude_tags.contains(&tag) {
                            pink_aptitude_tags.push(tag);
                        }
                    }
                }
                Some("race") => race_factors.push(id),
                _ => {
                    if meta.is_none() {
                        if let Some(stat) = Self::blue_stat(&id) {
                            let bonus = (stars * LegacyDeckConfig::per_star_cap_bonus())
                                .min(LegacyDeckConfig::max_spark_bonus());
                            *spark_caps.entry(stat).or_insert(0) += bonus;
                        }
                    }
                }
            }
        }
        LegacyState {
            parent_names,
            factor_ids,
            inherited_skill_ids: inherited_skills,
            spark_caps,
            pink_factor_ids: pink_factors,
            pink_aptitude_tags,
            race_factor_ids: race_factors,
            ..Default::default()
        }
    }

    pub fn apply_pink_aptitude(stats: TraineeStats, legacy: &LegacyState) -> TraineeStats {
        let bonus = legacy.pink_factor_ids.len() as i32 * Self::PINK_WIT_PER_FACTOR;
        if bonus > 0 {
            TraineeStats {
                wit: stats.wit + bonus,
                ..stats
            }
        } else {
            stats
        }
    }

    pub fn race_win_skill_bonus(legacy: &LegacyState) -> i32 {
        legacy.race_factor_ids.len() as i32 * Self::RACE_SP_BONUS_PER_FACTOR
    }

    pub fn effective_stat_cap(base_cap: i32, stat_key: &str, legacy: &LegacyState) -> i32 {
        base_cap + legacy.spark_caps.get(stat_key).copied().unwrap_or(0)
    }

    /// Choice 0: inherit skill slots. Choice 1: spark-stat focus (Speed +20, Stamina +20).
    pub fn apply_inheritance_choice(state: &CareerState, choice_index: i32) -> CareerState {
        if choice_index == 1 {
            let mut next = state.clone();
            next.stats.speed += 20;
            next.stats.stamina += 20;
            return next;
        }
        if choice_index != 0 {
            return state.clone();
        }
        let skills = &state.legacy.inherited_skill_ids;
        if skills.is_empty() {
            return state.clone();
        }
        let mut merged = state.learned_skill_ids.clone();
        for s in skills {
            if !merged.contains(s) {
                merged.push(s.clone());
            }
        }
        let mut next = state.clone();
        next.learned_skill_ids = merged;
        next
    }
}

fn parse_entry(entry: &str) -> (String, i32) {
    let Some(at) = entry.rfind('@') else {
        return (entry.to_string(), 3);
    };
    if at == 0 {
        return (entry.to_string(), 3);
    }
    let id = entry[..at].to_string();
    let stars = entry[at + 1..]
        .parse::<i32>()
        .map(|s| s.clamp(1, 3))
        .unwrap_or(3);
    (id, stars)
}
