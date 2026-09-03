//! Unity Cup spirit gauge + team-rank facility levels.

use crate::state::{CareerState, ScenarioResources, TrainingFacility};
use serde_json::Value;
use std::sync::Mutex;

struct UnityConfig {
    spirit_gain_base: i32,
    spirit_gain_per_level: i32,
    burst_threshold: i32,
    extreme_threshold: i32,
    extreme_stat_bonus: i32,
    max_team_rank: i32,
    min_team_rank: i32,
}

impl Default for UnityConfig {
    fn default() -> Self {
        Self {
            spirit_gain_base: 15,
            spirit_gain_per_level: 3,
            burst_threshold: 100,
            extreme_threshold: 150,
            extreme_stat_bonus: 10,
            max_team_rank: 5,
            min_team_rank: 1,
        }
    }
}

static CONFIG: Mutex<UnityConfig> = Mutex::new(UnityConfig {
    spirit_gain_base: 15,
    spirit_gain_per_level: 3,
    burst_threshold: 100,
    extreme_threshold: 150,
    extreme_stat_bonus: 10,
    max_team_rank: 5,
    min_team_rank: 1,
});

static RANK_LABELS: [&str; 5] = ["F/G", "D/E", "B/C", "A", "S"];

pub struct UnityCupMechanics;

impl UnityCupMechanics {
    pub fn load_research(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = CONFIG.lock().unwrap();
        if let Some(gauge) = root.get("spirit_gauge").and_then(|v| v.as_object()) {
            if let Some(gain) = gauge
                .get("gain_per_training_success")
                .and_then(|v| v.as_object())
            {
                if let Some(v) = gain.get("base").and_then(|v| v.as_i64()) {
                    cfg.spirit_gain_base = v as i32;
                }
                if let Some(v) = gain.get("per_facility_level").and_then(|v| v.as_i64()) {
                    cfg.spirit_gain_per_level = v as i32;
                }
            }
            if let Some(v) = gauge.get("burst_threshold").and_then(|v| v.as_i64()) {
                cfg.burst_threshold = v as i32;
            }
            if let Some(v) = gauge
                .get("extreme_burst_threshold")
                .and_then(|v| v.as_i64())
            {
                cfg.extreme_threshold = v as i32;
            }
            if let Some(v) = gauge.get("extreme_stat_bonus").and_then(|v| v.as_i64()) {
                cfg.extreme_stat_bonus = v as i32;
            }
        }
        if let Some(cfg_obj) = root
            .get("team_rank_facility_levels")
            .and_then(|v| v.as_object())
        {
            if let Some(v) = cfg_obj.get("max_rank").and_then(|v| v.as_i64()) {
                cfg.max_team_rank = (v as i32).clamp(1, 5);
            }
            if let Some(v) = cfg_obj.get("min_rank").and_then(|v| v.as_i64()) {
                cfg.min_team_rank = (v as i32).clamp(1, 5);
            }
        }
    }

    pub fn rank_resource_key(facility: TrainingFacility) -> String {
        format!("unity_rank_{}", facility.key())
    }

    pub fn initial_resources() -> ScenarioResources {
        let mut values = std::collections::HashMap::new();
        let min = CONFIG.lock().unwrap().min_team_rank;
        for f in TrainingFacility::ALL {
            values.insert(Self::rank_resource_key(f), min);
        }
        ScenarioResources { values }
    }

    pub fn facility_level_for(resources: &ScenarioResources, facility: TrainingFacility) -> i32 {
        let cfg = CONFIG.lock().unwrap();
        resources
            .get(&Self::rank_resource_key(facility))
            .clamp(cfg.min_team_rank, cfg.max_team_rank)
    }

    pub fn facility_levels_from_resources(
        resources: &ScenarioResources,
    ) -> std::collections::HashMap<String, i32> {
        TrainingFacility::ALL
            .iter()
            .map(|f| (f.key().to_string(), Self::facility_level_for(resources, *f)))
            .collect()
    }

    pub fn rank_label(rank: i32) -> String {
        let idx = (rank.clamp(1, 5) - 1) as usize;
        RANK_LABELS.get(idx).unwrap_or(&"?").to_string()
    }

