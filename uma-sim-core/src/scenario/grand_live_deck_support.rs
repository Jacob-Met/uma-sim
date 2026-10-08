//! Grand Live scenario-link support cards.

use crate::state::{CareerState, TrainingFacility};

pub struct GrandLiveDeckSupport;

impl GrandLiveDeckSupport {
    pub fn is_scenario_link(support_id: &str) -> bool {
        let normalized = support_id
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ");
        // Character-linked cards (any rarity) — IDs are examples for Light Hello SSR.
        const LINK_IDS: [&str; 2] = ["support:30052", "support:10083"];
        if LINK_IDS
            .iter()
            .any(|id| support_id.eq_ignore_ascii_case(id))
        {
            return true;
        }
        const CHARS: [&str; 5] = [
            "light hello",
            "smart falcon",
            "agnes tachyon",
            "silence suzuka",
            "mihono bourbon",
        ];
        CHARS.iter().any(|ch| {
            normalized.contains(ch) || normalized.replace(' ', "").contains(&ch.replace(' ', ""))
        })
    }

    pub fn is_light_hello(support_id: &str) -> bool {
        let normalized = support_id.to_lowercase();
        if normalized.contains("30052") || normalized.contains("10083") {
            return true;
        }
        normalized.contains("light") && normalized.contains("hello")
    }

    pub fn scenario_link_count(state: &CareerState, facility: TrainingFacility) -> i32 {
        state
            .deck
            .slots_on_facility(facility)
            .into_iter()
            .filter(|slot| Self::is_scenario_link(&slot.support_id))
            .count() as i32
    }

    pub fn has_light_hello(state: &CareerState, facility: TrainingFacility) -> bool {
        state
            .deck
            .slots_on_facility(facility)
            .into_iter()
            .any(|slot| Self::is_light_hello(&slot.support_id))
    }

    pub fn any_scenario_link_in_deck(state: &CareerState) -> bool {
        state
            .deck
            .slots
            .iter()
            .any(|slot| Self::is_scenario_link(&slot.support_id))
    }

    pub fn any_light_hello_in_deck(state: &CareerState) -> bool {
        state
            .deck
            .slots
            .iter()
            .any(|slot| Self::is_light_hello(&slot.support_id))
    }

    pub fn light_hello_bond(state: &CareerState) -> i32 {
        state
            .deck
            .slots
            .iter()
            .filter(|slot| Self::is_light_hello(&slot.support_id))
            .map(|slot| slot.bond)
            .max()
            .unwrap_or(0)
    }
}
