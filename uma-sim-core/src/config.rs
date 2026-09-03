use crate::rng::SimRandom;
use crate::scenario::{
    grand_live::GrandLiveMechanics, trackblazer::TrackblazerMechanics, unity::UnityCupMechanics,
    ura::UraMechanics,
};
use crate::state::{CareerState, DeckState, MoodLevel, TrainingFacility};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct WeightedOutcome {
    pub delta: i32,
    pub weight: f64,
}

// --- TrainingFailureConfig ---

#[derive(Clone)]
struct TrainingFailureState {
    energy_bands: Vec<(i32, i32)>,
    mood_modifiers: HashMap<MoodLevel, i32>,
    facility_level_modifier_per_level: i32,
    failure_energy_loss: i32,
    mood_drop_chance: f64,
    injury_chance: f64,
}

impl Default for TrainingFailureState {
    fn default() -> Self {
        Self {
            energy_bands: vec![(30, 45), (50, 30), (70, 15), (90, 5), (100, 0)],
            mood_modifiers: HashMap::from([
                (MoodLevel::Great, -5),
                (MoodLevel::Good, -2),
                (MoodLevel::Normal, 0),
                (MoodLevel::Bad, 5),
                (MoodLevel::Awful, 10),
            ]),
            facility_level_modifier_per_level: 2,
            failure_energy_loss: 15,
            mood_drop_chance: 0.3,
            injury_chance: 0.02,
        }
    }
}

static TRAINING_FAILURE: LazyLock<Mutex<TrainingFailureState>> =
    LazyLock::new(|| Mutex::new(TrainingFailureState::default()));

pub struct TrainingFailureConfig;

impl TrainingFailureConfig {
    pub fn reset_to_defaults() {
        *TRAINING_FAILURE.lock().unwrap() = TrainingFailureState::default();
    }

    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            Self::reset_to_defaults();
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = TRAINING_FAILURE.lock().unwrap();
        if let Some(arr) = root.get("base_failure_by_energy_pct").and_then(|v| v.as_array()) {
            let parsed: Vec<(i32, i32)> = arr
                .iter()
                .filter_map(|el| {
                    let obj = el.as_object()?;
                    let pct = obj.get("energy_max_pct")?.as_i64()? as i32;
                    let fail = obj.get("failure_pct")?.as_i64()? as i32;
                    Some((pct, fail))
                })
                .collect();
            if !parsed.is_empty() {
                let mut sorted = parsed;
                sorted.sort_by_key(|(p, _)| *p);
                cfg.energy_bands = sorted;
            }
        }
        if let Some(mood) = root.get("mood_modifiers").and_then(|v| v.as_object()) {
            let parsed: HashMap<MoodLevel, i32> = mood
                .iter()
                .filter_map(|(name, el)| {
                    let level = parse_mood_level(name)?;
                    let modv = el.as_i64()? as i32;
                    Some((level, modv))
                })
                .collect();
            if !parsed.is_empty() {
                cfg.mood_modifiers = parsed;
            }
        }
        if let Some(v) = root
            .get("facility_level_modifier_per_level")
            .and_then(|v| v.as_i64())
        {
            cfg.facility_level_modifier_per_level = v as i32;
        }
        if let Some(penalty) = root.get("failure_penalty").and_then(|v| v.as_object()) {
            if let Some(v) = penalty.get("energy_loss").and_then(|v| v.as_i64()) {
                cfg.failure_energy_loss = v as i32;
            }
            if let Some(v) = penalty.get("mood_drop_chance").and_then(|v| v.as_f64()) {
                cfg.mood_drop_chance = v;
            }
            if let Some(v) = penalty.get("injury_chance").and_then(|v| v.as_f64()) {
                cfg.injury_chance = v;
            }
        }
    }

    pub fn failure_chance_pct(
        energy_after: i32,
        max_energy: i32,
        mood: MoodLevel,
        facility_level: i32,
    ) -> i32 {
        let cfg = TRAINING_FAILURE.lock().unwrap();
        let pct = if max_energy <= 0 {
            0
        } else {
            energy_after * 100 / max_energy
        };
        let base = cfg
            .energy_bands
            .iter()
            .find(|(bound, _)| pct < *bound)
            .map(|(_, fail)| *fail)
            .unwrap_or(0);
        let mood_adj = cfg.mood_modifiers.get(&mood).copied().unwrap_or(0);
        let level_adj = (facility_level - 1).max(0) * cfg.facility_level_modifier_per_level;
        (base + mood_adj + level_adj).clamp(0, 100)
    }

    pub fn resolve_failure(
        energy_before: i32,
        training_energy_cost: i32,
        max_energy: i32,
        rng: &mut SimRandom,
    ) -> FailureOutcome {
        let cfg = TRAINING_FAILURE.lock().unwrap();
        let energy = (energy_before - training_energy_cost - cfg.failure_energy_loss).max(0);
        FailureOutcome {
            energy: energy.min(max_energy),
            mood_dropped: rng.next_boolean(cfg.mood_drop_chance),
            injured: rng.next_boolean(cfg.injury_chance),
        }
    }
}

