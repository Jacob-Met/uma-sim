#!/usr/bin/env python3
"""Cross-entropy method fit of LinearValueFunction weights against simulated U.

Writes data (not code) to scoring-shared resources and reports A/B mean U + CI.

Usage (from repo root):
  python scripts/fit_value_function.py --generations 4 --population 8 --seeds 40 --elite 2
  python scripts/fit_value_function.py --ab-only --seeds 200
"""

from __future__ import annotations

import argparse
import json
import math
import os
import random
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
UMA_SIM = ROOT / "uma-sim-core"
WEIGHTS_OUT = (
    ROOT
    / "uma-android-automation"
    / "android"
    / "scoring-shared"
    / "src"
    / "commonMain"
    / "resources"
    / "value_function_weights.json"
)
POLICY_BAT = (
    ROOT
    / "uma-android-automation"
    / "android"
    / "policy-server"
    / "build"
    / "install"
    / "policy-server"
    / "bin"
    / "policy-server.bat"
)
DIM = 18
CARGO = os.environ.get("CARGO", "cargo")
UMA_SIM_BIN = ROOT / "target" / "release" / "uma-sim.exe"


def mean_ci(xs: list[float]) -> tuple[float, float, float]:
    n = len(xs)
    if n == 0:
        return 0.0, 0.0, 0.0
    m = sum(xs) / n
    if n < 2:
        return m, m, m
    var = sum((x - m) ** 2 for x in xs) / (n - 1)
    se = math.sqrt(var / n)
    return m, m - 1.96 * se, m + 1.96 * se


def run_batch(seed_start: int, count: int, weights_path: Path | None, out_path: Path, policy: str) -> list[float]:
    env = os.environ.copy()
    cargo_home = Path.home() / ".cargo" / "bin"
    if cargo_home.is_dir():
        env["Path"] = str(cargo_home) + os.pathsep + env.get("Path", "")
    java_home = env.get("JAVA_HOME")
    if java_home and Path(java_home).is_dir():
        env["JAVA_HOME"] = java_home
        env["Path"] = str(Path(java_home) / "bin") + os.pathsep + env["Path"]
    if POLICY_BAT.is_file():
        env["UMA_POLICY_CMD"] = str(POLICY_BAT)
    if weights_path is not None:
        env["UMA_VALUE_WEIGHTS"] = str(weights_path)
    elif "UMA_VALUE_WEIGHTS" in env:
        del env["UMA_VALUE_WEIGHTS"]
    if UMA_SIM_BIN.is_file():
        cmd = [
            str(UMA_SIM_BIN),
            "batch",
            f"--count={count}",
            f"--seed={seed_start}",
            "--speed=100",
            f"--policy={policy}",
            f"--output={out_path.as_posix()}",
        ]
        cwd = str(ROOT)
    else:
        cmd = [
            CARGO,
            "run",
            "--release",
            "--quiet",
            "--manifest-path",
            str(UMA_SIM / "Cargo.toml"),
            "--bin",
            "uma-sim",
            "--",
            "batch",
            f"--count={count}",
            f"--seed={seed_start}",
            "--speed=100",
            f"--policy={policy}",
            f"--output={out_path.as_posix()}",
        ]
        cwd = str(UMA_SIM)
    subprocess.run(cmd, check=True, env=env, cwd=cwd)
    us: list[float] = []
    for line in out_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        us.append(float(json.loads(line)["u"]))
    return us


