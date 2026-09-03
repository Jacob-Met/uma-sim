//! KB-backed support card metadata for deck resolution.

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

const LEVEL_THRESHOLDS: [i32; 11] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50];

fn resolve_breakpoints(breakpoints: &[f64], level: i32) -> f64 {
    let mut resolved = 0.0;
    let capped = level.clamp(1, 50);
    for (i, &raw) in breakpoints.iter().enumerate() {
        if i >= LEVEL_THRESHOLDS.len() {
            break;
        }
        if LEVEL_THRESHOLDS[i] > capped {
            break;
        }
        if raw >= 0.0 {
            resolved = raw;
        }
    }
    resolved
}

#[derive(Debug, Clone)]
pub struct SupportCardMeta {
    pub id: String,
    pub name: String,
    pub card_type: String,
    pub friendship_bonus_pct: f64,
    pub mood_effect_pct: f64,
    pub training_effectiveness_pct: f64,
    pub initial_stat_bonus_pct: HashMap<String, f64>,
}

struct SupportCatalogState {
    by_id: HashMap<String, SupportCardMeta>,
    by_name: HashMap<String, SupportCardMeta>,
    loaded: bool,
}

impl Default for SupportCatalogState {
    fn default() -> Self {
        Self {
            by_id: HashMap::new(),
            by_name: HashMap::new(),
            loaded: false,
        }
    }
}

static CATALOG: LazyLock<Mutex<SupportCatalogState>> =
    LazyLock::new(|| Mutex::new(SupportCatalogState::default()));

pub struct SupportCatalog;

impl SupportCatalog {
    pub fn ensure_loaded(repo_root: Option<&Path>) {
        let loaded = CATALOG.lock().unwrap().loaded;
        if !loaded {
            Self::init_from_repo(repo_root);
        }
    }

    pub fn init_from_repo(repo_root: Option<&Path>) {
        let Some(root) = repo_root else {
            return;
        };
        let mut state = CATALOG.lock().unwrap();
        if state.loaded {
            return;
        }
        let path = root.join("knowledge/canonical/by_kind/support_card.json");
        if !path.exists() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            Self::load_json_inner(&text, &mut state);
        }
    }

    pub fn load_json(json_text: &str) {
        let mut state = CATALOG.lock().unwrap();
        Self::load_json_inner(json_text, &mut state);
    }

    fn load_json_inner(json_text: &str, state: &mut SupportCatalogState) {
        let Ok(Value::Array(arr)) = serde_json::from_str(json_text) else {
            return;
        };
        let cards: Vec<SupportCardMeta> = arr
            .iter()
            .filter_map(|el| el.as_object().and_then(Self::parse_card))
            .collect();
        state.by_id = cards.iter().map(|c| (c.id.clone(), c.clone())).collect();
        state.by_name = cards
            .iter()
            .map(|c| (c.name.to_lowercase(), c.clone()))
            .collect();
        state.loaded = true;
    }

    pub fn lookup(id_or_name: &str) -> Option<SupportCardMeta> {
        let state = CATALOG.lock().unwrap();
        state
            .by_id
            .get(id_or_name)
            .cloned()
            .or_else(|| state.by_name.get(&id_or_name.to_lowercase()).cloned())
    }

    fn parse_card(obj: &serde_json::Map<String, Value>) -> Option<SupportCardMeta> {
        let id = obj.get("id")?.as_str()?.to_string();
        let payload = obj.get("payload")?.as_object()?;
        let card_type = payload
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("speed")
            .to_string();
        let name = obj
            .get("name_en_fan")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("char_name").and_then(|v| v.as_str()))
            .unwrap_or(&id)
            .to_string();
        let effects = match payload.get("effects").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                return Some(SupportCardMeta {
                    id,
                    name,
                    card_type,
                    friendship_bonus_pct: 10.0,
                    mood_effect_pct: 0.0,
                    training_effectiveness_pct: 0.0,
                    initial_stat_bonus_pct: HashMap::new(),
                });
            }
        };

        let mut friendship = 10.0;
        let mut mood = 0.0;
        let mut training: f64 = 0.0;
        let mut initial = HashMap::new();

        for eff in effects {
            let Some(e) = eff.as_object() else {
                continue;
            };
            let effect_type = e.get("effect_type_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            match effect_type {
                1 => friendship = Self::resolve_breakpoints_obj(e),
                2 => mood = Self::resolve_breakpoints_obj(e),
                3 | 4 => training = training.max(Self::resolve_breakpoints_obj(e)),
                8 => {
                    initial.insert("speed".into(), Self::resolve_breakpoints_obj(e));
                }
                9 => {
                    initial.insert("stamina".into(), Self::resolve_breakpoints_obj(e));
                }
                10 => {
                    initial.insert("power".into(), Self::resolve_breakpoints_obj(e));
                }
                11 => {
                    initial.insert("guts".into(), Self::resolve_breakpoints_obj(e));
                }
                12 => {
                    initial.insert("wit".into(), Self::resolve_breakpoints_obj(e));
                }
                _ => {}
            }
        }

        Some(SupportCardMeta {
            id,
            name,
            card_type,
            friendship_bonus_pct: friendship,
            mood_effect_pct: mood,
            training_effectiveness_pct: training,
            initial_stat_bonus_pct: initial,
        })
    }

    fn resolve_breakpoints_obj(effect: &serde_json::Map<String, Value>) -> f64 {
        let bps: Vec<f64> = effect
            .get("breakpoints")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64())
                    .collect()
            })
            .unwrap_or_default();
        resolve_breakpoints(&bps, 30)
    }
}
