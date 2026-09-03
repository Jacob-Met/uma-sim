use crate::config::TrainingFailureConfig;
use crate::deck::DeckTrainingSignals;
use crate::rng::SimRandom;
use crate::scoring::apply_training_multipliers;
use crate::scoring::{
    scenario_display_name, to_game_date_snapshot, BarFillResult, TrainingConfig, TrainingOption,
};
use crate::state::{CareerState, MoodLevel, StatName, TrainingFacility};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub const DEFAULT_TABLES: &str = r#"{
  "facility_level_multipliers": {"1":1.0,"2":1.2,"3":1.4,"4":1.6,"5":2.0},
  "base_main_gain_by_level": {
    "1":{"typical":10},"2":{"typical":12},"3":{"typical":14},"4":{"typical":17},"5":{"typical":20}
  },
  "energy_cost_by_level": {"1":20,"2":21,"3":23,"4":25,"5":30},
  "wit_energy_recovery_by_level": {"1":5,"2":6,"3":7,"4":8,"5":10},
  "sub_stat_gain_ratio": {"secondary":0.5,"tertiary":0.2}
}"#;

#[derive(Debug, Clone)]
pub struct TrainingOutcome {
    pub main_gain: i32,
    pub secondary_gain: i32,
    pub tertiary_gain: i32,
    pub energy_cost: i32,
    pub failure_chance_pct: i32,
    pub facility_level: i32,
}

struct SubStatPair {
    secondary: TrainingFacility,
    tertiary: TrainingFacility,
}

type GrowthLookupFn = fn(&str, TrainingFacility) -> f64;

static GROWTH_LOOKUP: LazyLock<Mutex<Option<GrowthLookupFn>>> = LazyLock::new(|| Mutex::new(None));

static INSTALLED_TABLES: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub struct TrainingGainContext;

impl TrainingGainContext {
    pub fn set_trainee_growth_lookup(lookup: Option<GrowthLookupFn>) {
        *GROWTH_LOOKUP.lock().unwrap() = lookup;
    }

    pub fn growth_pct(trainee_name: &str, facility: TrainingFacility) -> f64 {
        GROWTH_LOOKUP
            .lock()
            .unwrap()
            .map(|f| f(trainee_name, facility))
            .unwrap_or(0.0)
    }

    pub fn support_stat_bonus_on_facility(state: &CareerState, facility: TrainingFacility) -> f64 {
        let key = facility.key();
        state
            .deck
            .slots_on_facility(facility)
            .into_iter()
            .filter_map(|slot| {
                crate::deck::DeckSupportBridge::card_lookup(&slot.support_id)
                    .and_then(|kb| kb.initial_stat_bonus_pct.get(key).copied())
            })
            .sum()
    }
}

pub struct TrainingResolver {
    tables_json: Option<String>,
    sub_stat_facilities: HashMap<TrainingFacility, SubStatPair>,
    secondary_ratio: f64,
    tertiary_ratio: f64,
}

impl Default for TrainingResolver {
    fn default() -> Self {
        Self::new(None)
    }
}

impl TrainingResolver {
    pub fn install_tables(json_text: Option<&str>) {
        if let Some(text) = json_text.filter(|s| !s.trim().is_empty()) {
            *INSTALLED_TABLES.lock().unwrap() = Some(text.to_string());
        }
    }

    pub fn tables_value() -> Option<serde_json::Value> {
        let text = INSTALLED_TABLES
            .lock()
            .unwrap()
            .clone()
            .or_else(|| Some(DEFAULT_TABLES.to_string()))?;
        serde_json::from_str(&text).ok()
    }

    pub fn from_installed_tables() -> Self {
        let tables = INSTALLED_TABLES.lock().unwrap().clone();
        Self::new(tables)
    }

    pub fn new(tables_json: Option<String>) -> Self {
        let mut resolver = Self {
            tables_json,
            sub_stat_facilities: HashMap::new(),
            secondary_ratio: 0.5,
            tertiary_ratio: 0.2,
        };
        if let Some(text) = resolver.tables_json.clone() {
            resolver.load_sub_stat_config(&text);
        }
        resolver
    }

    pub fn resolve_typical(
        &self,
        facility: TrainingFacility,
        facility_level: i32,
        mood: MoodLevel,
        state: Option<&CareerState>,
    ) -> TrainingOutcome {
        self.resolve(
            facility,
            facility_level,
            mood,
            &mut SimRandom::new(0),
            state,
            None,
        )
    }

