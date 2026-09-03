//! Grand Live catalog (songs / techniques).

use crate::scenario::grand_live::{GrandLiveConcertBonus, GrandLiveMechanics};
use crate::state::{CareerState, ScenarioResources, TrainingFacility};
use serde_json::Value;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct GrandLiveSong {
    pub id: String,
    pub song_list_id: i32,
    pub name: String,
    pub costs: std::collections::HashMap<String, i32>,
    pub purchase_bonus_text: String,
    pub concert_bonus: Option<GrandLiveConcertBonus>,
    pub availability_part: i32,
    pub purchasable: bool,
    /// Immediate mastery bonus applied on learn (training multipliers or flat grants).
    pub mastery: GrandLiveMasteryBonus,
}

/// Song lesson mastery (immediate on purchase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrandLiveMasteryBonus {
    None,
    /// Flat one-time stat grant (e.g. Speed +22).
    FlatStat { facility: String, value: i32 },
    /// Flat one-time skill points.
    FlatSkillPoints { value: i32 },
    /// Persistent +N to that facility's training gains.
    TrainStat { facility: String, value: i32 },
    /// Persistent +N skill points on every successful train.
    TrainSkillPoints { value: i32 },
    /// All performance tokens +N (Make Debut mastery).
    AllPerfTokens { value: i32 },
}

#[derive(Debug, Clone)]
pub struct GrandLiveTechnique {
    pub id: String,
    pub name: String,
    pub effect_text: String,
    pub costs: std::collections::HashMap<String, i32>,
    pub category: String,
}

struct CatalogState {
    songs: Vec<GrandLiveSong>,
    techniques: Vec<GrandLiveTechnique>,
    loaded: bool,
}

static CATALOG: LazyLock<Mutex<CatalogState>> = LazyLock::new(|| Mutex::new(CatalogState {
    songs: Vec::new(),
    techniques: Vec::new(),
    loaded: false,
}));

pub struct GrandLiveCatalog;

impl GrandLiveCatalog {
    pub fn install(songs: Vec<GrandLiveSong>, techniques: Vec<GrandLiveTechnique>) {
        let mut c = CATALOG.lock().unwrap();
        c.loaded = !songs.is_empty() || !techniques.is_empty();
        c.songs = songs;
        c.techniques = techniques;
    }

    pub fn loaded() -> bool {
        CATALOG.lock().unwrap().loaded
    }

    pub fn all_songs() -> Vec<GrandLiveSong> {
        let c = CATALOG.lock().unwrap();
        if c.songs.is_empty() {
            builtin_songs()
        } else {
            c.songs.clone()
        }
    }

    pub fn all_techniques() -> Vec<GrandLiveTechnique> {
        let c = CATALOG.lock().unwrap();
        if c.techniques.is_empty() {
            builtin_techniques()
        } else {
            c.techniques.clone()
        }
    }

    /// Songs eligible for purchase (owned filter + affordable).
    pub fn eligible_songs(state: &CareerState) -> Vec<GrandLiveSong> {
        Self::board_songs(state)
            .into_iter()
            .filter(|s| Self::can_afford(&state.scenario_resources, &s.costs))
            .collect()
    }

    /// Techniques eligible for purchase (affordable only).
    pub fn eligible_techniques(state: &CareerState) -> Vec<GrandLiveTechnique> {
        Self::board_techniques(state)
            .into_iter()
            .filter(|t| Self::can_afford(&state.scenario_resources, &t.costs))
            .collect()
    }

    /// Songs that may appear on the lesson board (includes unaffordable).
    pub fn board_songs(state: &CareerState) -> Vec<GrandLiveSong> {
        let part = GrandLiveMechanics::career_part(state);
        Self::all_songs()
            .into_iter()
            .filter(|s| s.purchasable && s.availability_part <= part)
            .filter(|s| !GrandLiveMechanics::owns_song(&state.scenario_resources, s.song_list_id))
            .collect()
    }