pub struct FailureOutcome {
    pub energy: i32,
    pub mood_dropped: bool,
    pub injured: bool,
}

// --- EventProbabilityConfig ---

#[derive(Clone)]
struct EventProbabilityState {
    random_event_chance_per_turn: f64,
    support_chain_chance_per_turn: f64,
    energy_variance_pattern: String,
    energy_variance_outcomes: Vec<WeightedOutcome>,
    inspiration_chance_per_turn: f64,
}

impl Default for EventProbabilityState {
    fn default() -> Self {
        Self {
            random_event_chance_per_turn: 0.10,
            support_chain_chance_per_turn: 0.35,
            energy_variance_pattern: "Energy -5/-20".into(),
            energy_variance_outcomes: vec![
                WeightedOutcome { delta: -5, weight: 0.5 },
                WeightedOutcome { delta: -20, weight: 0.5 },
            ],
            inspiration_chance_per_turn: 0.02,
        }
    }
}

static EVENT_PROBABILITY: LazyLock<Mutex<EventProbabilityState>> =
    LazyLock::new(|| Mutex::new(EventProbabilityState::default()));

pub struct EventProbabilityConfig;

impl EventProbabilityConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = EVENT_PROBABILITY.lock().unwrap();
        if let Some(v) = root
            .get("support_chain")
            .and_then(|v| v.get("base_trigger_chance_per_turn"))
            .and_then(|v| v.as_f64())
        {
            cfg.support_chain_chance_per_turn = v;
        }
        if let Some(ev) = root.get("energy_variance_options").and_then(|v| v.as_object()) {
            if let Some(p) = ev.get("pattern").and_then(|v| v.as_str()) {
                cfg.energy_variance_pattern = p.to_string();
            }
            if let Some(arr) = ev.get("outcomes").and_then(|v| v.as_array()) {
                let parsed: Vec<WeightedOutcome> = arr
                    .iter()
                    .filter_map(|el| {
                        let obj = el.as_object()?;
                        Some(WeightedOutcome {
                            delta: obj.get("delta")?.as_i64()? as i32,
                            weight: obj.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.5),
                        })
                    })
                    .collect();
                if !parsed.is_empty() {
                    cfg.energy_variance_outcomes = parsed;
                }
            }
        }
        if let Some(v) = root
            .get("inspiration")
            .and_then(|v| v.get("ura_chance_per_turn"))
            .and_then(|v| v.as_f64())
        {
            cfg.inspiration_chance_per_turn = v;
        }
    }

    pub fn inspiration_chance_per_turn() -> f64 {
        EVENT_PROBABILITY.lock().unwrap().inspiration_chance_per_turn
    }

    pub fn event_chance_for(state: &CareerState) -> f64 {
        let cfg = EVENT_PROBABILITY.lock().unwrap();
        if state.deck.slots.is_empty() {
            cfg.random_event_chance_per_turn.clamp(0.0, 1.0)
        } else {
            cfg.support_chain_chance_per_turn.clamp(0.0, 1.0)
        }
    }

    pub fn matches_energy_variance(text: &str) -> bool {
        let cfg = EVENT_PROBABILITY.lock().unwrap();
        let lower = text.to_lowercase();
        lower.contains(&cfg.energy_variance_pattern.to_lowercase())
            || lower.contains("energy -5/-20")
    }

    pub fn pick_energy_variance(rng: &mut SimRandom) -> i32 {
        let outcomes = EVENT_PROBABILITY.lock().unwrap().energy_variance_outcomes.clone();
        let total: f64 = outcomes.iter().map(|o| o.weight).sum();
        if total <= 0.0 {
            return -5;
        }
        let mut roll = rng.next_double() * total;
        for outcome in &outcomes {
            roll -= outcome.weight;
            if roll <= 0.0 {
                return outcome.delta;
            }
        }
        outcomes.last().map(|o| o.delta).unwrap_or(-5)
    }
}

