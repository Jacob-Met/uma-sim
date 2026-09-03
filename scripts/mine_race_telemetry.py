"""Mine live bot logs for race-result corpus (R8.6 V4 bootstrap)."""
from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNS = ROOT / "runs"

RE_LOOKUP = re.compile(
    r"Looking up race for turn (\d+) with detected name: \"([^\"]+)\""
)
RE_RESULT = re.compile(r"Race result detected - 1st place: (true|false)")
RE_STRATEGY = re.compile(r"Junior Year race strategy|set .* race strategy|strategy override: (\w+)", re.I)
RE_GRADE = re.compile(r"Racing process for .+ is completed\. Grade: (\w+)")

rows = []
for path in RUNS.rglob("*.txt"):
    if "http" not in str(path).replace("\\", "/") and "tail" not in path.name:
        # still allow http logs and flat tails
        if "logs" not in str(path) and not path.name.startswith("tail"):
            continue
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        continue
    pending = None
    for line in text.splitlines():
        m = RE_LOOKUP.search(line)
        if m:
            pending = {"turn": int(m.group(1)), "race_name": m.group(2), "source": str(path.relative_to(ROOT))}
            continue
        m = RE_RESULT.search(line)
        if m and pending:
            pending["win"] = m.group(1) == "true"
            pending["place_bucket"] = "first" if pending["win"] else "not_first"
            rows.append(pending)
            pending = None
            continue
        m = RE_GRADE.search(line)
        if m and rows and "grade" not in rows[-1]:
            # attach to most recent if same file flow
            rows[-1]["grade"] = m.group(1)

by_name = defaultdict(lambda: {"n": 0, "wins": 0})
for r in rows:
    k = r["race_name"]
    by_name[k]["n"] += 1
    by_name[k]["wins"] += int(r["win"])

summary = {
    "status": "bootstrap_from_live_logs — win/not_first only (no NPC stats/margins yet)",
    "schema_version": 1,
    "total_races": len(rows),
    "wins": sum(1 for r in rows if r["win"]),
    "win_rate": (sum(1 for r in rows if r["win"]) / len(rows)) if rows else 0.0,
    "by_race_name": {
        k: {
            "n": v["n"],
            "wins": v["wins"],
            "win_rate": v["wins"] / v["n"] if v["n"] else 0.0,
        }
        for k, v in sorted(by_name.items(), key=lambda kv: -kv[1]["n"])
    },
    "races": rows[:500],
    "notes": [
        "Extracted from bot [RACE] log lines under runs/**.",
        "V4 gate needs trainee stats + full placement + margins; this corpus is win-rate prior only.",
        "Use to calibrate placeholder_npc_field aggressiveness, not as full distribution match yet.",
    ],
}

out = ROOT / "research" / "race_telemetry_corpus.json"
out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
print(f"wrote {out} races={len(rows)} win_rate={summary['win_rate']:.3f}")
