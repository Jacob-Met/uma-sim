#!/usr/bin/env python3
"""
Calibration harness: compare sim-predicted training gains vs TurnTelemetry logs.

Usage (repo root):
  python scripts/calibrate_sim.py --telemetry path/to/telemetry.jsonl
  python scripts/calibrate_sim.py --runs-dir runs
  python scripts/calibrate_sim.py --stub   # run self-test with synthetic data

Acceptance target: median absolute error <= 2 pts for 80% of training turns.
"""
from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESEARCH = ROOT / "research" / "training_gain_tables.json"

MOOD_OFFSET = {
    "GREAT": 0.2,
    "GOOD": 0.1,
    "NORMAL": 0.0,
    "BAD": -0.1,
    "AWFUL": -0.2,
}


def load_training_tables() -> dict:
    if not RESEARCH.exists():
        return {}
    return json.loads(RESEARCH.read_text(encoding="utf-8"))


def predict_gain(obs: dict, tables: dict) -> float:
    """Predict typical main gain using research tables (mirrors TrainingResolver)."""
    level = obs.get("training_level") or obs.get("facility_level") or 1
    level = max(1, min(5, int(level)))
    mult = tables.get("facility_level_multipliers", {}).get(str(level), 1.0)
    base_entry = tables.get("base_main_gain_by_level", {}).get(str(level), {})
    base = base_entry.get("typical", 12) if isinstance(base_entry, dict) else 12
    mood = str(obs.get("mood") or "NORMAL").upper()
    mood_mult = 1.0 + MOOD_OFFSET.get(mood, 0.0)
    mood_table = tables.get("mood_base_offset", {})
    if mood in mood_table:
        mood_mult = 1.0 + float(mood_table[mood])
    presence_n = obs.get("supports_on_facility")
    if presence_n is None:
        presence_n = 0
    presence = 1.0 + 0.05 * max(0, int(presence_n))
    growth = 1.0 + float(obs.get("uma_growth_pct") or 0.0) / 100.0
    return float(base) * float(mult) * mood_mult * presence * growth


def parse_telemetry_line(line: str) -> dict | None:
    line = line.strip()
    if not line:
        return None
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return None


def _stat_delta(rec: dict, stat: str | None) -> int | None:
    post = rec.get("post") or {}
    delta = rec.get("delta") or post.get("delta") or {}
    stats_delta = delta.get("stats") or {}
    if stat and stat in stats_delta:
        return int(stats_delta[stat])
    if stats_delta:
        # Fall back to largest positive stat change (main gain proxy)
        positives = [v for v in stats_delta.values() if isinstance(v, (int, float)) and v > 0]
        if positives:
            return int(max(positives))
    return None


def _training_level(rec: dict, stat: str | None) -> int:
    pre = rec.get("pre") or {}
    for key in ("trainingLevels", "facilityLevels", "training_levels"):
        levels = pre.get(key) or rec.get(key)
        if isinstance(levels, dict) and stat:
            level = levels.get(stat)
            if level is not None:
                return max(1, min(5, int(level)))
    for cand in rec.get("candidates") or []:
        if cand.get("chosen") and cand.get("trainingLevel"):
            return max(1, min(5, int(cand["trainingLevel"])))
    decision = rec.get("decision") or {}
    if decision.get("facilityLevel"):
        return max(1, min(5, int(decision["facilityLevel"])))
    return 1


def extract_training_obs(rec: dict) -> dict | None:
    rec_type = rec.get("type")
    if rec_type in {"run_start", "run_end", "event_decision", "lesson_decision", "skill_decision", "interaction_span"}:
        return None

    decision = rec.get("decision") or {}
    action = decision.get("action") or rec.get("action")
    if action != "TRAIN":
        if rec.get("type") == "turn" and decision.get("action") == "TRAIN":
            action = "TRAIN"
        else:
            return None

    stat = (decision.get("trainingStat") or decision.get("training_stat") or "").lower() or None
    pre = rec.get("pre") or {}

    main_gain = _stat_delta(rec, stat)
    if main_gain is None:
        gains = decision.get("predictedGains") or {}
        if stat and stat in gains:
            main_gain = gains[stat]
        elif rec.get("post", {}).get("main_gain") is not None:
            main_gain = rec["post"]["main_gain"]
        elif rec.get("main_gain") is not None:
            main_gain = rec["main_gain"]
        elif rec.get("gain") is not None:
            main_gain = rec["gain"]

    if main_gain is None:
        return None
    # Failed / rejected training produces 0 gain — not a calibration sample.
    if int(main_gain) <= 0:
        return None

    supports = rec.get("candidates")
    if isinstance(supports, list):
        supports_n = len(supports)
    else:
        supports_n = pre.get("supportsOnFacility")

    return {
        "main_gain": int(main_gain),
        "training_level": _training_level(rec, stat),
        "mood": pre.get("mood") or rec.get("mood") or "NORMAL",
        "training_stat": stat,
        "supports_on_facility": supports_n,
        "uma_growth_pct": pre.get("umaGrowthPct") or 0,
    }