// --- RaceOutcomeConfig ---

/// Career stub placements (Phase 5 Outcomes v1). Career auto-play uses [`RacePlacement::First`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RacePlacement {
    First,
    Place25,
    Show,
}

#[derive(Clone)]
struct RaceOutcomeState {
    win_skill_points: i32,
    optional_skill_points: i32,
    win_fans_multiplier: f64,
    place_fans_multiplier: f64,
    show_fans_multiplier: f64,
    place_skill_points: i32,
    show_skill_points: i32,
    grade_modifiers: HashMap<String, f64>,
}

impl Default for RaceOutcomeState {
    fn default() -> Self {
        Self {
            win_skill_points: 45,
            optional_skill_points: 30,
            win_fans_multiplier: 1.0,
            place_fans_multiplier: 0.6,
            show_fans_multiplier: 0.3,
            place_skill_points: 35,
            show_skill_points: 20,
            grade_modifiers: HashMap::from([
                ("G1".into(), 2.0),
                ("G2".into(), 1.5),
                ("G3".into(), 1.2),
                ("OP".into(), 1.0),
                ("PRE_OP".into(), 0.8),
            ]),
        }
    }
}

static RACE_OUTCOME: LazyLock<Mutex<RaceOutcomeState>> =
    LazyLock::new(|| Mutex::new(RaceOutcomeState::default()));

pub struct RaceOutcomeConfig;

