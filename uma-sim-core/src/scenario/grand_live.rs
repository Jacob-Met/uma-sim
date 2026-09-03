//! Grand Live / Grand Concert mechanics.

use crate::rng::SimRandom;
use crate::scenario::grand_live_deck_support::GrandLiveDeckSupport;
use crate::state::{CareerState, ScenarioResources, TrainingFacility};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub const CONCERT_TURNS: [i32; 5] = [24, 36, 48, 60, 72];
/// Make Debut! is granted ~4 turns after career start (uma.guide), not at t=0.
pub const MAKE_DEBUT_GRANT_TURN: i32 = 5;
pub const CYCLE_SONGS_AT_START: i32 = 0;
pub const DEFAULT_GREAT_SUCCESS_REQUIRED: i32 = 3;
pub const DEFAULT_CYCLE_MAX: i32 = 4;
pub const TARGET_SONGS: i32 = 18;
pub const SOFT_CAP: i32 = 1200;
pub const PERF_MAX_BASE: i32 = 200;
pub const LIGHT_HELLO_CHANCE: f64 = 0.45;
pub const LIGHT_HELLO_GRANT: i32 = 20;

pub static PERF_CODES: [&str; 5] = ["Da", "Pa", "Vo", "Vi", "Me"];

static PERF_NAME_TO_CODE: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("Dance", "Da"),
        ("Passion", "Pa"),
        ("Vocal", "Vo"),
        ("Vocals", "Vo"),
        ("Visual", "Vi"),
        ("Visuals", "Vi"),
        ("Composure", "Me"),
        ("Mental", "Me"),
    ])
});

static FACILITY_TOKEN_SPLIT: LazyLock<HashMap<TrainingFacility, Vec<(String, i32)>>> =
    LazyLock::new(|| {
        HashMap::from([
            (
                TrainingFacility::Speed,
                vec![("Da".into(), 60), ("Pa".into(), 30), ("Vo".into(), 10)],
            ),
            (
                TrainingFacility::Stamina,
                vec![("Pa".into(), 60), ("Vi".into(), 30), ("Da".into(), 10)],
            ),
            (
                TrainingFacility::Power,
                vec![("Vo".into(), 60), ("Da".into(), 30), ("Pa".into(), 10)],
            ),
            (
                TrainingFacility::Guts,
                vec![("Vi".into(), 60), ("Pa".into(), 30), ("Vo".into(), 10)],
            ),
            (
                TrainingFacility::Wit,
                vec![("Me".into(), 60), ("Vo".into(), 30), ("Vi".into(), 10)],
            ),
        ])
    });

type CalibrationFn = fn(TrainingFacility, i32, i32, i32) -> Option<HashMap<String, i32>>;

struct GrandLiveConfig {
    facility_split_override: Option<HashMap<TrainingFacility, Vec<(String, i32)>>>,
    great_success_by_race: HashMap<String, i32>,
    cycle_max: i32,
    gate_before_1st: Vec<i32>,
    gate_before_2nd_4th: Vec<i32>,
    gate_before_grand: Vec<i32>,
    great_success_stat_bonus: i32,
    normal_concert_stat_bonus: i32,
    sp_per_technique_since_last: i32,
    sp_per_song_since_last: i32,
    perf_cap_raise_per_concert: i32,
    make_debut_token_grant: i32,
    calibration_lookup: Option<CalibrationFn>,
}

impl Default for GrandLiveConfig {
    fn default() -> Self {
        Self {
            facility_split_override: None,
            great_success_by_race: HashMap::new(),
            cycle_max: DEFAULT_CYCLE_MAX,
            gate_before_1st: vec![1, 2, 3, 4, 4, 2, 3],
            gate_before_2nd_4th: vec![2, 2, 2, 4, 5, 2, 2],
            gate_before_grand: vec![2, 2, 2, 4, 3, 2, 2],
            great_success_stat_bonus: 10,
            normal_concert_stat_bonus: 3,
            sp_per_technique_since_last: 5,
            sp_per_song_since_last: 25,
            perf_cap_raise_per_concert: 50,
            make_debut_token_grant: 10,
            calibration_lookup: None,
        }
    }
}

static CONFIG: LazyLock<Mutex<GrandLiveConfig>> =
    LazyLock::new(|| Mutex::new(GrandLiveConfig::default()));

#[derive(Debug, Clone)]
pub struct GrandLiveConcertBonus {
    pub effect: String,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct GrandLiveSong {
    pub song_list_id: i32,
    pub name: String,
    pub concert_bonus: Option<GrandLiveConcertBonus>,
}

pub struct GrandLiveMechanics;

impl GrandLiveMechanics {
    pub fn install_calibration(lookup: Option<CalibrationFn>) {
        CONFIG.lock().unwrap().calibration_lookup = lookup;
    }

