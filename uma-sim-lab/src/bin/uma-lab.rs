//! `uma-lab` — saved-career branch-and-compare laboratory CLI.

use std::path::PathBuf;
use uma_sim_core::SimSettings;
use uma_sim_lab::{
    compare_timelines, policy_names, render_comparison, status_line, LabError, LabLibrary,
};

fn fatal(e: LabError) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}

fn or_fatal<T>(r: Result<T, LabError>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => fatal(e),
    }
}

fn usage() -> ! {
    eprintln!(
        "uma-lab — saved-career branch-and-compare laboratory\n\
         \n\
         start <checkpoint> [--seed=N] [--scenario=ura] [--trainee=NAME] [--turns=N] [--note=...]\n\
         list [--branches]\n\
         show <checkpoint>\n\
         status <branch>\n\
         fork <checkpoint> <branch> [--note=...]\n\
         play <branch> [--policy=NAME] [--turns=N]\n\
         step <branch> <action-id>\n\
         compare <branch-a> <branch-b> [--out=FILE]\n\
         export <checkpoint> <file>\n\
         import <file>\n\
         export-branch <branch> <file>\n\
         delete [--branch] <name>\n\
         \n\
         policies: {}\n\
         library dir: $UMA_LAB_DIR or ~/.uma-sim/lab",
        policy_names().join("|")
    );
    std::process::exit(2);
}

fn flag(args: &[String], key: &str) -> Option<String> {
    let prefix = format!("--{key}=");
    args.iter()
        .find_map(|a| a.strip_prefix(&prefix).map(|v| v.to_string()))
}

