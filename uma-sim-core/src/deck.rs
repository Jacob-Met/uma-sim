//! Deck parsing, placement, support bridge, and training signals.

use crate::config::BondGainConfig;
use crate::scoring::SupportEffectSlice;
use crate::state::{CareerState, DeckSlot, TrainingFacility};
use std::sync::{LazyLock, Mutex};

pub const MAX_ON_FACILITY: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckSpec {
    pub support_id: String,
    pub facility: Option<String>,
    pub bond: i32,
}

impl DeckSpec {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Self {
                support_id: String::new(),
                facility: None,
                bond: 80,
            };
        }
        let Some(at) = trimmed.find('@') else {
            return Self {
                support_id: trimmed.to_string(),
                facility: None,
                bond: 80,
            };
        };
        let id = trimmed[..at].to_string();
        let tail = &trimmed[at + 1..];
        let colon = tail.rfind(':');
        let maybe_bond = colon.and_then(|c| tail[c + 1..].parse::<i32>().ok());
        let facility_part = if maybe_bond.is_some() {
            colon.map(|c| &tail[..c]).unwrap_or(tail)
        } else {
            tail
        };
        let facility = {
            let f = facility_part.to_lowercase();
            if f.is_empty() { None } else { Some(f) }
        };
        Self {
            support_id: id,
            facility,
            bond: maybe_bond.unwrap_or(80),
        }
    }

    pub fn parse_all(raw: &[String]) -> Vec<Self> {
        raw.iter()
            .map(|s| Self::parse(s))
            .filter(|s| !s.support_id.is_empty())
            .collect()
    }
}

pub struct DeckPlacement;

impl DeckPlacement {
    pub fn assign_by_specialty(slots: &[DeckSlot]) -> Vec<DeckSlot> {
        slots
            .iter()
            .map(|slot| {
                let ty = Self::resolve_type(slot);
                let mut s = slot.clone();
                s.specialty = Some(ty.clone());
                s.assigned_facility = Some(Self::specialty_to_facility(&ty).key().to_string());
                s
            })
            .collect()
    }

    pub fn build_from_specs(specs: &[String]) -> Vec<DeckSlot> {
        DeckSpec::parse_all(specs)
            .into_iter()
            .map(|spec| {
                let base = DeckSlot {
                    support_id: spec.support_id,
                    bond: spec.bond.clamp(0, 100),
                    specialty: None,
                    assigned_facility: None,
                };
                let ty = Self::resolve_type(&base);
                if let Some(fac) = spec.facility {
                    let mut s = base;
                    s.specialty = Some(ty);
                    s.assigned_facility = Some(Self::normalize_facility(&fac));
                    s
                } else {
                    let mut assigned = Self::assign_by_specialty(&[DeckSlot {
                        specialty: Some(ty),
                        ..base
                    }]);
                    assigned.pop().unwrap()
                }
            })
            .collect()
    }

    pub fn reassign(
        slots: &[DeckSlot],
        support_id: &str,
        facility: TrainingFacility,
    ) -> Option<Vec<DeckSlot>> {
        let idx = slots
            .iter()
            .position(|s| s.support_id.eq_ignore_ascii_case(support_id))?;
        let key = facility.key();
        let on_target = slots
            .iter()
            .filter(|s| s.assigned_facility.as_deref() == Some(key))
            .count();
        if on_target >= MAX_ON_FACILITY && slots[idx].assigned_facility.as_deref() != Some(key) {
            return None;
        }
        Some(
            slots
                .iter()
                .enumerate()
                .map(|(i, slot)| {
                    if i == idx {
                        let mut s = slot.clone();
                        s.assigned_facility = Some(key.to_string());
                        s
                    } else {
                        slot.clone()
                    }
                })
                .collect(),
        )
    }

    pub fn normalize_facility(raw: &str) -> String {
        match raw.to_lowercase().as_str() {
            "wit" | "wits" | "intelligence" => "wit".to_string(),
            other => other.to_string(),
        }
    }

    pub fn parse_facility_name(raw: &str) -> Option<TrainingFacility> {
        match raw.to_lowercase().as_str() {
            "speed" => Some(TrainingFacility::Speed),
            "stamina" => Some(TrainingFacility::Stamina),
            "power" => Some(TrainingFacility::Power),
            "guts" => Some(TrainingFacility::Guts),
            "wit" | "wits" | "intelligence" => Some(TrainingFacility::Wit),
            _ => None,
        }
    }

    pub fn resolve_type(slot: &DeckSlot) -> String {
        if let Some(ref s) = slot.specialty {
            return Self::normalize_type(s);
        }
        if let Some(meta) = DeckSupportBridge::card_lookup(&slot.support_id) {
            return Self::normalize_type(&meta.card_type);
        }
        Self::normalize_type(&DeckSupportBridge::infer_specialty_from_id(&slot.support_id))
    }