    pub fn load_research(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let mut cfg = CONFIG.lock().unwrap();
        if let Some(split) = root
            .get("facility_performance_split")
            .and_then(|v| v.as_object())
        {
            let mut parsed = HashMap::new();
            for fac in TrainingFacility::ALL {
                let key = fac.key();
                let Some(arr) = split.get(key).and_then(|v| v.as_array()) else {
                    continue;
                };
                let entries: Vec<(String, i32)> = arr
                    .iter()
                    .filter_map(|el| {
                        let o = el.as_object()?;
                        let ty = o.get("type")?.as_str()?;
                        let pct = o.get("pct")?.as_i64()? as i32;
                        Some((Self::perf_code(ty), pct))
                    })
                    .collect();
                if !entries.is_empty() {
                    parsed.insert(fac, entries);
                }
            }
            if !parsed.is_empty() {
                cfg.facility_split_override = Some(parsed);
            }
        }
        if let Some(obj) = root
            .get("great_success_required_by_concert")
            .and_then(|v| v.as_object())
        {
            for (race_id, el) in obj {
                if let Some(n) = el.as_i64() {
                    cfg.great_success_by_race.insert(race_id.clone(), n as i32);
                }
            }
        }
        if let Some(arr) = root.get("concerts").and_then(|v| v.as_array()) {
            for el in arr {
                if let Some(o) = el.as_object() {
                    if let (Some(race_id), Some(required)) = (
                        o.get("race_id").and_then(|v| v.as_str()),
                        o.get("great_success_required").and_then(|v| v.as_i64()),
                    ) {
                        cfg.great_success_by_race
                            .insert(race_id.to_string(), required as i32);
                    }
                }
            }
        }
        if let Some(hype) = root.get("hype_model").and_then(|v| v.as_object()) {
            if let Some(v) = hype
                .get("cycle_songs_maximum_per_concert")
                .and_then(|v| v.as_i64())
            {
                cfg.cycle_max = v as i32;
            }
        }
    }

    pub fn load_community_calibration(
        gate_before_1st: Option<Vec<i32>>,
        gate_before_2nd_4th: Option<Vec<i32>>,
        gate_before_grand: Option<Vec<i32>>,
        great_success_stat_bonus: Option<i32>,
        normal_concert_stat_bonus: Option<i32>,
        sp_per_technique_since_last: Option<i32>,
        sp_per_song_since_last: Option<i32>,
        perf_cap_raise_per_concert: Option<i32>,
        cycle_max: Option<i32>,
    ) {
        let mut cfg = CONFIG.lock().unwrap();
        if let Some(v) = gate_before_1st.filter(|v| !v.is_empty()) {
            cfg.gate_before_1st = v;
        }
        if let Some(v) = gate_before_2nd_4th.filter(|v| !v.is_empty()) {
            cfg.gate_before_2nd_4th = v;
        }
        if let Some(v) = gate_before_grand.filter(|v| !v.is_empty()) {
            cfg.gate_before_grand = v;
        }
        if let Some(v) = great_success_stat_bonus {
            cfg.great_success_stat_bonus = v;
        }
        if let Some(v) = normal_concert_stat_bonus {
            cfg.normal_concert_stat_bonus = v;
        }
        if let Some(v) = sp_per_technique_since_last {
            cfg.sp_per_technique_since_last = v;
        }
        if let Some(v) = sp_per_song_since_last {
            cfg.sp_per_song_since_last = v;
        }
        if let Some(v) = perf_cap_raise_per_concert {
            cfg.perf_cap_raise_per_concert = v;
        }
        if let Some(v) = cycle_max {
            cfg.cycle_max = v.max(1);
        }
    }

    pub fn facility_split(facility: TrainingFacility) -> Vec<(String, i32)> {
        let cfg = CONFIG.lock().unwrap();
        cfg.facility_split_override
            .as_ref()
            .and_then(|m| m.get(&facility).cloned())
            .or_else(|| FACILITY_TOKEN_SPLIT.get(&facility).cloned())
            .unwrap_or_default()
    }

    pub fn perf_code(name: &str) -> String {
        PERF_NAME_TO_CODE
            .get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.chars().take(2).collect())
    }

    pub fn perf_resource_key(code: &str) -> String {
        format!("perf_{code}")
    }

    pub fn perf_max(resources: &ScenarioResources) -> i32 {
        PERF_MAX_BASE + resources.get("perf_cap_bonus")
    }

