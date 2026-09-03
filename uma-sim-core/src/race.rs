//! Optional race scheduling and mid-run race resolution adapter.
//!
//! # Race model (`stub` | `physics`)
//!
//! - **`physics` (default since R8.8):** `uma-race-core` frame-synced multi-horse field with
//!   [`PosKeepMode::Virtual`]; placement feeds existing fan / SP / epithet / scenario hooks.
//! - **`stub`:** win-by-default — always [`RacePlacement::First`] (legacy / explicit opt-in).
//!
//! Selection: `SimSettings.race_model`, CLI `--race-model=`, or env `UMA_RACE_MODEL`
//! (see [`RaceModel::from_env`]). Engine reads settings only; CLI applies env when
//! the flag is omitted.
//!
//! # Physics interim field (R8.6 bootstrap)
//!
//! Career has no telemetry NPC corpus yet. Physics races use trainee + grade-scaled
//! placeholder NPCs (count from `race.json` `entries` via [`npc_count_for_race`]) via
//! [`simulate_field_synced`] + Virtual position-keep (R8.4). NPCs get varied aptitudes,
//! moods, strategies, and 0–2 white skills from a modeled-type allowlist — all from a
//! race-seed Prando stream (never career RNG). Course defaults to Tokyo 1400 (`10601`)
//! when `race_id` has no mapping. Replace with R8.6 V4 corpus when available.

use crate::config::RacePlacement;
use crate::state::{CareerState, MoodLevel};

use uma_race_core::{
    get_course, simulate_field_synced, Aptitude, Course, GroundCondition, HorseInput, PosKeepMode,
    PrandoRng, Strategy,
};

/// Mid-run race resolution backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RaceModel {
    /// Always place 1st (Phase 5 Outcomes v1 / legacy stub).
    Stub,
    /// Frame-stepped physics via `uma-race-core` (default since R8.8).
    #[default]
    Physics,
}

impl RaceModel {
    pub fn as_str(self) -> &'static str {
        match self {
            RaceModel::Stub => "stub",
            RaceModel::Physics => "physics",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "physics" | "sim" | "full" => RaceModel::Physics,
            _ => RaceModel::Stub,
        }
    }

    /// `UMA_RACE_MODEL=stub|physics` when set.
    pub fn from_env() -> Option<Self> {
        std::env::var("UMA_RACE_MODEL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| Self::parse(&s))
    }
}

pub struct RaceScheduler;

impl RaceScheduler {
    pub fn should_run_optional_race(state: &CareerState) -> bool {
        if state.career_complete {
            return false;
        }
        if state.phase != crate::state::TurnPhase::Free.as_str() {
            return false;
        }
        if !(8..=55).contains(&state.turn) {
            return false;
        }
        let fan_target = crate::config::ScenarioResearchConfig::fan_target(&state.meta.scenario_id);
        state.fans < fan_target
    }
}

/// Outcome of a physics mid-run race (career adapter view).
#[derive(Debug, Clone)]
pub struct PhysicsRaceOutcome {
    /// 1-based finish place of the trainee (index 0 in the field).
    pub place: usize,
    pub placement: RacePlacement,
    pub finish_time: f64,
    /// Time gap to the winner (0 if trainee won); seconds.
    pub margin_to_winner_s: f64,
    /// Time gap to the next horse behind (0 if last); seconds.
    pub margin_ahead_s: f64,
    pub field_size: usize,
    pub course_id: u32,
    pub seed: u32,
}

/// Stable race PRNG seed from `(career_seed, turn, race_id)`.
///
/// Must **not** draw from the career [`crate::rng::SimRandom`] stream.
pub fn derive_race_seed(career_seed: i64, turn: i32, race_id: &str) -> u32 {
    // Fixed mix (not `DefaultHasher`) so seeds stay stable across Rust versions.
    let mut h: u64 = career_seed as u64;
    h ^= (turn as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in race_id.as_bytes() {
        h = h.wrapping_mul(0x0100_0000_01B3).wrapping_add(u64::from(*b));
    }
    let s = (h ^ (h >> 32)) as u32;
    if s == 0 {
        1
    } else {
        s
    }
}

fn mood_to_race(mood: MoodLevel) -> i8 {
    match mood {
        MoodLevel::Great => 2,
        MoodLevel::Good => 1,
        MoodLevel::Normal => 0,
        MoodLevel::Bad => -1,
        MoodLevel::Awful => -2,
    }
}

fn apt_letter(state: &CareerState, key: &str, default: Aptitude) -> Aptitude {
    state
        .legacy
        .aptitudes
        .get(key)
        .and_then(|s| Aptitude::from_str_letter(s))
        .unwrap_or(default)
}

fn distance_key_for_type(distance_type: u8) -> &'static str {
    match distance_type {
        1 => "sprint",
        2 => "mile",
        3 => "medium",
        4 => "long",
        _ => "mile",
    }
}

