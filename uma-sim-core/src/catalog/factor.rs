//! Loads factor metadata from canonical factor.json.

use crate::legacy::{LegacyApplicator, LegacyFactorContext, LegacyFactorMeta};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct FactorMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub stat_key: Option<String>,
    pub skill_id: Option<String>,
    pub pink_tag: Option<String>,
    pub race_name: Option<String>,
}

fn default_blue_map() -> HashMap<String, String> {
    HashMap::from([
        ("factor:blue:1".into(), "speed".into()),
        ("factor:blue:2".into(), "stamina".into()),
        ("factor:blue:3".into(), "power".into()),
        ("factor:blue:4".into(), "guts".into()),
        ("factor:blue:5".into(), "wit".into()),
    ])
}

struct FactorCatalogState {
    by_id: HashMap<String, FactorMeta>,
    loaded: bool,
}

impl Default for FactorCatalogState {
    fn default() -> Self {
        Self {
            by_id: HashMap::new(),
            loaded: false,
        }
    }
}

static CATALOG: LazyLock<Mutex<FactorCatalogState>> =
    LazyLock::new(|| Mutex::new(FactorCatalogState::default()));

pub struct FactorCatalog;

impl FactorCatalog {
    pub fn lookup(id: &str) -> Option<FactorMeta> {
        CATALOG.lock().unwrap().by_id.get(id).cloned()
    }

    pub fn load_blue_stat_map(repo_root: &Path) -> HashMap<String, String> {
        Self::init_from_repo(Some(repo_root));
        let state = CATALOG.lock().unwrap();
        let map: HashMap<String, String> = state
            .by_id
            .values()
            .filter(|m| m.category == "blue")
            .filter_map(|m| m.stat_key.as_ref().map(|s| (m.id.clone(), s.clone())))
            .collect();
        if map.is_empty() {
            default_blue_map()
        } else {
            map
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
        let path = root.join("knowledge/canonical/by_kind/factor.json");
        if !path.exists() {
            let defaults = default_blue_map();
            state.by_id = defaults
                .iter()
                .map(|(id, stat)| {
                    (
                        id.clone(),
                        FactorMeta {
                            id: id.clone(),
                            name: stat.clone(),
                            category: "blue".into(),
                            stat_key: Some(stat.clone()),
                            skill_id: None,
                            pink_tag: None,
                            race_name: None,
                        },
                    )
                })
                .collect();
            LegacyApplicator::set_blue_stat_map(defaults);
            Self::install_factor_lookup();
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            Self::load_json_inner(&text, &mut state);
        }
        let blue_map: HashMap<String, String> = state
            .by_id
            .values()
            .filter(|m| m.category == "blue")
            .filter_map(|m| m.stat_key.as_ref().map(|s| (m.id.clone(), s.clone())))
            .collect();
        LegacyApplicator::set_blue_stat_map(if blue_map.is_empty() {
            default_blue_map()
        } else {
            blue_map
        });
        Self::install_factor_lookup();
        state.loaded = !state.by_id.is_empty();
    }

    fn install_factor_lookup() {
        LegacyFactorContext::set_lookup(Some(|id| {
            Self::lookup(id).map(|m| LegacyFactorMeta {
                id: m.id,
                category: m.category,
                stat_key: m.stat_key,
                skill_id: m.skill_id,
                pink_tag: m.pink_tag,
                race_name: m.race_name,
            })
        }));
    }

    /// Compact catalog rows for the web UI / REST `/v1/catalog/factors`.
    pub fn list_all() -> Vec<FactorMeta> {
        let mut rows: Vec<_> = CATALOG.lock().unwrap().by_id.values().cloned().collect();
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        rows
    }

    pub fn load_json(json_text: &str) {
        let mut state = CATALOG.lock().unwrap();
        Self::load_json_inner(json_text, &mut state);
        state.loaded = !state.by_id.is_empty();
    }

    fn load_json_inner(json_text: &str, state: &mut FactorCatalogState) {
        let Ok(Value::Array(arr)) = serde_json::from_str(json_text) else {
            return;
        };
        let mut index = HashMap::new();
        for el in arr {
            let Some(obj) = el.as_object() else {
                continue;
            };
            let id = match obj.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let Some(payload) = obj.get("payload").and_then(|v| v.as_object()) else {
                continue;
            };
            let category = payload
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string();
            let name = match obj.get("name_en_official").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let meta = match category.as_str() {
                "blue" => {
                    let stat = match Self::stat_from_factor_name(&name) {
                        Some(s) => s,
                        None => continue,
                    };
                    FactorMeta {
                        id: id.clone(),
                        name: name.clone(),
                        category,
                        stat_key: Some(stat),
                        skill_id: None,
                        pink_tag: None,
                        race_name: None,
                    }
                }
                "skill" => {
                    let skill_numeric = payload
                        .get("effects")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_object())
                        .and_then(|e| e.get("value_1"))
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_i64());
                    let skill_id = skill_numeric.map(|n| format!("skill:{n}"));
                    FactorMeta {
                        id: id.clone(),
                        name: name.clone(),
                        category,
                        stat_key: None,
                        skill_id,
                        pink_tag: None,
                        race_name: None,
                    }
                }
                "pink" => FactorMeta {
                    id: id.clone(),
                    name: name.clone(),
                    category,
                    stat_key: None,
                    skill_id: None,
                    pink_tag: Some(normalize_pink_tag(&name)),
                    race_name: None,
                },
                "race" => FactorMeta {
                    id: id.clone(),
                    name: name.clone(),
                    category,
                    stat_key: None,
                    skill_id: None,
                    pink_tag: None,
                    race_name: Some(name),
                },
                _ => FactorMeta {
                    id: id.clone(),
                    name: name.clone(),
                    category,
                    stat_key: None,
                    skill_id: None,
                    pink_tag: None,
                    race_name: None,
                },
            };
            index.insert(id, meta);
        }
        state.by_id = index;
    }

    fn stat_from_factor_name(name: &str) -> Option<String> {
        match name.to_lowercase().as_str() {
            "speed" => Some("speed".into()),
            "stamina" => Some("stamina".into()),
            "power" => Some("power".into()),
            "guts" => Some("guts".into()),
            "wits" | "wit" => Some("wit".into()),
            _ => None,
        }
    }
}

/// Map factor display names → aptitude keys used by inheritance + race.
fn normalize_pink_tag(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "turf" => "turf".into(),
        "dirt" => "dirt".into(),
        "sprint" | "short" => "sprint".into(),
        "mile" => "mile".into(),
        "medium" => "medium".into(),
        "long" => "long".into(),
        "front runner" | "front" | "nige" => "front".into(),
        "pace chaser" | "pace" | "senkou" => "pace".into(),
        "late surger" | "late" | "sashi" => "late".into(),
        "end closer" | "end" | "oikomi" => "end".into(),
        other => other.replace(' ', "_"),
    }
}