    pub fn token_totals_for_bot(resources: &ScenarioResources) -> HashMap<String, i32> {
        PERF_CODES
            .iter()
            .map(|code| {
                (
                    code.to_string(),
                    resources.get(&Self::perf_resource_key(code)),
                )
            })
            .collect()
    }

    pub fn cycle_songs(resources: &ScenarioResources) -> i32 {
        resources.get("hype")
    }

    pub fn cycle_max() -> i32 {
        CONFIG.lock().unwrap().cycle_max
    }

    pub fn great_success_required(
        resources: &ScenarioResources,
        upcoming_race_id: Option<&str>,
    ) -> i32 {
        let stored = resources.get("great_success_required");
        if stored > 0 {
            return stored;
        }
        if let Some(id) = upcoming_race_id {
            if let Some(v) = CONFIG.lock().unwrap().great_success_by_race.get(id) {
                return *v;
            }
        }
        DEFAULT_GREAT_SUCCESS_REQUIRED
    }

    pub fn is_hype_maxed(resources: &ScenarioResources) -> bool {
        Self::cycle_songs(resources) >= Self::great_success_required(resources, None)
    }

    pub fn great_success_required_for_race(race_id: &str) -> i32 {
        CONFIG
            .lock()
            .unwrap()
            .great_success_by_race
            .get(race_id)
            .copied()
            .unwrap_or(if race_id == "debut" {
                0
            } else {
                DEFAULT_GREAT_SUCCESS_REQUIRED
            })
    }

    pub fn career_part(state: &CareerState) -> i32 {
        if state.date.year >= 3 && state.date.month == 12 && state.date.half == 2 {
            4
        } else if state.date.year >= 3 {
            3
        } else if state.date.year >= 2 {
            2
        } else {
            1
        }
    }

    fn technique_gate_sequence(concert_index: i32) -> Vec<i32> {
        let cfg = CONFIG.lock().unwrap();
        // concert_index = completed promo/grand concerts (debut race does not count).
        if concert_index < 1 {
            cfg.gate_before_1st.clone()
        } else if concert_index < 4 {
            cfg.gate_before_2nd_4th.clone()
        } else {
            cfg.gate_before_grand.clone()
        }
    }

    pub fn techniques_required_for_next_song(resources: &ScenarioResources) -> i32 {
        let seq = Self::technique_gate_sequence(resources.get("concert_index"));
        let slot = resources
            .get("song_slot_index")
            .clamp(0, seq.len() as i32 - 1) as usize;
        seq[slot]
    }

    pub fn songs_unlocked_on_board(state: &CareerState) -> bool {
        state.scenario_resources.get("techniques_since_last_song")
            >= Self::techniques_required_for_next_song(&state.scenario_resources)
    }

    pub fn can_add_cycle_song(resources: &ScenarioResources) -> bool {
        Self::cycle_songs(resources) < Self::cycle_max()
    }

    pub fn initial_resources() -> ScenarioResources {
        let mut values = HashMap::from([
            ("hype".to_string(), CYCLE_SONGS_AT_START),
            ("songs_learned".to_string(), 0),
            ("techniques_learned".to_string(), 0),
            ("techniques_since_last_song".to_string(), 0),
            ("song_slot_index".to_string(), 0),
            ("cycle_techniques".to_string(), 0),
            ("concert_index".to_string(), 0),
            ("perf_cap_bonus".to_string(), 0),
            (
                "great_success_required".to_string(),
                Self::great_success_required_for_race("promo_1"),
            ),
            ("activated_songs".to_string(), 0),
            ("lesson_refresh".to_string(), 0),
        ]);
        for code in PERF_CODES {
            values.insert(Self::perf_resource_key(code), 0);
        }
        ScenarioResources { values }
    }

    /// Grant Make Debut! song + mastery tokens into the first concert cycle.
    pub fn grant_make_debut_song(
        resources: &ScenarioResources,
    ) -> (ScenarioResources, Vec<String>) {
        if Self::owns_song(resources, 1) {
            return (resources.clone(), Vec::new());
        }
        let mut res = Self::mark_song_owned(resources, 1, true);
        res = Self::add_perf_tokens(&res, &Self::make_debut_perf_grant());
        (
            res,
            vec![
                "Make Debut! added to setlist".to_string(),
                "Make Debut: all performance tokens +10".to_string(),
            ],
        )
    }

    pub fn days_to_concert(turn: i32) -> i32 {
        if turn >= *CONCERT_TURNS.last().unwrap() {
            return 0;
        }
        CONCERT_TURNS
            .iter()
            .find(|&&t| t >= turn)
            .map(|t| t - turn)
            .unwrap_or(-1)
    }

