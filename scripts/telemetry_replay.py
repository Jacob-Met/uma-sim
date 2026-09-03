#!/usr/bin/env python3
"""Convert Android TurnTelemetry JSONL to sim_replay lines for BotDecisionAdapter harness."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_turn(line: dict) -> dict | None:
    if line.get("type") not in (None, "turn") and "decision" not in line:
        return None
    decision = line.get("decision") or {}
    action = decision.get("action")
    pre = line.get("pre") or {}
    stats = pre.get("stats") or {}
    if action in ("TRAIN", "REST"):
        return {
            "type": "sim_replay",
            "kind": "training",
            "energy": pre.get("energy", 60),
            "mood": pre.get("mood", "NORMAL"),
            "speed": stats.get("speed", 200),
            "stamina": stats.get("stamina", 200),
            "power": stats.get("power", 200),
            "guts": stats.get("guts", 200),
            "wit": stats.get("wit", 200),
            "expectedFacility": decision.get("trainingStat") or "speed",
            "botTrainingStat": decision.get("trainingStat") or "speed",
            "scenario": line.get("scenario", "ura"),
        }
    return None


def parse_event_decision(line: dict) -> dict | None:
    if line.get("type") != "event_decision":
        return None
    return {
        "type": "sim_replay",
        "kind": "event",
        "expectedIndex": line.get("optionIndex", 0),
        "botChoiceIndex": line.get("optionIndex", 0),
        "options": line.get("options") or [],
        "energy": line.get("energy", 60),
        "scenario": line.get("scenario", "ura"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert bot telemetry JSONL to sim_replay format")
    parser.add_argument("input", type=Path, help="Android telemetry .jsonl file")
    parser.add_argument("-o", "--output", type=Path, help="Output sim_replay .jsonl")
    args = parser.parse_args()
    out_lines: list[str] = []
    for raw in args.input.read_text(encoding="utf-8").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            obj = json.loads(raw)
        except json.JSONDecodeError:
            continue
        converted = parse_turn(obj) or parse_event_decision(obj)
        if converted:
            out_lines.append(json.dumps(converted, separators=(",", ":")))
    text = "\n".join(out_lines) + ("\n" if out_lines else "")
    if args.output:
        args.output.write_text(text, encoding="utf-8")
        print(f"Wrote {len(out_lines)} replay lines to {args.output}", file=sys.stderr)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