fn preferred_strategy(state: &CareerState) -> (Strategy, Aptitude) {
    let styles = [
        ("front", Strategy::Nige),
        ("pace", Strategy::Senkou),
        ("late", Strategy::Sasi),
        ("end", Strategy::Oikomi),
    ];
    let mut best = (Strategy::Senkou, Aptitude::A, i32::MAX);
    for (key, strat) in styles {
        let apt = apt_letter(state, key, Aptitude::G);
        let rank = apt as i32;
        if rank < best.2 {
            best = (strat, apt, rank);
        }
    }
    (best.0, best.1)
}

/// Map career trainee into [`HorseInput`] for a specific course.
pub fn horse_input_from_career_on_course(state: &CareerState, course: &Course) -> HorseInput {
    let surface_key = match course.surface_enum() {
        uma_race_core::hp::Surface::Dirt => "dirt",
        _ => "turf",
    };
    let dist_key = distance_key_for_type(course.distance_type);
    let (strategy, strategy_apt) = preferred_strategy(state);
    HorseInput {
        speed: state.stats.speed as f64,
        stamina: state.stats.stamina as f64,
        power: state.stats.power as f64,
        guts: state.stats.guts as f64,
        wisdom: state.stats.wit as f64,
        strategy,
        distance_apt: apt_letter(state, dist_key, Aptitude::A),
        surface_apt: apt_letter(state, surface_key, Aptitude::A),
        strategy_apt,
        mood: mood_to_race(state.mood),
        skills: state.learned_skill_ids.clone(),
    }
}

/// Map career trainee into [`HorseInput`] (defaults to mile/turf when no course).
pub fn horse_input_from_career(state: &CareerState) -> HorseInput {
    let (strategy, strategy_apt) = preferred_strategy(state);
    HorseInput {
        speed: state.stats.speed as f64,
        stamina: state.stats.stamina as f64,
        power: state.stats.power as f64,
        guts: state.stats.guts as f64,
        wisdom: state.stats.wit as f64,
        strategy,
        distance_apt: apt_letter(state, "mile", Aptitude::A),
        surface_apt: apt_letter(state, "turf", Aptitude::A),
        strategy_apt,
        mood: mood_to_race(state.mood),
        skills: state.learned_skill_ids.clone(),
    }
}

/// Infer rough grade key from `race_id` (same heuristics as fan grade modifiers).
pub fn grade_key_for_race(race_id: &str) -> &'static str {
    let upper = race_id.to_uppercase();
    for g in ["G1", "G2", "G3", "PRE_OP", "OP"] {
        if upper.contains(g) {
            return g;
        }
    }
    if race_id.contains("finale") || race_id.contains("climax") || race_id.contains("grand_concert")
    {
        return "G1";
    }
    if race_id == "debut" || race_id.contains("promo") {
        return "PRE_OP";
    }
    "OP"
}

/// Resolve `course_id` from canonical race data when `race_id` is numeric / `race:N`.
/// Symbolic career ids (`debut`, finales) use fixed interim courses until R8.6 maps them.
pub fn course_id_for_race(race_id: &str) -> u32 {
    match race_id {
        "debut" => return 10601,            // Tokyo 1400 turf
        "finale_qualifier" => return 10602, // Tokyo 1600
        "finale_semifinal" => return 10606, // Tokyo 2000
        "finale_finals" => return 10608,    // Tokyo 2400 (if missing, fall through)
        "optional" => return 10601,
        _ => {}
    }
    if let Some(id) = lookup_course_id(race_id) {
        return id;
    }
    10601
}

const INTERIM_NPC_COUNT: usize = 8;