fn flag_u32(args: &[String], key: &str, default: u32) -> u32 {
    flag(args, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn flag_i64(args: &[String], key: &str, default: i64) -> i64 {
    flag(args, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    let lib = or_fatal(LabLibrary::new(LabLibrary::default_dir()));

    match args[0].as_str() {
        "start" => {
            let name = args.get(1).unwrap_or_else(|| usage()).clone();
            let seed = flag_i64(&args, "seed", 42);
            let scenario = flag(&args, "scenario").unwrap_or_else(|| "ura".to_string());
            let trainee = flag(&args, "trainee").unwrap_or_else(|| "Special Week".to_string());
            let pre_turns = flag_u32(&args, "turns", 0);
            let note = flag(&args, "note").unwrap_or_default();
            let mut engine = or_fatal(LabLibrary::start_career(
                seed,
                &scenario,
                &trainee,
                SimSettings::default(),
            ));
            if pre_turns > 0 {
                let policy = or_fatal(uma_sim_lab::resolve_policy("default"));
                let max_actions = pre_turns * 4 + 24;
                let mut actions = 0;
                while !engine.state().career_complete
                    && engine.state().turn < pre_turns as i32
                    && actions < max_actions
                {
                    let choices = engine.choices();
                    if choices.is_empty() {
                        break;
                    }
                    engine.step(policy(&choices, engine.state()));
                    actions += 1;
                }
            }
            let rec = or_fatal(lib.save_checkpoint(&engine, &name, &note));
            println!(
                "checkpoint '{}' saved: seed={} scenario={} turn={} rng_calls={} note={:?}",
                rec.name, rec.seed, rec.scenario_id, rec.turn, rec.rng_calls, rec.note
            );
        }
        "list" => {
            if args.iter().any(|a| a == "--branches") {
                for s in or_fatal(lib.list_branches()) {
                    println!(
                        "{:<20} turn={:<4} seed={:<8} {} {}  {}",
                        s.name, s.turn, s.seed, s.scenario_id, s.created_at, s.note
                    );
                }
            } else {
                for s in or_fatal(lib.list_checkpoints()) {
                    println!(
                        "{:<20} turn={:<4} seed={:<8} {} {}  {}",
                        s.name, s.turn, s.seed, s.scenario_id, s.created_at, s.note
                    );
                }
            }
        }
        "show" => {
            let name = args.get(1).unwrap_or_else(|| usage());
            let rec = or_fatal(lib.load_checkpoint(name));
            println!(
                "checkpoint '{}': seed={} scenario={} trainee={} turn={} complete={} rng_calls={}\nnote: {}",
                rec.name, rec.seed, rec.scenario_id, rec.trainee_name,
                rec.turn, rec.career_complete, rec.rng_calls, rec.note
            );
        }
        "status" => {
            let name = args.get(1).unwrap_or_else(|| usage());
            let rec = or_fatal(lib.load_branch(name));
            println!("{}", status_line(&rec));
        }
        "fork" => {
            let cp = args.get(1).unwrap_or_else(|| usage()).clone();
            let br = args.get(2).unwrap_or_else(|| usage()).clone();
            let note = flag(&args, "note").unwrap_or_default();
            let rec = or_fatal(lib.fork_checkpoint(&cp, &br, &note));
            println!(
                "branch '{}' forked from checkpoint '{}' (turn={}, rng_calls={})",
                rec.name, rec.parent_checkpoint, rec.snapshot.state.turn, rec.snapshot.rng_calls
            );
        }
        "play" => {
            let br = args.get(1).unwrap_or_else(|| usage()).clone();
            let policy = flag(&args, "policy").unwrap_or_else(|| "default".to_string());
            let turns = flag_u32(&args, "turns", 20);
            let tl = or_fatal(lib.play_branch(&br, &policy, turns));
            println!(
                "branch '{}' played {} steps (policy={}): turn {} -> {}, hash={}",
                tl.branch,
                tl.steps.len(),
                tl.policy,
                tl.start_turn,
                tl.end_turn,
                tl.hash
            );
            if let Some(l) = tl.steps.last() {
                println!(
                    "head: energy={} stats={}/{}/{}/{}/{} fans={} sp={} races={} complete={} rng_calls={}",
                    l.energy, l.speed, l.stamina, l.power, l.guts, l.wit,
                    l.fans, l.skill_points, l.races_completed, l.career_complete, l.rng_calls
                );
            }
        }
        "step" => {
            let br = args.get(1).unwrap_or_else(|| usage()).clone();
            let action = args.get(2).unwrap_or_else(|| usage()).clone();
            for l in or_fatal(lib.step_branch(&br, &action)) {
                println!("{l}");
            }
            println!("{}", status_line(&or_fatal(lib.load_branch(&br))));
        }
        "compare" => {
            let a = args.get(1).unwrap_or_else(|| usage()).clone();
            let b = args.get(2).unwrap_or_else(|| usage()).clone();
            let ta = or_fatal(lib.load_timeline(&a));
            let tb = or_fatal(lib.load_timeline(&b));
            let text = render_comparison(&compare_timelines(&ta, &tb));
            if let Some(out) = flag(&args, "out") {
                std::fs::write(&out, &text).unwrap_or_else(|e| {
                    eprintln!("cannot write {out}: {e}");
                    std::process::exit(1);
                });
                println!("comparison written to {out}");
            } else {
                print!("{text}");
            }
        }
        "export" => {
            let name = args.get(1).unwrap_or_else(|| usage()).clone();
            let file = args.get(2).unwrap_or_else(|| usage()).clone();
            or_fatal(lib.export_checkpoint(&name, &PathBuf::from(&file)));
            println!("checkpoint '{name}' exported to {file}");
        }
        "import" => {
            let file = args.get(1).unwrap_or_else(|| usage()).clone();
            let rec = or_fatal(lib.import_checkpoint(&PathBuf::from(&file)));
            println!("checkpoint '{}' imported (turn={})", rec.name, rec.turn);
        }
        "export-branch" => {
            let name = args.get(1).unwrap_or_else(|| usage()).clone();
            let file = args.get(2).unwrap_or_else(|| usage()).clone();
            or_fatal(lib.export_branch(&name, &PathBuf::from(&file)));
            println!("branch '{name}' exported to {file}");
        }
        "delete" => {
            let is_branch = args.iter().any(|a| a == "--branch");
            let mut positional = args.iter().skip(1).filter(|a| !a.starts_with("--"));
            let name = match positional.next() {
                Some(n) => n.clone(),
                None => usage(),
            };
            if is_branch {
                or_fatal(lib.delete_branch(&name));
            } else {
                or_fatal(lib.delete_checkpoint(&name));
            }
            println!("deleted {name}");
        }
        _ => usage(),
    }
}
