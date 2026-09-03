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
    pub name_ja: String,
    pub card_id: Option<i32>,
    pub char_id: Option<i32>,
    /// Growth bonus percent per stat: speed, stamina, power, guts, wit.
    pub growth_bonus_pct: [i32; 5],
}

#[derive(Debug, Clone)]
struct CharProfile {
    name_en: String,
    name_ja: String,
    #[allow(dead_code)]
    url_name: Option<String>,
}

struct TraineeCatalogState {
    by_name: HashMap<String, TraineeMeta>,
    char_name_to_id: HashMap<String, i32>,
    by_char_id: HashMap<i32, Vec<TraineeMeta>>,
    char_profiles: HashMap<i32, CharProfile>,
    loaded: bool,
}

impl Default for TraineeCatalogState {
    fn default() -> Self {
        Self {
            by_name: HashMap::new(),
            char_name_to_id: HashMap::new(),
            by_char_id: HashMap::new(),
            char_profiles: HashMap::new(),
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
        let mut profiles: HashMap<i32, CharProfile> = HashMap::new();

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
            let name_ja = obj
                .get("name_ja")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name_en = obj
                .get("name_en_official")
                .or_else(|| obj.get("name_en_fan"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url_name = payload
                .get("url_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let bonus: Option<Vec<i32>> =
                payload
                    .get("stat_bonus")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|n| n as i32))
                            .collect()
                    });

            if bonus.as_ref().map(|b| b.len()).unwrap_or(0) < 5 {
                // Character base row (no card growth) — identity for the picker.
                if !name_en.is_empty() {
                    char_names.insert(name_en.to_lowercase(), char_id);
                    profiles.insert(
                        char_id,
                        CharProfile {
                            name_en: name_en.clone(),
                            name_ja: name_ja.clone(),
                            url_name: url_name.clone(),
                        },
                    );
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
            // Prefer character-level JA if we already saw the profile.
            let ja = profiles
                .get(&char_id)
                .map(|p| p.name_ja.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| name_ja.clone());
            let meta = TraineeMeta {
                id: id.clone(),
                name: name.clone(),
                name_ja: ja.clone(),
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
            profiles.entry(char_id).or_insert(CharProfile {
                name_en: name,
                name_ja: ja,
                url_name,
            });
        }

        // Second pass: attach profile JA onto cards that loaded before the character row.
        for (cid, cards) in cards_by_char.iter_mut() {
            if let Some(profile) = profiles.get(cid) {
                for card in cards.iter_mut() {
                    if card.name_ja.is_empty() {
                        card.name_ja = profile.name_ja.clone();
                    }
                    // Prefer canonical character English name for picker identity.
                    if !profile.name_en.is_empty() {
                        card.name = profile.name_en.clone();
                    }
                }
            }
            cards.sort_by_key(|c| c.card_id.unwrap_or(i32::MAX));
        }

        state.loaded = !index.is_empty() || !profiles.is_empty();
        state.by_name = index;
        state.char_name_to_id = char_names;
        state.by_char_id = cards_by_char;
        state.char_profiles = profiles;
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

    /// Unique characters for the web UI / REST `/v1/catalog/trainees`.
    pub fn list_all() -> Vec<TraineeMeta> {
        let state = CATALOG.lock().unwrap();
        let mut rows: Vec<TraineeMeta> = Vec::new();

        // Prefer one entry per character (profile + default card growth when available).
        let mut char_ids: Vec<i32> = state.char_profiles.keys().copied().collect();
        for cid in state.by_char_id.keys() {
            if !char_ids.contains(cid) {
                char_ids.push(*cid);
            }
        }
        char_ids.sort_unstable();

        for cid in char_ids {
            let profile = state.char_profiles.get(&cid);
            let card = state.by_char_id.get(&cid).and_then(|v| v.first());
            let (name, name_ja, id, growth, card_id) = if let Some(c) = card {
                (
                    profile
                        .map(|p| p.name_en.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| c.name.clone()),
                    profile
                        .map(|p| p.name_ja.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| c.name_ja.clone()),
                    format!("trainee:{cid}"),
                    c.growth_bonus_pct,
                    c.card_id,
                )
            } else if let Some(p) = profile {
                (
                    p.name_en.clone(),
                    p.name_ja.clone(),
                    format!("trainee:{cid}"),
                    [0; 5],
                    None,
                )
            } else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            rows.push(TraineeMeta {
                id,
                name,
                name_ja,
                card_id,
                char_id: Some(cid),
                growth_bonus_pct: growth,
            });
        }

        rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        rows
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

    fn default_card_for_char_locked(
        state: &TraineeCatalogState,
        char_id: i32,
    ) -> Option<TraineeMeta> {
        state.by_char_id.get(&char_id)?.first().cloned()
    }
}
