"""Reproduce the published comparison using actual native batch consumers."""
from pathlib import Path
import json, os, subprocess, sys, time, resource, hashlib
source = json.loads((Path(__file__).parent / "outcomes.json").read_text())
baseline, candidate = [str(Path(p).resolve()) for p in sys.argv[1:3]]
out = Path(sys.argv[3]).resolve()
out.mkdir(exist_ok=False)
env = os.environ.copy()
for key in ["UMA_POLICY_CMD", "UMA_REPO_ROOT", "UMA_RACE_MODEL"]:
    env.pop(key, None)
costs = []
for name, context in source["contexts"].items():
    for kind, binary in [("baseline", baseline), ("candidate", candidate)]:
        folder = out / name / kind
        folder.mkdir(parents=True)
        for scenario in context[kind]["records"]:
            target = folder / (scenario + ".jsonl")
            args = [binary, "batch", "--seeds=" + ",".join(map(str, context["seeds"])),
                    "--scenario=" + scenario, "--policy=bot", "--race-model=physics",
                    "--trainee=" + context["context"]["trainee"], "--output=" + str(target)]
            if context["context"]["deck"]:
                args.append("--deck=" + context["context"]["deck"])
            before = resource.getrusage(resource.RUSAGE_CHILDREN)
            start = time.perf_counter()
            run = subprocess.run(args, cwd=folder, env=env, capture_output=True, text=True)
            wall = time.perf_counter() - start
            after = resource.getrusage(resource.RUSAGE_CHILDREN)
            (folder / (scenario + ".stdout")).write_text(run.stdout)
            (folder / (scenario + ".stderr")).write_text(run.stderr)
            run.check_returncode()
            rows = [json.loads(line) for line in target.read_text().splitlines()]
            expected = context[kind]["records"][scenario]
            identity = lambda records: [(x["seed"], x["scenario"], x["trainee"]) for x in records]
            assert identity(rows) == identity(expected), (name, kind, scenario, "missing or wrong rows")
            assert rows == expected, (name, kind, scenario, "terminal outcomes changed")
            costs.append(dict(context=name, kind=kind, scenario=scenario, wall_seconds=wall,
                              cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime,
                              binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest()))
    for scenario in context["baseline"]["records"]:
        result = subprocess.run([candidate, "compare-batches",
            "--input=" + str(out/name/"candidate"/(scenario+".jsonl")),
            "--baseline=" + str(out/name/"baseline"/(scenario+".jsonl")), "--format=json"],
            env=env, capture_output=True, text=True, check=True)
        (out/name/(scenario+"-paired.json")).write_text(result.stdout)
(out/"costs.json").write_text(json.dumps(costs, indent=2)+"\n")
print("All 768 terminal records reproduced exactly; native paired comparisons complete.")
