//! Event catalog loaded from canonical KB and content packs.

use crate::events::SimEventEntry;
use crate::rng::SimRandom;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

pub trait EventCatalog: Send + Sync {
    fn pick_random(
        &self,
        trainee_name: &str,
        turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry>;
    fn event_count(&self) -> usize;
}

pub struct BuiltinEventCatalog;

impl EventCatalog for BuiltinEventCatalog {
    fn pick_random(
        &self,
        trainee_name: &str,
        _turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry> {
        let samples = builtin_samples();
        let pool: Vec<_> = samples
            .into_iter()
            .filter(|e| e.owner_kind == "shared" || e.owner_name.eq_ignore_ascii_case(trainee_name))
            .collect();
        if pool.is_empty() {
            return None;
        }
        let idx = rng.next_int_until(pool.len() as i32) as usize;
        Some(pool[idx].clone())
    }

    fn event_count(&self) -> usize {
        3
    }
}

fn builtin_samples() -> Vec<SimEventEntry> {
    vec![
        SimEventEntry {
            id: "event:trainee:Special Week:fan_letter".into(),
            title: "Fan Letter".into(),
            owner_kind: "trainee".into(),
            owner_name: "Special Week".into(),
            options: vec![
                "Energy +10\nMood +1".into(),
                "Speed +5\nSkill points +15".into(),
            ],
        },
        SimEventEntry {
            id: "event:trainee:Special Week:extra_training".into(),
            title: "Extra Training".into(),
            owner_kind: "trainee".into(),
            owner_name: "Special Week".into(),
            options: vec![
                "Energy -10\nSpeed +15".into(),
                "Energy -5\nStamina +10".into(),
            ],
        },
        SimEventEntry {
            id: "event:shared:failed_training".into(),
            title: "Failed Training (Get Well Soon!)".into(),
            owner_kind: "shared".into(),
            owner_name: String::new(),
            options: vec![
                "Energy +20".into(),
                "Randomly either\n----------\nEnergy +30\n----------\nMood +1".into(),
            ],
        },
    ]
}

pub struct FileEventCatalog {
    events: Vec<SimEventEntry>,
}

impl FileEventCatalog {
    pub fn new(events: Vec<SimEventEntry>) -> Self {
        Self { events }
    }

    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::new(Vec::new());
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::new(Vec::new());
        };
        Self::parse_json(&text)
    }

    pub fn parse_json(json_text: &str) -> Self {
        let Ok(Value::Array(root)) = serde_json::from_str(json_text) else {
            return Self::new(Vec::new());
        };
        let events: Vec<SimEventEntry> = root
            .iter()
            .filter_map(|el| {
                let obj = el.as_object()?;
                let payload = obj.get("payload")?.as_object()?;
                let options: Vec<String> = payload
                    .get("options")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if options.is_empty() {
                    return None;
                }
                Some(SimEventEntry {
                    id: obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    title: obj
                        .get("name_en_official")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Event")
                        .to_string(),
                    owner_kind: payload
                        .get("owner_kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    owner_name: payload
                        .get("owner_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    options,
                })
            })
            .collect();
        Self::new(events)
    }

    pub fn merge(&self, extra: Vec<SimEventEntry>) -> Self {
        let mut merged = self.events.clone();
        merged.extend(extra);
        merged.sort_by(|a, b| a.id.cmp(&b.id));
        merged.dedup_by(|a, b| a.id == b.id);
        Self::new(merged)
    }

    pub fn default_path(repo_root: &Path) -> PathBuf {
        repo_root.join("knowledge/canonical/by_kind/event_local.json")
    }
}

impl EventCatalog for FileEventCatalog {
    fn pick_random(
        &self,
        trainee_name: &str,
        _turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry> {
        let pool: Vec<_> = self
            .events
            .iter()
            .filter(|e| {
                (e.owner_kind == "trainee" && e.owner_name.eq_ignore_ascii_case(trainee_name))
                    || e.owner_kind == "shared"
                    || e.owner_kind == "scenario"
            })
            .filter(|e| !e.title.to_lowercase().contains("victory"))
            .cloned()
            .collect();
        if pool.is_empty() {
            return None;
        }
        let bound = pool.len().min(500);
        let idx = rng.next_int_until(bound as i32) as usize;
        Some(pool[idx].clone())
    }

    fn event_count(&self) -> usize {
        self.events.len()
    }
}

static ACTIVE_CATALOG: LazyLock<Mutex<Option<Arc<dyn EventCatalog>>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn install_event_catalog(catalog: Arc<dyn EventCatalog>) {
    *ACTIVE_CATALOG.lock().unwrap() = Some(catalog);
}

pub fn active_event_catalog() -> Arc<dyn EventCatalog> {
    ACTIVE_CATALOG
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| Arc::new(BuiltinEventCatalog))
}

pub fn pick_random_event(
    trainee_name: &str,
    turn: i32,
    rng: &mut SimRandom,
) -> Option<SimEventEntry> {
    active_event_catalog().pick_random(trainee_name, turn, rng)
}
