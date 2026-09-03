//! Deterministic 3-slot Grand Live lesson board.
//!
//! Mirrors KUC `lesson_choices` from `next_square_info_array`: three slots, may include
//! unaffordable cards, and prefers a song slot once the technique gate unlocks songs.
//! Exact square IDs still come from a shuffled eligible pool (MDB board weights not ingested).

use crate::rng::SimRandom;
use crate::scenario::grand_live::GrandLiveMechanics;
use crate::scenario::grand_live_catalog::{GrandLiveCatalog, GrandLiveSong, GrandLiveTechnique};
use crate::state::{CareerState, TurnPhase};

pub const SLOTS: usize = 3;

#[derive(Debug, Clone)]
pub struct LessonSlot {
    pub action_id: String,
    pub label: String,
    pub is_song: bool,
    /// Packet `square_type`: 1 stat, 2 skill_hint, 3 recovery, 4 song.
    pub square_type: i32,
    pub affordable: bool,
}

pub struct GrandLiveLessonBoard;

impl GrandLiveLessonBoard {
    pub fn current_slots(state: &CareerState) -> Vec<LessonSlot> {
        if state.career_complete || state.phase != TurnPhase::Free.as_str() {
            return Vec::new();
        }
        if GrandLiveMechanics::cycle_songs(&state.scenario_resources)
            >= GrandLiveMechanics::cycle_max()
            && !GrandLiveMechanics::board_is_frozen(&state.scenario_resources)
        {
            return Vec::new();
        }
        // Song saving: unpaid song board persists across the concert.
        if GrandLiveMechanics::board_is_frozen(&state.scenario_resources) {
            let frozen = restore_frozen_slots(state);
            if !frozen.is_empty() {
                return frozen;
            }
        }
        let refresh = state.scenario_resources.get("lesson_refresh");
        let mut rng = SimRandom::new(
            state.meta.seed * 31 + state.turn as i64 * 17 + refresh as i64 * 13,
        );
        let pool = build_pool(state);
        if pool.is_empty() {
            return Vec::new();
        }

    let allow_songs = GrandLiveMechanics::songs_unlocked_on_board(state);
    // 21-song technique pivot (uma.guide): before Grand Concert, do not force a song slot.
    let technique_pivot = state.scenario_resources.get("songs_learned") >= 21
        && state.scenario_resources.get("concert_index") >= 4;
    let mut slots = pick_slots(state, &pool, allow_songs && !technique_pivot, &mut rng);

        // Prefer reserved lesson card in slot 0 when present in the pool.
        if let Some(reserve) = GrandLiveMechanics::reserve_square_id(&state.scenario_resources) {
            let want = format!("gl_tech_{reserve}");
            let want_song = format!("gl_song_{reserve}");
            if let Some(idx) = slots
                .iter()
                .position(|s| s.action_id == want || s.action_id == want_song)
            {
                slots.swap(0, idx);
            } else if let Some(entry) = pool.iter().find(|e| match e {
                PoolEntry::Tech(t) => t.id.trim_start_matches("lesson:") == reserve,
                PoolEntry::Song(s) => s.song_list_id.to_string() == reserve,
            }) {
                let reserved = slot_from_entry(entry, state);
                if slots.len() >= SLOTS {
                    slots[0] = reserved;
                } else {
                    slots.insert(0, reserved);
                    slots.truncate(SLOTS);
                }
            }
        }
        slots
    }

    pub fn find_slot(state: &CareerState, action_id: &str) -> Option<LessonSlot> {
        Self::current_slots(state)
            .into_iter()
            .find(|s| s.action_id == action_id)
    }

    pub fn frozen_contains(resources: &crate::state::ScenarioResources, action_id: &str) -> bool {
        GrandLiveMechanics::frozen_board_action_ids(resources)
            .iter()
            .any(|id| id == action_id)
    }
}

fn restore_frozen_slots(state: &CareerState) -> Vec<LessonSlot> {
    GrandLiveMechanics::frozen_board_action_ids(&state.scenario_resources)
        .into_iter()
        .filter_map(|action_id| {
            if let Some(rest) = action_id.strip_prefix("gl_song_") {
                let song = GrandLiveCatalog::find_song(rest)?;
                return Some(LessonSlot {
                    action_id,
                    label: format!("Song: {} ({})", song.name, format_cost(&song.costs)),
                    is_song: true,
                    square_type: 4,
                    affordable: GrandLiveCatalog::can_afford(
                        &state.scenario_resources,
                        &song.costs,
                    ),
                });
            }
            if let Some(rest) = action_id.strip_prefix("gl_tech_") {
                let tech = GrandLiveCatalog::find_technique(rest)
                    .or_else(|| GrandLiveCatalog::find_technique(&format!("lesson:{rest}")))?;
                return Some(LessonSlot {
                    action_id,
                    label: format!("Technique: {} ({})", tech.name, format_cost(&tech.costs)),
                    is_song: false,
                    square_type: square_type_for_category(&tech.category),
                    affordable: GrandLiveCatalog::can_afford(
                        &state.scenario_resources,
                        &tech.costs,
                    ),
                });
            }
            None
        })
        .collect()
}