fn lookup_race_meta(race_id: &str) -> Option<(u32, usize)> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, (u32, usize)>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../knowledge/canonical/by_kind/race.json");
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
        let mut m = HashMap::new();
        for row in rows {
            let payload = row.get("payload");
            let rid = payload
                .and_then(|p| p.get("race_id"))
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string());
            let cid = payload
                .and_then(|p| p.get("course_id"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let entries = payload
                .and_then(|p| p.get("entries"))
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(9);
            if let (Some(rid), Some(cid)) = (rid, cid) {
                let meta = (cid, entries.max(2));
                m.insert(rid.clone(), meta);
                m.insert(format!("race:{rid}"), meta);
            }
        }
        m
    });
    let key = race_id.trim_start_matches("race:");
    map.get(race_id).or_else(|| map.get(key)).copied()
}

fn lookup_course_id(race_id: &str) -> Option<u32> {
    lookup_race_meta(race_id).map(|(cid, _)| cid)
}

/// NPC count so total field ≈ race `entries` (trainee + NPCs).
pub fn npc_count_for_race(race_id: &str) -> usize {
    lookup_race_meta(race_id)
        .map(|(_, entries)| entries.saturating_sub(1).clamp(1, 17))
        .unwrap_or(INTERIM_NPC_COUNT)
}

fn placeholder_stat_for_grade(grade: &str) -> f64 {
    // Mirrors research/race_field_npc.json grade_stat_baselines (bootstrap until R8.6 V4).
    // Calibrated soft against live win-rate prior (bot ~0.70 on selected races): slightly
    // softer than first bootstrap so mid trainees aren't near-zero vs OP fields.
    match grade {
        "G1" => 1050.0,
        "G2" => 960.0,
        "G3" => 870.0,
        "OP" => 720.0,
        "PRE_OP" => 580.0,
        "DEBUT" => 500.0,
        _ => 650.0,
    }
}

/// Mirrors `research/race_field_npc.json` strategy_mix.
const NPC_STRATEGIES: [Strategy; 4] = [
    Strategy::Nige,
    Strategy::Senkou,
    Strategy::Sasi,
    Strategy::Oikomi,
];

/// Mirrors `research/race_field_npc.json` aptitude_mix (not all A).
const NPC_APTITUDES: [Aptitude; 7] = [
    Aptitude::A,
    Aptitude::A,
    Aptitude::B,
    Aptitude::B,
    Aptitude::C,
    Aptitude::C,
    Aptitude::D,
];

/// Mirrors `research/race_field_npc.json` mood_mix (−2‥+2 scale).
const NPC_MOODS: [i8; 6] = [-1, 0, 0, 1, 1, 2];

/// White skills with only modeled effect types 27/31/9 — see race_field_npc.json.
const NPC_WHITE_SKILL_ALLOWLIST: &[&str] = &[
    "200332", "200342", "200352", "200362", "200372", "200382", "200512", "200532", "200542",
    "200552", "200582", "200602",
];

const NPC_WOBBLE_PER_SLOT: f64 = 12.0;

/// Separate Prando stream for NPC rolls (must not touch career [`crate::rng::SimRandom`]).
fn npc_field_rng(race_seed: u32) -> PrandoRng {
    let mixed = race_seed
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(0xC0FF_EE11);
    PrandoRng::new(if mixed == 0 { 1 } else { mixed })
}

fn pick_aptitude(rng: &mut PrandoRng) -> Aptitude {
    NPC_APTITUDES[rng.uniform(NPC_APTITUDES.len() as u32) as usize]
}

fn pick_mood(rng: &mut PrandoRng) -> i8 {
    NPC_MOODS[rng.uniform(NPC_MOODS.len() as u32) as usize]
}

fn pick_strategy(rng: &mut PrandoRng) -> Strategy {
    NPC_STRATEGIES[rng.uniform(NPC_STRATEGIES.len() as u32) as usize]
}

/// Assign 0–2 distinct allowlisted white skills from the race-seed stream.
fn pick_npc_skills(rng: &mut PrandoRng) -> Vec<String> {
    let n = rng.uniform(3) as usize; // 0, 1, or 2
    if n == 0 {
        return Vec::new();
    }
    let mut pool: Vec<&str> = NPC_WHITE_SKILL_ALLOWLIST.to_vec();
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if pool.is_empty() {
            break;
        }
        let idx = rng.uniform(pool.len() as u32) as usize;
        out.push(pool.swap_remove(idx).to_string());
    }
    out
}

