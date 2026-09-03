//! KB-backed trainee identity, growth, aptitudes, and starting skills from `trainee.json`.

use crate::state::TrainingFacility;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

/// Aptitude keys in GameTora / card `aptitude[]` order.
pub const APTITUDE_KEYS: [&str; 10] = [
    "turf", "dirt", "sprint", "mile", "medium", "long", "front", "pace", "late", "end",
];

#[derive(Debug, Clone)]
pub struct TraineeMeta {
    pub id: String,
    pub name: String,
    pub name_ja: String,
    pub card_id: Option<i32>,
    pub char_id: Option<i32>,
    /// Growth bonus percent per stat: speed, stamina, power, guts, wit.
    pub growth_bonus_pct: [i32; 5],
    /// Card base stats (speed…wit); empty → engine default.
    pub base_stats: [i32; 5],
    /// Letters for [`APTITUDE_KEYS`] (length 0 or 10).
    pub aptitudes: Vec<String>,
    /// Innate skill numeric ids (start as hint level 1).
    pub skills_innate: Vec<i32>,
    /// Unique skill numeric ids (granted at career start).
    pub skills_unique: Vec<i32>,
    pub playable: bool,
    pub playable_en: bool,
}

#[derive(Debug, Clone)]
struct CharProfile {
    name_en: String,
    name_ja: String,
    #[allow(dead_code)]
    url_name: Option<String>,
    playable: bool,
    playable_en: bool,
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

fn parse_i32_list(v: Option<&Value>) -> Vec<i32> {
    v.and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|n| n.as_i64().map(|i| i as i32))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_aptitudes(v: Option<&Value>) -> Vec<String> {
    v.and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|n| n.as_str().map(|s| s.to_uppercase()))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_base_stats(v: Option<&Value>) -> [i32; 5] {
    let mut out = [0i32; 5];
    if let Some(arr) = v.and_then(|x| x.as_array()) {
        for (i, n) in arr.iter().take(5).enumerate() {
            if let Some(v) = n.as_i64() {
                out[i] = v as i32;
            }
        }
    }
    out
}

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
            let playable = payload
                .get("playable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let playable_en = payload
                .get("playable_en")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

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
                // Character base row (no card growth) — identity only.
                if !name_en.is_empty() {
                    char_names.insert(name_en.to_lowercase(), char_id);
                    profiles.insert(
                        char_id,
                        CharProfile {
                            name_en: name_en.clone(),
                            name_ja: name_ja.clone(),
                            url_name: url_name.clone(),
                            playable,
                            playable_en,
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
            let ja = profiles
                .get(&char_id)
                .map(|p| p.name_ja.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| name_ja.clone());
            let profile_playable = profiles.get(&char_id).map(|p| p.playable).unwrap_or(true);
            let profile_playable_en = profiles
                .get(&char_id)
                .map(|p| p.playable_en)
                .unwrap_or(false);
            let meta = TraineeMeta {
                id: id.clone(),
                name: name.clone(),
                name_ja: ja.clone(),
                card_id,
                char_id: Some(char_id),
                growth_bonus_pct: growth,
                base_stats: parse_base_stats(payload.get("base_stats")),
                aptitudes: parse_aptitudes(payload.get("aptitude")),
                skills_innate: parse_i32_list(payload.get("skills_innate")),
                skills_unique: parse_i32_list(payload.get("skills_unique")),
                playable: profile_playable,
                playable_en: profile_playable_en,
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
                playable: profile_playable,
                playable_en: profile_playable_en,
            });
        }

        for (cid, cards) in cards_by_char.iter_mut() {
            if let Some(profile) = profiles.get(cid) {
                for card in cards.iter_mut() {
                    if card.name_ja.is_empty() {
                        card.name_ja = profile.name_ja.clone();
                    }
                    if !profile.name_en.is_empty() {
                        card.name = profile.name_en.clone();
                    }
                    card.playable = profile.playable;
                    card.playable_en = profile.playable_en;
                }
            }
            cards.sort_by_key(|c| c.card_id.unwrap_or(i32::MAX));
            // Character English name resolves to the default (lowest card_id) variant.
            if let Some(default_card) = cards.first() {
                index.insert(default_card.name.to_lowercase(), default_card.clone());
                if let Some(profile) = profiles.get(cid) {
                    if !profile.name_en.is_empty() {
                        index.insert(profile.name_en.to_lowercase(), default_card.clone());
                    }
                }
            }
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

    /// Playable trainees only (must have a trainee card). Excludes staff/NPCs.
    pub fn list_all() -> Vec<TraineeMeta> {
        let state = CATALOG.lock().unwrap();
        let mut rows: Vec<TraineeMeta> = Vec::new();

        let mut char_ids: Vec<i32> = state.by_char_id.keys().copied().collect();
        char_ids.sort_unstable();

        for cid in char_ids {
            let profile = state.char_profiles.get(&cid);
            // Require an actual trainee card — no Tazuna / Happy Meek / staff, etc.
            let Some(card) = state.by_char_id.get(&cid).and_then(|v| v.first()) else {
                continue;
            };
            // Prefer explicit playable flag when present on the character row.
            if let Some(p) = profile {
                if !p.playable {
                    continue;
                }
            }
            let name = profile
                .map(|p| p.name_en.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| card.name.clone());
            let name_ja = profile
                .map(|p| p.name_ja.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| card.name_ja.clone());
            if name.is_empty() {
                continue;
            }
            rows.push(TraineeMeta {
                id: format!("trainee:{cid}"),
                name,
                name_ja,
                card_id: card.card_id,
                char_id: Some(cid),
                growth_bonus_pct: card.growth_bonus_pct,
                base_stats: card.base_stats,
                aptitudes: card.aptitudes.clone(),
                skills_innate: card.skills_innate.clone(),
                skills_unique: card.skills_unique.clone(),
                playable: profile.map(|p| p.playable).unwrap_or(true),
                playable_en: profile.map(|p| p.playable_en).unwrap_or(false),
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

    pub fn aptitude_map(meta: &TraineeMeta) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (i, key) in APTITUDE_KEYS.iter().enumerate() {
            let letter = meta.aptitudes.get(i).cloned().unwrap_or_else(|| "G".into());
            map.insert((*key).to_string(), letter);
        }
        map
    }

    fn default_card_for_char_locked(
        state: &TraineeCatalogState,
        char_id: i32,
    ) -> Option<TraineeMeta> {
        let mut card = state.by_char_id.get(&char_id)?.first()?.clone();
        if let Some(profile) = state.char_profiles.get(&char_id) {
            if !profile.name_en.is_empty() {
                card.name = profile.name_en.clone();
            }
            if !profile.name_ja.is_empty() {
                card.name_ja = profile.name_ja.clone();
            }
            card.playable = profile.playable;
            card.playable_en = profile.playable_en;
        }
        Some(card)
    }
}
