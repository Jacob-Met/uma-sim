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
    inherited_skill_slots: i32,
}

impl Default for LegacyDeckConfig {
    fn default() -> Self {
        Self {
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
        if let Some(v) = root.get("inherited_skill_slots").and_then(|v| v.as_i64()) {
            cfg.inherited_skill_slots = (v as i32).max(0);
        }
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

/// GameTora / Game8: initial inspiration blue ★ → starting stat.
pub fn blue_starting_stat_bonus(stars: i32) -> i32 {
    match stars.clamp(1, 3) {
        1 => 5,
        2 => 12,
        3 => 21,
        _ => 0,
    }
}

/// Soft-cap raise used for gene-value / training headroom (`parent_farming_utility.md`).
pub fn blue_cap_bonus(stars: i32) -> i32 {
    match stars.clamp(1, 3) {
        1 => 4,
        2 => 9,
        3 => 16,
        _ => 0,
    }
}

/// design.u-ma.org / `inheritance_planners.md`: matching pink★ total → aptitude rank-ups (cap A).
pub fn pink_aptitude_rank_ups(star_total: i32) -> i32 {
    match star_total {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 4,
    }
}

const APT_ORDER: [&str; 8] = ["G", "F", "E", "D", "C", "B", "A", "S"];

pub fn raise_aptitude_letter(letter: &str, ups: i32) -> String {
    if ups <= 0 {
        return letter.to_uppercase();
    }
    let cur = letter.to_uppercase();
    let idx = APT_ORDER.iter().position(|x| *x == cur).unwrap_or(0);
    // Red/pink inheritance alone cannot push past A (S is mid-run only).
    let next = (idx + ups as usize).min(APT_ORDER.iter().position(|x| *x == "A").unwrap_or(6));
    APT_ORDER[next].to_string()
}

/// Mid-run pink/red may reach S (initial inheritance cannot).
pub fn raise_aptitude_letter_uncapped(letter: &str, ups: i32) -> String {
    if ups <= 0 {
        return letter.to_uppercase();
    }
    let cur = letter.to_uppercase();
    let idx = APT_ORDER.iter().position(|x| *x == cur).unwrap_or(0);
    let next = (idx + ups as usize).min(APT_ORDER.len() - 1);
    APT_ORDER[next].to_string()
}

pub struct LegacyApplicator;

impl LegacyApplicator {
    pub fn set_blue_stat_map(map: HashMap<String, String>) {
        *BLUE_STAT_MAP.lock().unwrap() = map;
    }

    fn blue_stat(key: &str) -> Option<String> {
        BLUE_STAT_MAP.lock().unwrap().get(key).cloned()
    }

    pub fn blue_stat_public(key: &str) -> Option<String> {
        Self::blue_stat(key)
    }

    pub const RACE_SP_BONUS_PER_FACTOR: i32 = 5;

    pub fn build_legacy(factor_entries: &[String], parent_names: Vec<String>) -> LegacyState {
        let mut factor_ids = Vec::new();
        let mut spark_caps = HashMap::new();
        let mut blue_start = HashMap::new();
        let mut inherited_skills = Vec::new();
        let mut pink_factors = Vec::new();
        let mut pink_aptitude_tags = Vec::new();
        let mut pink_star_totals: HashMap<String, i32> = HashMap::new();
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
                        *spark_caps.entry(stat.clone()).or_insert(0) += blue_cap_bonus(stars);
                        *blue_start.entry(stat).or_insert(0) += blue_starting_stat_bonus(stars);
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
                        *pink_star_totals.entry(tag.clone()).or_insert(0) += stars;
                        if !pink_aptitude_tags.contains(&tag) {
                            pink_aptitude_tags.push(tag);
                        }
                    }
                }
                Some("race") => race_factors.push(id),
                _ => {
                    if meta.is_none() {
                        if let Some(stat) = Self::blue_stat(&id) {
                            *spark_caps.entry(stat.clone()).or_insert(0) += blue_cap_bonus(stars);
                            *blue_start.entry(stat).or_insert(0) += blue_starting_stat_bonus(stars);
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
            blue_start_bonuses: blue_start,
            pink_factor_ids: pink_factors,
            pink_aptitude_tags,
            pink_star_totals,
            race_factor_ids: race_factors,
            aptitudes: HashMap::new(),
            ..Default::default()
        }
    }

    /// Apply blue starting-stat bonuses (GameTora table).
    pub fn apply_blue_starting_stats(stats: TraineeStats, legacy: &LegacyState) -> TraineeStats {
        let b = &legacy.blue_start_bonuses;
        TraineeStats {
            speed: stats.speed + b.get("speed").copied().unwrap_or(0),
            stamina: stats.stamina + b.get("stamina").copied().unwrap_or(0),
            power: stats.power + b.get("power").copied().unwrap_or(0),
            guts: stats.guts + b.get("guts").copied().unwrap_or(0),
            wit: stats.wit + b.get("wit").copied().unwrap_or(0),
        }
    }

    /// Raise base aptitudes from pink★ totals (`inheritance_planners.md`). Caps at A.
    pub fn apply_pink_aptitudes(
        base: HashMap<String, String>,
        legacy: &LegacyState,
    ) -> HashMap<String, String> {
        let mut out = base;
        for (tag, stars) in &legacy.pink_star_totals {
            let ups = pink_aptitude_rank_ups(*stars);
            if ups == 0 {
                continue;
            }
            let key = tag.as_str();
            let cur = out.get(key).cloned().unwrap_or_else(|| "G".into());
            out.insert(key.to_string(), raise_aptitude_letter(&cur, ups));
        }
        out
    }

    /// Back-compat name: pink no longer adds wit; returns stats unchanged.
    pub fn apply_pink_aptitude(stats: TraineeStats, _legacy: &LegacyState) -> TraineeStats {
        stats
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

pub fn parse_entry_public(entry: &str) -> (String, i32) {
    parse_entry(entry)
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn pink_rank_table_matches_planner() {
        assert_eq!(pink_aptitude_rank_ups(0), 0);
        assert_eq!(pink_aptitude_rank_ups(1), 1);
        assert_eq!(pink_aptitude_rank_ups(3), 1);
        assert_eq!(pink_aptitude_rank_ups(4), 2);
        assert_eq!(pink_aptitude_rank_ups(9), 3);
        assert_eq!(pink_aptitude_rank_ups(10), 4);
    }

    #[test]
    fn aptitude_rise_caps_at_a() {
        assert_eq!(raise_aptitude_letter("G", 3), "D");
        assert_eq!(raise_aptitude_letter("B", 4), "A");
        assert_eq!(raise_aptitude_letter("A", 2), "A");
    }

    #[test]
    fn blue_star_tables() {
        assert_eq!(blue_starting_stat_bonus(1), 5);
        assert_eq!(blue_starting_stat_bonus(2), 12);
        assert_eq!(blue_starting_stat_bonus(3), 21);
        assert_eq!(blue_cap_bonus(3), 16);
    }
}