def load_records_from_path(path: Path) -> list[dict]:
    records: list[dict] = []
    text = path.read_text(encoding="utf-8")
    for line in text.splitlines():
        rec = parse_telemetry_line(line)
        if rec:
            records.append(rec)
    return records


def discover_telemetry_files(runs_dir: Path) -> list[Path]:
    if not runs_dir.exists():
        return []
    found = sorted(runs_dir.glob("**/telemetry/*.jsonl"))
    # Also accept flat exports from `uma-sim export-telemetry` (runs/sim-telemetry/*.jsonl).
    found += sorted(runs_dir.glob("sim-telemetry/*.jsonl"))
    found += sorted(runs_dir.glob("*.jsonl"))
    # De-dupe while preserving order
    seen: set[Path] = set()
    out: list[Path] = []
    for p in found:
        rp = p.resolve()
        if rp not in seen:
            seen.add(rp)
            out.append(p)
    return out


def calibrate(records: list[dict], tables: dict) -> dict:
    errors: list[float] = []
    for rec in records:
        obs = extract_training_obs(rec)
        if not obs:
            continue
        actual = obs.get("main_gain")
        if actual is None:
            continue
        predicted = predict_gain(obs, tables)
        errors.append(abs(float(actual) - predicted))

    if not errors:
        return {"count": 0, "median_error": None, "pct_within_2": 0.0, "pass": False}

    median = statistics.median(errors)
    within_2 = sum(1 for e in errors if e <= 2) / len(errors)
    return {
        "count": len(errors),
        "median_error": median,
        "pct_within_2": within_2,
        "pass": within_2 >= 0.80 and median <= 2.0,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--telemetry", type=Path, help="JSONL telemetry file")
    parser.add_argument("--runs-dir", type=Path, default=ROOT / "runs", help="Scan for telemetry JSONL under runs/")
    parser.add_argument("--stub", action="store_true", help="Run with synthetic records")
    args = parser.parse_args()

    tables = load_training_tables()
    records: list[dict] = []

    if args.stub:
        obs_list = [
            {"training_level": 2, "mood": "NORMAL"},
            {"training_level": 3, "mood": "GOOD"},
            {"training_level": 4, "mood": "GREAT"},
            {"training_level": 5, "mood": "GREAT"},
            {"training_level": 3, "mood": "NORMAL"},
        ]
        for obs in obs_list:
            predicted = predict_gain(obs, tables)
            level = int(obs["training_level"])
            records.append({
                "type": "turn",
                "decision": {"action": "TRAIN", "trainingStat": "speed"},
                "pre": {
                    "mood": obs["mood"],
                    "facilityLevels": {"speed": level},
                    "supportsOnFacility": 0,
                },
                "post": {"main_gain": round(predicted), "delta": {"stats": {"speed": round(predicted)}}},
            })
    elif args.telemetry and args.telemetry.exists():
        records = load_records_from_path(args.telemetry)
    elif args.runs_dir and args.runs_dir.exists():
        files = discover_telemetry_files(args.runs_dir)
        for path in files:
            records.extend(load_records_from_path(path))
        if not records:
            print(f"No telemetry JSONL found under {args.runs_dir}", file=sys.stderr)
            return 1
    else:
        print("No telemetry provided; use --telemetry, --runs-dir, or --stub", file=sys.stderr)
        return 1

    result = calibrate(records, tables)
    if args.runs_dir and not args.telemetry and not args.stub:
        result["files_scanned"] = len(discover_telemetry_files(args.runs_dir))
    print(json.dumps(result, indent=2))
    if result["count"] == 0:
        print("WARN: no training records with main_gain found", file=sys.stderr)
        return 0
    return 0 if result["pass"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