    /// Techniques that may appear on the lesson board (includes unaffordable).
    /// Filtered by concert-phase technique tiers (uma.guide / community research).
    pub fn board_techniques(state: &CareerState) -> Vec<GrandLiveTechnique> {
        let concerts_done = state.scenario_resources.get("concert_index");
        Self::all_techniques()
            .into_iter()
            .filter(|t| technique_unlocked_for_concerts(t, concerts_done))
            .collect()
    }

    pub fn find_song(id: &str) -> Option<GrandLiveSong> {
        Self::all_songs()
            .into_iter()
            .find(|s| s.id == id || s.song_list_id.to_string() == id)
    }

    pub fn find_technique(id: &str) -> Option<GrandLiveTechnique> {
        Self::all_techniques()
            .into_iter()
            .find(|t| t.id == id || t.id.trim_start_matches("lesson:") == id)
    }

    pub fn can_afford(resources: &ScenarioResources, costs: &std::collections::HashMap<String, i32>) -> bool {
        costs.iter().all(|(perf, amt)| {
            let code = GrandLiveMechanics::perf_code(perf);
            resources.get(&GrandLiveMechanics::perf_resource_key(&code)) >= *amt
        })
    }

    pub fn pay_costs(
        resources: &ScenarioResources,
        costs: &std::collections::HashMap<String, i32>,
    ) -> ScenarioResources {
        GrandLiveMechanics::pay_perf_tokens(resources, costs)
    }
}

/// uma.guide technique tiers by concerts completed:
/// before 1st → Stat/SP +5; before 2nd–4th → +8 / 2×Stat+4; before Grand → +12 / 2×Stat+6.
/// Recovery + skill hints are always available.
fn technique_unlocked_for_concerts(tech: &GrandLiveTechnique, concerts_done: i32) -> bool {
    let need = min_concerts_for_technique(tech);
    concerts_done >= need
}

fn min_concerts_for_technique(tech: &GrandLiveTechnique) -> i32 {
    match tech.category.as_str() {
        "recovery" | "skill_hint" => 0,
        _ => {
            let (max_stat, n_stats, sp) = parse_stat_sp_effect(&tech.effect_text);
            if n_stats >= 2 {
                // 2×Stat+4 mid; 2×Stat+6/+8 late
                return if max_stat >= 6 { 4 } else { 1 };
            }
            if max_stat > 0 && sp > 0 {
                // Stat+N + Skill Pts +N
                return if max_stat >= 6 { 4 } else { 1 };
            }
            if sp >= 12 || max_stat >= 12 {
                return 4;
            }
            if sp >= 8 || max_stat >= 8 {
                return 1;
            }
            0
        }
    }
}

fn parse_stat_sp_effect(effect: &str) -> (i32, i32, i32) {
    let lower = effect.to_lowercase();
    let mut max_stat = 0_i32;
    let mut n_stats = 0_i32;
    for name in ["speed", "stamina", "power", "guts", "wit"] {
        if let Some(v) = extract_plus_after(&lower, name) {
            n_stats += 1;
            max_stat = max_stat.max(v);
        }
    }
    let mut sp = 0_i32;
    for key in ["skill pts", "skill points", "skill pt"] {
        if let Some(v) = extract_plus_after(&lower, key) {
            sp = sp.max(v);
        }
    }
    (max_stat, n_stats, sp)
}

fn extract_plus_after(hay: &str, key: &str) -> Option<i32> {
    let mut search_from = 0;
    let mut best = None;
    while let Some(rel) = hay[search_from..].find(key) {
        let abs = search_from + rel + key.len();
        let rest = hay[abs..].trim_start();
        let rest = rest.strip_prefix('+').unwrap_or(rest).trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i32>() {
            best = Some(best.map_or(v, |b: i32| b.max(v)));
        }
        search_from = abs;
        if search_from >= hay.len() {
            break;
        }
    }
    best
}

