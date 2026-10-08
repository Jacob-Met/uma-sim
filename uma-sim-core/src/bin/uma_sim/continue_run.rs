//! Continue the career already stored by RunSession; never start a new engine run.
use uma_sim_core::session::RunSession;

const USAGE: &str = "Usage: uma-sim continue [--policy=default|bot]";

pub fn run(args: &[String]) -> i32 {
    if args.len() == 1 && args[0] == "--help" {
        println!("{USAGE}\nFinish the saved career in .uma-sim/session.json using its existing settings.");
        return 0;
    }
    let policy = match args {
        [] => "default",
        [arg] if arg == "--policy=default" => "default",
        [arg] if arg == "--policy=bot" => "bot",
        _ => {
            eprintln!("{USAGE}\nOnly one local policy option is accepted; saved career settings are retained.");
            return 2;
        }
    };

    let Some((mut engine, snapshot)) = RunSession::load() else {
        eprintln!("No readable saved career in .uma-sim/session.json. Start a career first or restore a valid save.");
        return 1;
    };
    if snapshot.meta != snapshot.state.meta {
        eprintln!("Saved career metadata is inconsistent; the save was not changed.");
        return 1;
    }
    if !uma_sim_core::is_known_scenario(&snapshot.meta.scenario_id) {
        eprintln!("Saved career has an unknown scenario; the save was not changed.");
        return 1;
    }
    if engine.state().career_complete {
        println!(
            "Saved career is already complete at turn {}; the save was not changed.",
            engine.state().turn
        );
        return 0;
    }

    let resumed_turn = engine.state().turn;
    match policy {
        "bot" => engine.play_to_completion_scoring(500),
        _ => engine.play_to_completion(500),
    }
    if !engine.state().career_complete {
        eprintln!(
            "Saved career did not complete within the action limit; the save was not changed."
        );
        return 1;
    }
    if let Err(error) = RunSession::save(&engine) {
        eprintln!("Failed to save continued career: {error}");
        return 1;
    }

    let summary = engine.run_summary();
    println!(
        "Continued saved career from turn {resumed_turn} to turn {} | policy={policy}",
        summary.turn
    );
    println!(
        "Ended: true | seed={} scenario={} fans={} speed={} SP={}",
        summary.seed, summary.scenario, summary.fans, summary.speed, summary.sp
    );
    0
}
