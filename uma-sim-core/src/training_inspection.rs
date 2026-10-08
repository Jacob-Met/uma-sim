//! Read-only, deterministic training samples for a player inspecting a career.
//! These are resolver samples, not promised next-action outcomes.

use crate::state::{CareerState, SimChoice, StatName, TrainingFacility, TurnPhase};
use crate::training::{TrainingOutcome, TrainingPreview, TrainingResolver};
use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingUnavailableReason {
    CareerComplete,
    MandatoryRace,
    PendingEvent,
    TrainingUnavailable,
}

impl TrainingUnavailableReason {
    fn message(self) -> &'static str {
        match self {
            Self::CareerComplete => "Career complete; there is no next training action.",
            Self::MandatoryRace => "Enter the mandatory race before training.",
            Self::PendingEvent => "Resolve the pending event choice before training.",
            Self::TrainingUnavailable => "Training is not offered in this phase.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingBlockReason {
    Injured,
    InsufficientEnergy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingStatGains {
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingInspectionRow {
    pub action_id: String,
    pub facility: String,
    pub label: String,
    pub level: i32,
    pub stat_gains: TrainingStatGains,
    /// Positive is required energy; negative is recovery.
    pub energy_cost: i32,
    /// Successful base effect after the ordinary cap, before later effects.
    /// No attempt is available for a blocked row, so its effect/risk are null.
    pub energy_delta: Option<i32>,
    pub failure_chance_pct: Option<i32>,
    pub num_rainbow: i32,
    pub num_skill_hints: i32,
    pub available: bool,
    pub blocked_reason: Option<TrainingBlockReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingInspection {
    pub schema_version: u8,
    pub turn: i32,
    pub date: String,
    pub phase: String,
    pub scenario_id: String,
    pub trainee_name: String,
    pub energy: i32,
    pub max_energy: i32,
    pub mood: String,
    pub sample_kind: &'static str,
    pub notes: Vec<&'static str>,
    pub unavailable_reason: Option<TrainingUnavailableReason>,
    pub rows: Vec<TrainingInspectionRow>,
}

impl TrainingInspection {
    pub(crate) fn build(
        state: &CareerState,
        choices: &[SimChoice],
        resolver: &TrainingResolver,
        effects: impl Fn(i32, &TrainingOutcome) -> (i32, i32),
    ) -> Self {
        let has_training = TrainingFacility::ALL.iter().any(|facility| {
            let action_id = format!("train_{}", facility.key());
            choices.iter().any(|choice| choice.id == action_id)
        });
        let unavailable_reason = if has_training {
            None
        } else if state.career_complete {
            Some(TrainingUnavailableReason::CareerComplete)
        } else if state.phase == TurnPhase::MandatoryRace.as_str() {
            Some(TrainingUnavailableReason::MandatoryRace)
        } else if state.phase == TurnPhase::Event.as_str() {
            Some(TrainingUnavailableReason::PendingEvent)
        } else {
            Some(TrainingUnavailableReason::TrainingUnavailable)
        };

        let mut rows = Vec::new();
        if has_training {
            let options = TrainingPreview::build_options(state, resolver);
            for facility in TrainingFacility::ALL {
                let action_id = format!("train_{}", facility.key());
                if !choices.iter().any(|choice| choice.id == action_id) {
                    continue;
                }
                // build_options always includes every TrainingFacility.
                let option = &options[&facility];
                let level = option.training_level.unwrap_or(1);
                let outcome = resolver.resolve_typical(facility, level, state.mood, Some(state));
                let blocked_reason = if state.is_injured() {
                    Some(TrainingBlockReason::Injured)
                } else if outcome.energy_cost > 0 && state.energy < outcome.energy_cost {
                    Some(TrainingBlockReason::InsufficientEnergy)
                } else {
                    None
                };
                let (failure_chance_pct, energy_delta) = if blocked_reason.is_none() {
                    let (risk, delta) = effects(level, &outcome);
                    (Some(risk), Some(delta))
                } else {
                    (None, None)
                };
                let gain = |stat| option.stat_gains.get(&stat).copied().unwrap_or(0);
                rows.push(TrainingInspectionRow {
                    action_id,
                    facility: facility.key().into(),
                    label: match facility {
                        TrainingFacility::Speed => "Speed",
                        TrainingFacility::Stamina => "Stamina",
                        TrainingFacility::Power => "Power",
                        TrainingFacility::Guts => "Guts",
                        TrainingFacility::Wit => "Wit",
                    }
                    .into(),
                    level,
                    stat_gains: TrainingStatGains {
                        speed: gain(StatName::Speed),
                        stamina: gain(StatName::Stamina),
                        power: gain(StatName::Power),
                        guts: gain(StatName::Guts),
                        wit: gain(StatName::Wit),
                    },
                    energy_cost: outcome.energy_cost,
                    energy_delta,
                    failure_chance_pct,
                    num_rainbow: option.num_rainbow,
                    num_skill_hints: option.num_skill_hints,
                    available: blocked_reason.is_none(),
                    blocked_reason,
                });
            }
        }
        Self {
            schema_version: 1,
            turn: state.turn,
            date: format!(
                "Year {}, Month {}, {}",
                state.date.year,
                state.date.month,
                if state.date.half == 1 { "Early" } else { "Late" }
            ),
            phase: state.phase.clone(),
            scenario_id: state.meta.scenario_id.clone(),
            trainee_name: state.meta.trainee_name.clone(),
            energy: state.energy,
            max_energy: state.max_energy,
            mood: state.mood.kotlin_name().into(),
            sample_kind: "deterministic_seed_0",
            notes: vec![
                "Stat gains are deterministic samples (local seed 0), before stat caps and later scenario/event effects; they are not guaranteed results or expected values.",
                "Energy is the base successful-training change after the ordinary energy cap, before later scenario/event effects. Failure is the current engine chance for an available attempt.",
                "Blocked rows have no energy result or failure roll. Rainbow and hint counts are the existing preview signals.",
                "Inspection does not take an action, advance the career RNG, or save the session.",
            ],
            unavailable_reason,
            rows,
        }
    }

    pub fn render_text(&self) -> String {
        let mut text = format!(
            "Training inspection — {} / {}\nTurn {} | {} | Phase {}\nEnergy {}/{} | Mood {}\n\n",
            self.trainee_name,
            self.scenario_id,
            self.turn,
            self.date,
            self.phase,
            self.energy,
            self.max_energy,
            self.mood
        );
        for note in &self.notes {
            writeln!(text, "{note}").expect("writing to a String");
        }
        text.push('\n');
        if let Some(reason) = self.unavailable_reason {
            writeln!(text, "No training choices: {}", reason.message())
                .expect("writing to a String");
            return text;
        }
        writeln!(
            text,
            "{:<8} {:>2} {:>5} {:>5} {:>5} {:>5} {:>5} {:>7} {:>6} {:>7} {:>5}",
            "Training",
            "Lv",
            "SPD",
            "STA",
            "POW",
            "GUT",
            "WIT",
            "Energy",
            "Fail",
            "Rainbow",
            "Hints"
        )
        .expect("writing to a String");
        for row in &self.rows {
            let energy = row
                .energy_delta
                .map_or_else(|| "—".into(), |n| format!("{n:+}"));
            let failure = row
                .failure_chance_pct
                .map_or_else(|| "—".into(), |n| format!("{n}%"));
            writeln!(
                text,
                "{:<8} {:>2} {:+5} {:+5} {:+5} {:+5} {:+5} {:>7} {:>6} {:>7} {:>5}",
                row.label,
                row.level,
                row.stat_gains.speed,
                row.stat_gains.stamina,
                row.stat_gains.power,
                row.stat_gains.guts,
                row.stat_gains.wit,
                energy,
                failure,
                row.num_rainbow,
                row.num_skill_hints
            )
            .expect("writing to a String");
            match row.blocked_reason {
                None => writeln!(text, "  {} — available", row.action_id),
                Some(TrainingBlockReason::Injured) => {
                    writeln!(
                        text,
                        "  {} — blocked: injured; rest to recover.",
                        row.action_id
                    )
                }
                Some(TrainingBlockReason::InsufficientEnergy) => writeln!(
                    text,
                    "  {} — blocked: requires {} energy; {} available.",
                    row.action_id, row.energy_cost, self.energy
                ),
            }
            .expect("writing to a String");
        }
        text
    }
}
