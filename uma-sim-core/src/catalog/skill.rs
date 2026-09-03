//! Skill name → numeric id lookup from `knowledge/canonical/by_kind/skill.json`.

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

struct SkillCatalogState {
    by_name: HashMap<String, i32>,
    loaded: bool,
}

static STATE: LazyLock<Mutex<SkillCatalogState>> = LazyLock::new(|| {
    Mutex::new(SkillCatalogState {
        by_name: HashMap::new(),
        loaded: false,
    })
});

fn normalize_name(raw: &str) -> String {
    raw.trim()
        .trim_end_matches(['○', '〇', '☆', '★', '◎'])
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '\'' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct SkillCatalog;

impl SkillCatalog {
    pub fn init_from_repo(repo_root: &Path) -> bool {
        let path = repo_root
            .join("knowledge")
            .join("canonical")
            .join("by_kind")
            .join("skill.json");
        Self::load_file(&path)
    }

    pub fn load_file(path: &Path) -> bool {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        Self::load_json(&text)
    }

    pub fn load_json(text: &str) -> bool {
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return false;
        };
        let Some(arr) = root.as_array() else {
            return false;
        };
        let mut by_name = HashMap::new();
        for item in arr {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let id = obj
                .get("payload")
                .and_then(|p| p.get("skill_id"))
                .and_then(|v| v.as_i64())
                .or_else(|| {
                    obj.get("id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.strip_prefix("skill:"))
                        .and_then(|s| s.parse().ok())
                })
                .map(|n| n as i32);
            let Some(sid) = id else {
                continue;
            };
            for key in ["name_en_official", "name_en_fan", "name_ja"] {
                if let Some(name) = obj.get(key).and_then(|v| v.as_str()) {
                    let n = normalize_name(name);
                    if !n.is_empty() {
                        by_name.entry(n).or_insert(sid);
                    }
                }
            }
            if let Some(aliases) = obj.get("aliases").and_then(|v| v.as_array()) {
                for a in aliases {
                    if let Some(name) = a.as_str() {
                        let n = normalize_name(name);
                        if !n.is_empty() {
                            by_name.entry(n).or_insert(sid);
                        }
                    }
                }
            }
        }
        let mut st = STATE.lock().unwrap();
        st.loaded = !by_name.is_empty();
        st.by_name = by_name;
        st.loaded
    }

    pub fn lookup_by_name(name: &str) -> Option<i32> {
        let key = normalize_name(name);
        if key.is_empty() {
            return None;
        }
        STATE.lock().unwrap().by_name.get(&key).copied()
    }

    pub fn is_loaded() -> bool {
        STATE.lock().unwrap().loaded
    }

    /// Test helper: inject a name→id mapping without loading skill.json.
    pub fn insert_for_test(name: &str, id: i32) {
        let mut st = STATE.lock().unwrap();
        st.by_name.insert(normalize_name(name), id);
        st.loaded = true;
    }

    /// Test helper: clear catalog state.
    pub fn clear_for_test() {
        let mut st = STATE.lock().unwrap();
        st.by_name.clear();
        st.loaded = false;
    }
}

/// Resolve an event hint display name to a `skill:{id}` key.
/// Falls back to the trainee's first `skills_event` id when the name is unknown;
/// only empty/generic hints roll the RNG (shared-pool pick).
pub fn resolve_hint_key(
    hint_name: &str,
    skills_event: &[i32],
    rng: Option<&mut crate::rng::SimRandom>,
) -> String {
    if hint_name.is_empty() || hint_name.eq_ignore_ascii_case("generic") {
        return pick_event_skill_key(skills_event, rng).unwrap_or_else(|| "generic".into());
    }
    if let Some(id) = SkillCatalog::lookup_by_name(hint_name) {
        return format!("skill:{id}");
    }
    // Deterministic fallback — do not draw career RNG for unresolved names.
    if let Some(id) = skills_event.first() {
        return format!("skill:{id}");
    }
    hint_name.to_string()
}

fn pick_event_skill_key(
    skills_event: &[i32],
    rng: Option<&mut crate::rng::SimRandom>,
) -> Option<String> {
    if skills_event.is_empty() {
        return None;
    }
    let idx = if let Some(rng) = rng {
        rng.next_int_until(skills_event.len() as i32) as usize
    } else {
        0
    };
    Some(format!("skill:{}", skills_event[idx]))
}