    pub fn concert_race_id(turn: i32) -> Option<&'static str> {
        match turn {
            1 => Some("debut"),
            24 => Some("promo_1"),
            36 => Some("promo_2"),
            48 => Some("promo_3"),
            60 => Some("promo_4"),
            72 => Some("grand_concert"),
            _ => None,
        }
    }

    pub fn next_concert_race_id(completed_concerts: i32) -> Option<&'static str> {
        match completed_concerts {
            0 => Some("debut"),
            1 => Some("promo_1"),
            2 => Some("promo_2"),
            3 => Some("promo_3"),
            4 => Some("promo_4"),
            5 => Some("grand_concert"),
            _ => None,
        }
    }

    pub fn concert_race_name(id: &str) -> &str {
        match id {
            "debut" => "Junior Make Debut",
            "promo_1" => "Promo Concert 1",
            "promo_2" => "Promo Concert 2",
            "promo_3" => "Promo Concert 3",
            "promo_4" => "Promo Concert 4",
            "grand_concert" => "Grand Concert",
            _ => id,
        }
    }

    /// Packet / MDB `live_type` for a concert race (confirmed via KUC captures).
    pub fn live_type_for_race(race_id: &str) -> i32 {
        match race_id {
            "debut" => 0,
            "promo_1" => 1,
            "promo_2" => 2,
            "promo_3" => 3,
            "promo_4" => 4,
            "grand_concert" => 5,
            _ => -1,
        }
    }

    /// Packet `live_results.result_state` (aligned with captured Great Success = 2).
    pub fn result_state_for_outcome(outcome: ConcertOutcome) -> i32 {
        outcome.as_resource_value()
    }

    /// Packet `training_bonuses.target_type` for activated concert bonus families.
    ///
    /// Mapping grounded in song bonus families + captured `{target_type: 6, effect_value: 2}`
    /// shape; values are stable sim ↔ packet mirrors until full MDB enum ingest.
    /// Packet `live_bonus_type` for a song concert-bonus family (GameTora: three families).
    pub fn live_bonus_type_for_effect(effect: &str) -> i32 {
        let lower = effect.to_lowercase();
        if lower.contains("friendship") {
            1
        } else if lower.contains("specialty") || lower.contains("speciality") {
            2
        } else if lower.contains("support chain") {
            3
        } else {
            0
        }
    }

    /// Packet `training_bonuses.target_type` for activated concert bonus families.
    ///
    /// Mapping grounded in song bonus families + captured `{target_type: 6, effect_value: 2}`
    /// shape; values are stable sim ↔ packet mirrors until full MDB enum ingest.
    pub fn training_bonus_target_type(resource_key: &str) -> i32 {
        match resource_key {
            "bonus_friendship_pct" => 1,
            "bonus_specialty_pct" => 2,
            "bonus_support_chain_pct" => 6, // matches captured target_type for chain-style bonus
            "bonus_training_misc" => 3,
            _ => 0,
        }
    }

    pub fn training_bonuses_packet(resources: &ScenarioResources) -> Vec<(i32, i32)> {
        [
            "bonus_friendship_pct",
            "bonus_specialty_pct",
            "bonus_support_chain_pct",
            "bonus_training_misc",
        ]
        .into_iter()
        .filter_map(|key| {
            let value = resources.get(key);
            if value <= 0 {
                None
            } else {
                Some((Self::training_bonus_target_type(key), value))
            }
        })
        .collect()
    }

    pub fn dating_unlocked(resources: &ScenarioResources) -> bool {
        resources.get("dating_unlocked") > 0
    }

    pub fn unlock_dating(resources: &ScenarioResources) -> ScenarioResources {
        resources.set("dating_unlocked", 1)
    }

    /// Light Hello / scenario-link dating-starts chance per free turn (not a fixed schedule).
    pub fn dating_start_event_chance() -> f64 {
        0.12
    }

    pub fn soft_cap_gain(current_stat: i32, raw_gain: i32) -> i32 {
        if raw_gain <= 0 {
            return 0;
        }
        if current_stat >= SOFT_CAP {
            return raw_gain / 2;
        }
        if current_stat + raw_gain <= SOFT_CAP {
            return raw_gain;
        }
        let before = SOFT_CAP - current_stat;
        let after = raw_gain - before;
        before + after / 2
    }

    pub fn performance_token_total(
        facility: TrainingFacility,
        facility_level: i32,
        deck_size: i32,
        scenario_links: i32,
    ) -> i32 {
        let s = if facility == TrainingFacility::Wit {
            5
        } else {
            9
        };
        let f = facility_level.clamp(1, 5);
        let c = deck_size.clamp(0, 5);
        let l = scenario_links.max(0);
        ((s + f) as f64 * 1.15_f64.powi(c) + (2 * l) as f64).floor() as i32
    }

    pub fn split_token_total(total: i32, facility: TrainingFacility) -> HashMap<String, i32> {
        if total <= 0 {
            return HashMap::new();
        }
        let split = Self::facility_split(facility);
        // Largest-remainder method so 60/30/10 of 13 → 8/4/1 (matches calibration),
        // not truncated integer division 7/3/3.
        let mut parts: Vec<(String, i32, f64)> = split
            .iter()
            .map(|(code, pct)| {
                let exact = total as f64 * (*pct as f64) / 100.0;
                let floor = exact.floor() as i32;
                (code.clone(), floor, exact - floor as f64)
            })
            .collect();
        let assigned: i32 = parts.iter().map(|(_, f, _)| *f).sum();
        let mut rem = total - assigned;
        // Distribute leftover to largest fractional parts (stable order on ties).
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by(|&a, &b| {
            parts[b]
                .2
                .partial_cmp(&parts[a].2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(&b))
        });
        for idx in order {
            if rem <= 0 {
                break;
            }
            parts[idx].1 += 1;
            rem -= 1;
        }
        parts
            .into_iter()
            .filter(|(_, amt, _)| *amt > 0)
            .map(|(code, amt, _)| (code, amt))
            .collect()
    }

    pub fn training_token_gain(
        facility: TrainingFacility,
        facility_level: i32,
        deck_size: i32,
        scenario_links: i32,
        resources: Option<&ScenarioResources>,
    ) -> HashMap<String, i32> {
        let calibration_lookup = CONFIG.lock().unwrap().calibration_lookup;
        if let Some(lookup) = calibration_lookup {
            if let Some(calibrated) = lookup(facility, facility_level, deck_size, scenario_links) {
                let base: HashMap<String, i32> =
                    calibrated.into_iter().filter(|(_, v)| *v > 0).collect();
                return if let Some(res) = resources {
                    Self::apply_blocked_performance_types(
                        &Self::apply_friendship_training_bias(&base, res, facility),
                        res,
                    )
                } else {
                    base
                };
            }
        }
        let total =
            Self::performance_token_total(facility, facility_level, deck_size, scenario_links);
        let split = Self::split_token_total(total, facility);
        if let Some(res) = resources {
            Self::apply_blocked_performance_types(
                &Self::apply_friendship_training_bias(&split, res, facility),
                res,
            )
        } else {
            split
        }
    }

    pub fn least_owned_perf_code(resources: &ScenarioResources) -> String {
        PERF_CODES
            .iter()
            .min_by_key(|code| resources.get(&Self::perf_resource_key(code)))
            .map(|s| s.to_string())
            .unwrap_or_else(|| PERF_CODES[0].to_string())
    }

    pub fn apply_friendship_training_bias(
        gains: &HashMap<String, i32>,
        resources: &ScenarioResources,
        facility: TrainingFacility,
    ) -> HashMap<String, i32> {
        if resources.get("bonus_friendship_pct") <= 0 || gains.is_empty() {
            return gains.clone();
        }
        let split = Self::facility_split(facility);
        if split.len() < 2 {
            return gains.clone();
        }
        let secondary_code = &split[1].0;
        let secondary_amount = gains.get(secondary_code).copied().unwrap_or(0);
        if secondary_amount <= 0 {
            return gains.clone();
        }
        let least = Self::least_owned_perf_code(resources);
        if least == *secondary_code {
            return gains.clone();
        }
        let mut adjusted = gains.clone();
        let cur = adjusted.get(secondary_code).copied().unwrap_or(0) - secondary_amount;
        if cur == 0 {
            adjusted.remove(secondary_code);
        } else {
            adjusted.insert(secondary_code.clone(), cur);
        }
        *adjusted.entry(least).or_insert(0) += secondary_amount;
        adjusted.retain(|_, v| *v > 0);
        adjusted
    }

    pub fn roll_light_hello(
        resources: &ScenarioResources,
        state: &CareerState,
        facility: TrainingFacility,
        rng: &mut SimRandom,
    ) -> (HashMap<String, i32>, Vec<String>) {
        if !GrandLiveDeckSupport::has_light_hello(state, facility) {
            return (HashMap::new(), Vec::new());
        }
        if !rng.next_boolean(LIGHT_HELLO_CHANCE) {
            return (HashMap::new(), Vec::new());
        }
        let code = Self::least_owned_perf_code(resources);
        (
            HashMap::from([(code.clone(), LIGHT_HELLO_GRANT)]),
            vec![format!("Light Hello: {code} +{LIGHT_HELLO_GRANT}")],
        )
    }

    pub fn resolve_training_perf_gains(
        state: &CareerState,
        facility: TrainingFacility,
        rng: &mut SimRandom,
    ) -> (HashMap<String, i32>, Vec<String>) {
        let level = state
            .facility_levels
            .get(facility.key())
            .copied()
            .unwrap_or(1);
        let deck_size = state.deck.count_on_facility(facility);
        let scenario_links = GrandLiveDeckSupport::scenario_link_count(state, facility);
        let res_before = &state.scenario_resources;
        let mut gains =
            Self::training_token_gain(facility, level, deck_size, scenario_links, Some(res_before));
        let mut lines = Vec::new();
        let (light_gains, light_lines) = Self::roll_light_hello(res_before, state, facility, rng);
        for (code, amt) in light_gains {
            *gains.entry(code).or_insert(0) += amt;
        }
        lines.extend(light_lines);
        gains.retain(|_, v| *v > 0);
        (gains, lines)
    }

    pub fn training_stat_multiplier(resources: &ScenarioResources) -> f64 {
        let friendship = resources.get("bonus_friendship_pct");
        1.0 + friendship as f64 / 100.0
    }

    /// Flat +N added to training main/secondary gains for a facility from song mastery.
    pub fn mastery_train_stat_bonus(
        resources: &ScenarioResources,
        facility: TrainingFacility,
    ) -> i32 {
        resources.get(&format!("mastery_train_{}", facility.key()))
    }

    /// Flat +N skill points on every successful train from song mastery (Extra Skill Pt Gain).
    pub fn mastery_train_sp_bonus(resources: &ScenarioResources) -> i32 {
        resources.get("mastery_train_sp")
    }

    pub fn apply_song_mastery(
        resources: &ScenarioResources,
        mastery: &crate::scenario::grand_live_catalog::GrandLiveMasteryBonus,
        stats: &crate::state::TraineeStats,
    ) -> (ScenarioResources, crate::state::TraineeStats, Vec<String>) {
        use crate::scenario::grand_live_catalog::GrandLiveMasteryBonus;
        let mut res = resources.clone();
        let mut stats = stats.clone();
        let mut lines = Vec::new();
        match mastery {
            GrandLiveMasteryBonus::None => {}
            GrandLiveMasteryBonus::FlatStat { facility, value } => {
                if let Some(fac) = TrainingFacility::ALL
                    .iter()
                    .find(|f| f.key() == facility.as_str())
                {
                    stats = stats.with_delta(*fac, *value);
                    lines.push(format!("Mastery: {} +{value}", fac.name()));
                }
            }
            GrandLiveMasteryBonus::FlatSkillPoints { value } => {
                // Applied by caller onto CareerState.skill_points.
                res = res.add("pending_mastery_sp", *value);
                lines.push(format!("Mastery: Skill points +{value}"));
            }
            GrandLiveMasteryBonus::TrainStat { facility, value } => {
                res = res.add(&format!("mastery_train_{facility}"), *value);
                lines.push(format!("Mastery: {facility} training +{value}"));
            }
            GrandLiveMasteryBonus::TrainSkillPoints { value } => {
                res = res.add("mastery_train_sp", *value);
                lines.push(format!("Mastery: training SP +{value}"));
            }
            GrandLiveMasteryBonus::AllPerfTokens { value } => {
                let grant: HashMap<String, i32> =
                    PERF_CODES.iter().map(|c| (c.to_string(), *value)).collect();
                res = Self::add_perf_tokens(&res, &grant);
                lines.push(format!("Mastery: all performance +{value}"));
            }
        }
        (res, stats, lines)
    }

    pub fn owns_song(resources: &ScenarioResources, song_list_id: i32) -> bool {
        resources.get(&format!("song_owned:{song_list_id}")) > 0
    }

    pub fn mark_song_owned(
        resources: &ScenarioResources,
        song_list_id: i32,
        increment_cycle: bool,
    ) -> ScenarioResources {
        if Self::owns_song(resources, song_list_id) {
            return resources.clone();
        }
        let mut res = resources
            .add(&format!("song_owned:{song_list_id}"), 1)
            .add("songs_learned", 1);
        if increment_cycle && Self::can_add_cycle_song(&res) {
            res = res
                .add("hype", 1)
                .set(&format!("cycle_song:{song_list_id}"), 1);
        }
        res.set("techniques_since_last_song", 0)
            .add("song_slot_index", 1)
    }

    pub fn record_technique_purchase(resources: &ScenarioResources) -> ScenarioResources {
        resources
            .add("techniques_learned", 1)
            .add("techniques_since_last_song", 1)
            .add("cycle_techniques", 1)
    }

    pub fn add_perf_tokens(
        resources: &ScenarioResources,
        gains: &HashMap<String, i32>,
    ) -> ScenarioResources {
        let cap = Self::perf_max(resources);
        let mut res = resources.clone();
        for (code, amt) in gains {
            if *amt <= 0 {
                continue;
            }
            let key = Self::perf_resource_key(code);
            res = res.set(&key, (res.get(&key) + amt).min(cap));
        }
        res
    }

    pub fn pay_perf_tokens(
        resources: &ScenarioResources,
        costs: &HashMap<String, i32>,
    ) -> ScenarioResources {
        let mut res = resources.clone();
        for (perf, amt) in costs {
            let code = Self::perf_code(perf);
            res = res.add(&Self::perf_resource_key(&code), -amt);
        }
        res
    }

    pub fn concert_stat_bonus(great_success: bool) -> i32 {
        let cfg = CONFIG.lock().unwrap();
        if great_success {
            cfg.great_success_stat_bonus
        } else {
            cfg.normal_concert_stat_bonus
        }
    }

    pub fn sp_between_concerts(cycle_techniques: i32, cycle_songs: i32) -> i32 {
        let cfg = CONFIG.lock().unwrap();
        cycle_techniques * cfg.sp_per_technique_since_last
            + cycle_songs * cfg.sp_per_song_since_last
    }

    pub fn reset_cycle_after_concert(resources: &ScenarioResources) -> ScenarioResources {
        let mut res = resources.set("hype", 0);
        for key in resources
            .values
            .keys()
            .filter(|k| k.starts_with("cycle_song:"))
        {
            res = res.set(key, 0);
        }
        let completed = res.get("concert_index") + 1;
        res = res
            .set("concert_index", completed)
            .set("song_slot_index", 0)
            .set("techniques_since_last_song", 0)
            .set("cycle_techniques", 0);
        if let Some(next_race) = Self::next_concert_race_id(completed) {
            res = res.set(
                "great_success_required",
                Self::great_success_required_for_race(next_race),
            );
        }
        res
    }

    pub fn raise_perf_cap_after_concert(resources: &ScenarioResources) -> ScenarioResources {
        resources.add(
            "perf_cap_bonus",
            CONFIG.lock().unwrap().perf_cap_raise_per_concert,
        )
    }

    pub fn make_debut_perf_grant() -> HashMap<String, i32> {
        let grant = CONFIG.lock().unwrap().make_debut_token_grant;
        PERF_CODES.iter().map(|c| (c.to_string(), grant)).collect()
    }

    pub fn activate_cycle_song_bonuses(
        resources: &ScenarioResources,
        song: &crate::scenario::grand_live_catalog::GrandLiveSong,
        _great_success: bool,
    ) -> ScenarioResources {
        let res = resources
            .set(&format!("activated_song:{}", song.song_list_id), 1)
            .add("activated_songs", 1);
        let Some(bonus) = &song.concert_bonus else {
            return res;
        };
        let key = if bonus.effect.to_lowercase().contains("friendship") {
            "bonus_friendship_pct"
        } else if bonus.effect.to_lowercase().contains("specialty") {
            "bonus_specialty_pct"
        } else if bonus.effect.to_lowercase().contains("support chain") {
            "bonus_support_chain_pct"
        } else {
            "bonus_training_misc"
        };
        let live_type = Self::live_bonus_type_for_effect(&bonus.effect);
        res.add(key, bonus.value)
            .set(&format!("live_bonus_type:{live_type}"), bonus.value)
    }

    pub fn cycle_song_ids(resources: &ScenarioResources) -> Vec<i32> {
        resources
            .values
            .keys()
            .filter(|k| k.starts_with("cycle_song:") && resources.get(k) > 0)
            .filter_map(|k| k.trim_start_matches("cycle_song:").parse().ok())
            .collect()
    }

    pub fn bump_lesson_refresh(resources: &ScenarioResources) -> ScenarioResources {
        resources.add("lesson_refresh", 1)
    }

    pub fn clear_frozen_board(resources: &ScenarioResources) -> ScenarioResources {
        let mut res = resources.set("board_frozen", 0);
        let keys: Vec<String> = resources
            .values
            .keys()
            .filter(|k| k.starts_with("board_freeze:"))
            .cloned()
            .collect();
        for key in keys {
            res = res.set(&key, 0);
        }
        res
    }

    pub fn freeze_lesson_board(
        resources: &ScenarioResources,
        action_ids: &[String],
    ) -> ScenarioResources {
        let mut res = Self::clear_frozen_board(resources).set("board_frozen", 1);
        for id in action_ids.iter().take(3) {
            res = res.set(&format!("board_freeze:{id}"), 1);
        }
        res
    }

    pub fn board_is_frozen(resources: &ScenarioResources) -> bool {
        resources.get("board_frozen") > 0
    }

    pub fn frozen_board_action_ids(resources: &ScenarioResources) -> Vec<String> {
        resources
            .values
            .keys()
            .filter(|k| k.starts_with("board_freeze:") && resources.get(k) > 0)
            .map(|k| k.trim_start_matches("board_freeze:").to_string())
            .collect()
    }

    /// Zero token gains for performance types marked blocked (`blocked_perf_Da`, …).
    pub fn apply_blocked_performance_types(
        gains: &HashMap<String, i32>,
        resources: &ScenarioResources,
    ) -> HashMap<String, i32> {
        gains
            .iter()
            .filter_map(|(code, amt)| {
                let key = format!("blocked_perf_{}", Self::perf_code(code));
                if resources.get(&key) > 0 {
                    None
                } else {
                    Some((code.clone(), *amt))
                }
            })
            .collect()
    }

    pub fn set_blocked_performance(
        resources: &ScenarioResources,
        code: &str,
        blocked: bool,
    ) -> ScenarioResources {
        resources.set(
            &format!("blocked_perf_{}", Self::perf_code(code)),
            if blocked { 1 } else { 0 },
        )
    }

    /// Concert live outcome assuming the race was won (Great Success vs Normal).
    pub fn concert_outcome(race_id: &str, resources: &ScenarioResources) -> ConcertOutcome {
        Self::concert_outcome_with_race(race_id, resources, true)
    }

    /// Packet `live_results.result_state` maps to Great Success / Normal from cycle songs.
    /// Race loss (`won=false`) or `force_concert_fail` resource → Failure.
    pub fn concert_outcome_with_race(
        race_id: &str,
        resources: &ScenarioResources,
        won: bool,
    ) -> ConcertOutcome {
        if race_id == "debut" {
            return ConcertOutcome::Normal;
        }
        if !won || resources.get("force_concert_fail") > 0 {
            return ConcertOutcome::Failure;
        }
        let required = Self::great_success_required_for_race(race_id);
        let cycle = Self::cycle_songs(resources);
        if cycle >= required {
            ConcertOutcome::GreatSuccess
        } else {
            ConcertOutcome::Normal
        }
    }

    pub fn average_perf_tokens(resources: &ScenarioResources) -> i32 {
        let sum: i32 = PERF_CODES
            .iter()
            .map(|c| resources.get(&Self::perf_resource_key(c)))
            .sum();
        sum / PERF_CODES.len() as i32
    }

    pub fn concert_stat_bonus_for_outcome(outcome: ConcertOutcome) -> i32 {
        match outcome {
            ConcertOutcome::GreatSuccess => Self::concert_stat_bonus(true),
            ConcertOutcome::Normal => Self::concert_stat_bonus(false),
            ConcertOutcome::Failure => Self::concert_stat_bonus(false) / 2,
        }
    }

    /// Fan-scaled unique skill multiplier in basis points (1000 = 1.0×).
    ///
    /// GameTora `MultiplyFanCount` / `value_scale` 12 (skills 210071 / 210072):
    /// `[0,20k)→0.8`, `[20k,50k)→0.9`, `[50k,100k)→1.0`, `[100k,160k)→1.1`, `[160k,∞)→1.2`.
    pub fn unique_skill_power(fans: i32) -> i32 {
        let fans = fans.max(0);
        if fans < 20_000 {
            800
        } else if fans < 50_000 {
            900
        } else if fans < 100_000 {
            1000
        } else if fans < 160_000 {
            1100
        } else {
            1200
        }
    }

    /// Scaled velocity for a finale unique (`base` from skill.json effect type 27).
    pub fn unique_skill_velocity(base_velocity: i32, fans: i32) -> i32 {
        (base_velocity as i64 * Self::unique_skill_power(fans) as i64 / 1000) as i32
    }

    pub fn member_ready_count(state: &CareerState) -> i32 {
        state.deck.slots.len() as i32
    }

    pub fn reserve_square_id(resources: &ScenarioResources) -> Option<String> {
        let raw = resources.get("reserve_square_id");
        if raw <= 0 {
            None
        } else {
            Some(raw.to_string())
        }
    }

    pub fn set_reserve_square_id(
        resources: &ScenarioResources,
        lesson_key: i32,
    ) -> ScenarioResources {
        resources.set("reserve_square_id", lesson_key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcertOutcome {
    GreatSuccess,
    Normal,
    Failure,
}

impl ConcertOutcome {
    pub fn as_resource_value(self) -> i32 {
        match self {
            Self::GreatSuccess => 2,
            Self::Normal => 1,
            Self::Failure => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GreatSuccess => "great_success",
            Self::Normal => "normal",
            Self::Failure => "fail",
        }
    }
}
