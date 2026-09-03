"""Cross-check research/race_course_data.json geometry against fork course_data.

mdb metadata is separate (race_course_mdb_crosscheck.json). This script verifies
corners/straights/slopes (+ shared meta) vs uma-skill-tools/data/course_data.json.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OURS = ROOT / "research" / "race_course_data.json"
FORK = Path(r"C:\Programming\umalator-ref\uma-tools\uma-skill-tools\data\course_data.json")
OUT = ROOT / "research" / "race_course_fork_crosscheck.json"


def norm_corners(c: dict):
    return [
        (round(float(x["start"]), 6), round(float(x["length"]), 6))
        for x in c.get("corners") or []
    ]


def norm_straights(c: dict):
    return [
        (
            round(float(x["start"]), 6),
            round(float(x["end"]), 6),
            x.get("frontType"),
        )
        for x in c.get("straights") or []
    ]


def norm_slopes(c: dict):
    return [
        (
            round(float(x["start"]), 6),
            round(float(x["length"]), 6),
            round(float(x.get("slope", 0)), 6),
        )
        for x in c.get("slopes") or []
    ]


def main() -> None:
    root = json.loads(OURS.read_text(encoding="utf-8"))
    ours = root["courses"]
    fork = json.loads(FORK.read_text(encoding="utf-8"))
    ids = sorted(int(k) for k in ours)
    mismatches = []
    for i in ids:
        a = ours.get(str(i))
        b = fork.get(str(i))
        if not a or not b:
            mismatches.append(
                {"id": i, "field": "missing", "ours": a is not None, "fork": b is not None}
            )
            continue
        for key in ("distance", "surface", "turn", "raceTrackId", "distanceType", "laneMax"):
            va, vb = a.get(key), b.get(key)
            if va != vb:
                mismatches.append({"id": i, "field": key, "ours": va, "fork": vb})
        if norm_corners(a) != norm_corners(b):
            mismatches.append({"id": i, "field": "corners"})
        if norm_straights(a) != norm_straights(b):
            mismatches.append({"id": i, "field": "straights"})
        if norm_slopes(a) != norm_slopes(b):
            mismatches.append({"id": i, "field": "slopes"})

    geom_m = [m for m in mismatches if m.get("field") in ("corners", "straights", "slopes")]
    meta_ok = all(
        m.get("field") == "laneMax" and m.get("id") == 10301 for m in mismatches if m not in geom_m
    ) or not [m for m in mismatches if m.get("field") not in ("corners", "straights", "slopes")]
    # Allow only the documented 10301 laneMax mdb override among meta mismatches.
    allowed_meta = [
        m
        for m in mismatches
        if m.get("field") == "laneMax" and m.get("id") == 10301
    ]
    other_meta = [
        m
        for m in mismatches
        if m.get("field") not in ("corners", "straights", "slopes", "laneMax")
        or (m.get("field") == "laneMax" and m.get("id") != 10301)
    ]
    out = {
        "provenance": "R8.2: research/race_course_data.json vs fork uma-skill-tools/data/course_data.json",
        "courses_compared": len(ids),
        "career_course_ids_required": root.get("career_course_ids_required"),
        "career_course_ids_covered": root.get("career_course_ids_covered"),
        "fork_geometry_identical": len(geom_m) == 0,
        "mismatches": mismatches,
        "allowed_mdb_overrides": allowed_meta,
        "notes": [
            "10301 laneMax: ours 13500 from master.mdb float_lane_max; fork 23500 — mdb/telemetry wins",
            "Raw courseeventparams differ systematically (final corner length / last straight start); fork course_data is the processed physics table we ship",
            "mdb metadata: research/race_course_mdb_crosscheck.json (JP-only 11605/11612 absent from this mdb)",
        ],
        "ok": len(geom_m) == 0 and len(other_meta) == 0,
    }
    OUT.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print(f"wrote {OUT} ok={out['ok']} mismatches={len(mismatches)}")


if __name__ == "__main__":
    main()
