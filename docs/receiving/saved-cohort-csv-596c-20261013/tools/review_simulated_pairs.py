# SPDX-License-Identifier: GPL-3.0-only
"""Reconcile actual terminal records with saved paired CSV and JSON outputs.

This offline review does not run the simulator or authenticate production data.
The caller supplies the exact requested scenario, trainee and seed cohort.
"""
import argparse
import csv
import hashlib
import io
import json
import math
from pathlib import Path
import statistics


COLUMNS = ["scenario", "trainee", "seed", "baseline_score", "input_score",
           "score_delta", "baseline_grade", "input_grade"]
TERMINAL_FIELDS = {"seed", "scenario", "trainee", "u", "phi_blue", "psi_grade",
                   "grade", "score", "score_before_shop", "sp_spent", "sp_remaining",
                   "brackets", "stats"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, "Repeated JSON field: " + key)
        value[key] = item
    return value


def decode(text):
    def bad_constant(value):
        raise ValueError("Nonfinite JSON constant: " + value)
    return json.loads(text, object_pairs_hook=unique_object,
                      parse_constant=bad_constant)


def integer(value, low, high, label):
    require(type(value) is int and low <= value <= high, label)


def records(raw, expected, label):
    rows = {}
    for line in raw.decode("utf-8").splitlines():
        if not line.strip():
            continue
        row = decode(line)
        require(type(row) is dict and set(row) == TERMINAL_FIELDS,
                label + ": incomplete or changed terminal record")
        integer(row["seed"], -(2 ** 63), 2 ** 63 - 1, label + ": seed")
        for field in ["scenario", "trainee", "grade"]:
            require(type(row[field]) is str and bool(row[field]), label + ": " + field)
        for field in ["score", "score_before_shop", "sp_spent", "sp_remaining"]:
            integer(row[field], -(2 ** 31), 2 ** 31 - 1, label + ": " + field)
        for field in ["u", "phi_blue", "psi_grade"]:
            require(type(row[field]) in (int, float) and math.isfinite(row[field]),
                    label + ": " + field)
        for field, keys in [("brackets", {"at_or_above_600", "at_or_above_1100"}),
                            ("stats", {"speed", "stamina", "power", "guts", "wit"})]:
            require(type(row[field]) is dict and set(row[field]) == keys,
                    label + ": " + field)
            for key, value in row[field].items():
                integer(value, -(2 ** 31), 2 ** 31 - 1, label + ": " + field + "." + key)
        identity = (row["scenario"], row["trainee"], row["seed"])
        require(identity not in rows, label + ": repeated career")
        rows[identity] = row
    require(set(rows) == expected, label + ": produced cohort differs from request")
    return rows


def review(baseline, input_records, csv_raw, json_raw):
    pairs = []
    for scenario, trainee, seed in sorted(baseline):
        key = (scenario, trainee, seed)
        old, new = baseline[key], input_records[key]
        pairs.append(dict(zip(COLUMNS, [scenario, trainee, seed, old["score"],
                                       new["score"], new["score"] - old["score"],
                                       old["grade"], new["grade"]])))
    # The standard CSV writer independently supplies quoting and CRLF handling.
    stream = io.StringIO(newline="")
    stream.write(",".join(COLUMNS) + "\r\n")
    writer = csv.writer(stream, quoting=csv.QUOTE_NONNUMERIC, lineterminator="\r\n")
    writer.writerows([[row[key] for key in COLUMNS] for row in pairs])
    require(csv_raw == stream.getvalue().encode("utf-8"),
            "Saved CSV does not exactly represent all produced pairs")
    parsed = list(csv.reader(io.StringIO(csv_raw.decode("utf-8"), newline="")))
    require(parsed == [COLUMNS] + [[str(row[key]) for key in COLUMNS] for row in pairs],
            "Parsed CSV does not preserve every produced value")
    comparison = decode(json_raw.decode("utf-8"))
    require(type(comparison) is dict and set(comparison) ==
            {"format", "direction", "identity_fields", "summary", "comparisons"},
            "JSON comparison shape")
    require(comparison["format"] == "uma-sim.paired-batches.v1" and
            comparison["direction"] == "input_minus_baseline" and
            comparison["identity_fields"] == ["scenario", "trainee", "seed"],
            "JSON comparison meaning")
    require(comparison["comparisons"] == pairs, "Complete JSON pairs differ")
    deltas = [row["score_delta"] for row in pairs]
    summary = {"count": len(deltas), "wins": sum(d > 0 for d in deltas),
               "ties": sum(d == 0 for d in deltas), "losses": sum(d < 0 for d in deltas),
               "mean_score_delta": statistics.mean(deltas),
               "median_score_delta": statistics.median(deltas),
               "min_score_delta": min(deltas), "max_score_delta": max(deltas),
               "population_stddev_score_delta": statistics.pstdev(deltas)}
    actual = comparison["summary"]
    require(type(actual) is dict and set(actual) == set(summary), "Summary fields differ")
    for key, expected in summary.items():
        value = actual[key]
        require(type(value) in (int, float) and math.isfinite(value), "Invalid " + key)
        # Only the square-root result gets tolerance for independently evaluated
        # floating arithmetic; every other reported statistic is exact here.
        matches = math.isclose(value, expected, rel_tol=1e-12, abs_tol=1e-12) \
            if key == "population_stddev_score_delta" else value == expected
        require(matches, "Summary differs: " + key)
    return {"comparisons": pairs, "summary": summary,
            "recorded_shop_gains": {
                name: [{"seed": key[2], "gain": row["score"] - row["score_before_shop"]}
                       for key, row in sorted(values.items())]
                for name, values in [("baseline", baseline), ("input", input_records)]}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["baseline", "input", "csv", "json", "output"]:
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--trainee", required=True)
    parser.add_argument("--seed", action="append", type=int, required=True)
    args = parser.parse_args()
    require(len(set(args.seed)) == len(args.seed), "Repeated requested seed")
    expected = {(args.scenario, args.trainee, seed) for seed in args.seed}
    paths = {name: getattr(args, name) for name in ["baseline", "input", "csv", "json"]}
    raw = {name: path.read_bytes() for name, path in paths.items()}
    before = {name: (len(raw[name]), hashlib.sha256(raw[name]).hexdigest(),
                     path.stat().st_mtime_ns) for name, path in paths.items()}
    result = review(records(raw["baseline"], expected, "baseline"),
                    records(raw["input"], expected, "input"), raw["csv"], raw["json"])
    for name, path in paths.items():
        data = path.read_bytes()
        require((len(data), hashlib.sha256(data).hexdigest(), path.stat().st_mtime_ns)
                == before[name], "Input changed during review: " + name)
    result.update(success=True, scope="offline reconciliation of authored simulation output; no policy superiority or production-data claim",
                  requested={"scenario": args.scenario, "trainee": args.trainee,
                             "seeds": sorted(args.seed)},
                  inputs={name: {"bytes": row[0], "sha256": row[1], "mtime_ns": row[2]}
                          for name, row in before.items()}, input_bytes_and_mtimes_preserved=True)
    with args.output.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(result, output, indent=2, ensure_ascii=False, allow_nan=False)
        output.write("\n")
    print(json.dumps({"success": True, "pairs": len(result["comparisons"]),
                      "output": str(args.output)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
