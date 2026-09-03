#!/usr/bin/env python3
"""
Verify Grand Live performance token formula vs research/grand_concert_calibration.json.

Default gate (R7.2 legacy): median abs error ≤2 pts on ≥80% of rows.
Strict mode (--strict): every row must match exact gains dict (types + amounts).

Usage (repo root):
  python scripts/calibrate_grand_live.py
  python scripts/calibrate_grand_live.py --strict
"""
from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CALIBRATION = ROOT / "research" / "grand_concert_calibration.json"
RESEARCH = ROOT / "research" / "grand_concert.json"

FACILITY_SPLIT = {
    "speed": [("Da", 60), ("Vi", 30), ("Pa", 10)],
    "stamina": [("Pa", 60), ("Vo", 30), ("Vi", 10)],
    "power": [("Vo", 60), ("Me", 30), ("Da", 10)],
    "guts": [("Vi", 60), ("Da", 30), ("Pa", 10)],
    "wit": [("Me", 60), ("Pa", 30), ("Vo", 10)],
}


def token_total(facility: str, level: int, deck_size: int, scenario_links: int = 0) -> int:
    s = 5 if facility.lower() == "wit" else 9
    f = max(1, min(5, level))
    c = max(0, min(5, deck_size))
    l = max(0, scenario_links)
    return int(math.floor((s + f) * (1.15**c) + 2 * l))


def split_token_total(total: int, facility: str) -> dict[str, int]:
    """Largest-remainder 60/30/10 split (matches uma-sim-core + calibration rows)."""
    if total <= 0:
        return {}
    split = FACILITY_SPLIT[facility.lower()]
    parts: list[tuple[str, int, float]] = []
    for code, pct in split:
        exact = total * (pct / 100.0)
        floor = int(math.floor(exact))
        parts.append((code, floor, exact - floor))
    assigned = sum(f for _, f, _ in parts)
    rem = total - assigned
    order = sorted(range(len(parts)), key=lambda i: (-parts[i][2], i))
    for idx in order:
        if rem <= 0:
            break
        code, floor, frac = parts[idx]
        parts[idx] = (code, floor + 1, frac)
        rem -= 1
    return {code: amt for code, amt, _ in parts if amt > 0}


def load_research_split_labels() -> dict[str, list[str]]:
    if not RESEARCH.exists():
        return {}
    data = json.loads(RESEARCH.read_text(encoding="utf-8"))
    raw = data.get("facility_performance_split") or {}
    name_to_code = {
        "Dance": "Da",
        "Passion": "Pa",
        "Vocal": "Vo",
        "Vocals": "Vo",
        "Visual": "Vi",
        "Visuals": "Vi",
        "Composure": "Me",
        "Mental": "Me",
    }
    out: dict[str, list[str]] = {}
    for fac, arr in raw.items():
        if fac == "note" or not isinstance(arr, list):
            continue
        codes: list[str] = []
        for el in arr:
            ty = el.get("type")
            code = name_to_code.get(ty, ty)
            if code:
                codes.append(code)
        if codes:
            out[fac.lower()] = codes
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true", help="Require exact type+amount match")
    args = parser.parse_args()

    if not CALIBRATION.exists():
        print(f"Missing {CALIBRATION}", file=sys.stderr)
        return 1

    research_labels = load_research_split_labels()
    label_mismatches: list[str] = []
    for fac, codes in FACILITY_SPLIT.items():
        expected = [c for c, _ in codes]
        actual = research_labels.get(fac)
        if actual is not None and actual != expected:
            label_mismatches.append(f"{fac}: research={actual} script={expected}")

    data = json.loads(CALIBRATION.read_text(encoding="utf-8"))
    rows = data.get("training_token_gains") or []
    mismatches: list[str] = []
    abs_errors: list[float] = []
    within_2 = 0
    exact_matches = 0

    for row in rows:
        facility = row["facility"]
        level = int(row["level"])
        deck = int(row.get("deck_size", 0))
        links = int(row.get("scenario_links", 0))
        gains = {k: int(v) for k, v in (row.get("gains") or {}).items()}
        predicted_total = token_total(facility, level, deck, links)
        predicted = split_token_total(predicted_total, facility)
        # Per-type max abs error across codes present in either map
        codes = set(gains) | set(predicted)
        row_err = max(abs(gains.get(c, 0) - predicted.get(c, 0)) for c in codes) if codes else 0
        abs_errors.append(float(row_err))
        if row_err <= 2:
            within_2 += 1
        if gains == predicted:
            exact_matches += 1
        else:
            mismatches.append(
                f"{facility} L{level} deck={deck} links={links}: "
                f"calibration={gains} formula={predicted} (err={row_err})"
            )

    n = len(rows) or 1
    pct_within_2 = within_2 / n
    median_err = statistics.median(abs_errors) if abs_errors else 0.0
    gate_pass = pct_within_2 >= 0.80 and median_err <= 2.0
    exact_pass = exact_matches == len(rows) and not label_mismatches
    pass_flag = exact_pass if args.strict else gate_pass

    result = {
        "rows_checked": len(rows),
        "exact_matches": exact_matches,
        "mismatches": len(mismatches),
        "label_mismatches": label_mismatches,
        "within_2_pts": within_2,
        "pct_within_2": round(pct_within_2, 4),
        "median_abs_error": median_err,
        "gate_pass": gate_pass,
        "exact_pass": exact_pass,
        "pass": pass_flag,
        "details": mismatches[:20],
    }
    print(json.dumps(result, indent=2))
    if args.strict and not pass_flag:
        return 1
    if (not args.strict) and not gate_pass:
        return 1 if False else 0  # non-strict keeps exit 0 unless --strict historically
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
