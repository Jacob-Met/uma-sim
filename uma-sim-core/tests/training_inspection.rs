use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use uma_sim_core::training_inspection::{TrainingBlockReason, TrainingUnavailableReason};
use uma_sim_core::{
    parse_sim_action, MoodLevel, RunMeta, RunSnapshotCodec, SimEngine, SimSettings,
    TrainingFailureConfig, TurnPhase, INJURED,
};

static ENGINE_CONFIG: Mutex<()> = Mutex::new(());
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

fn free_engine(scenario: &str, energy: i32) -> SimEngine {
    let mut engine = SimEngine::new(SimSettings {
        trace_rng: true,
        ..SimSettings::default()
    });
    engine.start(RunMeta::new(701, scenario, "Training inspection fixture"));
    let mut snapshot = engine.export();
    snapshot.state.phase = TurnPhase::Free.as_str().into();
    snapshot.state.pending_race_id = None;
    snapshot.state.pending_event_title = None;
    snapshot.state.pending_event_options.clear();
    snapshot.state.awaiting_choice = false;
    snapshot.state.energy = energy;
    snapshot.state.mood = MoodLevel::Normal;
    engine.restore(snapshot);
    engine
}

fn value(engine: &SimEngine) -> Value {
    serde_json::from_str(&RunSnapshotCodec::encode(&engine.export())).unwrap()
}

#[test]
fn repeated_inspection_preserves_snapshot_rng_and_next_action() {
    let _guard = ENGINE_CONFIG.lock().unwrap();
    TrainingFailureConfig::reset_to_defaults();
    let mut engine = free_engine("ura", 100);
    let mut snapshot = engine.export();
    snapshot.state.facility_levels.insert("power".into(), 3);
    snapshot.state.hint_levels.insert("power".into(), 2);
    engine.restore(snapshot);
    let before = value(&engine);
    let rng_calls = engine.rng_calls();
    let first = engine.training_inspection();
    assert_eq!(first, engine.training_inspection());
    assert_eq!(first.schema_version, 1);
    assert_eq!(first.sample_kind, "deterministic_seed_0");
    assert_eq!(
        first
            .rows
            .iter()
            .map(|row| row.action_id.as_str())
            .collect::<Vec<_>>(),
        [
            "train_speed",
            "train_stamina",
            "train_power",
            "train_guts",
            "train_wit"
        ]
    );
    assert_eq!(first.rows[2].level, 3);
    assert_eq!(first.rows[2].num_skill_hints, 2);
    assert_eq!(value(&engine), before);
    assert_eq!(engine.rng_calls(), rng_calls);
    let mut twin = SimEngine::new(SimSettings::default());
    twin.restore(engine.export());
    engine.step(parse_sim_action("train_power"));
    twin.step(parse_sim_action("train_power"));
    assert_eq!(value(&engine), value(&twin));
}

#[test]
fn current_energy_risk_uses_the_native_post_cost_boundary() {
    let _guard = ENGINE_CONFIG.lock().unwrap();
    TrainingFailureConfig::reset_to_defaults();
    // Default level-one training costs 20; remaining 29 and 30 cross a risk band.
    let lower = free_engine("ura", 49).training_inspection();
    let upper = free_engine("ura", 50).training_inspection();
    assert_eq!(lower.rows[0].energy_cost, 20);
    assert_eq!(lower.rows[0].energy_delta, Some(-20));
    assert_eq!(lower.rows[0].failure_chance_pct, Some(45));
    assert_eq!(upper.rows[0].failure_chance_pct, Some(30));
    // Wit uses the engine's existing pre-recovery risk rule, not recovered energy.
    assert_eq!(lower.rows[4].energy_cost, -5);
    assert_eq!(lower.rows[4].failure_chance_pct, Some(30));
}