fn builtin_songs() -> Vec<GrandLiveSong> {
    vec![
        GrandLiveSong {
            id: "song:3".into(),
            song_list_id: 3,
            name: "Here Comes Our Time".into(),
            costs: std::collections::HashMap::from([("Vocal".into(), 32), ("Mental".into(), 12)]),
            purchase_bonus_text: "Power +22".into(),
            concert_bonus: Some(GrandLiveConcertBonus {
                effect: "Friendship Training Effectiveness".into(),
                value: 5,
            }),
            availability_part: 1,
            purchasable: true,
            mastery: GrandLiveMasteryBonus::FlatStat {
                facility: "power".into(),
                value: 22,
            },
        },
        GrandLiveSong {
            id: "song:4".into(),
            song_list_id: 4,
            name: "Full Speed Ahead!".into(),
            costs: std::collections::HashMap::from([("Dance".into(), 24), ("Passion".into(), 12)]),
            purchase_bonus_text: "Speed +20".into(),
            concert_bonus: Some(GrandLiveConcertBonus {
                effect: "Specialty Rate Up".into(),
                value: 5,
            }),
            availability_part: 1,
            purchasable: true,
            mastery: GrandLiveMasteryBonus::FlatStat {
                facility: "speed".into(),
                value: 20,
            },
        },
    ]
}

fn builtin_techniques() -> Vec<GrandLiveTechnique> {
    vec![
        GrandLiveTechnique {
            id: "lesson:11001".into(),
            name: "Dance Step Basics".into(),
            effect_text: "Speed +5".into(),
            costs: std::collections::HashMap::from([("Dance".into(), 10)]),
            category: "stat".into(),
        },
        GrandLiveTechnique {
            id: "lesson:11003".into(),
            name: "Vocal Training Basics".into(),
            effect_text: "Power +5".into(),
            costs: std::collections::HashMap::from([("Vocal".into(), 10)]),
            category: "stat".into(),
        },
    ]
}

// --- JVM loaders (song.json, lesson.json, grand_concert_calibration.json) ---

pub struct GrandLiveCatalogLoader;

impl GrandLiveCatalogLoader {
    pub fn init_from_repo(repo_root: Option<&Path>) {
        let Some(root) = repo_root else {
            return;
        };
        let songs = Self::load_songs(&root.join("knowledge/canonical/by_kind/song.json"));
        let techniques = Self::load_techniques(&root.join("knowledge/canonical/by_kind/lesson.json"));
        if !songs.is_empty() || !techniques.is_empty() {
            GrandLiveCatalog::install(songs, techniques);
        }
    }

    pub fn load_songs(path: &Path) -> Vec<GrandLiveSong> {
        if !path.exists() {
            return Vec::new();
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(Value::Array(arr)) = serde_json::from_str(&text) else {
            return Vec::new();
        };
        arr.iter()
            .filter_map(|el| el.as_object().and_then(parse_song))
            .collect()
    }

    pub fn load_techniques(path: &Path) -> Vec<GrandLiveTechnique> {
        if !path.exists() {
            return Vec::new();
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(Value::Array(arr)) = serde_json::from_str(&text) else {
            return Vec::new();
        };
        arr.iter()
            .filter_map(|el| el.as_object().and_then(parse_technique))
            .collect()
    }
}

fn parse_song(obj: &serde_json::Map<String, Value>) -> Option<GrandLiveSong> {
    let payload = obj.get("payload")?.as_object()?;
    let cost_el = payload.get("cost")?.as_object()?;
    let costs: std::collections::HashMap<String, i32> = cost_el
        .iter()
        .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n as i32)))
        .filter(|(_, v)| *v > 0)
        .collect();
    if costs.is_empty() {
        return None;
    }
    let bonus = payload.get("purchase_bonus").and_then(|v| v.as_object());
    let mastery = parse_mastery_bonus(bonus);
    let purchase_bonus_text = mastery_to_event_text(&mastery);
    let concert_bonus = payload
        .get("successful_live_bonus")
        .and_then(|v| v.as_object())
        .map(|b| GrandLiveConcertBonus {
            effect: b
                .get("effect")
                .and_then(|v| v.as_str())
                .unwrap_or("Bonus")
                .to_string(),
            value: b.get("value").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        });
    Some(GrandLiveSong {
        id: obj.get("id")?.as_str()?.to_string(),
        song_list_id: payload.get("song_list_id")?.as_i64()? as i32,
        name: obj
            .get("name_en_official")
            .and_then(|v| v.as_str())
            .unwrap_or("Song")
            .to_string(),
        costs,
        purchase_bonus_text,
        concert_bonus,
        availability_part: payload
            .get("availability_part")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32,
        purchasable: payload
            .get("purchasable")
            .and_then(|v| v.as_str())
            .map(|s| s != "false")
            .unwrap_or(true),
        mastery,
    })
}