impl RaceOutcomeConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = RACE_OUTCOME.lock().unwrap();
        if let Some(v) = root
            .get("win")
            .and_then(|v| v.get("skill_points_base"))
            .and_then(|v| v.as_i64())
        {
            cfg.win_skill_points = v as i32;
        }
        if let Some(v) = root
            .get("win")
            .and_then(|v| v.get("fans_multiplier"))
            .and_then(|v| v.as_f64())
        {
            cfg.win_fans_multiplier = v;
        }
        if let Some(place) = root.get("place_2_5") {
            if let Some(v) = place.get("fans_multiplier").and_then(|v| v.as_f64()) {
                cfg.place_fans_multiplier = v;
            }
            if let Some(v) = place.get("skill_points_base").and_then(|v| v.as_i64()) {
                cfg.place_skill_points = v as i32;
            }
        }
        if let Some(show) = root.get("show") {
            if let Some(v) = show.get("fans_multiplier").and_then(|v| v.as_f64()) {
                cfg.show_fans_multiplier = v;
            }
            if let Some(v) = show.get("skill_points_base").and_then(|v| v.as_i64()) {
                cfg.show_skill_points = v as i32;
            }
        }
        if let Some(grades) = root.get("grade_modifiers").and_then(|v| v.as_object()) {
            let parsed: HashMap<String, f64> = grades
                .iter()
                .filter_map(|(k, v)| v.as_f64().map(|m| (k.to_uppercase(), m)))
                .collect();
            if !parsed.is_empty() {
                cfg.grade_modifiers = parsed;
            }
        }
    }

    pub fn fans_multiplier(placement: RacePlacement) -> f64 {
        let cfg = RACE_OUTCOME.lock().unwrap();
        match placement {
            RacePlacement::First => cfg.win_fans_multiplier,
            RacePlacement::Place25 => cfg.place_fans_multiplier,
            RacePlacement::Show => cfg.show_fans_multiplier,
        }
    }

    pub fn skill_points_for(mandatory: bool, placement: RacePlacement) -> i32 {
        let cfg = RACE_OUTCOME.lock().unwrap();
        match placement {
            RacePlacement::First => {
                if mandatory {
                    cfg.win_skill_points
                } else {
                    cfg.optional_skill_points
                }
            }
            RacePlacement::Place25 => cfg.place_skill_points,
            RacePlacement::Show => cfg.show_skill_points,
        }
    }

    pub fn fan_gain(mandatory: bool, race_id: &str, rng: &mut SimRandom) -> i32 {
        Self::fan_gain_placed(mandatory, race_id, RacePlacement::First, rng)
    }

    pub fn fan_gain_placed(
        mandatory: bool,
        race_id: &str,
        placement: RacePlacement,
        rng: &mut SimRandom,
    ) -> i32 {
        let cfg = RACE_OUTCOME.lock().unwrap();
        let base = if mandatory {
            800 + rng.next_int_until(300)
        } else {
            400 + rng.next_int_until(200)
        };
        let grade_key = cfg
            .grade_modifiers
            .keys()
            .find(|k| race_id.to_uppercase().contains(k.as_str()))
            .cloned()
            .unwrap_or_else(|| "OP".into());
        let grade_mult = cfg.grade_modifiers.get(&grade_key).copied().unwrap_or(1.0);
        let place_mult = match placement {
            RacePlacement::First => cfg.win_fans_multiplier,
            RacePlacement::Place25 => cfg.place_fans_multiplier,
            RacePlacement::Show => cfg.show_fans_multiplier,
        };
        (base as f64 * place_mult * grade_mult) as i32
    }

    pub fn skill_points(mandatory: bool) -> i32 {
        Self::skill_points_for(mandatory, RacePlacement::First)
    }

    /// Stub epithet ids granted on win (Phase 5 Outcomes v1). Full nickname catalog deferred.
    pub fn epithet_for_win(race_id: &str) -> Option<&'static str> {
        let upper = race_id.to_uppercase();
        if upper.contains("G1") {
            Some("epithet:sim_g1_win")
        } else if race_id.contains("climax") {
            Some("epithet:sim_climax_win")
        } else if race_id.contains("finale") {
            Some("epithet:sim_ura_finale_win")
        } else {
            None
        }
    }

    pub fn grant_epithet(statuses: &mut Vec<String>, race_id: &str) -> Option<&'static str> {
        let Some(id) = Self::epithet_for_win(race_id) else {
            return None;
        };
        if statuses.iter().any(|s| s == id) {
            return None;
        }
        statuses.push(id.to_string());
        Some(id)
    }
}

// --- HintProgressionConfig ---

#[derive(Clone)]
struct HintProgressionState {
    max_hint_level: i32,
    gain_per_training: i32,
    gain_per_event: i32,
    training_hint_chance: f64,
}

impl Default for HintProgressionState {
    fn default() -> Self {
        Self {
            max_hint_level: 5,
            gain_per_training: 1,
            gain_per_event: 1,
            training_hint_chance: 0.15,
        }
    }
}

static HINT_PROGRESSION: LazyLock<Mutex<HintProgressionState>> =
    LazyLock::new(|| Mutex::new(HintProgressionState::default()));

pub struct HintProgressionConfig;

impl HintProgressionConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = HINT_PROGRESSION.lock().unwrap();
        if let Some(v) = root.get("max_hint_level").and_then(|v| v.as_i64()) {
            cfg.max_hint_level = v as i32;
        }
        if let Some(v) = root.get("gain_per_hint_training").and_then(|v| v.as_i64()) {
            cfg.gain_per_training = v as i32;
        }
        if let Some(v) = root.get("gain_per_event").and_then(|v| v.as_i64()) {
            cfg.gain_per_event = v as i32;
        }
    }

    pub fn training_hint_chance() -> f64 {
        HINT_PROGRESSION.lock().unwrap().training_hint_chance
    }

    pub fn apply_training_hint(current: i32) -> i32 {
        let cfg = HINT_PROGRESSION.lock().unwrap();
        (current + cfg.gain_per_training).min(cfg.max_hint_level)
    }

    pub fn apply_event_hint(current: i32) -> i32 {
        let cfg = HINT_PROGRESSION.lock().unwrap();
        (current + cfg.gain_per_event).min(cfg.max_hint_level)
    }
}