    pub fn resolve(
        &self,
        facility: TrainingFacility,
        facility_level: i32,
        mood: MoodLevel,
        rng: &mut SimRandom,
        state: Option<&CareerState>,
        support_slices: Option<&[crate::scoring::SupportEffectSlice]>,
    ) -> TrainingOutcome {
        let slices = support_slices
            .map(|s| s.to_vec())
            .or_else(|| state.map(|s| crate::deck::DeckSupportBridge::slices_for(s, facility)))
            .unwrap_or_default();
        let level = facility_level.clamp(1, 5);
        let tables = self.parse_tables();
        let typical = tables
            .get("base_main_gain_by_level")
            .and_then(|v| v.get(&level.to_string()))
            .and_then(|v| v.get("typical"))
            .and_then(|v| v.as_i64())
            .unwrap_or(12) as i32;
        let level_mult = tables
            .get("facility_level_multipliers")
            .and_then(|v| v.get(&level.to_string()))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let jitter = rng.next_int_range(0, 3);
        let growth_pct = state
            .map(|s| TrainingGainContext::growth_pct(&s.meta.trainee_name, facility))
            .unwrap_or(0.0);
        let support_stat_bonus = state
            .map(|s| TrainingGainContext::support_stat_bonus_on_facility(s, facility))
            .unwrap_or(0.0);
        let base_plus_bonus = typical as f64 * level_mult + jitter as f64 + support_stat_bonus;

        let gain = apply_training_multipliers(
            base_plus_bonus,
            &slices,
            mood,
            slices.len().max(1) as i32,
            growth_pct,
        );

        let energy_cost = if facility == TrainingFacility::Wit {
            let recovery = tables
                .get("wit_energy_recovery_by_level")
                .and_then(|v| v.get(&level.to_string()))
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as i32;
            -recovery
        } else {
            tables
                .get("energy_cost_by_level")
                .and_then(|v| v.get(&level.to_string()))
                .and_then(|v| v.as_i64())
                .unwrap_or((20 + (level - 1) * 2) as i64) as i32
        };

        let failure_pct = TrainingFailureConfig::failure_chance_pct(100, 100, mood, level);

        TrainingOutcome {
            main_gain: gain,
            secondary_gain: ((gain as f64 * self.secondary_ratio) as i32).max(0),
            tertiary_gain: ((gain as f64 * self.tertiary_ratio) as i32).max(0),
            energy_cost,
            failure_chance_pct: failure_pct,
            facility_level: level,
        }
    }

    pub fn secondary_facility(&self, facility: TrainingFacility) -> TrainingFacility {
        self.sub_stat_facilities
            .get(&facility)
            .map(|p| p.secondary)
            .unwrap_or_else(|| default_secondary(facility))
    }

    pub fn tertiary_facility(&self, facility: TrainingFacility) -> TrainingFacility {
        self.sub_stat_facilities
            .get(&facility)
            .map(|p| p.tertiary)
            .unwrap_or_else(|| default_tertiary(facility))
    }

    pub fn secondary_stat(&self, facility: TrainingFacility) -> StatName {
        self.secondary_facility(facility).to_stat_name()
    }

    pub fn tertiary_stat(&self, facility: TrainingFacility) -> StatName {
        self.tertiary_facility(facility).to_stat_name()
    }

