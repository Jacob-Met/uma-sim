//! Text output for CLI / REST / MCP.

use crate::state::{CareerState, DialogueMode, SimSettings};

pub struct TextRenderer {
    settings: SimSettings,
}

impl TextRenderer {
    pub fn new(settings: SimSettings) -> Self {
        Self { settings }
    }

    pub fn render(&self, state: &CareerState, event_lines: &[String]) -> Vec<String> {
        if self.settings.is_turbo() {
            return vec![format!(
                "T{} Y{}M{} SPD={} E={} F={}",
                state.turn,
                state.date.year,
                state.date.month,
                state.stats.speed,
                state.energy,
                state.fans
            )];
        }

        let mut out = Vec::new();
        out.push(format!("--- State (turn {}) ---", state.turn));
        out.push(format!(
            "Date: Year {}, Month {}, {}",
            state.date.year,
            state.date.month,
            half_label(state.date.half)
        ));
        out.push(format!(
            "Stats: SPD {} STA {} POW {} GUT {} WIT {}",
            state.stats.speed,
            state.stats.stamina,
            state.stats.power,
            state.stats.guts,
            state.stats.wit
        ));
        out.push(format!(
            "Energy: {}/{}  Mood: {:?}  Fans: {}  SP: {}",
            state.energy, state.max_energy, state.mood, state.fans, state.skill_points
        ));
        if !state.scenario_resources.values.is_empty() {
            let parts: Vec<String> = state
                .scenario_resources
                .values
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            out.push(format!("Scenario: {}", parts.join(" ")));
        }

        let mode = self.settings.effective_dialogue_mode();
        if mode != DialogueMode::Off {
            out.extend(event_lines.iter().cloned());
            if mode == DialogueMode::Full {
                if let Some(ref title) = state.pending_event_title {
                    out.push(format!("[Event flavor] {title}"));
                }
            }
        } else if let Some(last) = event_lines.last() {
            out.push(last.clone());
        }

        if self.settings.is_fast() && event_lines.len() > 3 {
            out.push(format!(
                "... ({} lines suppressed at x{})",
                event_lines.len().saturating_sub(2),
                self.settings.clamped_speed()
            ));
        }

        out
    }
}

fn half_label(half: i32) -> &'static str {
    if half == 1 {
        "Early"
    } else {
        "Late"
    }
}