#[derive(Clone)]
enum PoolEntry {
    Song(GrandLiveSong),
    Tech(GrandLiveTechnique),
}

fn build_pool(state: &CareerState) -> Vec<PoolEntry> {
    // Board composition includes unaffordable lessons (API `affordable: false`).
    let allow_songs = GrandLiveMechanics::songs_unlocked_on_board(state);
    let technique_pivot = state.scenario_resources.get("songs_learned") >= 21
        && state.scenario_resources.get("concert_index") >= 4;
    let songs = if allow_songs || technique_pivot {
        // Pivot still offers remaining songs, but techniques are prioritized in pick_slots.
        GrandLiveCatalog::board_songs(state)
    } else {
        Vec::new()
    };
    let techniques = GrandLiveCatalog::board_techniques(state);
    songs
        .into_iter()
        .map(PoolEntry::Song)
        .chain(techniques.into_iter().map(PoolEntry::Tech))
        .collect()
}

fn pick_slots(
    state: &CareerState,
    pool: &[PoolEntry],
    allow_songs: bool,
    rng: &mut SimRandom,
) -> Vec<LessonSlot> {
    let songs: Vec<PoolEntry> = pool
        .iter()
        .filter(|e| matches!(e, PoolEntry::Song(_)))
        .cloned()
        .collect();
    let techs: Vec<PoolEntry> = pool
        .iter()
        .filter(|e| matches!(e, PoolEntry::Tech(_)))
        .cloned()
        .collect();

    let mut chosen: Vec<PoolEntry> = Vec::with_capacity(SLOTS);

    // When songs are unlocked, reserve one slot for a song if any exist.
    if allow_songs && !songs.is_empty() {
        let idx = rng.next_int_range(0, songs.len() as i32) as usize;
        chosen.push(songs[idx].clone());
    }

    // Fill with category-diverse techniques first.
    let mut used_categories = std::collections::HashSet::new();
    let tech_shuffled = shuffled(&techs, rng);
    for entry in tech_shuffled {
        if chosen.len() >= SLOTS {
            break;
        }
        if let PoolEntry::Tech(ref t) = entry {
            if used_categories.insert(t.category.clone()) || chosen.len() + 1 >= SLOTS {
                chosen.push(entry);
            }
        }
    }

    if chosen.len() < SLOTS {
        let rest = shuffled(pool, rng);
        for entry in rest {
            if chosen.len() >= SLOTS {
                break;
            }
            let key = entry_key(&entry);
            if !chosen.iter().any(|c| entry_key(c) == key) {
                chosen.push(entry);
            }
        }
    }

    chosen
        .into_iter()
        .take(SLOTS)
        .map(|e| slot_from_entry(&e, state))
        .collect()
}

fn slot_from_entry(entry: &PoolEntry, state: &CareerState) -> LessonSlot {
    match entry {
        PoolEntry::Song(song) => LessonSlot {
            action_id: format!("gl_song_{}", song.song_list_id),
            label: format!("Song: {} ({})", song.name, format_cost(&song.costs)),
            is_song: true,
            square_type: 4,
            affordable: GrandLiveCatalog::can_afford(&state.scenario_resources, &song.costs),
        },
        PoolEntry::Tech(tech) => {
            let id = tech.id.trim_start_matches("lesson:");
            LessonSlot {
                action_id: format!("gl_tech_{id}"),
                label: format!("Technique: {} ({})", tech.name, format_cost(&tech.costs)),
                is_song: false,
                square_type: square_type_for_category(&tech.category),
                affordable: GrandLiveCatalog::can_afford(&state.scenario_resources, &tech.costs),
            }
        }
    }
}

fn square_type_for_category(category: &str) -> i32 {
    match category {
        "stat" => 1,
        "skill_hint" => 2,
        "recovery" => 3,
        "song" => 4,
        _ => 1,
    }
}

fn entry_key(entry: &PoolEntry) -> String {
    match entry {
        PoolEntry::Song(s) => format!("song:{}", s.song_list_id),
        PoolEntry::Tech(t) => t.id.clone(),
    }
}

fn format_cost(costs: &std::collections::HashMap<String, i32>) -> String {
    costs
        .iter()
        .map(|(k, v)| format!("{}{v}", GrandLiveMechanics::perf_code(k)))
        .collect::<Vec<_>>()
        .join("+")
}

fn shuffled<T: Clone>(items: &[T], rng: &mut SimRandom) -> Vec<T> {
    let mut copy = items.to_vec();
    for i in (1..copy.len()).rev() {
        let j = rng.next_int_range(0, i as i32 + 1) as usize;
        copy.swap(i, j);
    }
    copy
}
