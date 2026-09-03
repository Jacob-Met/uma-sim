//! uma-sim CLI — parity with Kotlin `SimCliMain`.

use std::path::PathBuf;
use std::time::Instant;
use uma_sim_core::deck::DeckPlacement;
use uma_sim_core::factory::detect_repo_root;
use uma_sim_core::render::TextRenderer;
use uma_sim_core::session::{parse_sim_action, RunSession};
use uma_sim_core::state::DialogueMode;
use uma_sim_core::{ContentPackLoader, RunMeta, SimEngine, SimSettings};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        return;
    }
    match args[0].as_str() {
        "start" => cmd_start(&args[1..]),
        "state" => cmd_state(),
        "step" => cmd_step(&args[1..]),
        "fast" => cmd_fast(&args[1..]),
        "batch" => cmd_batch(&args[1..]),
        "export-telemetry" => cmd_export_telemetry(&args[1..]),
        "validate" => cmd_validate(&args[1..]),
        "content" => match args.get(1).map(|s| s.as_str()) {
            Some("validate") => cmd_validate(&args[2..]),
            _ => print_usage(),
        },
        "clear" => {
            RunSession::clear();
            println!("Session cleared.");
        }
        "deck" => match args.get(1).map(|s| s.as_str()) {
            Some("place") => cmd_deck_place(&args[2..]),
            _ => print_usage(),
        },
        "serve" => {
            let port = args
                .iter()
                .find_map(|a| a.strip_prefix("--port="))
                .and_then(|p| p.parse().ok())
                .unwrap_or(8765);
            let open = args.iter().any(|a| a == "--open");
            uma_sim_core::api::serve_opts(port, open);
        }
        _ => print_usage(),
    }
}

#[derive(Clone)]
struct CliFlags {
    seed: i64,
    scenario: String,
    trainee: String,
    speed: i32,
    dialogue: DialogueMode,
    deck: Vec<String>,
    legacy: Vec<String>,
    policy: String,
    trace_rng: bool,
    trace_telemetry: bool,
    output: Option<String>,
    race_model: Option<uma_sim_core::RaceModel>,
}

impl Default for CliFlags {
    fn default() -> Self {
        Self {
            seed: 42,
            scenario: "ura".into(),
            trainee: "Special Week".into(),
            speed: 1,
            dialogue: DialogueMode::ChoicesOnly,
            deck: Vec::new(),
            legacy: Vec::new(),
            policy: "default".into(),
            trace_rng: false,
            trace_telemetry: false,
            output: None,
            race_model: None,
        }
    }
}

