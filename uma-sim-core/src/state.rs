use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoodLevel {
    Great,
    Good,
    Normal,
    Bad,
    Awful,
}

impl MoodLevel {
    pub const ALL: [MoodLevel; 5] = [
        MoodLevel::Great,
        MoodLevel::Good,
        MoodLevel::Normal,
        MoodLevel::Bad,
        MoodLevel::Awful,
    ];

    pub fn kotlin_name(self) -> &'static str {
        match self {
            MoodLevel::Great => "GREAT",
            MoodLevel::Good => "GOOD",
            MoodLevel::Normal => "NORMAL",
            MoodLevel::Bad => "BAD",
            MoodLevel::Awful => "AWFUL",
        }
    }

    pub fn index(self) -> usize {
        match self {
            MoodLevel::Great => 0,
            MoodLevel::Good => 1,
            MoodLevel::Normal => 2,
            MoodLevel::Bad => 3,
            MoodLevel::Awful => 4,
        }
    }

    pub fn from_index(idx: i32) -> MoodLevel {
        MoodLevel::ALL[idx.clamp(0, 4) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrainingFacility {
    Speed,
    Stamina,
    Power,
    Guts,
    Wit,
}

impl TrainingFacility {
    pub const ALL: [TrainingFacility; 5] = [
        TrainingFacility::Speed,
        TrainingFacility::Stamina,
        TrainingFacility::Power,
        TrainingFacility::Guts,
        TrainingFacility::Wit,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TrainingFacility::Speed => "SPEED",
            TrainingFacility::Stamina => "STAMINA",
            TrainingFacility::Power => "POWER",
            TrainingFacility::Guts => "GUTS",
            TrainingFacility::Wit => "WIT",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            TrainingFacility::Speed => "speed",
            TrainingFacility::Stamina => "stamina",
            TrainingFacility::Power => "power",
            TrainingFacility::Guts => "guts",
            TrainingFacility::Wit => "wit",
        }
    }

    pub fn ordinal(&self) -> i32 {
        match self {
            TrainingFacility::Speed => 0,
            TrainingFacility::Stamina => 1,
            TrainingFacility::Power => 2,
            TrainingFacility::Guts => 3,
            TrainingFacility::Wit => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatName {
    Speed,
    Stamina,
    Power,
    Guts,
    Wit,
}

impl StatName {
    pub const ALL: [StatName; 5] = [
        StatName::Speed,
        StatName::Stamina,
        StatName::Power,
        StatName::Guts,
        StatName::Wit,
    ];
}

impl TrainingFacility {
    pub fn to_stat_name(self) -> StatName {
        match self {
            TrainingFacility::Speed => StatName::Speed,
            TrainingFacility::Stamina => StatName::Stamina,
            TrainingFacility::Power => StatName::Power,
            TrainingFacility::Guts => StatName::Guts,
            TrainingFacility::Wit => StatName::Wit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Free,
    MandatoryRace,
    Event,
    Finale,
    Complete,
}

impl TurnPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            TurnPhase::Free => "FREE",
            TurnPhase::MandatoryRace => "MANDATORY_RACE",
            TurnPhase::Event => "EVENT",
            TurnPhase::Finale => "FINALE",
            TurnPhase::Complete => "COMPLETE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimActionKind {
    Train,
    Rest,
    Recreation,
    Race,
    Advance,
    Choose,
    Lesson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimAction {
    pub kind: SimActionKind,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioResources {
    pub values: HashMap<String, i32>,
}

impl ScenarioResources {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn from_map(values: HashMap<String, i32>) -> Self {
        Self { values }
    }

    pub fn get(&self, key: &str) -> i32 {
        *self.values.get(key).unwrap_or(&0)
    }

    pub fn add(&self, key: &str, delta: i32) -> Self {
        if delta == 0 {
            return self.clone();
        }
        let mut next = self.values.clone();
        *next.entry(key.to_string()).or_insert(0) += delta;
        Self { values: next }
    }

    pub fn set(&self, key: &str, value: i32) -> Self {
        let mut next = self.values.clone();
        next.insert(key.to_string(), value);
        Self { values: next }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyState {
    pub parent_names: Vec<String>,
    pub factor_ids: Vec<String>,
    pub inherited_skill_ids: Vec<String>,
    /// Soft-cap raise from blue ★ (`parent_farming_utility.md`: 4/9/16).
    pub spark_caps: HashMap<String, i32>,
    /// Starting-stat bonuses from blue ★ (GameTora: 5/12/21).
    #[serde(default)]
    pub blue_start_bonuses: HashMap<String, i32>,
    pub pink_factor_ids: Vec<String>,
    /// Distance/surface/style aptitude tags from pink factors (turf, dirt, mile, …).
    #[serde(default)]
    pub pink_aptitude_tags: Vec<String>,
    /// Sum of pink ★ per aptitude tag (for rank-up table).
    #[serde(default)]
    pub pink_star_totals: HashMap<String, i32>,
    /// Effective aptitudes after pink inheritance (letter grades).
    #[serde(default)]
    pub aptitudes: HashMap<String, String>,
    pub race_factor_ids: Vec<String>,
    pub inheritance_complete: bool,
    /// Classic + Senior April mid-run inspiration events completed (0–2).
    #[serde(default)]
    pub inspiration_events_done: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckSlot {
    pub support_id: String,
    pub bond: i32,
    pub specialty: Option<String>,
    pub assigned_facility: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckState {
    pub slots: Vec<DeckSlot>,
}

const MAX_ON_FACILITY: usize = 5;

impl DeckState {
    pub fn slots_on_facility(&self, facility: TrainingFacility) -> Vec<&DeckSlot> {
        let key = facility.key();
        self.slots
            .iter()
            .filter(|s| s.assigned_facility.as_deref() == Some(key))
            .collect()
    }

    pub fn count_on_facility(&self, facility: TrainingFacility) -> i32 {
        self.slots_on_facility(facility).len().min(MAX_ON_FACILITY) as i32
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraineeStats {
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
}

impl TraineeStats {
    pub fn get(&self, facility: TrainingFacility) -> i32 {
        match facility {
            TrainingFacility::Speed => self.speed,
            TrainingFacility::Stamina => self.stamina,
            TrainingFacility::Power => self.power,
            TrainingFacility::Guts => self.guts,
            TrainingFacility::Wit => self.wit,
        }
    }

    pub fn with_delta(&self, facility: TrainingFacility, delta: i32) -> Self {
        let mut s = self.clone();
        match facility {
            TrainingFacility::Speed => s.speed += delta,
            TrainingFacility::Stamina => s.stamina += delta,
            TrainingFacility::Power => s.power += delta,
            TrainingFacility::Guts => s.guts += delta,
            TrainingFacility::Wit => s.wit += delta,
        }
        s
    }

    pub fn add_all(&self, delta: i32) -> Self {
        Self {
            speed: self.speed + delta,
            stamina: self.stamina + delta,
            power: self.power + delta,
            guts: self.guts + delta,
            wit: self.wit + delta,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimDate {
    pub year: i32,
    pub month: i32,
    pub half: i32,
}

/// One spark slot on a parent or grandparent (matches UI `SparkSlot`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparkSlot {
    #[serde(default)]
    pub factor_id: String,
    #[serde(default = "default_spark_stars")]
    pub stars: i32,
}

fn default_spark_stars() -> i32 {
    1
}

/// Per-ancestor sparks: blue / pink / white / green / race.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AncestorSparks {
    #[serde(default)]
    pub uma: String,
    #[serde(default)]
    pub blue: SparkSlot,
    #[serde(default)]
    pub pink: SparkSlot,
    #[serde(default)]
    pub white: SparkSlot,
    #[serde(default)]
    pub green: SparkSlot,
    #[serde(default)]
    pub race: SparkSlot,
}

/// Structured 2 parents × 2 grandparents inheritance tree.
/// When present and non-empty, preferred over flat [`RunMeta::legacy_factors`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTree {
    #[serde(default)]
    pub parent_a: AncestorSparks,
    #[serde(default)]
    pub gp_a1: AncestorSparks,
    #[serde(default)]
    pub gp_a2: AncestorSparks,
    #[serde(default)]
    pub parent_b: AncestorSparks,
    #[serde(default)]
    pub gp_b1: AncestorSparks,
    #[serde(default)]
    pub gp_b2: AncestorSparks,
}

impl LegacyTree {
    pub fn ancestors(&self) -> [&AncestorSparks; 6] {
        [
            &self.parent_a,
            &self.gp_a1,
            &self.gp_a2,
            &self.parent_b,
            &self.gp_b1,
            &self.gp_b2,
        ]
    }

    pub fn flatten_factors(&self) -> Vec<String> {
        let mut out = Vec::new();
        for node in self.ancestors() {
            for slot in [&node.blue, &node.pink, &node.white, &node.green, &node.race] {
                if slot.factor_id.is_empty() {
                    continue;
                }
                let stars = slot.stars.clamp(1, 3);
                out.push(format!("{}@{}", slot.factor_id, stars));
            }
        }
        out
    }

    pub fn parent_names(&self) -> Vec<String> {
        [&self.parent_a.uma, &self.parent_b.uma]
            .into_iter()
            .filter(|n| !n.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    pub fn is_populated(&self) -> bool {
        !self.flatten_factors().is_empty() || !self.parent_names().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMeta {
    pub seed: i64,
    pub scenario_id: String,
    pub trainee_name: String,
    pub objective_profile: String,
    /// Flat `factor:id@stars` list — used when [`Self::legacy_tree`] is absent/empty.
    pub legacy_factors: Vec<String>,
    /// Structured 2×2 ancestor tree (preferred when populated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_tree: Option<LegacyTree>,
    pub parent_names: Vec<String>,
    pub deck_supports: Vec<String>,
    /// Overall lineage compatibility score (◎/〇/△ input). Scales mid-run proc odds.
    #[serde(default)]
    pub compatibility_score: i32,
}

impl RunMeta {
    pub fn new(seed: i64, scenario_id: impl Into<String>, trainee_name: impl Into<String>) -> Self {
        Self {
            seed,
            scenario_id: scenario_id.into(),
            trainee_name: trainee_name.into(),
            objective_profile: "general".to_string(),
            legacy_factors: Vec::new(),
            legacy_tree: None,
            parent_names: Vec::new(),
            deck_supports: Vec::new(),
            compatibility_score: 0,
        }
    }

    /// Resolve factors: structured tree wins when it has sparks; else flat list.
    pub fn effective_legacy_factors(&self) -> Vec<String> {
        if let Some(tree) = &self.legacy_tree {
            let flat = tree.flatten_factors();
            if !flat.is_empty() {
                return flat;
            }
        }
        self.legacy_factors.clone()
    }

    /// Resolve parent names: tree parents when set; else flat `parent_names`.
    pub fn effective_parent_names(&self) -> Vec<String> {
        if let Some(tree) = &self.legacy_tree {
            let names = tree.parent_names();
            if !names.is_empty() {
                return names;
            }
        }
        self.parent_names.clone()
    }
}

pub fn default_facility_levels() -> HashMap<String, i32> {
    HashMap::from([
        ("speed".to_string(), 1),
        ("stamina".to_string(), 1),
        ("power".to_string(), 1),
        ("guts".to_string(), 1),
        ("wit".to_string(), 1),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSpark {
    pub color: String,
    pub factor_id: String,
    pub stars: i32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerState {
    pub meta: RunMeta,
    pub date: SimDate,
    pub turn: i32,
    pub stats: TraineeStats,
    pub energy: i32,
    pub max_energy: i32,
    pub mood: MoodLevel,
    pub fans: i32,
    pub skill_points: i32,
    pub career_complete: bool,
    pub awaiting_choice: bool,
    pub pending_event_title: Option<String>,
    pub pending_race_id: Option<String>,
    pub phase: String,
    pub completed_races: Vec<String>,
    pub facility_levels: HashMap<String, i32>,
    pub facility_train_counts: HashMap<String, i32>,
    pub pending_event_options: Vec<String>,
    pub hint_levels: HashMap<String, i32>,
    pub statuses: Vec<String>,
    pub performance_tokens: HashMap<String, i32>,
    pub scenario_resources: ScenarioResources,
    pub legacy: LegacyState,
    pub learned_skill_ids: Vec<String>,
    pub deck: DeckState,
    pub log: Vec<String>,
    /// End-of-career generated inheritance sparks (blue/red/white/green).
    #[serde(default)]
    pub generated_sparks: Vec<GeneratedSpark>,
}

impl CareerState {
    pub fn with_resources(&self, scenario_resources: ScenarioResources) -> Self {
        let mut s = self.clone();
        s.scenario_resources = scenario_resources;
        s
    }

    pub fn copy_with(
        &self,
        scenario_resources: ScenarioResources,
        stats: Option<TraineeStats>,
        energy: Option<i32>,
        mood: Option<MoodLevel>,
        skill_points: Option<i32>,
        phase: Option<String>,
        awaiting_choice: Option<bool>,
        pending_event_title: Option<Option<String>>,
        pending_race_id: Option<Option<String>>,
        pending_event_options: Option<Vec<String>>,
        facility_levels: Option<HashMap<String, i32>>,
        max_energy: Option<i32>,
        log: Option<Vec<String>>,
    ) -> Self {
        Self {
            meta: self.meta.clone(),
            date: self.date.clone(),
            turn: self.turn,
            stats: stats.unwrap_or_else(|| self.stats.clone()),
            energy: energy.unwrap_or(self.energy),
            max_energy: max_energy.unwrap_or(self.max_energy),
            mood: mood.unwrap_or(self.mood),
            fans: self.fans,
            skill_points: skill_points.unwrap_or(self.skill_points),
            career_complete: self.career_complete,
            awaiting_choice: awaiting_choice.unwrap_or(self.awaiting_choice),
            pending_event_title: pending_event_title
                .unwrap_or_else(|| self.pending_event_title.clone()),
            pending_race_id: pending_race_id.unwrap_or_else(|| self.pending_race_id.clone()),
            phase: phase.unwrap_or_else(|| self.phase.clone()),
            completed_races: self.completed_races.clone(),
            facility_levels: facility_levels.unwrap_or_else(|| self.facility_levels.clone()),
            facility_train_counts: self.facility_train_counts.clone(),
            pending_event_options: pending_event_options
                .unwrap_or_else(|| self.pending_event_options.clone()),
            hint_levels: self.hint_levels.clone(),
            statuses: self.statuses.clone(),
            performance_tokens: self.performance_tokens.clone(),
            scenario_resources,
            legacy: self.legacy.clone(),
            learned_skill_ids: self.learned_skill_ids.clone(),
            deck: self.deck.clone(),
            log: log.unwrap_or_else(|| self.log.clone()),
            generated_sparks: self.generated_sparks.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimChoice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MandatoryRace {
    pub id: String,
    pub name: String,
    pub year: i32,
    pub month: i32,
    pub half: i32,
}

impl StatName {
    pub fn to_facility(self) -> TrainingFacility {
        match self {
            StatName::Speed => TrainingFacility::Speed,
            StatName::Stamina => TrainingFacility::Stamina,
            StatName::Power => TrainingFacility::Power,
            StatName::Guts => TrainingFacility::Guts,
            StatName::Wit => TrainingFacility::Wit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueMode {
    Off,
    ChoicesOnly,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimSettings {
    pub dialogue_mode: DialogueMode,
    pub speed_multiplier: i32,
    pub allow_dialogue_at_high_speed: bool,
    pub trace_telemetry: bool,
    pub trace_rng: bool,
    /// Mid-run race backend. Default `stub` until R8.8; see [`crate::race::RaceModel`].
    #[serde(default)]
    pub race_model: crate::race::RaceModel,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            dialogue_mode: DialogueMode::ChoicesOnly,
            speed_multiplier: 1,
            allow_dialogue_at_high_speed: false,
            trace_telemetry: false,
            trace_rng: false,
            // Default physics since R8.8 (goldens re-frozen); use Stub for legacy parity.
            race_model: crate::race::RaceModel::Physics,
        }
    }
}

impl SimSettings {
    pub fn clamped_speed(&self) -> i32 {
        self.speed_multiplier.clamp(1, 100)
    }

    pub fn effective_dialogue_mode(&self) -> DialogueMode {
        if self.clamped_speed() >= 20 && !self.allow_dialogue_at_high_speed {
            return DialogueMode::Off;
        }
        if self.clamped_speed() >= 11 && self.dialogue_mode == DialogueMode::Full {
            return DialogueMode::ChoicesOnly;
        }
        self.dialogue_mode
    }

    pub fn is_turbo(&self) -> bool {
        self.clamped_speed() >= 51
    }

    pub fn is_fast(&self) -> bool {
        (11..=50).contains(&self.clamped_speed())
    }
}

pub const INJURED: &str = "injured";

impl CareerState {
    pub fn is_injured(&self) -> bool {
        self.statuses
            .iter()
            .any(|s| s.eq_ignore_ascii_case(INJURED) || s.to_lowercase().contains("injury"))
    }

    pub fn without_injury(statuses: &[String]) -> Vec<String> {
        statuses
            .iter()
            .filter(|s| !s.eq_ignore_ascii_case(INJURED) && !s.to_lowercase().contains("injury"))
            .cloned()
            .collect()
    }
}

pub fn upgrade_mood(m: MoodLevel) -> MoodLevel {
    match m {
        MoodLevel::Awful => MoodLevel::Bad,
        MoodLevel::Bad => MoodLevel::Normal,
        MoodLevel::Normal => MoodLevel::Good,
        MoodLevel::Good => MoodLevel::Great,
        MoodLevel::Great => MoodLevel::Great,
    }
}

pub fn downgrade_mood(m: MoodLevel) -> MoodLevel {
    match m {
        MoodLevel::Great => MoodLevel::Good,
        MoodLevel::Good => MoodLevel::Normal,
        MoodLevel::Normal => MoodLevel::Bad,
        MoodLevel::Bad => MoodLevel::Awful,
        MoodLevel::Awful => MoodLevel::Awful,
    }
}

pub fn shift_mood(current: MoodLevel, delta: i32) -> MoodLevel {
    let idx = current.index() as i32 + delta;
    MoodLevel::from_index(idx)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TurnSnapshot {
    pub turn: i32,
    pub energy: i32,
    pub mood: String,
    pub fans: i32,
    #[serde(rename = "sp")]
    pub skill_points: i32,
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
    pub phase: String,
    #[serde(rename = "scenarioResources")]
    pub scenario_resources: HashMap<String, i32>,
    #[serde(rename = "rngCallCount")]
    pub rng_call_count: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TurnTraceFixture {
    pub seed: i64,
    pub scenario: String,
    pub snapshots: Vec<TurnSnapshot>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RngTraceFixture {
    pub seed: i64,
    pub scenario: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoldenSummary {
    pub seed: i64,
    pub scenario: String,
    pub turn: i32,
    pub fans: i32,
    pub speed: i32,
    pub sp: i32,
}
