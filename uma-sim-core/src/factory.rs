//! Builds sim engine resources from repo-root research tables and canonical KB.

use crate::catalog::event::{self, EventCatalog, FileEventCatalog};
use crate::catalog::factor::FactorCatalog;
use crate::catalog::support::SupportCatalog;
use crate::catalog::trainee::TraineeCatalog;
use crate::config::{
    BondGainConfig, EventProbabilityConfig, EventRewardSchemaConfig, FacilityLevelConfig,
    HintProgressionConfig, InspirationConfig, MoodEnergyConfig, RaceOutcomeConfig,
    ScenarioResearchConfig, TrainingFailureConfig,
};
use crate::content::{ContentPackLoader, ContentPackRegistry};
use crate::deck::DeckSupportBridge;
use crate::legacy::{LegacyDeckConfig, LegacyFactorContext, LegacyFactorMeta};
use crate::scenario::grand_live_catalog::{
    GrandLiveCalibrationLoader, GrandLiveCatalogLoader, GrandLiveCommunityLoader,
};
use crate::training::{TrainingGainContext, TrainingResolver};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

static INITIALIZED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// All research/*.json artifacts in the repo (16 files).
pub const RESEARCH_FILES: [&str; 16] = [
    "bond_gain.json",
    "event_probabilities.json",
    "event_reward_schema.json",
    "grand_concert.json",
    "grand_concert_calibration.json",
    "grand_concert_community.json",
    "hint_progression.json",
    "inspiration.json",
    "legacy_deck_schema.json",
    "mood_energy.json",
    "race_outcomes.json",
    "trackblazer.json",
    "training_failure.json",
    "training_gain_tables.json",
    "unity_cup.json",
    "ura_finale.json",
];

fn looks_like_repo_root(dir: &Path) -> bool {
    dir.join("knowledge/canonical/by_kind/event_local.json")
        .exists()
}

pub fn detect_repo_root() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("UMA_REPO_ROOT") {
        let pb = PathBuf::from(p);
        if looks_like_repo_root(&pb) {
            return Some(pb);
        }
    }
    let cwd = std::env::current_dir().ok()?;
    // Private GrokWiring layout: public sim lives under ./sim
    let nested = cwd.join("sim");
    if looks_like_repo_root(&nested) {
        return Some(nested);
    }
    let mut dir = cwd;
    for _ in 0..8 {
        if looks_like_repo_root(&dir) {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

fn read_research_file(repo_root: Option<&Path>, name: &str) -> Option<String> {
    let root = repo_root?;
    let path = root.join("research").join(name);
    if path.exists() {
        std::fs::read_to_string(path).ok()
    } else {
        None
    }
}

pub fn load_research(repo_root: Option<&Path>, name: &str) -> Option<String> {
    read_research_file(repo_root, name)
}

pub fn load_training_tables(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "training_gain_tables.json")
}

pub fn load_event_probabilities(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "event_probabilities.json")
}

pub fn load_race_outcomes(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "race_outcomes.json")
}

pub fn load_hint_progression(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "hint_progression.json")
}

pub fn load_inspiration(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "inspiration.json")
}

pub fn load_bond_gain(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "bond_gain.json")
}

pub fn load_training_failure(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "training_failure.json")
}

pub fn load_legacy_deck_schema(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "legacy_deck_schema.json")
}

pub fn load_mood_energy(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "mood_energy.json")
}

pub fn load_event_reward_schema(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "event_reward_schema.json")
}

pub fn load_grand_concert_community(repo_root: Option<&Path>) -> Option<String> {
    read_research_file(repo_root, "grand_concert_community.json")
}

fn load_scenario_research(repo_root: Option<&Path>) {
    let Some(root) = repo_root else {
        return;
    };
    for (file, id) in [
        ("ura_finale.json", "ura"),
        ("grand_concert.json", "grand_concert"),
        ("unity_cup.json", "unity"),
        ("trackblazer.json", "trackblazer"),
    ] {
        let path = root.join("research").join(file);
        if path.exists() {
            if let Ok(text) = std::fs::read_to_string(path) {
                ScenarioResearchConfig::load_scenario_json(id, Some(&text));
            }
        }
    }
}

