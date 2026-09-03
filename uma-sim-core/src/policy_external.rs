//! External Kotlin policy client (`--policy=external`).
//! Spawns the JVM policy-server once and speaks NDJSON on its stdin/stdout.

use crate::bot::BotDecisionAdapter;
use crate::calendar::CAREER_TURNS;
use crate::scenario::ScenarioPlugin;
use crate::state::{CareerState, SimAction, SimActionKind, SimChoice, StatName, TrainingFacility};
use crate::training::{TrainingPreview, TrainingResolver};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

static EXTERNAL: Mutex<Option<ExternalPolicy>> = Mutex::new(None);

pub struct ExternalPolicy {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl ExternalPolicy {
    pub fn spawn(command: &str) -> Result<Self, String> {
        let mut child = if cfg!(windows) && command.to_lowercase().ends_with(".bat") {
            Command::new("cmd")
                .args(["/C", command])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| format!("spawn policy server ({command}): {e}"))?
        } else {
            let mut parts = command.split_whitespace();
            let prog = parts.next().ok_or_else(|| "empty UMA_POLICY_CMD".to_string())?;
            let args: Vec<&str> = parts.collect();
            Command::new(prog)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| format!("spawn policy server ({command}): {e}"))?
        };
        let stdin = child.stdin.take().ok_or("policy server missing stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("policy server missing stdout")?);
        let mut policy = Self {
            child,
            stdin,
            stdout,
        };
        let pong = policy.roundtrip(r#"{"cmd":"ping"}"#)?;
        if !pong.contains("\"ok\":true") {
            return Err(format!("policy ping failed: {pong}"));
        }
        Ok(policy)
    }

    fn roundtrip(&mut self, line: &str) -> Result<String, String> {
        writeln!(self.stdin, "{line}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut resp = String::new();
        self.stdout
            .read_line(&mut resp)
            .map_err(|e| e.to_string())?;
        if resp.is_empty() {
            return Err("policy server closed stdout".into());
        }
        Ok(resp.trim().to_string())
    }

    pub fn choose(
        &mut self,
        choices: &[SimChoice],
        state: &CareerState,
        plugin: &dyn ScenarioPlugin,
        resolver: &TrainingResolver,
    ) -> Result<SimAction, String> {
        let req = build_request(choices, state, plugin, resolver)?;
        let resp = self.roundtrip(&req)?;
        parse_action(&resp)
    }
}

impl Drop for ExternalPolicy {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "{}", r#"{"cmd":"quit"}"#);
        let _ = self.child.wait();
    }
}

fn policy_command() -> Result<String, String> {
    std::env::var("UMA_POLICY_CMD").map_err(|_| {
        "UMA_POLICY_CMD is not set (path to the external policy-server binary)".to_string()
    })
}

pub fn ensure_external() -> Result<(), String> {
    let mut guard = EXTERNAL.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(ExternalPolicy::spawn(&policy_command()?)?);
    }
    Ok(())
}

pub fn external_auto_policy(
    choices: &[SimChoice],
    state: &CareerState,
    resolver: &TrainingResolver,
    plugin: &dyn ScenarioPlugin,
) -> SimAction {
    if let Err(e) = ensure_external() {
        panic!("[external-policy] failed to start: {e}");
    }
    let mut guard = EXTERNAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let policy = guard
        .as_mut()
        .expect("external policy missing after ensure");
    match policy.choose(choices, state, plugin, resolver) {
        Ok(action) => action,
        Err(e) => panic!("[external-policy] choose failed: {e}"),
    }
}

fn build_request(
    choices: &[SimChoice],
    state: &CareerState,
    plugin: &dyn ScenarioPlugin,
    resolver: &TrainingResolver,
) -> Result<String, String> {
    let ctx = BotDecisionAdapter::to_decision_context(state, plugin);
    let choices_json: Vec<Value> = choices
        .iter()
        .map(|c| json!({"id": c.id, "label": c.label}))
        .collect();

    let previews = TrainingPreview::build_options(state, resolver);
    let mut trainings = Vec::new();
    for facility in TrainingFacility::ALL {
        let Some(option) = previews.get(&facility) else {
            continue;
        };
        trainings.push(json!({
            "name": facility.key(),
            "statGains": {
                "speed": option.stat_gains.get(&StatName::Speed).copied().unwrap_or(0),
                "stamina": option.stat_gains.get(&StatName::Stamina).copied().unwrap_or(0),
                "power": option.stat_gains.get(&StatName::Power).copied().unwrap_or(0),
                "guts": option.stat_gains.get(&StatName::Guts).copied().unwrap_or(0),
                "wit": option.stat_gains.get(&StatName::Wit).copied().unwrap_or(0),
            },
            "numRainbow": option.num_rainbow,
            "numSkillHints": option.num_skill_hints,
            "trainingLevel": option.training_level.unwrap_or(1),
            "failureChancePercent": 0,
        }));
    }

    let year = match state.date.year {
        1 => "junior",
        3 => "senior",
        _ => "classic",
    };

    let req = json!({
        "cmd": "choose",
        "scoringModel": "marginal",
        "choices": choices_json,
        "trainings": trainings,
        "state": {
            "energy": state.energy,
            "injured": state.is_injured(),
            "turn": state.turn,
            "day": state.turn,
            "year": year,
            "scenario": state.meta.scenario_id,
            "traineeName": state.meta.trainee_name,
            "objectiveProfile": ctx.objective_profile,
            "remainingTurns": (CAREER_TURNS - state.turn).max(0),
            "moodOrdinal": ctx.mood_ordinal,
            "stats": {
                "speed": state.stats.speed,
                "stamina": state.stats.stamina,
                "power": state.stats.power,
                "guts": state.stats.guts,
                "wit": state.stats.wit,
            },
            "statCaps": {
                "speed": ctx.stat_caps.get(&StatName::Speed).copied().unwrap_or(1200),
                "stamina": ctx.stat_caps.get(&StatName::Stamina).copied().unwrap_or(1200),
                "power": ctx.stat_caps.get(&StatName::Power).copied().unwrap_or(1200),
                "guts": ctx.stat_caps.get(&StatName::Guts).copied().unwrap_or(1200),
                "wit": ctx.stat_caps.get(&StatName::Wit).copied().unwrap_or(1200),
            },
            "pendingEventOptions": state.pending_event_options,
            "optionalRacePreferred": crate::race::RaceScheduler::should_run_optional_race(state),
            "songsLearned": ctx.songs_learned,
            "isHypeMaxed": ctx.is_hype_maxed,
            "daysToConcert": ctx.days_to_concert,
        }
    });
    serde_json::to_string(&req).map_err(|e| e.to_string())
}

fn parse_action(resp: &str) -> Result<SimAction, String> {
    let v: Value = serde_json::from_str(resp).map_err(|e| format!("bad policy JSON {e}: {resp}"))?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(format!("policy error: {err}"));
    }
    let kind = v
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| format!("no kind in {resp}"))?;
    let payload = v.get("payload").and_then(|p| {
        if p.is_null() {
            None
        } else {
            p.as_str().map(|s| s.to_string())
        }
    });
    let action_kind = match kind {
        "race" => SimActionKind::Race,
        "rest" => SimActionKind::Rest,
        "train" => SimActionKind::Train,
        "choose" => SimActionKind::Choose,
        "lesson" => SimActionKind::Lesson,
        other => return Err(format!("unknown kind {other}")),
    };
    Ok(SimAction {
        kind: action_kind,
        payload,
    })
}