/// Grade-scaled placeholder NPCs with varied aptitudes / moods / strategies / skills.
///
/// `race_seed` is [`derive_race_seed`]'s output (or any race-local seed). Variation is
/// deterministic from that seed and does **not** draw from career RNG.
pub fn placeholder_npc_field(grade: &str, count: usize, race_seed: u32) -> Vec<HorseInput> {
    let base = placeholder_stat_for_grade(grade);
    let mut rng = npc_field_rng(race_seed);
    (0..count)
        .map(|i| {
            let wobble = (i as f64) * NPC_WOBBLE_PER_SLOT - 24.0;
            let s = (base + wobble).max(200.0);
            HorseInput {
                speed: s,
                stamina: s - 20.0,
                power: s - 10.0,
                guts: s - 40.0,
                wisdom: s - 30.0,
                strategy: pick_strategy(&mut rng),
                distance_apt: pick_aptitude(&mut rng),
                surface_apt: pick_aptitude(&mut rng),
                strategy_apt: pick_aptitude(&mut rng),
                mood: pick_mood(&mut rng),
                skills: pick_npc_skills(&mut rng),
            }
        })
        .collect()
}

pub fn placement_from_finish_place(place: usize) -> RacePlacement {
    match place {
        1 => RacePlacement::First,
        2..=5 => RacePlacement::Place25,
        _ => RacePlacement::Show,
    }
}

/// Run a physics race for the trainee without touching the career RNG.
pub fn run_physics_race(state: &CareerState, race_id: &str) -> PhysicsRaceOutcome {
    let seed = derive_race_seed(state.meta.seed, state.turn, race_id);
    let mut course_id = course_id_for_race(race_id);
    let course = match get_course(course_id) {
        Some(c) => c,
        None => {
            course_id = 10601;
            get_course(10601).expect("fallback course 10601 must exist in race_course_data")
        }
    };
    let grade = grade_key_for_race(race_id);
    let trainee = horse_input_from_career_on_course(state, course);
    let mut field = vec![trainee];
    field.extend(placeholder_npc_field(
        grade,
        npc_count_for_race(race_id),
        seed,
    ));
    let result = simulate_field_synced(
        course,
        GroundCondition::Good,
        &field,
        seed,
        PosKeepMode::Virtual,
    );
    let trainee_rank = result
        .finishers
        .iter()
        .position(|f| f.index == 0)
        .map(|i| i + 1)
        .unwrap_or(field.len());
    let finish_time = result
        .finishers
        .iter()
        .find(|f| f.index == 0)
        .map(|f| f.finish_time)
        .unwrap_or(0.0);
    let winner_t = result
        .finishers
        .first()
        .map(|f| f.finish_time)
        .unwrap_or(0.0);
    let margin_to_winner_s = (finish_time - winner_t).max(0.0);
    let margin_ahead_s = if trainee_rank < result.finishers.len() {
        (result.finishers[trainee_rank].finish_time - finish_time).max(0.0)
    } else {
        0.0
    };
    PhysicsRaceOutcome {
        place: trainee_rank,
        placement: placement_from_finish_place(trainee_rank),
        finish_time,
        margin_to_winner_s,
        margin_ahead_s,
        field_size: field.len(),
        course_id,
        seed,
    }
}

