"""Linux-only checked-output receiving of the maintained module and adapter.

This isolated harness uses a small main wrapper; it is not a full Cargo build.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
CANDIDATE = ROOT.parent / "candidate"
RUSTC = Path("/workspace/scratch/196c5c518e98/product_increments/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu/bin/rustc")
DEPS = Path("/workspace/scratch/196c5c518e98/product_increments/tsuka-portability-target/release/deps")
EXE = ROOT / "paired-adapter"


def snapshot(root):
    return {str(p.relative_to(root)): {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
                                     "mtime_ns": p.stat().st_mtime_ns}
            for p in sorted(root.rglob("*")) if p.is_file()}


def main():
    source_before = snapshot(CANDIDATE)
    command = [str(RUSTC), "--edition=2021", str(ROOT / "main.rs"),
               "--extern", f"serde={DEPS / 'libserde-d52c8a740f15dbd7.rlib'}",
               "--extern", f"serde_json={DEPS / 'libserde_json-f7bb92b971bd72f0.rlib'}",
               "-L", f"dependency={DEPS}", "-C", "debuginfo=0", "-C", "strip=symbols",
               "-C", "codegen-units=1", "-o", str(EXE)]
    env = dict(os.environ, TMPDIR=str(ROOT))
    build = subprocess.run(command, env=env, capture_output=True)
    (ROOT / "build.stdout").write_bytes(build.stdout)
    (ROOT / "build.stderr").write_bytes(build.stderr)
    receipt = {"scope": "standalone maintained batch_pairs and private CLI adapter; not full Cargo",
               "rustc": subprocess.check_output([str(RUSTC), "--version"], text=True).strip(),
               "command": command, "build_exit": build.returncode, "runs": [], "checks": []}
    try:
        if build.returncode:
            raise RuntimeError("isolated adapter compilation failed")
        work = ROOT / "work"
        work.mkdir()
        (work / ".uma-sim").mkdir()
        (work / ".uma-sim/session.json").write_bytes(b"unread native session sentinel\n")
        for name, score in [("input", 2147483647), ("baseline", -2147483648)]:
            data = {"scenario": "s", "trainee": "t", "seed": -9223372036854775808,
                    "score": score, "grade": "F"}
            (work / (name + ".jsonl")).write_text(json.dumps(data) + "\n")
        before = snapshot(work)
        env.update(UMA_REPO_ROOT=str(work / "absent-root"), UMA_RACE_MODEL="invalid-unused-setting")
        base = [str(EXE), "compare-batches", "--input=input.jsonl", "--baseline=baseline.jsonl"]
        positive = subprocess.run(base + ["--format=csv"], cwd=work, env=env, capture_output=True)
        expected = (b"scenario,trainee,seed,baseline_score,input_score,score_delta,baseline_grade,input_grade\r\n"
                    b'"s","t",-9223372036854775808,-2147483648,2147483647,4294967295,"F","F"\r\n')
        receipt["runs"].append({"case": "csv writable control", "exit": positive.returncode,
                                 "stdout_hex": positive.stdout.hex(), "stderr_hex": positive.stderr.hex()})
        receipt["checks"].append({"name": "csv exact writable control", "pass":
                                  positive.returncode == 0 and positive.stdout == expected and not positive.stderr})
        for fmt in ["text", "json", "csv"]:
            with (work / "baseline.jsonl").open("rb") as sink:
                result = subprocess.run(base + ["--format=" + fmt], cwd=work, env=env,
                                        stdout=sink, stderr=subprocess.PIPE)
            receipt["runs"].append({"case": fmt + " read-only stdout descriptor", "exit": result.returncode,
                                     "stderr_hex": result.stderr.hex()})
            receipt["checks"].append({"name": fmt + " checked write failure", "pass":
                                      result.returncode == 1 and b"could not deliver comparison output" in result.stderr
                                      and b"panicked" not in result.stderr})
        receipt["checks"].extend([
            {"name": "input/session bytes and mtimes preserved", "pass": snapshot(work) == before},
            {"name": "candidate bytes and mtimes preserved", "pass": snapshot(CANDIDATE) == source_before},
            {"name": "no simulator root created", "pass": not (work / "absent-root").exists()},
        ])
    finally:
        receipt["success"] = receipt["build_exit"] == 0 and len(receipt["checks"]) == 7 and all(c["pass"] for c in receipt["checks"])
        receipt["candidate_snapshot"] = source_before
        if EXE.exists():
            receipt["executable_bytes"] = EXE.stat().st_size
            receipt["executable_sha256"] = hashlib.sha256(EXE.read_bytes()).hexdigest()
            EXE.unlink()
        receipt["free_after_cleanup"] = shutil.disk_usage(ROOT).free
        (ROOT / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({k: receipt[k] for k in ["scope", "build_exit", "checks", "success", "free_after_cleanup"]}, indent=2))
    return 0 if receipt["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
