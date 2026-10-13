"""Receive the exact UMA CSV native evidence without running the executable.

Standard library only. Both input hashes are fixed to the reviewed publication.
The native data retains full streams and before/after custody; no normalization
or timing inference is used. This is evidence replay, not a new native run.
"""
import argparse
import base64
import csv
import gzip
import hashlib
import io
import json
from pathlib import Path
import re

PACKET_SHA = "c99bb18ef451251ac22e9744c126a2590a7e65dc6665f0f296f4e6a643c7cdb2"
BINDINGS_SHA = "4a6b2726a604b34f4870f3edde44ab1a1b77e65df31d435fb6693f7394575515"
SOURCE = "392499683b3b52cf032222d4ed9ea6274fb10131"
EXE_SHA = "9a57e48af9c23bb42a25593c74d6508ac428e75948bb18f0eeb875767cd86255"
COLS = ["scenario", "trainee", "seed", "baseline_score", "input_score", "score_delta", "baseline_grade", "input_grade"]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, label):
    if not condition:
        raise ValueError(label)


def review(packet_path, bindings_path):
    packet = packet_path.read_bytes()
    bindings_bytes = bindings_path.read_bytes()
    require(len(packet) == 291888 and sha(packet) == PACKET_SHA, "Native packet pin")
    require(sha(bindings_bytes) == BINDINGS_SHA, "Independent remote source bindings pin")
    files = json.loads(gzip.decompress(packet))["files"]
    require(len(files) == 171, "Native payload count")
    payloads = {}
    for name, record in files.items():
        data = base64.b64decode(record["base64"], validate=True)
        require(len(data) == record["bytes"] and sha(data) == record["sha256"], name)
        payloads[name] = data
    read = lambda name: json.loads(payloads[name])
    original = read("original/receipt.json")
    candidate = read("candidate/receipt.json")
    build = read("candidate-build-receipt.json")
    portable = read("portable-use-receipt.json")
    for receipt, checks, runs in [(original, 73, 23), (candidate, 176, 31), (build, 12, 7), (portable, 10, 2)]:
        require(receipt["success"] and not receipt["errors"], "Native result")
        require(len(receipt["checks"]) == checks and len(receipt["runs"]) == runs, "Distinct course counts")
        require(all(c["pass"] and c["actual"] == c["expected"] for c in receipt["checks"]), "Native predicates")
    for name, expected in [("receive_paired_csv.py", "dee7eccae391a9fb363f055fe1875ffbb363505e38b3f2bc99bd57d8d7ab9f4d"),
                           ("build_receive_candidate.py", "56e11bac4f03a57112d80562230697331e88c1ac3b0308d46de922f7c1e49a94")]:
        require(sha(payloads[name]) == expected, "Reviewed runner " + name)
    streams = 0
    for label, receipt, template in [("original", original, "original/{name}/{stream}.bin"),
                                     ("candidate", candidate, "candidate/{name}/{stream}.bin"),
                                     ("build", build, "build-evidence/{name}/{stream}.txt"),
                                     ("portable", portable, "{name}-{stream}.bin")]:
        for run in receipt["runs"]:
            for stream in ["stdout", "stderr"]:
                data = payloads[template.format(name=run["name"], stream=stream)]
                require(len(data) == run[stream + "_bytes"] and sha(data) == run[stream + "_sha256"], label + " output binding")
                streams += 1
    require(original["before"] == original["after"] and candidate["before"] == candidate["after"], "Receiver custody")
    require(original["after"]["work"] == candidate["after"]["work"] == portable["before"]["work"], "Cross-course fixture custody")
    require(original["after"]["source"] == candidate["after"]["source"] == build["original_before"] == build["original_after"], "Original source custody")
    require(build["source_before"] == build["source_after"] and len(build["source_after"]) == 228, "Candidate source custody")
    require(portable["before"] == portable["after"], "Portable-use custody")
    require(original["no_simulator_root_created"] and candidate["no_simulator_root_created"], "No simulation root")
    require(build["executable"]["sha256"] == EXE_SHA and build["executable"]["bytes"] == 8274114, "Release identity")
    require(candidate["after"]["exe"]["sha256"] == portable["after"]["exe"]["sha256"] == EXE_SHA, "Copied release custody")
    old_runs = {r["name"]: r for r in original["runs"]}
    paired_runs = 0
    for name, old in old_runs.items():
        if name.endswith("-csv"):
            require(old["exit"] == 1 and not payloads["original/" + name + "/stdout.bin"], "Original CSV refusal")
            continue
        new = next(r for r in candidate["runs"] if r["name"] == name)
        require(new["exit"] == old["exit"], "Existing command exit parity")
        for stream in ["stdout", "stderr"]:
            require(payloads[f"original/{name}/{stream}.bin"] == payloads[f"candidate/{name}/{stream}.bin"], "Existing output byte parity")
        paired_runs += 1
    expected = payloads["original/expected.csv"]
    require(payloads["candidate/expected.csv"] == expected, "Frozen CSV oracle unchanged")
    require(payloads["candidate-portable/comparison.csv"] == expected, "Actual CMD redirected file bytes")
    expected_rows = original["expected_pairs"]
    require(len(expected_rows) == 14 and candidate["expected_pairs"] == expected_rows, "All expected pairs")
    for suffix in ["", "-reordered"]:
        data = payloads[f"candidate/valid{suffix}-csv/stdout.bin"]
        require(data == expected, "CSV complete exact bytes")
        table = list(csv.reader(io.StringIO(data.decode("utf-8"), newline=""), strict=True))
        require(table == [COLS] + [[str(row[col]) for col in COLS] for row in expected_rows], "CSV literal and integer roundtrip")
        require(read(f"candidate/valid{suffix}-json/stdout.bin")["comparisons"] == expected_rows, "Complete JSON pairs")
    failures = ["duplicate-input", "duplicate-baseline", "late-invalid", "unmatched-input", "unmatched-baseline", "empty", "seed-overflow", "score-overflow"]
    for label in failures:
        run = next(r for r in candidate["runs"] if r["name"] == label + "-csv")
        require(run["exit"] == 1 and not payloads[f"candidate/{label}-csv/stdout.bin"], "CSV whole-input refusal")
        require(payloads[f"candidate/{label}-csv/stderr.bin"] == payloads[f"original/{label}-json/stderr.bin"], "CSV refusal class parity")
    counts = re.findall(rb"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out", payloads["build-evidence/maintained-pairing-tests/stdout.txt"])
    require(counts == [b"7", b"4", b"3"], "Actual maintained test counts")
    require(read("portable-analyze-stdout.bin")["summary"]["shop_gain_mean"] == 4294967295.0, "Retained arithmetic capability")
    require(read("portable-analyze-stdout.bin")["summary"]["n"] == 1, "Retained arithmetic fixture count")
    require(portable["source_commit"] == SOURCE and len(expected) == 814, "Portable source and CSV size")
    bindings = json.loads(bindings_bytes)
    native_pins = read("independent-source-receipt.json")
    require(bindings["source_commit"] == native_pins["source_commit"] == SOURCE, "Published source")
    require(bindings["source_tree"] == native_pins["source_tree"] == "d5aed1e3a90b4e9bf9646a5ac81f186102f4f56a", "Published tree")
    actual_pins = {r["path"]: r for r in native_pins["full_source_files"]}
    require(len(actual_pins) == len(bindings["files"]) == 228, "Source projection size")
    for row in bindings["files"]:
        actual = actual_pins[row["path"]]
        require((actual["git_blob_sha"], actual["mode"], actual["bytes"], actual["sha256"]) == (row["git_blob"], row["mode"], row["bytes"], row["sha256"]), "Independent source binding")
        snap = build["source_after"][row["path"]]
        require(snap["bytes"] == row["bytes"] and snap["sha256"] == row["sha256"], "Built source bytes")
    return {"verdict": "PASS", "source_commit": SOURCE, "packet_sha256": PACKET_SHA,
            "independent_source_bindings_sha256": BINDINGS_SHA, "payloads_verified": 171,
            "raw_stream_bindings_verified": streams, "unchanged_original_command_results": paired_runs,
            "original": {"invocations": 23, "predicates": 73}, "candidate": {"invocations": 31, "predicates": 176},
            "maintained_test_methods": [7, 4, 3], "build_predicates": 12,
            "portable_use": {"invocations": 2, "predicates": 10, "csv_bytes": 814, "csv_sha256": sha(expected), "shop_gain_mean": 4294967295.0},
            "source_files": 228, "source_inputs_session_executable_custody": "PASS including cross-course equality",
            "release_executable": {"bytes": 8274114, "sha256": EXE_SHA},
            "rustc_verbose": payloads["build-evidence/rustc-version/stdout.txt"].decode(),
            "scope": "Offline verification of recorded native observations; no new executable invocation, install, simulation or benchmark."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path)
    parser.add_argument("--source-bindings", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = review(args.packet, args.source_bindings)
    with args.output.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(result, output, indent=2)
        output.write("\n")
    print(json.dumps(result, indent=2))