fn parse_mastery_bonus(
    bonus: Option<&serde_json::Map<String, Value>>,
) -> GrandLiveMasteryBonus {
    let Some(b) = bonus else {
        return GrandLiveMasteryBonus::None;
    };
    let effect = b
        .get("effect")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let value = b.get("value").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if value == 0 {
        return GrandLiveMasteryBonus::None;
    }
    if effect.contains("extra skill") {
        return GrandLiveMasteryBonus::TrainSkillPoints { value };
    }
    if effect.contains("extra stat") {
        let facility = map_stat_name_to_facility(
            b.get("stat").and_then(|v| v.as_str()).unwrap_or(""),
        );
        return GrandLiveMasteryBonus::TrainStat { facility, value };
    }
    if effect.contains("skill pt") {
        return GrandLiveMasteryBonus::FlatSkillPoints { value };
    }
    if effect.contains("all point") || effect.contains("all performance") {
        return GrandLiveMasteryBonus::AllPerfTokens { value };
    }
    for (name, fac) in [
        ("speed", "speed"),
        ("stamina", "stamina"),
        ("power", "power"),
        ("guts", "guts"),
        ("wit", "wit"),
        ("wisdom", "wit"),
        ("intelligence", "wit"),
    ] {
        if effect == name || effect.starts_with(name) {
            return GrandLiveMasteryBonus::FlatStat {
                facility: fac.into(),
                value,
            };
        }
    }
    GrandLiveMasteryBonus::None
}

fn map_stat_name_to_facility(stat: &str) -> String {
    match stat.to_lowercase().as_str() {
        "speed" => "speed".into(),
        "stamina" => "stamina".into(),
        "power" => "power".into(),
        "guts" => "guts".into(),
        "wit" | "wisdom" | "intelligence" => "wit".into(),
        _ => "speed".into(),
    }
}

fn mastery_to_event_text(mastery: &GrandLiveMasteryBonus) -> String {
    match mastery {
        GrandLiveMasteryBonus::None => String::new(),
        GrandLiveMasteryBonus::FlatStat { facility, value } => {
            format!("{} +{value}", facility_display(facility))
        }
        GrandLiveMasteryBonus::FlatSkillPoints { value } => {
            format!("Skill points +{value}")
        }
        GrandLiveMasteryBonus::TrainStat { facility, value } => {
            format!("{} Training +{value}", facility_display(facility))
        }
        GrandLiveMasteryBonus::TrainSkillPoints { value } => {
            format!("Training Skill Pt Gain +{value}")
        }
        GrandLiveMasteryBonus::AllPerfTokens { value } => {
            format!("All performance +{value}")
        }
    }
}

fn facility_display(facility: &str) -> &'static str {
    match facility {
        "speed" => "Speed",
        "stamina" => "Stamina",
        "power" => "Power",
        "guts" => "Guts",
        "wit" => "Wit",
        _ => "Speed",
    }
}

