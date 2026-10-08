//! R8.6 V4 soft gate: place + margin distributions from physics NPC fields
//! and mid-run career physics logs.
//!
//! Live bot game capture is still win/not_first (`race_telemetry_corpus.json`).
//! Career physics logs emit ordinal place + margin_win/margin_ahead; this gate
//! harvests those into `research/race_v4_career_place.json` and keeps the
//! Monte-Carlo placeholder field corpus in `race_v4_physics_dist.json`.

use serde_json::json;
use std::fs;
use std::path::PathBuf;

use uma_race_core::{
    get_course, simulate_field_synced, Aptitude, GroundCondition, HorseInput, PosKeepMode, Strategy,
};
use uma_sim_core::race::placeholder_npc_field;
use uma_sim_core::state::{DialogueMode, RunMeta, SimSettings};
use uma_sim_core::{RaceModel, SimEngine};

fn research_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../research")
        .join(name)
}

fn trainee(speed: f64, stam: f64, power: f64, guts: f64, wit: f64) -> HorseInput {
    HorseInput {
        speed,
        stamina: stam,
        power,
        guts,
        wisdom: wit,
        strategy: Strategy::Senkou,
        distance_apt: Aptitude::A,
        surface_apt: Aptitude::A,
        strategy_apt: Aptitude::A,
        mood: 1,
        skills: vec![],
    }
}