def write_weights(path: Path, bias: float, weights: list[float]) -> None:
    path.write_text(
        json.dumps(
            {
                "bias": bias,
                "weights": weights,
                "feature_order": [
                    "dist600_speed",
                    "dist600_stamina",
                    "dist600_power",
                    "dist600_guts",
                    "dist600_wit",
                    "dist1100_speed",
                    "dist1100_stamina",
                    "dist1100_power",
                    "dist1100_guts",
                    "dist1100_wit",
                    "facility_speed",
                    "facility_stamina",
                    "facility_power",
                    "facility_guts",
                    "facility_wit",
                    "energy",
                    "turns_left",
                    "mean_bond",
                ],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def cem(generations: int, population: int, seeds: int, elite: int, policy: str) -> list[float]:
    mu = [0.0] * DIM
    # Prefer closing cliffs (negative dist weights) and raising underleveled facilities (positive).
    sigma = [0.08] * 10 + [0.15] * 5 + [0.02, 0.01, 0.02]
    best_w = list(mu)
    best_mean = float("-inf")
    with tempfile.TemporaryDirectory() as td:
        td_path = Path(td)
        for g in range(generations):
            scored: list[tuple[float, list[float]]] = []
            for p in range(population):
                w = [mu[i] + random.gauss(0.0, sigma[i]) for i in range(DIM)]
                # Box-Muller-ish cheap noise already above; clamp facility weights helpful.
                wpath = td_path / f"w_g{g}_p{p}.json"
                write_weights(wpath, 0.0, w)
                out = td_path / f"batch_g{g}_p{p}.jsonl"
                us = run_batch(1, seeds, wpath, out, policy)
                m = sum(us) / max(1, len(us))
                scored.append((m, w))
                print(f"gen={g} pop={p} meanU={m:.4f}")
            scored.sort(key=lambda t: t[0], reverse=True)
            elites = scored[:elite]
            if elites[0][0] > best_mean:
                best_mean = elites[0][0]
                best_w = elites[0][1]
            for i in range(DIM):
                vals = [e[1][i] for e in elites]
                mu[i] = sum(vals) / len(vals)
                var = sum((v - mu[i]) ** 2 for v in vals) / max(1, len(vals) - 1)
                sigma[i] = max(0.01, math.sqrt(var) * 1.1)
            print(f"gen={g} elite_mean={elites[0][0]:.4f} best={best_mean:.4f}")
    return best_w


def ab(seeds: int, weights: list[float], policy: str) -> None:
    with tempfile.TemporaryDirectory() as td:
        td_path = Path(td)
        zero = td_path / "zero.json"
        fitted = td_path / "fitted.json"
        write_weights(zero, 0.0, [0.0] * DIM)
        write_weights(fitted, 0.0, weights)
        base = run_batch(1000, seeds, zero, td_path / "base.jsonl", policy)
        treat = run_batch(1000, seeds, fitted, td_path / "treat.jsonl", policy)
        # Matched seeds: pair by index
        diffs = [t - b for b, t in zip(base, treat)]
        bm, blo, bhi = mean_ci(base)
        tm, tlo, thi = mean_ci(treat)
        dm, dlo, dhi = mean_ci(diffs)
        print(f"A/B matched seeds={seeds} policy={policy}")
        print(f"  baseline mean U={bm:.4f} 95%CI=[{blo:.4f},{bhi:.4f}]")
        print(f"  fitted   mean U={tm:.4f} 95%CI=[{tlo:.4f},{thi:.4f}]")
        print(f"  delta    mean U={dm:.4f} 95%CI=[{dlo:.4f},{dhi:.4f}]")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--generations", type=int, default=3)
    ap.add_argument("--population", type=int, default=6)
    ap.add_argument("--seeds", type=int, default=30)
    ap.add_argument("--elite", type=int, default=2)
    ap.add_argument("--policy", default="external")
    ap.add_argument("--ab-only", action="store_true")
    ap.add_argument("--ab-seeds", type=int, default=200)
    args = ap.parse_args()

    if args.ab_only:
        data = json.loads(WEIGHTS_OUT.read_text(encoding="utf-8"))
        ab(args.ab_seeds, list(data["weights"]), args.policy)
        return

    best = cem(args.generations, args.population, args.seeds, args.elite, args.policy)
    write_weights(WEIGHTS_OUT, 0.0, best)
    print(f"Wrote {WEIGHTS_OUT}")
    ab(args.ab_seeds, best, args.policy)


if __name__ == "__main__":
    main()