fn parse_technique(obj: &serde_json::Map<String, Value>) -> Option<GrandLiveTechnique> {
    let payload = obj.get("payload")?.as_object()?;
    let cost_arr = payload.get("cost")?.as_array()?;
    let mut costs = std::collections::HashMap::new();
    for el in cost_arr {
        let c = el.as_object()?;
        let perf = c.get("performance")?.as_str()?;
        let value = c.get("value")?.as_i64()? as i32;
        costs.insert(perf.to_string(), value);
    }
    if costs.is_empty() {
        return None;
    }
    Some(GrandLiveTechnique {
        id: obj.get("id")?.as_str()?.to_string(),
        name: obj
            .get("name_en_official")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("title").and_then(|v| v.as_str()))
            .unwrap_or("Technique")
            .to_string(),
        effect_text: payload
            .get("effect")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        costs,
        category: payload
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("stat")
            .to_string(),
    })
}

#[derive(Clone)]
struct TokenGainRow {
    facility: TrainingFacility,
    level: i32,
    deck_size: i32,
    scenario_links: i32,
    gains: std::collections::HashMap<String, i32>,
}

struct CalibrationState {
    rows: Vec<TokenGainRow>,
    loaded: bool,
    confidence_tier: String,
}

static CALIBRATION: LazyLock<Mutex<CalibrationState>> = LazyLock::new(|| {
    Mutex::new(CalibrationState {
        rows: Vec::new(),
        loaded: false,
        confidence_tier: "estimated".into(),
    })
});

pub struct GrandLiveCalibrationLoader;

impl GrandLiveCalibrationLoader {
    pub fn init_from_repo(repo_root: Option<&Path>) {
        let Some(root) = repo_root else {
            return;
        };
        Self::load_path(&root.join("research/grand_concert_calibration.json"));
    }

    pub fn load_path(path: &Path) {
        if !path.exists() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            Self::parse(&text);
        }
    }

    pub fn loaded() -> bool {
        CALIBRATION.lock().unwrap().loaded
    }

    pub fn confidence_tier() -> String {
        CALIBRATION.lock().unwrap().confidence_tier.clone()
    }

    pub fn lookup(
        facility: TrainingFacility,
        level: i32,
        deck_size: i32,
        scenario_links: i32,
    ) -> Option<std::collections::HashMap<String, i32>> {
        let state = CALIBRATION.lock().unwrap();
        if state.rows.is_empty() {
            return None;
        }
        let clamped_deck = deck_size.clamp(0, 5);
        let clamped_level = level.clamp(1, 5);
        let clamped_links = scenario_links.max(0);
        state
            .rows
            .iter()
            .find(|row| {
                row.facility == facility
                    && row.level == clamped_level
                    && row.deck_size == clamped_deck
                    && row.scenario_links == clamped_links
            })
            .or_else(|| {
                state.rows.iter().find(|row| {
                    row.facility == facility
                        && row.level == clamped_level
                        && row.deck_size == clamped_deck
                })
            })
            .or_else(|| {
                state
                    .rows
                    .iter()
                    .find(|row| row.facility == facility && row.level == clamped_level)
            })
            .map(|row| row.gains.clone())
    }

    fn parse(json_text: &str) {
        let Ok(root) = serde_json::from_str::<Value>(json_text) else {
            return;
        };
        {
            let mut state = CALIBRATION.lock().unwrap();
            state.confidence_tier = root
                .get("confidence_tier")
                .and_then(|v| v.as_str())
                .unwrap_or("estimated")
                .to_string();
        }
        Self::parse_token_rows(&root);
        Self::parse_technique_gate(&root);
        Self::parse_concert_rewards(&root);
        Self::parse_cycle_max(&root);
    }

    fn parse_token_rows(root: &Value) {
        let Some(arr) = root.get("training_token_gains").and_then(|v| v.as_array()) else {
            return;
        };
        let parsed: Vec<TokenGainRow> = arr
            .iter()
            .filter_map(|el| {
                let obj = el.as_object()?;
                let fac_name = obj.get("facility")?.as_str()?;
                let facility = TrainingFacility::ALL
                    .iter()
                    .find(|f| f.name().eq_ignore_ascii_case(fac_name))?;
                let level = obj.get("level")?.as_i64()? as i32;
                let deck = obj.get("deck_size").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let links = obj
                    .get("scenario_links")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let gains_obj = obj.get("gains")?.as_object()?;
                let gains: std::collections::HashMap<String, i32> = gains_obj
                    .iter()
                    .filter_map(|(code, v)| v.as_i64().map(|n| (code.clone(), n as i32)))
                    .collect();
                if gains.is_empty() {
                    return None;
                }
                Some(TokenGainRow {
                    facility: *facility,
                    level,
                    deck_size: deck,
                    scenario_links: links,
                    gains,
                })
            })
            .collect();
        if !parsed.is_empty() {
            let mut state = CALIBRATION.lock().unwrap();
            state.rows = parsed;
            state.loaded = true;
            GrandLiveMechanics::install_calibration(Some(calibration_lookup));
        }
    }

    fn parse_technique_gate(root: &Value) {
        let Some(gate) = root.get("technique_gate_sequences").and_then(|v| v.as_object()) else {
            return;
        };
        GrandLiveMechanics::load_community_calibration(
            parse_int_array(gate.get("before_1st_promo")),
            parse_int_array(gate.get("before_2nd_through_4th")),
            parse_int_array(gate.get("before_grand")),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }

    fn parse_concert_rewards(root: &Value) {
        let Some(rewards) = root.get("concert_rewards").and_then(|v| v.as_object()) else {
            return;
        };
        GrandLiveMechanics::load_community_calibration(
            None,
            None,
            None,
            rewards
                .get("great_success_all_stats")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            rewards
                .get("normal_all_stats")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            rewards
                .get("sp_per_technique_since_last")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            rewards
                .get("sp_per_song_since_last")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            rewards
                .get("perf_cap_raise")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            None,
        );
    }

    fn parse_cycle_max(root: &Value) {
        let Some(cycle) = root.get("cycle_songs_per_concert").and_then(|v| v.as_object()) else {
            return;
        };
        if let Some(max) = cycle.get("maximum").and_then(|v| v.as_i64()) {
            GrandLiveMechanics::load_community_calibration(
                None, None, None, None, None, None, None, None, Some(max as i32),
            );
        }
    }
}