    pub fn spirit_gain(facility_level: i32) -> i32 {
        let cfg = CONFIG.lock().unwrap();
        cfg.spirit_gain_base + facility_level.clamp(1, 5) * cfg.spirit_gain_per_level
    }

    pub fn apply_training_spirit(
        resources: &ScenarioResources,
        gain: i32,
        facility: TrainingFacility,
    ) -> (ScenarioResources, Vec<String>, bool) {
        let (burst_threshold, extreme_threshold) = {
            let cfg = CONFIG.lock().unwrap();
            (cfg.burst_threshold, cfg.extreme_threshold)
        };
        let mut res = resources.clone();
        let mut lines = vec![format!("Unity spirit +{gain}")];
        let mut rank_up = false;
        let spirit = res.get("unity_spirit") + gain;

        if spirit >= extreme_threshold {
            res = res.set("unity_spirit", 0).add("unity_extreme_ready", 1);
            lines.push("Extreme Spirit Burst ready!".to_string());
            let (bumped, line) = Self::bump_team_rank(&res, facility);
            res = bumped;
            if let Some(l) = line {
                lines.push(l);
                rank_up = true;
            }
        } else if spirit >= burst_threshold {
            res = res.set("unity_spirit", 0).add("unity_burst_ready", 1);
            let bursts = res.get("unity_burst_count") + 1;
            res = res.set("unity_burst_count", bursts);
            lines.push("Spirit Burst ready!".to_string());
            let (bumped, line) = Self::bump_team_rank(&res, facility);
            res = bumped;
            if let Some(l) = line {
                lines.push(l);
                rank_up = true;
            }
        } else {
            res = res.set("unity_spirit", spirit);
        }
        (res, lines, rank_up)
    }

    pub fn bump_team_rank(
        resources: &ScenarioResources,
        facility: TrainingFacility,
    ) -> (ScenarioResources, Option<String>) {
        let cfg = CONFIG.lock().unwrap();
        let key = Self::rank_resource_key(facility);
        let current = resources
            .get(&key)
            .clamp(cfg.min_team_rank, cfg.max_team_rank);
        if current >= cfg.max_team_rank {
            return (resources.clone(), None);
        }
        let next = current + 1;
        let line = format!(
            "{} team rank {} → {} (training Lv{next})",
            facility.name(),
            Self::rank_label(current),
            Self::rank_label(next),
        );
        (resources.set(&key, next), Some(line))
    }

    pub fn bump_all_team_ranks(resources: &ScenarioResources) -> (ScenarioResources, Vec<String>) {
        let mut res = resources.clone();
        let mut lines = Vec::new();
        for facility in TrainingFacility::ALL {
            let (next, line) = Self::bump_team_rank(&res, facility);
            res = next;
            if let Some(l) = line {
                lines.push(l);
            }
        }
        (res, lines)
    }

    pub fn consume_extreme_burst(
        state: &CareerState,
        facility: TrainingFacility,
    ) -> (CareerState, Vec<String>) {
        if state.scenario_resources.get("unity_extreme_ready") <= 0 {
            return (state.clone(), Vec::new());
        }
        let bonus = CONFIG.lock().unwrap().extreme_stat_bonus;
        let mut res = state.scenario_resources.add("unity_extreme_ready", -1);
        let bursts = res.get("unity_burst_count") + 1;
        res = res.set("unity_burst_count", bursts);
        let mut s = state.clone();
        s.scenario_resources = res;
        s.stats = state.stats.with_delta(facility, bonus);
        let mut lines = vec![format!("Extreme burst bonus +{bonus} {}", facility.name())];
        // Ignited Spirit (≤9 bursts + matching extreme) — research/unity_cup.json.
        if bursts <= 9 {
            let hint_key = format!("ignited_spirit_{}", facility.key());
            let lv = s.hint_levels.get(&hint_key).copied().unwrap_or(0) + 1;
            s.hint_levels.insert(hint_key.clone(), lv);
            lines.push(format!("Ignited Spirit hint {hint_key} L{lv}"));
        }
        (s, lines)
    }

    pub fn zero_failure_when_burst_ready(resources: &ScenarioResources) -> bool {
        resources.get("unity_burst_ready") > 0 || resources.get("unity_extreme_ready") > 0
    }
}
