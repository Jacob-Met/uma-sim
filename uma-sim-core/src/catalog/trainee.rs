//! KB-backed trainee growth bonuses from `trainee.json` stat_bonus arrays.

use crate::state::TrainingFacility;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct TraineeMeta {
    pub id: String,
    pub name: String,
    pub card_id: Option<i32>,
    pub char_id: Option<i32>,
    /// Growth bonus percent per stat: speed, stamina, power, guts, wit.
    pub growth_bonus_pct: [i32; 5],
}

struct TraineeCatalogState {
    by_name: HashMap<String, TraineeMeta>,
    char_name_to_id: HashMap<String, i32>,
    by_char_id: HashMap<i32, Vec<TraineeMeta>>,
    loaded: bool,
}

impl Default for TraineeCatalogState {
    fn default() -> Self {
        Self {
            by_name: HashMap::new(),
            char_name_to_id: HashMap::new(),
            by_char_id: HashMap::new(),
            loaded: false,
        }
    }
}

static CATALOG: LazyLock<Mutex<TraineeCatalogState>> =
    LazyLock::new(|| Mutex::new(TraineeCatalogState::default()));

pub struct TraineeCatalog;

impl TraineeCatalog {
    pub fn init_from_repo(repo_root: Option<&Path>) {
        let Some(root) = repo_root else {
            return;
        };
        let mut state = CATALOG.lock().unwrap();
        if state.loaded {
            return;
        }
        let path = root.join("knowledge/canonical/by_kind/trainee.json");
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

    fn load_json_inner(json_text: &str, state: &mut TraineeCatalogState) {
        let Ok(Value::Array(arr)) = serde_json::from_str(json_text) else {
            return;
        };

        let mut index: HashMap<String, TraineeMeta> = HashMap::new();
        let mut char_names: HashMap<String, i32> = HashMap::new();
        let mut cards_by_char: HashMap<i32, Vec<TraineeMeta>> = HashMap::new();

        for el in arr {
            let Some(obj) = el.as_object() else {
                continue;
            };
            let Some(payload) = obj.get("payload").and_then(|v| v.as_object()) else {
                continue;
            };
            let Some(char_id) = payload.get("char_id").and_then(|v| v.as_i64()) else {
                continue;
            };
            let char_id = char_id as i32;

            let bonus: Option<Vec<i32>> = payload
                .get("stat_bonus")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_i64().map(|n| n as i32))
                        .collect()
                });

            if bonus.as_ref().map(|b| b.len()).unwrap_or(0) < 5 {
                let char_name = obj
                    .get("name_en_official")
                    .or_else(|| obj.get("name_en_fan"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !char_name.is_empty() {
                    char_names.insert(char_name.to_lowercase(), char_id);
                }
                if let Some(aliases) = obj.get("aliases").and_then(|v| v.as_array()) {
                    for alias in aliases {
                        if let Some(s) = alias.as_str() {
                            char_names.insert(s.to_lowercase(), char_id);
                        }
                    }
                }
                continue;
            }

            let bonus = bonus.unwrap();
            let id = match obj.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let name = obj
                .get("name_en_fan")
                .or_else(|| obj.get("name_en_official"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let card_id = payload
                .get("card_id")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);
            let mut growth = [0i32; 5];
            for (i, v) in bonus.iter().take(5).enumerate() {
                growth[i] = *v;
            }
            let meta = TraineeMeta {
                id: id.clone(),
                name: name.clone(),
                card_id,
                char_id: Some(char_id),
                growth_bonus_pct: growth,
            };
            index.insert(name.to_lowercase(), meta.clone());
            if let Some(cn) = payload.get("char_name").and_then(|v| v.as_str()) {
                index.insert(cn.to_lowercase(), meta.clone());
            }
            if let Some(aliases) = obj.get("aliases").and_then(|v| v.as_array()) {
                for alias in aliases {
                    if let Some(s) = alias.as_str() {
                        index.insert(s.to_lowercase(), meta.clone());
                    }
                }
            }
            cards_by_char.entry(char_id).or_default().push(meta);
        }

        for cards in cards_by_char.values_mut() {
            cards.sort_by_key(|c| c.card_id.unwrap_or(i32::MAX));
        }

        state.loaded = !index.is_empty();
        state.by_name = index;
        state.char_name_to_id = char_names;
        state.by_char_id = cards_by_char;
    }

    pub fn lookup(name_or_id: &str) -> Option<TraineeMeta> {
        let state = CATALOG.lock().unwrap();
        let key = name_or_id.to_lowercase();
        if let Some(meta) = state.by_name.get(&key) {
            return Some(meta.clone());
        }
        if let Some(&char_id) = state.char_name_to_id.get(&key) {
            return Self::default_card_for_char_locked(&state, char_id);
        }
        state
            .by_name
            .iter()
            .find(|(k, _)| key.contains(k.as_str()) || k.contains(key.as_str()))
            .map(|(_, v)| v.clone())
    }

    pub fn growth_pct(trainee_name: &str, facility: TrainingFacility) -> f64 {
        let Some(meta) = Self::lookup(trainee_name) else {
            return 0.0;
        };
        let idx = match facility {
            TrainingFacility::Speed => 0,
            TrainingFacility::Stamina => 1,
            TrainingFacility::Power => 2,
            TrainingFacility::Guts => 3,
            TrainingFacility::Wit => 4,
        };
        meta.growth_bonus_pct[idx] as f64
    }

    fn default_card_for_char_locked(state: &TraineeCatalogState, char_id: i32) -> Option<TraineeMeta> {
        state.by_char_id.get(&char_id)?.first().cloned()
    }
}