// --- BondGainConfig ---

#[derive(Clone)]
struct BondGainState {
    regular_training: i32,
    hint_training: i32,
    friendship_threshold: i32,
    max_bond: i32,
}

impl Default for BondGainState {
    fn default() -> Self {
        Self {
            regular_training: 7,
            hint_training: 5,
            friendship_threshold: 80,
            max_bond: 100,
        }
    }
}

static BOND_GAIN: LazyLock<Mutex<BondGainState>> =
    LazyLock::new(|| Mutex::new(BondGainState::default()));

pub struct BondGainConfig;

impl BondGainConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = BOND_GAIN.lock().unwrap();
        if let Some(v) = root.get("regular_training").and_then(|v| v.as_i64()) {
            cfg.regular_training = v as i32;
        }
        if let Some(v) = root.get("hint_training").and_then(|v| v.as_i64()) {
            cfg.hint_training = v as i32;
        }
        if let Some(v) = root
            .get("friendship_training_threshold")
            .and_then(|v| v.as_i64())
        {
            cfg.friendship_threshold = v as i32;
        }
        if let Some(v) = root.get("max_bond").and_then(|v| v.as_i64()) {
            cfg.max_bond = v as i32;
        }
    }

    pub fn friendship_threshold() -> i32 {
        BOND_GAIN.lock().unwrap().friendship_threshold
    }

    pub fn apply_training_bond(deck: &DeckState, facility: TrainingFacility) -> DeckState {
        let cfg = BOND_GAIN.lock().unwrap();
        let key = facility.key();
        DeckState {
            slots: deck
                .slots
                .iter()
                .map(|slot| {
                    if slot.assigned_facility.as_deref() == Some(key) {
                        let mut s = slot.clone();
                        s.bond = (s.bond + cfg.regular_training).min(cfg.max_bond);
                        s
                    } else {
                        slot.clone()
                    }
                })
                .collect(),
        }
    }
}

// --- FacilityLevelConfig ---

#[derive(Clone)]
struct FacilityLevelState {
    trains_per_level: i32,
    max_level: i32,
    train_count_scenarios: Option<Vec<String>>,
}

impl Default for FacilityLevelState {
    fn default() -> Self {
        Self {
            trains_per_level: 4,
            max_level: 5,
            train_count_scenarios: None,
        }
    }
}

static FACILITY_LEVEL: LazyLock<Mutex<FacilityLevelState>> =
    LazyLock::new(|| Mutex::new(FacilityLevelState::default()));

const UNITY_RANK_SCENARIOS: [&str; 2] = ["unity", "unity_cup"];

pub struct FacilityLevelConfig;

impl FacilityLevelConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = FACILITY_LEVEL.lock().unwrap();
        if let Some(level_up) = root.get("facility_level_up").and_then(|v| v.as_object()) {
            if let Some(v) = level_up.get("trains_per_level").and_then(|v| v.as_i64()) {
                cfg.trains_per_level = (v as i32).max(1);
            }
            if let Some(v) = level_up.get("max_level").and_then(|v| v.as_i64()) {
                cfg.max_level = (v as i32).clamp(1, 5);
            }
        }
        if let Some(mode) = root.get("facility_level_mode").and_then(|v| v.as_object()) {
            if let Some(arr) = mode.get("train_count_scenarios").and_then(|v| v.as_array()) {
                let parsed: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                    .collect();
                if !parsed.is_empty() {
                    cfg.train_count_scenarios = Some(parsed);
                }
            }
        }
    }

    pub fn uses_train_count_leveling(scenario_id: &str) -> bool {
        let id = scenario_id.to_lowercase().replace(' ', "_");
        let cfg = FACILITY_LEVEL.lock().unwrap();
        if let Some(explicit) = &cfg.train_count_scenarios {
            return explicit.contains(&id) || explicit.contains(&id.replace('_', ""));
        }
        !UNITY_RANK_SCENARIOS.contains(&id.as_str())
    }

    pub fn level_for_train_count(train_count: i32) -> i32 {
        let cfg = FACILITY_LEVEL.lock().unwrap();
        (1 + train_count / cfg.trains_per_level).clamp(1, cfg.max_level)
    }

    pub fn apply_successful_train(
        facility: TrainingFacility,
        facility_levels: &HashMap<String, i32>,
        facility_train_counts: &HashMap<String, i32>,
    ) -> (HashMap<String, i32>, HashMap<String, i32>) {
        let key = facility.key().to_string();
        let count = facility_train_counts.get(&key).copied().unwrap_or(0) + 1;
        let mut new_counts = facility_train_counts.clone();
        new_counts.insert(key.clone(), count);
        let new_level = Self::level_for_train_count(count);
        let mut new_levels = facility_levels.clone();
        new_levels.insert(key, new_level);
        (new_levels, new_counts)
    }
}