    pub fn specialty_to_facility(ty: &str) -> TrainingFacility {
        match Self::normalize_type(ty).as_str() {
            "stamina" => TrainingFacility::Stamina,
            "power" => TrainingFacility::Power,
            "guts" => TrainingFacility::Guts,
            "intelligence" | "wit" => TrainingFacility::Wit,
            "friend" | "group" => TrainingFacility::Wit,
            _ => TrainingFacility::Speed,
        }
    }

    /// Daily facility placement with specialty-rate weights (Grand Live Specialty Priority Up).
    ///
    /// Base weight 100 per facility; specialty facility gets +80 + `specialty_bonus_pct`.
    /// Friend/group cards use equal weights. Caps at [`MAX_ON_FACILITY`] per facility.
    pub fn roll_for_turn(
        slots: &[DeckSlot],
        rng: &mut crate::rng::SimRandom,
        specialty_bonus_pct: i32,
    ) -> Vec<DeckSlot> {
        let bonus = specialty_bonus_pct.max(0);
        let mut placed: Vec<DeckSlot> = Vec::with_capacity(slots.len());
        let mut counts = [0usize; 5];
        for slot in slots {
            let ty = Self::resolve_type(slot);
            let is_friend = matches!(ty.as_str(), "friend" | "group");
            let specialty_fac = Self::specialty_to_facility(&ty);
            let mut weights = [100i32; 5];
            if !is_friend {
                let idx = specialty_fac.ordinal() as usize;
                weights[idx] = weights[idx].saturating_add(80 + bonus);
            }
            // Prefer facilities under the cap.
            for (i, w) in weights.iter_mut().enumerate() {
                if counts[i] >= MAX_ON_FACILITY {
                    *w = 0;
                }
            }
            let total: i32 = weights.iter().sum();
            let chosen = if total <= 0 {
                // All full — fall back to specialty or first open.
                TrainingFacility::ALL
                    .into_iter()
                    .find(|f| counts[f.ordinal() as usize] < MAX_ON_FACILITY)
                    .unwrap_or(TrainingFacility::Speed)
            } else {
                let roll = rng.next_int_range(0, total);
                let mut acc = 0;
                let mut pick = TrainingFacility::Speed;
                for (i, &w) in weights.iter().enumerate() {
                    acc += w;
                    if roll < acc {
                        pick = TrainingFacility::ALL[i];
                        break;
                    }
                }
                pick
            };
            counts[chosen.ordinal() as usize] += 1;
            let mut next = slot.clone();
            next.specialty = Some(ty);
            next.assigned_facility = Some(chosen.key().to_string());
            placed.push(next);
        }
        placed
    }

