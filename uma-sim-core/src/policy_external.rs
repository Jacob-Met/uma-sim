//! External Kotlin policy client (`--policy=external`).
//! Spawns the JVM policy-server on first use and speaks NDJSON on its
//! stdin/stdout. A dead child is reaped and respawned on next use; each
//! round trip is bounded by a timeout so an unresponsive server cannot wedge
//! the single-threaded REST loop forever.

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

/// Test-only serialization: every test in this binary that touches
/// `UMA_POLICY_CMD` or the global `EXTERNAL` slot must hold this lock, and
/// reset the slot before/after, so tests cannot observe each other's stubs.
#[cfg(test)]
pub(crate) static EXTERNAL_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Test-only: kill any live policy child and clear the slot.
/// Must be called with `EXTERNAL_TEST_LOCK` held.
#[cfg(test)]
pub(crate) fn reset_external_for_tests() {
    let mut guard = EXTERNAL.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut policy) = guard.take() {
        // Kill first: Drop's wait() must not block on a wedged child.
        let _ = policy.child.kill();
        drop(policy);
    }
}

/// Max time for one policy-server round trip (write request + read reply).
/// A live-but-unresponsive server must not wedge the single-threaded REST
/// loop forever; on timeout the child is killed and the request fails loudly.
/// Override with `UMA_POLICY_TIMEOUT_MS` (milliseconds); default 30s.
fn policy_timeout() -> std::time::Duration {
    std::env::var("UMA_POLICY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .map(std::time::Duration::from_millis)
        .unwrap_or_else(|| std::time::Duration::from_secs(30))
}

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
            let prog = parts
                .next()
                .ok_or_else(|| "empty UMA_POLICY_CMD".to_string())?;
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
        // Run the blocking write+read on a scoped thread so a live-but-
        // unresponsive child cannot wedge the caller (the single-threaded REST
        // loop) forever. The kill must happen INSIDE the scope: the scope
        // joins the reader thread before returning, so killing after the
        // scope would deadlock against the very thread we need to unblock.
        // Killing the child closes its stdout pipe, which unblocks the
        // reader, so the join completes promptly.
        let timeout = policy_timeout();
        let (tx, rx) = std::sync::mpsc::channel();
        let outcome = std::thread::scope(|s| {
            s.spawn(|| {
                let result = (|| -> Result<String, String> {
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
                })();
                let _ = tx.send(result);
            });
            let r = rx.recv_timeout(timeout);
            if r.is_err() {
                // Wedged (or vanished) reader: SIGKILL the child so the
                // reader's blocked read observes EOF/ERR and the scope's
                // join completes. (If the child forked grandchildren that
                // inherited stdout, reclamation waits for them; the JVM
                // policy server does not fork.)
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
            r
        });
        match outcome {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(format!(
                "policy server timeout after {}ms (UMA_POLICY_CMD unresponsive)",
                timeout.as_millis()
            )),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("policy server reader thread failed".to_string())
            }
        }
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
    // A policy child that died mid-run must not wedge every later request:
    // try_wait() reaps a zombie; any error (e.g. already reaped) also means
    // the handle is dead. Respawn on next use.
    let dead = match guard.as_mut() {
        None => true,
        Some(policy) => !matches!(policy.child.try_wait(), Ok(None)),
    };
    if dead {
        *guard = Some(ExternalPolicy::spawn(&policy_command()?)?);
    }
    Ok(())
}

/// Non-panicking external-policy step: spawns the policy server on first use
/// (via `UMA_POLICY_CMD`) and asks it to choose an action. Used by the REST
/// API so a request for `policy=external` can fail loudly with an HTTP error
/// instead of panicking the server or silently falling back to the default
/// heuristic.
pub fn try_external_auto_policy(
    choices: &[SimChoice],
    state: &CareerState,
    resolver: &TrainingResolver,
    plugin: &dyn ScenarioPlugin,
) -> Result<SimAction, String> {
    if let Err(e) = ensure_external() {
        return Err(format!("[external-policy] failed to start: {e}"));
    }
    let mut guard = EXTERNAL.lock().unwrap_or_else(|e| e.into_inner());
    let policy = guard
        .as_mut()
        .expect("external policy missing after ensure");
    policy
        .choose(choices, state, plugin, resolver)
        .map_err(|e| format!("[external-policy] choose failed: {e}"))
}