#[test]
fn scenario_zero_failure_and_effective_unity_level_are_inspected() {
    let _guard = ENGINE_CONFIG.lock().unwrap();
    TrainingFailureConfig::reset_to_defaults();
    for (scenario, ready) in [
        ("unity", "unity_burst_ready"),
        ("unity", "unity_extreme_ready"),
        ("trackblazer", "tb_zero_fail_turns"),
    ] {
        let mut engine = free_engine(scenario, 49);
        assert!(
            engine.training_inspection().rows[0]
                .failure_chance_pct
                .unwrap()
                > 0
        );
        let mut snapshot = engine.export();
        snapshot.state.scenario_resources = snapshot.state.scenario_resources.set(ready, 1);
        engine.restore(snapshot);
        assert!(engine
            .training_inspection()
            .rows
            .iter()
            .all(|row| { row.failure_chance_pct == Some(0) }));
    }
    let mut engine = free_engine("unity", 49);
    let mut snapshot = engine.export();
    snapshot.state.facility_levels.insert("speed".into(), 1);
    snapshot.state.scenario_resources =
        snapshot.state.scenario_resources.set("unity_rank_speed", 4);
    engine.restore(snapshot);
    let before = value(&engine);
    let view = engine.training_inspection();
    assert_eq!(view.rows[0].level, 4);
    assert_eq!(view.rows[0].energy_cost, 25);
    assert_eq!(view.rows[0].failure_chance_pct, Some(51));
    assert_eq!(value(&engine), before);
    assert_eq!(engine.state().facility_levels.get("speed"), Some(&1));
}

#[test]
fn unavailable_phases_and_blocked_rows_have_no_attempt_results() {
    let _guard = ENGINE_CONFIG.lock().unwrap();
    TrainingFailureConfig::reset_to_defaults();
    for (phase, complete, reason) in [
        (
            "MANDATORY_RACE",
            false,
            TrainingUnavailableReason::MandatoryRace,
        ),
        ("EVENT", false, TrainingUnavailableReason::PendingEvent),
        ("COMPLETE", true, TrainingUnavailableReason::CareerComplete),
    ] {
        let mut engine = free_engine("ura", 100);
        let mut snapshot = engine.export();
        snapshot.state.phase = phase.into();
        snapshot.state.career_complete = complete;
        snapshot.state.pending_event_options = vec!["First choice".into()];
        engine.restore(snapshot);
        let view = engine.training_inspection();
        assert_eq!(view.unavailable_reason, Some(reason));
        assert!(view.rows.is_empty());
        assert!(view.render_text().contains("No training choices:"));
    }
    let mut engine = free_engine("ura", 10);
    let low = engine.training_inspection();
    assert!(low.rows[..4].iter().all(|row| {
        !row.available
            && row.blocked_reason == Some(TrainingBlockReason::InsufficientEnergy)
            && row.energy_delta.is_none()
            && row.failure_chance_pct.is_none()
    }));
    assert!(low.rows[4].available);
    assert_eq!(low.rows[4].energy_delta, Some(5));
    assert!(low
        .render_text()
        .contains("requires 20 energy; 10 available"));
    let mut snapshot = engine.export();
    snapshot.state.statuses.push(INJURED.into());
    engine.restore(snapshot);
    let injured = engine.training_inspection();
    assert!(injured.rows.iter().all(|row| {
        !row.available
            && row.blocked_reason == Some(TrainingBlockReason::Injured)
            && row.energy_delta.is_none()
            && row.failure_chance_pct.is_none()
    }));
    assert!(injured.render_text().contains("blocked: injured"));
}