    fn normalize_type(ty: &str) -> String {
        match ty.to_lowercase().as_str() {
            "intelligence" => "intelligence".to_string(),
            "wit" | "wits" => "intelligence".to_string(),
            other => other.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SupportCardMeta {
    pub card_type: String,
    pub friendship_bonus_pct: f64,
    pub mood_effect_pct: f64,
    pub training_effectiveness_pct: f64,
    pub initial_stat_bonus_pct: std::collections::HashMap<String, f64>,
}

type CardLookupFn = fn(&str) -> Option<SupportCardMeta>;

static CARD_LOOKUP: LazyLock<Mutex<Option<CardLookupFn>>> = LazyLock::new(|| Mutex::new(None));

pub struct DeckSupportBridge;

impl DeckSupportBridge {
    pub fn set_card_lookup(lookup: Option<CardLookupFn>) {
        *CARD_LOOKUP.lock().unwrap() = lookup;
    }

    pub fn card_lookup(support_id: &str) -> Option<SupportCardMeta> {
        CARD_LOOKUP.lock().unwrap().and_then(|f| f(support_id))
    }

    pub fn slices_for(state: &CareerState, facility: TrainingFacility) -> Vec<SupportEffectSlice> {
        let on_facility: Vec<_> = state.deck.slots_on_facility(facility).into_iter().cloned().collect();
        if on_facility.is_empty() {
            return Vec::new();
        }
        let fac = Self::facility_to_support_type(facility);
        let cards: Vec<_> = on_facility.iter().map(|s| Self::to_resolved_card(s)).collect();
        let present = on_facility.len().min(MAX_ON_FACILITY) as i32;
        let rainbow = on_facility
            .iter()
            .filter(|slot| {
                let ty = DeckPlacement::resolve_type(slot);
                slot.bond >= BondGainConfig::friendship_threshold()
                    && (ty == fac || ty == "friend" || ty == "group")
            })
            .count() as i32;
        let mut slices = estimate_facility_slices(&fac, &cards, present, rainbow);
        if rainbow > 0 {
            for s in &mut slices {
                s.on_specialty = true;
            }
        }
        slices
    }

    pub fn to_resolved_card(slot: &DeckSlot) -> ResolvedDeckCard {
        let kb = Self::card_lookup(&slot.support_id);
        let specialty = kb
            .as_ref()
            .map(|m| m.card_type.clone())
            .or_else(|| slot.specialty.clone())
            .unwrap_or_else(|| Self::infer_specialty_from_id(&slot.support_id));
        ResolvedDeckCard {
            id: slot.support_id.clone(),
            card_type: specialty,
            level: 30,
            friendship_bonus_pct: kb
                .as_ref()
                .map(|m| m.friendship_bonus_pct)
                .unwrap_or_else(|| friendship_pct(slot.bond)),
            mood_effect_pct: kb
                .as_ref()
                .map(|m| m.mood_effect_pct)
                .unwrap_or(if slot.bond >= 60 { 5.0 } else { 0.0 }),
            training_effectiveness_pct: kb
                .as_ref()
                .map(|m| m.training_effectiveness_pct)
                .unwrap_or(if slot.bond >= 40 { 3.0 } else { 0.0 }),
        }
    }

    pub fn facility_to_support_type(f: TrainingFacility) -> String {
        match f {
            TrainingFacility::Wit => "intelligence".to_string(),
            other => other.key().to_string(),
        }
    }

    pub fn infer_specialty_from_id(support_id: &str) -> String {
        let lower = support_id.to_lowercase();
        if lower.contains("speed") {
            "speed".to_string()
        } else if lower.contains("stamina") || lower.contains("sta") {
            "stamina".to_string()
        } else if lower.contains("power") || lower.contains("pow") {
            "power".to_string()
        } else if lower.contains("guts") || lower.contains("gut") {
            "guts".to_string()
        } else if lower.contains("wit") || lower.contains("intelligence") {
            "intelligence".to_string()
        } else if lower.contains("friend") {
            "friend".to_string()
        } else {
            "speed".to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedDeckCard {
    pub id: String,
    pub card_type: String,
    pub level: i32,
    pub friendship_bonus_pct: f64,
    pub mood_effect_pct: f64,
    pub training_effectiveness_pct: f64,
}

fn friendship_pct(bond: i32) -> f64 {
    const TIERS: [(i32, f64); 4] = [(80, 20.0), (60, 12.0), (40, 8.0), (0, 5.0)];
    TIERS
        .iter()
        .find(|(min, _)| bond >= *min)
        .map(|(_, pct)| *pct)
        .unwrap_or(5.0)
}

pub fn estimate_facility_slices(
    facility: &str,
    deck: &[ResolvedDeckCard],
    present_support_count: i32,
    rainbow_count: i32,
) -> Vec<SupportEffectSlice> {
    if deck.is_empty() || present_support_count <= 0 {
        return Vec::new();
    }
    let fac = facility.to_lowercase();
    let mut sorted: Vec<_> = deck.to_vec();
    sorted.sort_by(|a, b| {
        let score_a = specialty_match_score(&fac, a);
        let score_b = specialty_match_score(&fac, b);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let take = present_support_count.min(sorted.len() as i32) as usize;
    let present = &sorted[..take];
    let rainbow_slots = rainbow_count.clamp(0, present.len() as i32) as usize;
    present
        .iter()
        .enumerate()
        .map(|(index, card)| {
            let on_spec = index < rainbow_slots
                && (card.card_type == fac || card.card_type == "friend" || card.card_type == "group");
            SupportEffectSlice {
                friendship_bonus_pct: card.friendship_bonus_pct,
                mood_effect_pct: card.mood_effect_pct,
                training_effectiveness_pct: card.training_effectiveness_pct,
                on_specialty: on_spec,
            }
        })
        .collect()
}

fn specialty_match_score(fac: &str, card: &ResolvedDeckCard) -> f64 {
    let m = match card.card_type.as_str() {
        t if t == fac => 3.0,
        "friend" | "group" => 2.0,
        _ => 0.0,
    };
    m * 1000.0 + card.friendship_bonus_pct
}

pub struct DeckTrainingSignals;

#[derive(Debug, Clone)]
pub struct BarFillResult {
    pub dominant_color: String,
    pub fill_percent: f64,
    pub is_trainer_support: bool,
}

impl DeckTrainingSignals {
    pub fn relationship_bars(state: &CareerState, facility: TrainingFacility) -> Vec<BarFillResult> {
        state
            .deck
            .slots_on_facility(facility)
            .into_iter()
            .map(|slot| BarFillResult {
                dominant_color: bond_color(slot.bond),
                fill_percent: slot.bond as f64,
                is_trainer_support: DeckPlacement::resolve_type(slot) == "friend",
            })
            .collect()
    }

    pub fn num_rainbow(state: &CareerState, facility: TrainingFacility) -> i32 {
        let fac_type = DeckSupportBridge::facility_to_support_type(facility);
        state
            .deck
            .slots_on_facility(facility)
            .into_iter()
            .filter(|slot| {
                let ty = DeckPlacement::resolve_type(slot);
                slot.bond >= BondGainConfig::friendship_threshold()
                    && (ty == fac_type || ty == "friend" || ty == "group")
            })
            .count() as i32
    }
}

fn bond_color(bond: i32) -> String {
    if bond >= BondGainConfig::friendship_threshold() {
        "blue".to_string()
    } else if bond >= 40 {
        "green".to_string()
    } else {
        "orange".to_string()
    }
}
