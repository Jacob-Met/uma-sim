use super::formula_gain::SupportEffectSlice;
use crate::state::MoodLevel;
use super::types::StatName;

pub struct SupportLevelBreakpoints;

impl SupportLevelBreakpoints {
    pub const LEVEL_THRESHOLDS: [i32; 11] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50];

    pub fn resolve(breakpoints: &[f64], level: i32) -> f64 {
        let mut resolved = 0.0;
        let capped = level.clamp(1, 50);
        for (i, raw) in breakpoints.iter().enumerate() {
            if i >= Self::LEVEL_THRESHOLDS.len() {
                break;
            }
            if Self::LEVEL_THRESHOLDS[i] > capped {
                break;
            }
            if *raw >= 0.0 {
                resolved = *raw;
            }
        }
        resolved
    }

    pub fn resolve_int_list(breakpoints: &[f64], level: i32) -> f64 {
        Self::resolve(breakpoints, level)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDeckCard {
    pub id: String,
    pub title: String,
    pub char_name: String,
    pub r#type: String,
    pub level: i32,
    pub uncap: i32,
    pub friendship_bonus_pct: f64,
    pub mood_effect_pct: f64,
    pub training_effectiveness_pct: f64,
    pub specialty_priority: f64,
    pub speed_bonus: f64,
    pub stamina_bonus: f64,
    pub power_bonus: f64,
    pub guts_bonus: f64,
    pub wit_bonus: f64,
}

impl Default for ResolvedDeckCard {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            char_name: String::new(),
            r#type: String::new(),
            level: 1,
            uncap: 0,
            friendship_bonus_pct: 0.0,
            mood_effect_pct: 0.0,
            training_effectiveness_pct: 0.0,
            specialty_priority: 0.0,
            speed_bonus: 0.0,
            stamina_bonus: 0.0,
            power_bonus: 0.0,
            guts_bonus: 0.0,
            wit_bonus: 0.0,
        }
    }
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
    let mut specialty_first: Vec<&ResolvedDeckCard> = deck.iter().collect();
    specialty_first.sort_by(|a, b| {
        let match_a = card_type_match(&a.r#type, &fac);
        let match_b = card_type_match(&b.r#type, &fac);
        let score_a = match_a * 1000.0 + a.friendship_bonus_pct + a.specialty_priority;
        let score_b = match_b * 1000.0 + b.friendship_bonus_pct + b.specialty_priority;
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    let present_count = present_support_count.min(deck.len() as i32) as usize;
    let present = &specialty_first[..present_count];
    let rainbow_slots = rainbow_count.clamp(0, present.len() as i32) as usize;
    present
        .iter()
        .enumerate()
        .map(|(index, card)| {
            let on_spec = index < rainbow_slots
                && (card.r#type == fac || card.r#type == "friend" || card.r#type == "group");
            SupportEffectSlice {
                friendship_bonus_pct: card.friendship_bonus_pct,
                mood_effect_pct: card.mood_effect_pct,
                training_effectiveness_pct: card.training_effectiveness_pct,
                on_specialty: on_spec,
            }
        })
        .collect()
}

fn card_type_match(card_type: &str, fac: &str) -> f64 {
    if card_type == fac {
        3.0
    } else if card_type == "friend" || card_type == "group" {
        2.0
    } else {
        0.0
    }
}

pub fn mood_adjust_with_deck(raw_score: f64, mood: MoodLevel, slices: &[SupportEffectSlice]) -> f64 {
    let mut mood_effect_sum = 0.0;
    for s in slices {
        mood_effect_sum += s.mood_effect_pct;
    }
    let mood_mult = 1.0 + mood.base_mood() * (1.0 + mood_effect_sum / 100.0);
    raw_score * mood_mult
}

pub fn deck_specialty_bias(facility: &str, deck: &[ResolvedDeckCard]) -> f64 {
    let fac = facility.to_lowercase();
    let mut bias = 0.0;
    for card in deck {
        let m = match card.r#type.as_str() {
            t if t == fac => 1.0,
            "friend" | "group" => 0.35,
            _ => 0.0,
        };
        if m <= 0.0 {
            continue;
        }
        bias += m
            * (card.friendship_bonus_pct * 0.35
                + card.training_effectiveness_pct * 0.5
                + card.specialty_priority * 0.15);
    }
    bias
}

pub fn stat_name_to_support_type(stat: StatName) -> &'static str {
    match stat {
        StatName::Speed => "speed",
        StatName::Stamina => "stamina",
        StatName::Power => "power",
        StatName::Guts => "guts",
        StatName::Wit => "intelligence",
    }
}
