//! Loads optional content packs from content_packs directory.

use crate::events::SimEventEntry;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// Runtime-loaded content pack events (merged at engine create).
pub struct ContentPackRegistry;

static REGISTERED_EVENTS: LazyLock<Mutex<Vec<SimEventEntry>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl ContentPackRegistry {
    pub fn register(events: Vec<SimEventEntry>) {
        REGISTERED_EVENTS.lock().unwrap().extend(events);
    }

    pub fn clear() {
        REGISTERED_EVENTS.lock().unwrap().clear();
    }

    pub fn all() -> Vec<SimEventEntry> {
        REGISTERED_EVENTS.lock().unwrap().clone()
    }
}

pub struct ContentPackLoader;

impl ContentPackLoader {
    pub fn load_events(repo_root: &Path) -> Vec<SimEventEntry> {
        let dir = repo_root.join("content_packs");
        if !dir.exists() {
            return Vec::new();
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("json")
                    && e.file_name().to_string_lossy() != "example.json"
            })
            .flat_map(|e| Self::parse_pack_events(&e.path()))
            .collect()
    }

    pub fn load_pack_file(path: &Path) -> Vec<SimEventEntry> {
        if path.exists() {
            Self::parse_pack_events(path)
        } else {
            Vec::new()
        }
    }

    fn parse_pack_events(path: &Path) -> Vec<SimEventEntry> {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(root_el) = serde_json::from_str::<Value>(&text) else {
            return Vec::new();
        };
        let arr = match &root_el {
            Value::Object(obj) => obj
                .get("kinds")
                .and_then(|k| k.get("event_local"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            Value::Array(a) => a.clone(),
            _ => return Vec::new(),
        };

        arr.iter()
            .filter_map(|el| {
                let obj = el.as_object()?;
                let payload = obj
                    .get("payload")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_else(|| obj.clone());
                let options: Vec<String> = payload
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .or_else(|| {
                        obj.get("options")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                    })?;
                if options.is_empty() {
                    return None;
                }
                let file_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("pack");
                Some(SimEventEntry {
                    id: obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| format!("pack:{file_name}:{options:?}")),
                    title: obj
                        .get("name_en_official")
                        .or_else(|| obj.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Pack Event")
                        .to_string(),
                    owner_kind: payload
                        .get("owner_kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("shared")
                        .to_string(),
                    owner_name: payload
                        .get("owner_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    options,
                })
            })
            .collect()
    }

    pub fn content_packs_dir(repo_root: &Path) -> PathBuf {
        repo_root.join("content_packs")
    }
}