pub fn external_auto_policy(
    choices: &[SimChoice],
    state: &CareerState,
    resolver: &TrainingResolver,
    plugin: &dyn ScenarioPlugin,
) -> SimAction {
    match try_external_auto_policy(choices, state, resolver, plugin) {
        Ok(action) => action,
        Err(e) => panic!("{e}"),
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
    let v: Value =
        serde_json::from_str(resp).map_err(|e| format!("bad policy JSON {e}: {resp}"))?;
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::engine::SimEngine;
    use crate::scenario::scenario_plugin_for;
    use crate::state::{RunMeta, SimSettings};
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    /// Stub policy servers (shell scripts) exercising the failure modes:
    /// - `ok`: answers ping and every choose with a rest action.
    /// - `hang`: answers ping, then never answers choose (wedged child).
    /// - `die`: answers ping, then exits immediately (mid-run child death).
    struct Stubs {
        dir: std::path::PathBuf,
    }

    impl Stubs {
        fn write() -> Self {
            let dir = std::env::temp_dir()
                .join(format!("umasim-policy-stubs-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let scripts: &[(&str, &str)] = &[
                (
                    "ok.sh",
                    "#!/bin/sh\nwhile IFS= read -r line; do\n  case \"$line\" in\n    *ping*) echo '{\"ok\":true}' ;;\n    *quit*) exit 0 ;;\n    *) echo '{\"kind\":\"rest\",\"payload\":null}' ;;\n  esac\ndone\n",
                ),
                (
                    "hang.sh",
                    "#!/bin/sh\n# Wedged server: answers ping, then blocks forever on stdin.\n# Deliberately a shell builtin (no fork): killing the direct child must\n# close the stdout pipe so the reader thread unblocks, like a real\n# wedged-but-not-forking policy binary.\nwhile IFS= read -r line; do\n  case \"$line\" in\n    *ping*) echo '{\"ok\":true}' ;;\n    *quit*) exit 0 ;;\n    *) IFS= read -r line2 ;;\n  esac\ndone\n",
                ),
                (
                    "die.sh",
                    "#!/bin/sh\nIFS= read -r line\necho '{\"ok\":true}'\nexit 0\n",
                ),
            ];
            for (name, body) in scripts {
                let path = dir.join(name);
                std::fs::write(&path, body).unwrap();
                let mut perms = std::fs::metadata(&path).unwrap().permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms).unwrap();
            }
            Self { dir }
        }

        fn cmd(&self, name: &str) -> String {
            self.dir.join(name).to_string_lossy().into_owned()
        }
    }

    impl Drop for Stubs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Build real fixtures via the engine, the same way the REST layer does.
    fn fixtures() -> (Vec<SimChoice>, CareerState, TrainingResolver, Box<dyn ScenarioPlugin>) {
        let mut engine = SimEngine::new(SimSettings::default());
        engine.start(RunMeta::new(7, "ura", "Special Week"));
        let choices = engine.choices();
        assert!(!choices.is_empty(), "test needs at least one choice");
        let state = engine.state().clone();
        let resolver = TrainingResolver::default();
        let plugin = scenario_plugin_for("ura");
        (choices, state, resolver, plugin)
    }

    fn set_env(cmd: Option<&str>, timeout_ms: Option<&str>) {
        match cmd {
            Some(c) => std::env::set_var("UMA_POLICY_CMD", c),
            None => std::env::remove_var("UMA_POLICY_CMD"),
        }
        match timeout_ms {
            Some(t) => std::env::set_var("UMA_POLICY_TIMEOUT_MS", t),
            None => std::env::remove_var("UMA_POLICY_TIMEOUT_MS"),
        }
    }

    /// (a) A wedged policy server must fail loudly within the timeout instead
    /// of blocking the round trip forever, and (b) the killed child must be
    /// respawned on the next request rather than 503ing until restart.
    #[test]
    fn wedged_policy_server_times_out_loudly_and_respawns() {
        let _lock = EXTERNAL_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let old_cmd = std::env::var("UMA_POLICY_CMD").ok();
        let old_timeout = std::env::var("UMA_POLICY_TIMEOUT_MS").ok();
        reset_external_for_tests();

        let stubs = Stubs::write();
        let (choices, state, resolver, plugin) = fixtures();

        // Wedged child: ping succeeds, choose never answers.
        set_env(Some(&stubs.cmd("hang.sh")), Some("800"));
        let start = Instant::now();
        let err = try_external_auto_policy(&choices, &state, &resolver, plugin.as_ref())
            .expect_err("wedged policy server must fail, not hang or succeed");
        let elapsed = start.elapsed();
        assert!(
            err.contains("timeout"),
            "failure must name the timeout; got: {err}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "round trip must be bounded by the timeout, took {elapsed:?}"
        );

        // The wedged child was killed: pointing at a healthy server must
        // respawn and succeed instead of failing forever.
        set_env(Some(&stubs.cmd("ok.sh")), None);
        let action =
            try_external_auto_policy(&choices, &state, &resolver, plugin.as_ref())
                .expect("healthy server after a wedged one must respawn and succeed");
        assert_eq!(action.kind, SimActionKind::Rest);

        // Restore environment and global slot.
        set_env(old_cmd.as_deref(), old_timeout.as_deref());
        reset_external_for_tests();
    }

    /// (b) A policy child that dies mid-run must be reaped and respawned on
    /// the next request: the first request fails loudly, the next one (with a
    /// healthy server configured) succeeds.
    #[test]
    fn dead_policy_child_is_reaped_and_respawned() {
        let _lock = EXTERNAL_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let old_cmd = std::env::var("UMA_POLICY_CMD").ok();
        let old_timeout = std::env::var("UMA_POLICY_TIMEOUT_MS").ok();
        reset_external_for_tests();

        let stubs = Stubs::write();
        let (choices, state, resolver, plugin) = fixtures();

        // Child answers ping then dies: choose hits a closed stdout (or EPIPE
        // on the write, depending on timing) — either way a loud failure.
        set_env(Some(&stubs.cmd("die.sh")), Some("5000"));
        let err = try_external_auto_policy(&choices, &state, &resolver, plugin.as_ref())
            .expect_err("dead policy child must fail loudly");
        assert!(
            err.contains("[external-policy]"),
            "failure must be loud and name the external-policy layer; got: {err}"
        );

        // Next request with a healthy server must respawn, not reuse the corpse.
        set_env(Some(&stubs.cmd("ok.sh")), None);
        let action =
            try_external_auto_policy(&choices, &state, &resolver, plugin.as_ref())
                .expect("healthy server after a dead child must respawn and succeed");
        assert_eq!(action.kind, SimActionKind::Rest);

        set_env(old_cmd.as_deref(), old_timeout.as_deref());
        reset_external_for_tests();
    }
}