fn parse_flags(args: &[String]) -> CliFlags {
    let mut f = CliFlags::default();
    for arg in args {
        if let Some(v) = arg.strip_prefix("--seed=") {
            if let Ok(n) = v.parse() {
                f.seed = n;
            }
        } else if let Some(v) = arg.strip_prefix("--scenario=") {
            f.scenario = v.to_string();
        } else if let Some(v) = arg.strip_prefix("--trainee=") {
            f.trainee = v.to_string();
        } else if let Some(v) = arg.strip_prefix("--speed=") {
            if let Ok(n) = v.parse::<i32>() {
                f.speed = n.clamp(1, 100);
            }
        } else if let Some(v) = arg.strip_prefix("--dialogue=") {
            f.dialogue = match v.to_lowercase().as_str() {
                "off" => DialogueMode::Off,
                "full" => DialogueMode::Full,
                _ => DialogueMode::ChoicesOnly,
            };
        } else if let Some(v) = arg.strip_prefix("--deck=") {
            f.deck = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(v) = arg.strip_prefix("--legacy=") {
            f.legacy = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(v) = arg.strip_prefix("--policy=") {
            f.policy = v.to_string();
        } else if let Some(v) = arg.strip_prefix("--race-model=") {
            f.race_model = Some(uma_sim_core::RaceModel::parse(v));
        } else if arg == "--trace-rng" {
            f.trace_rng = true;
        } else if arg == "--trace-telemetry" {
            f.trace_telemetry = true;
        } else if let Some(v) = arg.strip_prefix("--output=") {
            f.output = Some(v.to_string());
        }
    }
    f
}

fn build_settings(f: &CliFlags) -> SimSettings {
    SimSettings {
        dialogue_mode: f.dialogue,
        speed_multiplier: f.speed,
        trace_rng: f.trace_rng,
        trace_telemetry: f.trace_telemetry,
        race_model: f
            .race_model
            .or_else(uma_sim_core::RaceModel::from_env)
            .unwrap_or_default(),
        ..Default::default()
    }
}

fn build_meta(f: &CliFlags) -> RunMeta {
    let mut meta = RunMeta::new(f.seed, &f.scenario, &f.trainee);
    meta.legacy_factors = f.legacy.clone();
    meta.deck_supports = f.deck.clone();
    meta
}

fn play_with_policy(engine: &mut SimEngine, policy: &str) {
    match policy {
        "bot" => engine.play_to_completion_scoring(500),
        "external" => engine.play_to_completion_external(500),
        _ => engine.play_to_completion(500),
    }
}

fn cmd_start(args: &[String]) {
    let f = parse_flags(args);
    let mut engine = SimEngine::create(build_settings(&f));
    let result = engine.start(build_meta(&f));
    if let Err(e) = RunSession::save(&engine) {
        eprintln!("Failed to save session: {e}");
    }
    let choice_ids: Vec<String> = result.choices.into_iter().map(|c| c.id).collect();
    print_result(&result.text_lines, &choice_ids);
}

fn cmd_state() {
    let Some((engine, _)) = RunSession::load() else {
        println!("No session. Run: start --seed=42");
        return;
    };
    let s = engine.state();
    println!(
        "Turn {} | {}/{} half {}\n\
         Stats SPD={} STA={} POW={} GUT={} WIT={}\n\
         Energy={} Mood={} Fans={} SP={}\n\
         Phase={} PendingRace={}\n\
         Deck={} Legacy={}\n\
         Complete={}",
        s.turn,
        s.date.year,
        s.date.month,
        s.date.half,
        s.stats.speed,
        s.stats.stamina,
        s.stats.power,
        s.stats.guts,
        s.stats.wit,
        s.energy,
        s.mood.kotlin_name(),
        s.fans,
        s.skill_points,
        s.phase,
        s.pending_race_id.as_deref().unwrap_or("null"),
        s.deck.slots.len(),
        s.legacy.factor_ids.len(),
        s.career_complete,
    );
}

fn cmd_step(args: &[String]) {
    let Some((mut engine, _)) = RunSession::load() else {
        println!("No session. Run: start --seed=42");
        return;
    };
    let action_id = args.first().map(|s| s.as_str()).unwrap_or("train_speed");
    let result = engine.step(parse_sim_action(action_id));
    if let Err(e) = RunSession::save(&engine) {
        eprintln!("Failed to save session: {e}");
    }
    let choice_ids: Vec<String> = result.choices.into_iter().map(|c| c.id).collect();
    print_result(&result.text_lines, &choice_ids);
}

fn cmd_export_telemetry(args: &[String]) {
    let mut f = parse_flags(args);
    f.trace_telemetry = true;
    if f.speed == 1 {
        f.speed = 20;
    }
    let root = detect_repo_root().unwrap_or_else(|| PathBuf::from("."));
    let out_rel = f
        .output
        .clone()
        .unwrap_or_else(|| format!("out/sim-telemetry/sim-{}-{}.jsonl", f.seed, f.scenario));
    let out_path = root.join(&out_rel);
    let mut engine = SimEngine::create(build_settings(&f));
    engine.start(build_meta(&f));
    play_with_policy(&mut engine, &f.policy);
    let jsonl = engine.export_telemetry_jsonl();
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = if jsonl.is_empty() {
        String::new()
    } else {
        format!("{jsonl}\n")
    };
    if let Err(e) = std::fs::write(&out_path, &content) {
        eprintln!("Failed to write telemetry: {e}");
        return;
    }
    let line_count = jsonl.lines().filter(|l| !l.trim().is_empty()).count();
    println!(
        "Wrote {line_count} telemetry lines to {}",
        out_path.display()
    );
    println!(
        "Calibrate: python scripts/calibrate_sim.py --telemetry {}",
        out_path.display()
    );
}

fn cmd_fast(args: &[String]) {
    let mut f = parse_flags(args);
    if f.speed == 1 {
        f.speed = 20;
    }
    let mut engine = SimEngine::create(build_settings(&f));
    engine.start(build_meta(&f));
    let start = Instant::now();
    play_with_policy(&mut engine, &f.policy);
    let elapsed = start.elapsed().as_millis();
    if let Err(e) = RunSession::save(&engine) {
        eprintln!("Failed to save session: {e}");
    }
    let lines = TextRenderer::new(build_settings(&f)).render(engine.state(), &[]);
    let tail: Vec<String> = lines.iter().rev().take(5).cloned().collect::<Vec<_>>();
    let tail: Vec<String> = tail.into_iter().rev().collect();
    let choice_ids: Vec<String> = engine.choices().into_iter().map(|c| c.id).collect();
    print_result(&tail, &choice_ids);
    let s = engine.state();
    println!(
        "Ended: career={} turn={} fans={} elapsed={}ms speed=x{} policy={}",
        s.career_complete, s.turn, s.fans, elapsed, f.speed, f.policy
    );
    if let Some(t) = engine.last_terminal() {
        println!(
            "Terminal: U={:.3} grade={} score={} sp_spent={} φ={:.2} ψ={:.2}",
            t.u, t.grade, t.score, t.sp_spent, t.phi_blue, t.psi_grade
        );
    }
}

fn cmd_batch(args: &[String]) {
    let mut f = parse_flags(args);
    if f.speed == 1 {
        f.speed = 100;
    }
    let count: i64 = args
        .iter()
        .find_map(|a| a.strip_prefix("--count="))
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let seed_start = f.seed;
    let root = detect_repo_root().unwrap_or_else(|| PathBuf::from("."));
    let out_rel = f
        .output
        .clone()
        .unwrap_or_else(|| format!("out/sim-batch/batch-{}-{}.jsonl", f.scenario, seed_start));
    let out_path = root.join(&out_rel);
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut file = match std::fs::File::create(&out_path) {
        Ok(fh) => fh,
        Err(e) => {
            eprintln!("Failed to open {}: {e}", out_path.display());
            return;
        }
    };
    use std::io::Write;
    let start = Instant::now();
    let mut written = 0i64;
    let mut nonzero_u = 0i64;
    for i in 0..count {
        f.seed = seed_start + i;
        let mut engine = SimEngine::create(build_settings(&f));
        engine.start(build_meta(&f));
        play_with_policy(&mut engine, &f.policy);
        let Some(rec) = engine.take_terminal() else {
            eprintln!("seed={} missing terminal record", f.seed);
            continue;
        };
        if rec.u > 0.0 {
            nonzero_u += 1;
        }
        match serde_json::to_string(&rec) {
            Ok(line) => {
                if let Err(e) = writeln!(file, "{line}") {
                    eprintln!("write error: {e}");
                    return;
                }
                written += 1;
            }
            Err(e) => eprintln!("serialize error seed={}: {e}", f.seed),
        }
    }
    let elapsed = start.elapsed().as_secs_f64().max(1e-9);
    let cpm = (written as f64) / elapsed * 60.0;
    println!(
        "Batch wrote {written}/{count} records to {} (nonzero U={nonzero_u}) in {elapsed:.2}s → {cpm:.1} careers/min policy={}",
        out_path.display(),
        f.policy
    );
}

fn cmd_validate(args: &[String]) {
    let path_arg = args
        .iter()
        .find_map(|a| a.strip_prefix("--path="))
        .unwrap_or("content_packs/example.json");
    let root = detect_repo_root().unwrap_or_else(|| PathBuf::from("."));
    let path = root.join(path_arg);
    if !path.exists() {
        println!("Not found: {}", path.display());
        return;
    }
    let events = ContentPackLoader::load_pack_file(&path);
    println!(
        "Content pack {path_arg}: {} events (valid JSON structure)",
        events.len()
    );
}

fn cmd_deck_place(args: &[String]) {
    let support_id = args.first().map(|s| s.as_str());
    let facility_raw = args.get(1).map(|s| s.as_str());
    let (Some(support_id), Some(facility_raw)) = (support_id, facility_raw) else {
        println!("Usage: deck place <supportId> <facility>");
        return;
    };
    if support_id.is_empty() || facility_raw.is_empty() {
        println!("Usage: deck place <supportId> <facility>");
        return;
    }
    let Some(facility) = DeckPlacement::parse_facility_name(facility_raw) else {
        println!("Unknown facility: {facility_raw}");
        return;
    };
    let Some((mut engine, _)) = RunSession::load() else {
        println!("No session. Run: start --seed=42");
        return;
    };
    if !engine.assign_deck_slot(support_id, facility) {
        println!("Failed to place {support_id} on {facility_raw} (missing card or facility full)");
        return;
    }
    if let Err(e) = RunSession::save(&engine) {
        eprintln!("Failed to save session: {e}");
    }
    println!("Placed {support_id} on {}", facility.key());
}

fn print_result(lines: &[String], choice_ids: &[String]) {
    for line in lines {
        println!("{line}");
    }
    if !choice_ids.is_empty() {
        println!("Choices: {}", choice_ids.join(", "));
    }
}

fn print_usage() {
    println!(
        "\
uma-sim CLI v0.4 (Rust)
  start [--seed=N] [--scenario=ura|grand_concert|unity|trackblazer]
        [--trainee=Name] [--speed=1-100] [--dialogue=off|choices|full]
        [--deck=id1,id2,...] [--legacy=factor:blue:1@3,...]
        Deck: support:10001@speed:85 (manual facility + bond)
  deck place <supportId> <facility>
        [--policy=default|bot] [--race-model=stub|physics] [--trace-rng] [--trace-telemetry]
  state
  step [train_speed|rest|race|event_0|...]
  fast [--seed=N] [--speed=20] [--policy=default|bot|external]
  batch [--count=100] [--seed=N] [--scenario=ura] [--policy=external|bot|default] [--output=out/sim-batch/...]
  validate [--path=content_packs/example.json]
  content validate [--path=...]
  serve [--port=8765] [--open]
  clear
Env: UMA_RACE_MODEL=stub|physics (default stub until R8.8)
Session persisted to .uma-sim/session.json"
    );
}
