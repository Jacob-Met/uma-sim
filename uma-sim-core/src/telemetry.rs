//! Telemetry recording and replay loading.

use crate::bot::BotDecisionAdapter;
use crate::scenario::scenario_plugin_for;
use crate::scoring::scenario_display_name;
use crate::state::{
    default_facility_levels, CareerState, MoodLevel, RunMeta, SimAction, SimActionKind, SimChoice,
    TraineeStats, TrainingFacility, TurnPhase,
};
use crate::training::TrainingResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnTelemetryRecord {
    pub turn: i32,
    pub phase: String,
    pub energy: i32,
    pub mood: String,
    pub speed: i32,
    pub fans: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default)]
    pub choice_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStatsSnapshot {
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPrePost {
    pub energy: i32,
    pub mood: String,
    pub stats: TelemetryStatsSnapshot,
    #[serde(default)]
    #[serde(rename = "facilityLevels")]
    pub facility_levels: std::collections::HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryDecision {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "trainingStat")]
    pub training_stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryStatDelta {
    #[serde(default)]
    pub stats: std::collections::HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPost {
    pub stats: TelemetryStatsSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<TelemetryStatDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_gain: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidTelemetryLine {
    #[serde(default = "default_schema_version")]
    #[serde(rename = "schemaVersion")]
    pub schema_version: i32,
    #[serde(rename = "runId")]
    pub run_id: String,
    pub turn: i32,
    pub scenario: String,
    #[serde(default = "default_turn_type")]
    #[serde(rename = "type")]
    pub line_type: String,
    pub pre: TelemetryPrePost,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<TelemetryDecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<TelemetryPost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "optionIndex")]
    pub option_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy: Option<i32>,
}

fn default_schema_version() -> i32 {
    1
}

fn default_turn_type() -> String {
    "turn".to_string()
}

pub struct SimTelemetry {
    records: Vec<TurnTelemetryRecord>,
    android_lines: Vec<AndroidTelemetryLine>,
    run_id: String,
    scenario_label: String,
}

impl Default for SimTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl SimTelemetry {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            android_lines: Vec::new(),
            run_id: String::new(),
            scenario_label: "URA Finale".to_string(),
        }
    }

    pub fn on_run_start(&mut self, meta: &RunMeta) {
        self.run_id = format!("sim-{}-{}", meta.seed, meta.scenario_id);
        self.scenario_label = scenario_display_name(&meta.scenario_id);
        self.records.clear();
        self.android_lines.clear();
    }

    pub fn record(
        &mut self,
        state: &CareerState,
        choices: &[SimChoice],
        action: Option<&SimAction>,
    ) {
        self.records.push(TurnTelemetryRecord {
            turn: state.turn,
            phase: state.phase.clone(),
            energy: state.energy,
            mood: state.mood.kotlin_name().to_string(),
            speed: state.stats.speed,
            fans: state.fans,
            action: action.map(|a| {
                format!(
                    "{:?}{}",
                    a.kind,
                    a.payload
                        .as_ref()
                        .map(|p| format!(":{p}"))
                        .unwrap_or_default()
                )
            }),
            choice_ids: choices.iter().map(|c| c.id.clone()).collect(),
        });
    }

    pub fn record_transition(
        &mut self,
        pre: &CareerState,
        post: &CareerState,
        action: &SimAction,
        event_choice_index: Option<i32>,
    ) {
        if action.kind == SimActionKind::Choose {
            let idx = event_choice_index
                .or_else(|| action.payload.as_ref().and_then(|p| p.parse().ok()))
                .unwrap_or(0);
            self.android_lines.push(AndroidTelemetryLine {
                schema_version: 1,
                run_id: self.run_id.clone(),
                turn: pre.turn,
                scenario: self.scenario_label.clone(),
                line_type: "event_decision".to_string(),
                pre: pre.to_telemetry_pre(),
                decision: None,
                post: None,
                option_index: Some(idx),
                options: Some(pre.pending_event_options.clone()),
                energy: Some(pre.energy),
            });
            return;
        }

        let decision = action.to_telemetry_decision();
        let stat_key = decision.training_stat.clone();
        let main_gain = if action.kind == SimActionKind::Train {
            stat_key.as_ref().and_then(|k| {
                let before = pre.stats.get(facility_from_stat(k));
                let after = post.stats.get(facility_from_stat(k));
                Some((after - before).max(0))
            })
        } else {
            None
        };
        let delta = if let (Some(gain), Some(ref key)) = (main_gain, &stat_key) {
            Some(TelemetryStatDelta {
                stats: std::collections::HashMap::from([(key.clone(), gain)]),
            })
        } else {
            stat_delta(&pre.stats, &post.stats)
        };

        self.android_lines.push(AndroidTelemetryLine {
            schema_version: 1,
            run_id: self.run_id.clone(),
            turn: pre.turn,
            scenario: self.scenario_label.clone(),
            line_type: "turn".to_string(),
            pre: pre.to_telemetry_pre(),
            decision: Some(decision),
            post: Some(TelemetryPost {
                stats: post.stats.to_telemetry_stats(),
                delta,
                main_gain,
            }),
            option_index: None,
            options: None,
            energy: None,
        });
    }

    pub fn records(&self) -> &[TurnTelemetryRecord] {
        &self.records
    }

    pub fn android_lines(&self) -> &[AndroidTelemetryLine] {
        &self.android_lines
    }

    pub fn export_json(&self) -> String {
        serde_json::to_string(&self.records).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn export_jsonl(&self) -> String {
        self.android_lines
            .iter()
            .filter_map(|line| serde_json::to_string(line).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.android_lines.clear();
    }
}

trait TelemetryPre {
    fn to_telemetry_pre(&self) -> TelemetryPrePost;
}

impl TelemetryPre for CareerState {
    fn to_telemetry_pre(&self) -> TelemetryPrePost {
        TelemetryPrePost {
            energy: self.energy,
            mood: self.mood.kotlin_name().to_string(),
            stats: self.stats.to_telemetry_stats(),
            facility_levels: self.facility_levels.clone(),
        }
    }
}

trait TelemetryStats {
    fn to_telemetry_stats(&self) -> TelemetryStatsSnapshot;
}

impl TelemetryStats for TraineeStats {
    fn to_telemetry_stats(&self) -> TelemetryStatsSnapshot {
        TelemetryStatsSnapshot {
            speed: self.speed,
            stamina: self.stamina,
            power: self.power,
            guts: self.guts,
            wit: self.wit,
        }
    }
}

trait TelemetryDecisionFromAction {
    fn to_telemetry_decision(&self) -> TelemetryDecision;
}

impl TelemetryDecisionFromAction for SimAction {
    fn to_telemetry_decision(&self) -> TelemetryDecision {
        match self.kind {
            SimActionKind::Train => TelemetryDecision {
                action: "TRAIN".to_string(),
                training_stat: self.payload.as_ref().map(|p| p.to_lowercase()),
                reason: Some("sim".to_string()),
            },
            SimActionKind::Rest => TelemetryDecision {
                action: "REST".to_string(),
                training_stat: None,
                reason: Some("sim".to_string()),
            },
            SimActionKind::Recreation => TelemetryDecision {
                action: "RECREATION".to_string(),
                training_stat: None,
                reason: Some("sim".to_string()),
            },
            SimActionKind::Race => TelemetryDecision {
                action: "RACE".to_string(),
                training_stat: None,
                reason: Some("sim".to_string()),
            },
            SimActionKind::Lesson => TelemetryDecision {
                action: "LESSON".to_string(),
                training_stat: None,
                reason: self.payload.clone(),
            },
            other => TelemetryDecision {
                action: format!("{other:?}"),
                training_stat: None,
                reason: self.payload.clone(),
            },
        }
    }
}

fn stat_delta(before: &TraineeStats, after: &TraineeStats) -> Option<TelemetryStatDelta> {
    let mut deltas = std::collections::HashMap::new();
    if after.speed != before.speed {
        deltas.insert("speed".into(), after.speed - before.speed);
    }
    if after.stamina != before.stamina {
        deltas.insert("stamina".into(), after.stamina - before.stamina);
    }
    if after.power != before.power {
        deltas.insert("power".into(), after.power - before.power);
    }
    if after.guts != before.guts {
        deltas.insert("guts".into(), after.guts - before.guts);
    }
    if after.wit != before.wit {
        deltas.insert("wit".into(), after.wit - before.wit);
    }
    if deltas.is_empty() {
        None
    } else {
        Some(TelemetryStatDelta { stats: deltas })
    }
}

fn facility_from_stat(stat: &str) -> TrainingFacility {
    match stat.to_lowercase().as_str() {
        "stamina" => TrainingFacility::Stamina,
        "power" => TrainingFacility::Power,
        "guts" => TrainingFacility::Guts,
        "wit" | "wits" => TrainingFacility::Wit,
        _ => TrainingFacility::Speed,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimReplayLine {
    #[serde(default = "default_replay_type")]
    #[serde(rename = "type")]
    pub line_type: String,
    pub kind: String,
    #[serde(default = "default_energy")]
    pub energy: i32,
    #[serde(default = "default_mood")]
    pub mood: String,
    #[serde(default = "default_stat")]
    pub speed: i32,
    #[serde(default = "default_stat")]
    pub stamina: i32,
    #[serde(default = "default_stat")]
    pub power: i32,
    #[serde(default = "default_stat")]
    pub guts: i32,
    #[serde(default = "default_stat")]
    pub wit: i32,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub expected_index: i32,
    #[serde(default = "default_facility")]
    pub expected_facility: String,
    #[serde(default = "default_scenario")]
    pub scenario: String,
    #[serde(default)]
    pub bot_choice_index: Option<i32>,
    #[serde(default)]
    pub bot_training_stat: Option<String>,
    #[serde(default)]
    pub deck_supports: Vec<String>,
}

fn default_replay_type() -> String {
    "sim_replay".to_string()
}
fn default_energy() -> i32 {
    60
}
fn default_mood() -> String {
    "NORMAL".to_string()
}
fn default_stat() -> i32 {
    200
}
fn default_facility() -> String {
    "speed".to_string()
}
fn default_scenario() -> String {
    "ura".to_string()
}

pub struct TelemetryReplayLoader;

impl TelemetryReplayLoader {
    pub fn parse_jsonl(text: &str) -> Vec<SimReplayLine> {
        text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .filter_map(|l| Self::parse_line(l))
            .collect()
    }

    pub fn parse_line(line: &str) -> Option<SimReplayLine> {
        let root: serde_json::Value = serde_json::from_str(line).ok()?;
        let line_type = root.get("type")?.as_str()?;
        if line_type == "sim_replay" {
            return serde_json::from_str(line).ok();
        }
        if line_type == "event_decision" {
            let options: Vec<String> = root
                .get("options")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let idx = root
                .get("optionIndex")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            return Some(SimReplayLine {
                line_type: line_type.to_string(),
                kind: "event".to_string(),
                energy: root.get("energy").and_then(|v| v.as_i64()).unwrap_or(60) as i32,
                options,
                expected_index: idx,
                bot_choice_index: Some(idx),
                scenario: map_scenario(
                    root.get("scenario")
                        .and_then(|v| v.as_str())
                        .unwrap_or("URA Finale"),
                ),
                ..Default::default()
            });
        }
        if line_type == "turn" || root.get("decision").is_some() {
            return parse_turn_record(&root);
        }
        None
    }

    pub fn sim_choice_index(line: &SimReplayLine) -> i32 {
        let plugin = scenario_plugin_for(&line.scenario);
        BotDecisionAdapter::choose_event_option(&replay_state(line), plugin.as_ref())
    }

    pub fn sim_training_facility(line: &SimReplayLine) -> TrainingFacility {
        let plugin = scenario_plugin_for(&line.scenario);
        let resolver = TrainingResolver::default();
        BotDecisionAdapter::choose_training_facility(
            &replay_state(line),
            plugin.as_ref(),
            &resolver,
        )
    }

    pub fn match_rate(lines: &[SimReplayLine]) -> (i32, i32) {
        let mut matches = 0;
        let mut total = 0;
        for line in lines {
            match line.kind.as_str() {
                "event" => {
                    if line.options.len() < 2 {
                        continue;
                    }
                    total += 1;
                    let expected = line.bot_choice_index.unwrap_or(line.expected_index);
                    if Self::sim_choice_index(line) == expected {
                        matches += 1;
                    }
                }
                "training" => {
                    total += 1;
                    let expected = line
                        .bot_training_stat
                        .as_deref()
                        .unwrap_or(&line.expected_facility);
                    if Self::sim_training_facility(line).key() == expected.to_lowercase() {
                        matches += 1;
                    }
                }
                _ => {}
            }
        }
        (matches, total)
    }
}

impl Default for SimReplayLine {
    fn default() -> Self {
        Self {
            line_type: default_replay_type(),
            kind: String::new(),
            energy: default_energy(),
            mood: default_mood(),
            speed: default_stat(),
            stamina: default_stat(),
            power: default_stat(),
            guts: default_stat(),
            wit: default_stat(),
            options: Vec::new(),
            expected_index: 0,
            expected_facility: default_facility(),
            scenario: default_scenario(),
            bot_choice_index: None,
            bot_training_stat: None,
            deck_supports: Vec::new(),
        }
    }
}

fn parse_turn_record(root: &serde_json::Value) -> Option<SimReplayLine> {
    let decision = root.get("decision")?;
    let action = decision.get("action")?.as_str()?;
    let pre = root.get("pre");
    let energy = pre
        .and_then(|p| p.get("energy"))
        .and_then(|v| v.as_i64())
        .unwrap_or(60) as i32;
    let mood = pre
        .and_then(|p| p.get("mood"))
        .and_then(|v| v.as_str())
        .unwrap_or("NORMAL")
        .to_string();
    let stats = pre.and_then(|p| p.get("stats"));
    let scenario = map_scenario(
        root.get("scenario")
            .and_then(|v| v.as_str())
            .unwrap_or("URA Finale"),
    );
    if action == "TRAIN" || action == "REST" {
        let stat = decision
            .get("trainingStat")
            .and_then(|v| v.as_str())
            .unwrap_or("speed")
            .to_string();
        return Some(SimReplayLine {
            line_type: "turn".to_string(),
            kind: "training".to_string(),
            energy,
            mood,
            speed: stats
                .and_then(|s| s.get("speed"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200) as i32,
            stamina: stats
                .and_then(|s| s.get("stamina"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200) as i32,
            power: stats
                .and_then(|s| s.get("power"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200) as i32,
            guts: stats
                .and_then(|s| s.get("guts"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200) as i32,
            wit: stats
                .and_then(|s| s.get("wit"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200) as i32,
            expected_facility: stat.clone(),
            bot_training_stat: Some(stat),
            scenario,
            ..Default::default()
        });
    }
    None
}

fn map_scenario(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("grand") {
        "grand_concert".to_string()
    } else if lower.contains("unity") {
        "unity".to_string()
    } else if lower.contains("track") {
        "trackblazer".to_string()
    } else {
        "ura".to_string()
    }
}

fn replay_state(line: &SimReplayLine) -> CareerState {
    CareerState {
        meta: RunMeta::new(1, &line.scenario, "Replay"),
        date: crate::state::SimDate {
            year: 1,
            month: 7,
            half: 1,
        },
        turn: 5,
        stats: TraineeStats {
            speed: line.speed,
            stamina: line.stamina,
            power: line.power,
            guts: line.guts,
            wit: line.wit,
        },
        energy: line.energy,
        max_energy: 100,
        mood: parse_mood(&line.mood),
        fans: 0,
        skill_points: 0,
        career_complete: false,
        phase: if line.kind == "event" {
            TurnPhase::Event.as_str().to_string()
        } else {
            TurnPhase::Free.as_str().to_string()
        },
        pending_event_title: if line.kind == "event" {
            Some("Replay".to_string())
        } else {
            None
        },
        pending_event_options: line.options.clone(),
        awaiting_choice: line.kind == "event",
        facility_levels: default_facility_levels(),
        facility_train_counts: std::collections::HashMap::new(),
        hint_levels: std::collections::HashMap::new(),
        statuses: Vec::new(),
        performance_tokens: std::collections::HashMap::new(),
        scenario_resources: crate::state::ScenarioResources::new(),
        legacy: crate::state::LegacyState::default(),
        learned_skill_ids: Vec::new(),
        deck: crate::state::DeckState::default(),
        log: Vec::new(),
        pending_race_id: None,
        completed_races: Vec::new(),
        generated_sparks: Vec::new(),
    }
}

fn parse_mood(raw: &str) -> MoodLevel {
    match raw.to_uppercase().as_str() {
        "GREAT" => MoodLevel::Great,
        "GOOD" => MoodLevel::Good,
        "BAD" => MoodLevel::Bad,
        "AWFUL" => MoodLevel::Awful,
        _ => MoodLevel::Normal,
    }
}