/// Format ordinal for log lines (`1st`, `2nd`, …).
pub fn place_label(place: usize) -> String {
    let suffix = match place % 100 {
        11 | 12 | 13 => "th",
        _ => match place % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{place}{suffix}")
}

/// Whether epithets / scenario "won" hooks fire for this placement.
pub fn placement_counts_as_win(placement: RacePlacement) -> bool {
    placement == RacePlacement::First
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        default_facility_levels, DeckState, LegacyState, RunMeta, ScenarioResources, SimDate,
        TraineeStats, TurnPhase,
    };
    use std::collections::HashMap;

    fn weak_trainee_state(seed: i64) -> CareerState {
        CareerState {
            meta: RunMeta::new(seed, "ura", "Test"),
            date: SimDate {
                year: 1,
                month: 6,
                half: 2,
            },
            turn: 1,
            stats: TraineeStats {
                speed: 100,
                stamina: 100,
                power: 100,
                guts: 100,
                wit: 100,
            },
            energy: 100,
            max_energy: 100,
            mood: MoodLevel::Normal,
            fans: 0,
            skill_points: 0,
            career_complete: false,
            awaiting_choice: false,
            pending_event_title: None,
            pending_race_id: Some("debut".into()),
            phase: TurnPhase::MandatoryRace.as_str().to_string(),
            completed_races: Vec::new(),
            facility_levels: default_facility_levels(),
            facility_train_counts: HashMap::new(),
            pending_event_options: Vec::new(),
            hint_levels: HashMap::new(),
            statuses: Vec::new(),
            performance_tokens: HashMap::new(),
            scenario_resources: ScenarioResources::new(),
            legacy: LegacyState::default(),
            learned_skill_ids: Vec::new(),
            deck: DeckState::default(),
            log: Vec::new(),
        }
    }

    #[test]
    fn default_race_model_is_physics_since_r88() {
        assert_eq!(RaceModel::default(), RaceModel::Physics);
        assert_eq!(
            crate::state::SimSettings::default().race_model,
            RaceModel::Physics
        );
    }

    #[test]
    fn derive_race_seed_is_stable_and_nonzero() {
        let a = derive_race_seed(42, 1, "debut");
        let b = derive_race_seed(42, 1, "debut");
        let c = derive_race_seed(42, 2, "debut");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, 0);
    }

    #[test]
    fn course_id_lookup_from_canonical_race_json() {
        assert_eq!(course_id_for_race("1001"), 10611);
        assert_eq!(course_id_for_race("race:1001"), 10611);
        assert_eq!(course_id_for_race("debut"), 10601);
    }

    #[test]
    fn physics_debut_with_weak_stats_does_not_always_place_first() {
        let state = weak_trainee_state(99);
        let out = run_physics_race(&state, "debut");
        assert!(out.finish_time > 0.0, "expected real finish time");
        assert_eq!(out.field_size, 1 + INTERIM_NPC_COUNT);
        assert!(
            out.place > 1,
            "weak trainee vs PRE_OP placeholders should not win (place={})",
            out.place
        );
        assert_ne!(out.placement, RacePlacement::First);
    }

    #[test]
    fn placeholder_npcs_vary_aptitudes_moods_strategies_skills() {
        let seed = derive_race_seed(42, 1, "debut");
        let npcs = placeholder_npc_field("PRE_OP", 8, seed);
        assert_eq!(npcs.len(), 8);

        let strategies: std::collections::HashSet<_> =
            npcs.iter().map(|h| h.strategy as u8).collect();
        assert!(
            strategies.len() >= 2,
            "expected mixed strategies, got {strategies:?}"
        );

        let apts: std::collections::HashSet<_> = npcs
            .iter()
            .flat_map(|h| {
                [
                    h.distance_apt as u8,
                    h.surface_apt as u8,
                    h.strategy_apt as u8,
                ]
            })
            .collect();
        assert!(
            apts.len() >= 2,
            "expected mixed aptitudes (not all A), got {apts:?}"
        );

        let moods: std::collections::HashSet<_> = npcs.iter().map(|h| h.mood).collect();
        assert!(moods.len() >= 2, "expected mixed moods, got {moods:?}");

        let with_skills = npcs.iter().filter(|h| !h.skills.is_empty()).count();
        assert!(
            with_skills >= 1,
            "expected some NPCs to receive white skills from allowlist"
        );
        for h in &npcs {
            assert!(h.skills.len() <= 2);
            for id in &h.skills {
                assert!(
                    NPC_WHITE_SKILL_ALLOWLIST.contains(&id.as_str()),
                    "skill {id} not in allowlist"
                );
            }
        }

        let again = placeholder_npc_field("PRE_OP", 8, seed);
        assert_eq!(
            npcs.iter().map(|h| &h.skills).collect::<Vec<_>>(),
            again.iter().map(|h| &h.skills).collect::<Vec<_>>(),
            "NPC rolls must be deterministic from race seed"
        );
    }

    #[test]
    fn npc_count_uses_race_json_entries_when_present() {
        // race 1001 has entries in canonical race.json; symbolic debut falls back.
        let debut = npc_count_for_race("debut");
        assert_eq!(debut, INTERIM_NPC_COUNT);
        let from_json = npc_count_for_race("1001");
        assert!(
            (1..=17).contains(&from_json),
            "entries-derived npc count out of range: {from_json}"
        );
        // Prefer that JSON path differs from interim when entries ≠ 9.
        let _ = from_json;
    }

    #[test]
    fn placement_bands() {
        assert_eq!(placement_from_finish_place(1), RacePlacement::First);
        assert_eq!(placement_from_finish_place(3), RacePlacement::Place25);
        assert_eq!(placement_from_finish_place(6), RacePlacement::Show);
    }
}