fn calibration_lookup(
    facility: TrainingFacility,
    level: i32,
    deck_size: i32,
    scenario_links: i32,
) -> Option<std::collections::HashMap<String, i32>> {
    GrandLiveCalibrationLoader::lookup(facility, level, deck_size, scenario_links)
}

fn parse_int_array(value: Option<&Value>) -> Option<Vec<i32>> {
    value.and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_i64().map(|n| n as i32))
            .collect()
    })
}

// --- Community research (research/grand_concert_community.json) ---

pub struct GrandLiveCommunityLoader;

impl GrandLiveCommunityLoader {
    pub fn init_from_repo(repo_root: Option<&Path>) {
        let Some(root) = repo_root else {
            return;
        };
        Self::load_path(&root.join("research/grand_concert_community.json"));
    }

    pub fn load_path(path: &Path) {
        if !path.exists() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            Self::parse(&text);
        }
    }

    pub fn parse(json_text: &str) {
        let Ok(root) = serde_json::from_str::<Value>(json_text) else {
            return;
        };
        if let Some(gate) = root
            .get("technique_gate_sequences")
            .and_then(|v| v.as_object())
        {
            GrandLiveMechanics::load_community_calibration(
                parse_int_array(gate.get("before_1st_promo")),
                parse_int_array(gate.get("before_2nd_through_4th")),
                parse_int_array(gate.get("before_grand")),
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }
        if let Some(hype) = root.get("hype_and_concerts").and_then(|v| v.as_object()) {
            GrandLiveMechanics::load_community_calibration(
                None,
                None,
                None,
                hype.get("great_success_stat_bonus")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                hype.get("normal_concert_stat_bonus")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                hype.get("sp_between_concerts")
                    .and_then(|v| v.get("per_technique"))
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                hype.get("sp_between_concerts")
                    .and_then(|v| v.get("per_song"))
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                root.get("performance_token_formula")
                    .and_then(|v| v.get("cap_raise_per_concert"))
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                hype.get("songs_recommended_max_per_cycle")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
            );
        }
    }
}