fn parse_place_token(tok: &str) -> Option<usize> {
    let digits: String = tok.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn parse_margin(phys: &str, key: &str) -> Option<f64> {
    let marker = format!("{key}=");
    let rest = phys.split(&marker).nth(1)?;
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num.parse().ok()
}

#[test]
fn r86_v4_place_margin_distribution_soft_gate() {
    let course = get_course(10606).expect("Tokyo 2000");
    let mid = trainee(800.0, 700.0, 700.0, 600.0, 600.0);
    let n = 48usize;
    let mut places = vec![0usize; 18];
    let mut win_margins = Vec::new();
    let mut loss_margins = Vec::new();
    let mut rows = Vec::new();

    for seed in 2000u32..(2000 + n as u32) {
        let mut field = vec![mid.clone()];
        field.extend(placeholder_npc_field("OP", 8, seed));
        let res = simulate_field_synced(
            course,
            GroundCondition::Good,
            &field,
            seed,
            PosKeepMode::Virtual,
        );
        let place = res
            .finishers
            .iter()
            .position(|f| f.index == 0)
            .map(|i| i + 1)
            .unwrap_or(field.len());
        let t0 = res
            .finishers
            .iter()
            .find(|f| f.index == 0)
            .map(|f| f.finish_time)
            .unwrap_or(0.0);
        let winner_t = res.finishers[0].finish_time;
        let margin_to_winner = (t0 - winner_t).max(0.0);
        let margin_ahead = if place < res.finishers.len() {
            (res.finishers[place].finish_time - t0).max(0.0)
        } else {
            0.0
        };
        if place <= places.len() {
            places[place - 1] += 1;
        }
        if place == 1 {
            win_margins.push(margin_ahead);
        } else {
            loss_margins.push(margin_to_winner);
        }
        rows.push(json!({
            "seed": seed,
            "place": place,
            "finish_time": t0,
            "margin_to_winner_s": margin_to_winner,
            "margin_ahead_s": margin_ahead,
        }));
    }

    let wins = places[0];
    let mean_place = {
        let mut s = 0.0;
        let mut c = 0.0;
        for (i, &cnt) in places.iter().enumerate() {
            s += (i + 1) as f64 * cnt as f64;
            c += cnt as f64;
        }
        s / c
    };
    let mean_loss_margin = if loss_margins.is_empty() {
        0.0
    } else {
        loss_margins.iter().sum::<f64>() / loss_margins.len() as f64
    };
    let mean_win_margin = if win_margins.is_empty() {
        0.0
    } else {
        win_margins.iter().sum::<f64>() / win_margins.len() as f64
    };

    let out = json!({
        "status": "physics Monte-Carlo V4 bootstrap — not live-game placement telemetry",
        "schema_version": 1,
        "n": n,
        "grade": "OP",
        "course_id": 10606,
        "trainee": "mid Senkou ~800",
        "place_hist": places,
        "wins": wins,
        "win_rate": wins as f64 / n as f64,
        "mean_place": mean_place,
        "mean_margin_to_winner_when_losing_s": mean_loss_margin,
        "mean_margin_ahead_when_winning_s": mean_win_margin,
        "live_log_prior": "research/race_telemetry_corpus.json win_rate≈0.70 (bot-selected races)",
        "samples": rows,
        "notes": [
            "Career physics log lines include ordinal place + margin_win/margin_ahead (engine do_race).",
            "Live-game capture still win/not_first only (research/race_telemetry_corpus.json).",
            "This corpus is the physics-side place/margin distribution gate vs placeholder NPCs.",
        ],
    });
    let path = research_path("race_v4_physics_dist.json");
    fs::write(&path, serde_json::to_string_pretty(&out).unwrap()).expect("write v4 dist");
    eprintln!(
        "V4 soft: win_rate={:.2} mean_place={:.2} loss_margin={:.3}s win_margin={:.3}s → {}",
        wins as f64 / n as f64,
        mean_place,
        mean_loss_margin,
        mean_win_margin,
        path.display()
    );

    assert!(
        wins < n && wins > 0,
        "mid@OP should sometimes win and sometimes not (wins={wins}/{n})"
    );
    assert!(
        mean_place > 1.5 && mean_place < 7.0,
        "mean place out of band: {mean_place}"
    );
    if wins < n {
        assert!(
            mean_loss_margin > 0.05,
            "losing margins should be >1 frame, got {mean_loss_margin}"
        );
    }
}

#[test]
fn r86_v4_career_physics_logs_emit_place_and_margins() {
    let seeds: &[i64] = &[7, 42, 99, 1234, 2026];
    let mut place_hist = vec![0usize; 18];
    let mut races = Vec::new();
    let mut total = 0usize;
    let mut with_margins = 0usize;

    for &seed in seeds {
        let mut engine = SimEngine::new(SimSettings {
            dialogue_mode: DialogueMode::Off,
            speed_multiplier: 50,
            race_model: RaceModel::Physics,
            ..Default::default()
        });
        engine.start(RunMeta::new(seed, "ura", "V4CareerHarvest"));
        engine.play_to_completion(500);
        assert!(
            engine.state().career_complete,
            "physics career seed={seed} should complete"
        );
        for line in &engine.state().log {
            if !line.starts_with("Race ") || !line.contains("physics t=") {
                continue;
            }
            let place_tok = line.split_whitespace().nth(2).unwrap_or("");
            let place = match parse_place_token(place_tok) {
                Some(p) if (1..=18).contains(&p) => p,
                _ => continue,
            };
            let phys = line
                .split('[')
                .nth(1)
                .map(|s| s.trim_end_matches(']'))
                .unwrap_or("");
            let mw = parse_margin(phys, "margin_win");
            let ma = parse_margin(phys, "margin_ahead");
            if mw.is_some() && ma.is_some() {
                with_margins += 1;
            }
            place_hist[place - 1] += 1;
            total += 1;
            races.push(json!({
                "seed": seed,
                "line": line,
                "place": place,
                "margin_win_s": mw,
                "margin_ahead_s": ma,
            }));
        }
    }

    let wins = place_hist[0];
    let mean_place = {
        let mut s = 0.0;
        let mut c = 0.0;
        for (i, &cnt) in place_hist.iter().enumerate() {
            s += (i + 1) as f64 * cnt as f64;
            c += cnt as f64;
        }
        if c == 0.0 {
            0.0
        } else {
            s / c
        }
    };
    let out = json!({
        "status": "career physics log harvest — ordinal place + margins from mid-run races",
        "schema_version": 1,
        "career_seeds": seeds,
        "n_races": total,
        "n_with_margins": with_margins,
        "wins": wins,
        "win_rate": if total == 0 { 0.0 } else { wins as f64 / total as f64 },
        "mean_place": mean_place,
        "place_hist": place_hist,
        "samples": races,
        "notes": [
            "Harvested from SimEngine physics careers (not live-game JSONL).",
            "Complements race_v4_physics_dist.json Monte-Carlo vs placeholder NPCs.",
            "Live-game telemetry still lacks ordinal place (race_telemetry_corpus.json).",
        ],
    });
    let path = research_path("race_v4_career_place.json");
    fs::write(&path, serde_json::to_string_pretty(&out).unwrap()).expect("write career place");
    eprintln!(
        "V4 career harvest: n={total} win_rate={:.2} mean_place={:.2} margins={with_margins} → {}",
        if total == 0 {
            0.0
        } else {
            wins as f64 / total as f64
        },
        mean_place,
        path.display()
    );

    assert!(
        total >= 10,
        "expected ≥10 physics races across seeds, got {total}"
    );
    assert_eq!(
        with_margins, total,
        "every physics race log should include margin_win and margin_ahead"
    );
    assert!(
        wins < total,
        "careers should not win every race under physics (wins={wins}/{total})"
    );
    assert!(
        mean_place >= 1.0 && mean_place <= 12.0,
        "mean place out of band: {mean_place}"
    );
}