#[test]
fn samples_stay_raw_while_energy_recovery_respects_the_cap() {
    let _guard = ENGINE_CONFIG.lock().unwrap();
    TrainingFailureConfig::reset_to_defaults();
    let mut engine = free_engine("ura", 98);
    let ordinary = engine.training_inspection();
    let mut snapshot = engine.export();
    snapshot.state.stats.speed = 1200;
    snapshot.state.stats.stamina = 1200;
    snapshot.state.stats.power = 1200;
    snapshot.state.stats.guts = 1200;
    snapshot.state.stats.wit = 1200;
    engine.restore(snapshot);
    let capped = engine.training_inspection();
    assert_eq!(ordinary.rows[0].stat_gains, capped.rows[0].stat_gains);
    assert!(capped.rows[0].stat_gains.speed > 0);
    assert_eq!(capped.rows[4].energy_cost, -5);
    assert_eq!(capped.rows[4].energy_delta, Some(2));
    let text = capped.render_text();
    for disclosure in [
        "deterministic samples",
        "before stat caps",
        "not guaranteed",
        "does not take an action",
    ] {
        assert!(text.contains(disclosure), "missing {disclosure:?}");
    }
}

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "uma-training-inspection-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn session(&self) -> PathBuf {
        self.0.join(".uma-sim/session.json")
    }

    fn cli(&self, args: &[&str]) -> Output {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        Command::new(env!("CARGO_BIN_EXE_uma-sim"))
            .args(args)
            .current_dir(&self.0)
            .env("UMA_REPO_ROOT", repo)
            .env_remove("UMA_POLICY_CMD")
            .output()
            .unwrap()
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn actual_cli_inspects_saved_native_career_without_rewriting_it() {
    let fixture = FixtureDir::new();
    let start = fixture.cli(&[
        "start",
        "--seed=7",
        "--scenario=ura",
        "--race-model=stub",
        "--trace-rng",
    ]);
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let first_bytes = fs::read(fixture.session()).unwrap();
    let pending = fixture.cli(&["training", "--format=json"]);
    assert!(pending.status.success());
    let pending: Value = serde_json::from_slice(&pending.stdout).unwrap();
    assert_eq!(pending["unavailableReason"], "mandatory_race");
    assert_eq!(pending["rows"], serde_json::json!([]));
    assert_eq!(fs::read(fixture.session()).unwrap(), first_bytes);

    // Follow offered native pre-training gates instead of fabricating a phase.
    for _ in 0..12 {
        let snapshot =
            RunSnapshotCodec::decode(&fs::read_to_string(fixture.session()).unwrap()).unwrap();
        if snapshot.state.phase == "FREE" {
            break;
        }
        let action = if snapshot.state.phase == "EVENT" {
            "event_0"
        } else {
            "race"
        };
        assert!(fixture.cli(&["step", action]).status.success());
    }
    let before = fs::read(fixture.session()).unwrap();
    let json = fixture.cli(&["training", "--format=json"]);
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let view: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(view["schemaVersion"], 1);
    assert_eq!(view["rows"].as_array().unwrap().len(), 5);
    assert_eq!(view["rows"][0]["actionId"], "train_speed");
    assert_eq!(view["rows"][4]["actionId"], "train_wit");
    let repeat = fixture.cli(&["training", "--format=json"]);
    assert_eq!(json.stdout, repeat.stdout);
    let text = fixture.cli(&["training"]);
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains("deterministic samples"));
    assert_eq!(fs::read(fixture.session()).unwrap(), before);
}

#[test]
fn actual_cli_refuses_arguments_and_unreadable_sessions_without_writes() {
    let fixture = FixtureDir::new();
    let missing = fixture.cli(&["training"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(!fixture.0.join(".uma-sim").exists());
    fs::create_dir(fixture.0.join(".uma-sim")).unwrap();
    let malformed = b"{not a saved career}\n";
    fs::write(fixture.session(), malformed).unwrap();
    for args in [
        vec!["training", "--format="],
        vec!["training", "--format=xml"],
        vec!["training", "--format=json", "--format=text"],
        vec!["training", "--format=json", "extra"],
        vec!["training", "--policy=external"],
        vec!["training", "--format", "json"],
    ] {
        let output = fixture.cli(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Usage:"));
        assert_eq!(fs::read(fixture.session()).unwrap(), malformed);
    }
    for args in [
        vec!["training"],
        vec!["training", "--format=text"],
        vec!["training", "--format=json"],
    ] {
        let output = fixture.cli(&args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(fixture.session()).unwrap(), malformed);
    }
}