    fn parse_tables(&self) -> serde_json::Value {
        let installed = INSTALLED_TABLES.lock().unwrap().clone();
        let raw = self
            .tables_json
            .as_deref()
            .or(installed.as_deref())
            .unwrap_or(DEFAULT_TABLES);
        serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn load_sub_stat_config(&mut self, json_text: &str) {
        let Ok(root) = serde_json::from_str::<serde_json::Value>(json_text) else {
            return;
        };
        if let Some(ratios) = root.get("sub_stat_gain_ratio").and_then(|v| v.as_object()) {
            if let Some(v) = ratios.get("secondary").and_then(|v| v.as_f64()) {
                self.secondary_ratio = v;
            }
            if let Some(v) = ratios.get("tertiary").and_then(|v| v.as_f64()) {
                self.tertiary_ratio = v;
            }
        }
        if let Some(mapping) = root.get("sub_stat_facilities").and_then(|v| v.as_object()) {
            for (key, el) in mapping {
                if matches!(key.as_str(), "confidence" | "source" | "notes") {
                    continue;
                }
                let Some(fac) = parse_facility_key(key) else {
                    continue;
                };
                let Some(obj) = el.as_object() else {
                    continue;
                };
                let sec = obj
                    .get("secondary")
                    .and_then(|v| v.as_str())
                    .and_then(parse_facility_key);
                let ter = obj
                    .get("tertiary")
                    .and_then(|v| v.as_str())
                    .and_then(parse_facility_key);
                if let (Some(s), Some(t)) = (sec, ter) {
                    self.sub_stat_facilities.insert(
                        fac,
                        SubStatPair {
                            secondary: s,
                            tertiary: t,
                        },
                    );
                }
            }
        }
    }
}

fn default_secondary(facility: TrainingFacility) -> TrainingFacility {
    match facility {
        TrainingFacility::Speed => TrainingFacility::Power,
        TrainingFacility::Stamina => TrainingFacility::Guts,
        TrainingFacility::Power => TrainingFacility::Stamina,
        TrainingFacility::Guts => TrainingFacility::Speed,
        TrainingFacility::Wit => TrainingFacility::Speed,
    }
}

fn default_tertiary(facility: TrainingFacility) -> TrainingFacility {
    match facility {
        TrainingFacility::Speed => TrainingFacility::Guts,
        TrainingFacility::Stamina => TrainingFacility::Power,
        TrainingFacility::Power => TrainingFacility::Speed,
        TrainingFacility::Guts => TrainingFacility::Power,
        TrainingFacility::Wit => TrainingFacility::Stamina,
    }
}

fn parse_facility_key(name: &str) -> Option<TrainingFacility> {
    match name.to_uppercase().as_str() {
        "SPEED" => Some(TrainingFacility::Speed),
        "STAMINA" => Some(TrainingFacility::Stamina),
        "POWER" => Some(TrainingFacility::Power),
        "GUTS" => Some(TrainingFacility::Guts),
        "WIT" => Some(TrainingFacility::Wit),
        _ => None,
    }
}

pub struct TrainingPreview;

impl TrainingPreview {
    pub fn build_options(
        state: &CareerState,
        resolver: &TrainingResolver,
    ) -> HashMap<TrainingFacility, TrainingOption> {
        TrainingFacility::ALL
            .into_iter()
            .map(|facility| {
                let key = facility.key();
                let level = state.facility_levels.get(key).copied().unwrap_or(1);
                let outcome = resolver.resolve_typical(facility, level, state.mood, Some(state));
                let stat = facility.to_stat_name();
                let secondary = resolver.secondary_stat(facility);
                let tertiary = resolver.tertiary_stat(facility);
                let mut stat_gains = HashMap::from([(stat, outcome.main_gain)]);
                stat_gains.insert(secondary, outcome.secondary_gain);
                if outcome.tertiary_gain > 0 {
                    stat_gains.insert(tertiary, outcome.tertiary_gain);
                }
                let bars: Vec<BarFillResult> =
                    DeckTrainingSignals::relationship_bars(state, facility)
                        .into_iter()
                        .map(|b| BarFillResult {
                            dominant_color: b.dominant_color,
                            fill_percent: b.fill_percent,
                            is_trainer_support: b.is_trainer_support,
                        })
                        .collect();
                let option = TrainingOption {
                    name: stat,
                    stat_gains,
                    relationship_bars: bars,
                    num_rainbow: DeckTrainingSignals::num_rainbow(state, facility),
                    num_skill_hints: state.hint_levels.get(key).copied().unwrap_or(0),
                    training_level: Some(level),
                };
                (facility, option)
            })
            .collect()
    }

    pub fn to_training_config(
        state: &CareerState,
        stat_caps: HashMap<StatName, i32>,
    ) -> TrainingConfig {
        let mut config = TrainingConfig::new(
            HashMap::from([
                (StatName::Speed, state.stats.speed),
                (StatName::Stamina, state.stats.stamina),
                (StatName::Power, state.stats.power),
                (StatName::Guts, state.stats.guts),
                (StatName::Wit, state.stats.wit),
            ]),
            StatName::ALL.to_vec(),
            StatName::ALL.to_vec(),
            HashMap::new(),
            to_game_date_snapshot(&state.date, state.turn),
            scenario_display_name(&state.meta.scenario_id),
            true,
        );
        config.enable_training_level_weighting = true;
        config.stat_caps = stat_caps;
        config
    }
}