// --- InspirationConfig ---

#[derive(Clone)]
struct InspirationState {
    stat_bonus_min: i32,
    stat_bonus_max: i32,
}

impl Default for InspirationState {
    fn default() -> Self {
        Self {
            stat_bonus_min: 10,
            stat_bonus_max: 30,
        }
    }
}

static INSPIRATION: LazyLock<Mutex<InspirationState>> =
    LazyLock::new(|| Mutex::new(InspirationState::default()));

pub struct InspirationConfig;

impl InspirationConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = INSPIRATION.lock().unwrap();
        if let Some(range) = root.get("stat_bonus_range").and_then(|v| v.as_object()) {
            if let Some(v) = range.get("min").and_then(|v| v.as_i64()) {
                cfg.stat_bonus_min = v as i32;
            }
            if let Some(v) = range.get("max").and_then(|v| v.as_i64()) {
                cfg.stat_bonus_max = v as i32;
            }
        }
    }

    pub fn stat_bonus_min() -> i32 {
        INSPIRATION.lock().unwrap().stat_bonus_min
    }

    pub fn stat_bonus_max() -> i32 {
        INSPIRATION.lock().unwrap().stat_bonus_max
    }

    pub fn roll_bonus(rng: &mut SimRandom) -> i32 {
        let cfg = INSPIRATION.lock().unwrap();
        let span = (cfg.stat_bonus_max - cfg.stat_bonus_min).max(0);
        if span == 0 {
            cfg.stat_bonus_min
        } else {
            cfg.stat_bonus_min + rng.next_int_until(span + 1)
        }
    }

    pub fn event_options(bonus: i32) -> Vec<String> {
        vec![
            format!("Focus on speed\nSpeed +{bonus}"),
            format!("Focus on stamina\nStamina +{bonus}"),
            format!("Focus on wit\nWit +{bonus}"),
        ]
    }
}

// --- ScenarioResearchConfig ---

#[derive(Clone)]
struct ScenarioResearchState {
    songs_for_best_unique: i32,
    fan_targets: HashMap<String, i32>,
}

impl Default for ScenarioResearchState {
    fn default() -> Self {
        Self {
            songs_for_best_unique: 18,
            fan_targets: HashMap::from([
                ("ura".into(), 3500),
                ("grand_concert".into(), 4500),
                ("unity".into(), 4000),
                ("trackblazer".into(), 5000),
            ]),
        }
    }
}

static SCENARIO_RESEARCH: LazyLock<Mutex<ScenarioResearchState>> =
    LazyLock::new(|| Mutex::new(ScenarioResearchState::default()));

pub struct ScenarioResearchConfig;

impl ScenarioResearchConfig {
    pub fn load_scenario_json(scenario_id: &str, json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let normalized = Self::normalize(scenario_id);
        match normalized.as_str() {
            "ura" => UraMechanics::load_research(Some(text)),
            "grand_concert" => GrandLiveMechanics::load_research(Some(text)),
            "unity" => UnityCupMechanics::load_research(Some(text)),
            "trackblazer" => TrackblazerMechanics::load_research(Some(text)),
            _ => {}
        }
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = SCENARIO_RESEARCH.lock().unwrap();
        if let Some(v) = root.get("songs_for_best_unique").and_then(|v| v.as_i64()) {
            cfg.songs_for_best_unique = v as i32;
        }
        if let Some(v) = root.get("fan_target_optional_race").and_then(|v| v.as_i64()) {
            cfg.fan_targets.insert(normalized, v as i32);
        }
    }

