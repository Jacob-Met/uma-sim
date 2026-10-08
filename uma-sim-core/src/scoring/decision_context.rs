use std::collections::{HashMap, HashSet};

use super::objective_profiles::{ObjectiveProfiles, ObjectiveWeights};
use super::types::{DateYear, StatName};

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionContext {
    pub trainee_name: String,
    pub energy: i32,
    pub mood_ordinal: i32,
    pub stats: HashMap<StatName, i32>,
    pub stat_caps: HashMap<StatName, i32>,
    pub stat_prioritization: Vec<StatName>,
    pub event_choice_stat_priority: Vec<StatName>,
    pub day: i32,
    pub year: DateYear,
    pub is_finale_season: bool,
    pub remaining_turns: i32,
    pub objective_profile: String,
    pub prioritize_energy: bool,
    pub dating_schedule_enabled: bool,
    pub dating_chain_complete: bool,
    pub preferred_skill_hints: HashSet<String>,
    pub deck_support_names: Vec<String>,
    pub is_hype_maxed: bool,
    pub songs_learned: i32,
    pub days_to_concert: i32,
    pub token_totals: HashMap<String, i32>,
}

impl Default for DecisionContext {
    fn default() -> Self {
        Self {
            trainee_name: String::new(),
            energy: 100,
            mood_ordinal: 2,
            stats: HashMap::new(),
            stat_caps: HashMap::new(),
            stat_prioritization: StatName::ALL.to_vec(),
            event_choice_stat_priority: StatName::ALL.to_vec(),
            day: 1,
            year: DateYear::Junior,
            is_finale_season: false,
            remaining_turns: 72,
            objective_profile: "stat_total".to_string(),
            prioritize_energy: false,
            dating_schedule_enabled: false,
            dating_chain_complete: false,
            preferred_skill_hints: HashSet::new(),
            deck_support_names: Vec::new(),
            is_hype_maxed: false,
            songs_learned: 0,
            days_to_concert: -1,
            token_totals: HashMap::new(),
        }
    }
}

impl DecisionContext {
    pub fn objective_weights(&self) -> ObjectiveWeights {
        ObjectiveProfiles::by_name(&self.objective_profile)
    }

    pub fn current_stat(&self, stat: StatName) -> i32 {
        self.stats.get(&stat).copied().unwrap_or(0)
    }

    pub fn soft_cap_discount(&self, stat: StatName, gain: i32) -> f64 {
        let current = self.current_stat(stat);
        let soft_cap = 1200;
        if current >= soft_cap {
            return gain as f64 * 0.5;
        }
        if current + gain <= soft_cap {
            return gain as f64;
        }
        let before = soft_cap - current;
        let after = gain - before;
        before as f64 + after as f64 * 0.5
    }

    /// Approx. Grand Live promo concerts (Junior Late Dec onward every 6 months) + Grand Concert day 72.
    pub const GRAND_LIVE_CONCERT_DAYS: [i32; 5] = [24, 36, 48, 60, 72];

    pub fn days_to_next_concert(day: i32, concert_days: &[i32]) -> i32 {
        let next = concert_days.iter().copied().find(|d| *d >= day);
        match next {
            Some(n) => n - day,
            None => -1,
        }
    }
}

/// Mood ordinal mirroring Android Mood enum order (AWFUL=0 … GREAT=4).
pub mod mood_ordinal {
    pub const AWFUL: i32 = 0;
    pub const BAD: i32 = 1;
    pub const NORMAL: i32 = 2;
    pub const GOOD: i32 = 3;
    pub const GREAT: i32 = 4;
}