/// Load all 16 research/*.json files into config, scenario, and catalog loaders.
pub fn load_all_research(repo_root: Option<&Path>) {
    let tables_json = load_training_tables(repo_root);
    TrainingResolver::install_tables(tables_json.as_deref());
    if let Some(ref text) = tables_json {
        FacilityLevelConfig::load_from_json(Some(text));
    }

    EventProbabilityConfig::load_from_json(load_event_probabilities(repo_root).as_deref());
    RaceOutcomeConfig::load_from_json(load_race_outcomes(repo_root).as_deref());
    HintProgressionConfig::load_from_json(load_hint_progression(repo_root).as_deref());
    InspirationConfig::load_from_json(load_inspiration(repo_root).as_deref());
    BondGainConfig::load_from_json(load_bond_gain(repo_root).as_deref());
    TrainingFailureConfig::load_from_json(load_training_failure(repo_root).as_deref());
    LegacyDeckConfig::load_from_json(load_legacy_deck_schema(repo_root).as_deref());
    MoodEnergyConfig::load_from_json(load_mood_energy(repo_root).as_deref());
    EventRewardSchemaConfig::load_from_json(load_event_reward_schema(repo_root).as_deref());

    load_scenario_research(repo_root);
    GrandLiveCalibrationLoader::init_from_repo(repo_root);
    if let Some(text) = load_grand_concert_community(repo_root) {
        GrandLiveCommunityLoader::parse(&text);
    }
}

/// Initialize catalogs, research configs, training tables, and event catalog from repo.
pub fn init_engine_resources(repo_root: Option<&Path>, use_file_events: bool) {
    {
        let mut init = INITIALIZED.lock().unwrap();
        if *init {
            return;
        }
        *init = true;
    }

    FactorCatalog::init_from_repo(repo_root);
    LegacyFactorContext::set_lookup(Some(factor_lookup));
    SupportCatalog::init_from_repo(repo_root);
    init_support_bridge();
    TraineeCatalog::init_from_repo(repo_root);
    TrainingGainContext::set_trainee_growth_lookup(Some(trainee_growth_lookup));

    load_all_research(repo_root);
    GrandLiveCatalogLoader::init_from_repo(repo_root);

    if use_file_events {
        if let Some(root) = repo_root {
            let path = FileEventCatalog::default_path(root);
            let base = FileEventCatalog::load(&path);
            let mut pack_events = ContentPackLoader::load_events(root);
            pack_events.extend(ContentPackRegistry::all());
            let merged = base.merge(pack_events);
            if merged.event_count() > 0 {
                event::install_event_catalog(Arc::new(merged));
            } else {
                event::install_event_catalog(Arc::new(event::BuiltinEventCatalog));
            }
        }
    }
}

/// Convenience: detect repo root and initialize with file-backed events when available.
pub fn init_from_detected_repo(use_file_events: bool) -> Option<PathBuf> {
    let root = detect_repo_root();
    init_engine_resources(root.as_deref(), use_file_events);
    root
}

pub fn training_tables_value() -> Option<Value> {
    TrainingResolver::tables_value()
}

fn init_support_bridge() {
    DeckSupportBridge::set_card_lookup(Some(|id| {
        SupportCatalog::ensure_loaded(detect_repo_root().as_deref());
        SupportCatalog::lookup(id).map(|c| crate::deck::SupportCardMeta {
            card_type: c.card_type,
            friendship_bonus_pct: c.friendship_bonus_pct,
            mood_effect_pct: c.mood_effect_pct,
            training_effectiveness_pct: c.training_effectiveness_pct,
            initial_stat_bonus_pct: c.initial_stat_bonus_pct,
        })
    }));
}

fn factor_lookup(id: &str) -> Option<LegacyFactorMeta> {
    FactorCatalog::lookup(id).map(|m| LegacyFactorMeta {
        id: m.id,
        category: m.category,
        stat_key: m.stat_key,
        skill_id: m.skill_id,
        pink_tag: m.pink_tag,
        race_name: m.race_name,
    })
}

fn trainee_growth_lookup(name: &str, facility: crate::state::TrainingFacility) -> f64 {
    TraineeCatalog::growth_pct(name, facility)
}