    pub fn fan_target(scenario_id: &str) -> i32 {
        let id = Self::normalize(scenario_id);
        SCENARIO_RESEARCH
            .lock()
            .unwrap()
            .fan_targets
            .get(&id)
            .copied()
            .unwrap_or(3500)
    }

    pub fn songs_for_best_unique() -> i32 {
        SCENARIO_RESEARCH.lock().unwrap().songs_for_best_unique
    }

    fn normalize(id: &str) -> String {
        match id.to_lowercase().replace(' ', "_").as_str() {
            "grand_live" | "gl" => "grand_concert".into(),
            "unity_cup" => "unity".into(),
            "tb" => "trackblazer".into(),
            "ura_finale" => "ura".into(),
            other => other.into(),
        }
    }
}

// --- MoodEnergyConfig ---

#[derive(Clone)]
struct MoodEnergyState {
    rest_gain_typical: i32,
    recreation_gain_typical: i32,
    rest_upgrade_chance: f64,
    recreation_upgrade_chance: f64,
}

impl Default for MoodEnergyState {
    fn default() -> Self {
        Self {
            rest_gain_typical: 50,
            recreation_gain_typical: 20,
            rest_upgrade_chance: 0.4,
            recreation_upgrade_chance: 0.25,
        }
    }
}

static MOOD_ENERGY: LazyLock<Mutex<MoodEnergyState>> =
    LazyLock::new(|| Mutex::new(MoodEnergyState::default()));

pub struct MoodEnergyConfig;

impl MoodEnergyConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = MOOD_ENERGY.lock().unwrap();
        if let Some(energy) = root.get("energy").and_then(|v| v.as_object()) {
            if let Some(rest) = energy.get("rest_gain").and_then(|v| v.as_object()) {
                if let Some(v) = rest.get("typical").and_then(|v| v.as_i64()) {
                    cfg.rest_gain_typical = v as i32;
                }
            }
            if let Some(rec) = energy.get("recreation_gain").and_then(|v| v.as_object()) {
                if let Some(v) = rec.get("typical").and_then(|v| v.as_i64()) {
                    cfg.recreation_gain_typical = v as i32;
                }
            }
        }
        if let Some(mood) = root.get("mood").and_then(|v| v.as_object()) {
            if let Some(v) = mood.get("rest_upgrade_chance").and_then(|v| v.as_f64()) {
                cfg.rest_upgrade_chance = v;
            }
            if let Some(v) = mood.get("recreation_upgrade_chance").and_then(|v| v.as_f64()) {
                cfg.recreation_upgrade_chance = v;
            }
        }
    }

    pub fn rest_energy_gain() -> i32 {
        MOOD_ENERGY.lock().unwrap().rest_gain_typical
    }

    pub fn recreation_energy_gain() -> i32 {
        MOOD_ENERGY.lock().unwrap().recreation_gain_typical
    }

    pub fn rest_mood_upgrade_chance() -> f64 {
        MOOD_ENERGY.lock().unwrap().rest_upgrade_chance
    }

    pub fn recreation_mood_upgrade_chance() -> f64 {
        MOOD_ENERGY.lock().unwrap().recreation_upgrade_chance
    }
}

// --- EventRewardSchemaConfig (research metadata; reserved for parser alignment) ---

pub struct EventRewardSchemaConfig;

impl EventRewardSchemaConfig {
    pub fn load_from_json(json_text: Option<&str>) {
        let _ = json_text.filter(|s| !s.trim().is_empty());
    }
}

fn parse_mood_level(name: &str) -> Option<MoodLevel> {
    match name.to_uppercase().as_str() {
        "GREAT" => Some(MoodLevel::Great),
        "GOOD" => Some(MoodLevel::Good),
        "NORMAL" => Some(MoodLevel::Normal),
        "BAD" => Some(MoodLevel::Bad),
        "AWFUL" => Some(MoodLevel::Awful),
        _ => None,
    }
}
