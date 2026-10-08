#!/usr/bin/env python3
"""Qualify exact race-results identity allowances with native Gitleaks 8.24.3.

The source directory is read-only input. Every Git history and changed-value/path
control is created under a new work directory. No scanner defaults are replaced.
This is a bounded reproduction of five finding files, not a full-repository scan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time
import tomllib


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def blob(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode()
    return hashlib.sha1(header + data).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def expected_rows(rows: list[dict[str, object]]) -> list[tuple[str, int, str]]:
    return sorted((str(row["path"]), int(row["line"]), str(row["rule"])) for row in rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--bindings", type=Path, required=True)
    parser.add_argument("--published-config", type=Path, required=True)
    parser.add_argument("--current-config", type=Path, required=True)
    parser.add_argument("--candidate-config", type=Path, required=True)
    parser.add_argument("--scanner", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    args = parser.parse_args()
    for name in ("source", "bindings", "published_config", "current_config",
                 "candidate_config", "scanner", "work"):
        setattr(args, name, getattr(args, name).resolve())

    assert not args.work.exists(), "work directory must be new"
    args.work.mkdir(mode=0o700)
    out = args.work / "evidence"
    out.mkdir()
    bound = json.loads(args.bindings.read_text())
    original = {
        path: (args.source / path).read_bytes()
        for path in bound["file_blobs"]
    }
    for path, data in original.items():
        assert blob(data) == bound["file_blobs"][path], f"source pin mismatch: {path}"

    configs = {
        "published": args.published_config.read_bytes(),
        "current": args.current_config.read_bytes(),
        "candidate": args.candidate_config.read_bytes(),
    }
    assert blob(configs["published"]) == bound["published_policy_blob"]
    assert blob(configs["current"]) == bound["current_policy_blob"]
    assert configs["candidate"].startswith(configs["current"])
    current_toml = tomllib.loads(configs["current"].decode())
    candidate_toml = tomllib.loads(configs["candidate"].decode())
    assert candidate_toml["extend"] == {"useDefault": True}
    assert set(candidate_toml) == {"extend", "rules"}
    assert len(candidate_toml["rules"]) == 1
    assert candidate_toml["rules"][0]["id"] == "generic-api-key"
    prior = current_toml["rules"][0]["allowlists"]
    allowances = candidate_toml["rules"][0]["allowlists"]
    assert allowances[:len(prior)] == prior
    assert len(allowances) == len(prior) + 3
    for allowance in allowances[len(prior):]:
        assert allowance["condition"] == "AND"
        assert allowance["regexTarget"] == "secret"
        assert all(p.startswith("^") and p.endswith("$") for p in allowance["paths"])
        assert all(r.startswith("^") and r.endswith("$") for r in allowance["regexes"])

    assert digest(args.scanner.read_bytes()) == bound["scanner_sha256"]
    version = subprocess.run(
        [str(args.scanner), "version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    assert version == bound["scanner_version"]
    receipt = {
        "contract": "Exact path AND whole identity; retain default and other rules",
        "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "repository": bound["repository"],
        "finding_source_commit": bound["finding_source_commit"],
        "current_policy_commit": bound["current_policy_commit"],
        "scanner": {"version": version, "sha256": digest(args.scanner.read_bytes())},
        "driver": {"name": Path(__file__).name, "sha256": digest(Path(__file__).read_bytes())},
        "bindings_sha256": digest(args.bindings.read_bytes()),
        "python": platform.python_version(),
        "platform": platform.platform(),
        "configurations": {
            name: {"bytes": len(data), "sha256": digest(data), "git_blob": blob(data)}
            for name, data in configs.items()
        },
        "original_files": {
            path: {"bytes": len(data), "sha256": digest(data), "git_blob": blob(data)}
            for path, data in original.items()
        },
        "current_policy_prefix_unchanged": True,
        "existing_rule_allowances_unchanged": len(prior),
        "new_rule_allowances": 3,
        "capacity_floor_bytes": 1024**3,
        "groups": [],
    }
    baseline_expected = expected_rows(bound["expected_findings"])
    env = os.environ.copy()
    env.update({
        "GIT_AUTHOR_DATE": "2000-01-01T00:00:00Z",
        "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z",
    })

    def scan(
        name: str,
        files: dict[str, bytes],
        config: Path,
        expected: list[tuple[str, int, str]],
    ) -> None:
        assert shutil.disk_usage(args.work).free > receipt["capacity_floor_bytes"]
        repository = args.work / name
        repository.mkdir()
        for relative, data in files.items():
            path = repository / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        subprocess.run(
            ["git", "init", "--quiet", "--template=", str(repository)],
            capture_output=True, text=True, check=True, env=env,
        )
        subprocess.run(
            ["git", "-C", str(repository), "add", "--", "."],
            capture_output=True, text=True, check=True, env=env,
        )
        subprocess.run(
            ["git", "-C", str(repository), "-c", "user.name=HAMON fixture",
             "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
             "commit", "--quiet", "-m", "Synthetic scanner receiving fixture"],
            capture_output=True, text=True, check=True, env=env,
        )
        commit = subprocess.run(
            ["git", "-C", str(repository), "rev-parse", "HEAD"],
            capture_output=True, text=True, check=True, env=env,
        ).stdout.strip()
        report = out / f"{name}.json"
        argv = [
            str(args.scanner), "detect", "--redact", "-v", "--exit-code=2",
            "--report-format=json", f"--report-path={report}", "--log-level=debug",
            "--log-opts=-1", f"--config={config}",
        ]
        result = subprocess.run(
            argv, cwd=repository, capture_output=True, text=True, timeout=30, env=env
        )
        log = out / f"{name}.log"
        log.write_text(result.stdout + result.stderr)
        assert report.is_file(), f"scanner did not produce report: {name}"
        observed = json.loads(report.read_text()) or []
        rows = sorted(
            (item["File"], item["StartLine"], item["RuleID"])
            for item in observed
        )
        expected_rc = 2 if expected else 0
        group = {
            "name": name,
            "fixture_commit": commit,
            "input_files": {
                path: {"bytes": len(data), "sha256": digest(data), "git_blob": blob(data)}
                for path, data in files.items()
            },
            "command": argv,
            "exit_code": result.returncode,
            "expected_exit_code": expected_rc,
            "expected": expected,
            "observed": rows,
            "report": {"name": report.name, "sha256": digest(report.read_bytes())},
            "log": {"name": log.name, "sha256": digest(log.read_bytes())},
            "pass": result.returncode == expected_rc and rows == sorted(expected),
        }
        receipt["groups"].append(group)
        write_json(out / "receipt.json", receipt)
        print(json.dumps({
            "group": name, "pass": group["pass"], "exit_code": result.returncode,
            "findings": len(rows), "expected_findings": len(expected),
        }), flush=True)

    scan("published-policy-baseline", original, args.published_config, baseline_expected)
    scan("current-policy-baseline", original, args.current_config, baseline_expected)
    scan("candidate-exact-inputs", original, args.candidate_config, [])

    identities = list(bound["identities"].values())
    replacements = {
        value: hashlib.sha256(f"wrong public provenance {index}".encode()).hexdigest()[:len(value)]
        for index, value in enumerate(identities)
    }
    assert all(value != replacements[value] for value in identities)

    def replace_values(mapping: dict[str, str]) -> dict[str, bytes]:
        changed = {}
        for path, data in original.items():
            text = data.decode()
            for before, after in mapping.items():
                text = text.replace(before, after)
            changed[path] = text.encode()
        return changed

    scan("wrong-value", replace_values(replacements), args.candidate_config, baseline_expected)
    scan(
        "wrong-path", {path + ".copy": data for path, data in original.items()},
        args.candidate_config,
        [(path + ".copy", line, rule) for path, line, rule in baseline_expected],
    )
    scan(
        "whole-value-prefix", replace_values({value: "f" + value for value in identities}),
        args.candidate_config, baseline_expected,
    )
    scan(
        "whole-value-suffix", replace_values({value: value + "f" for value in identities}),
        args.candidate_config, baseline_expected,
    )

    prefix = "docs/receiving/race-results-afe225d6c6be/"
    cross = {
        prefix + "baseline-native/baseline-witness.json":
            ("native_api_sha256", bound["identities"]["executable_digest"]),
        prefix + "source-freeze.json":
            ("nativeApiSource", bound["identities"]["source_commit"]),
        prefix + "current-044/composition.json":
            ("uma-sim-core/src/api.rs", bound["identities"]["source_digest"]),
    }
    cross_files = {
        path: (json.dumps({label: value}) + "\n").encode()
        for path, (label, value) in cross.items()
    }
    scan(
        "cross-identity-path", cross_files, args.candidate_config,
        [(path, 1, "generic-api-key") for path in cross_files],
    )

    source_value = bound["identities"]["source_digest"]
    exact_path = prefix + "baseline-native/baseline-witness.json"
    other_config = args.work / "candidate-with-test-canary.toml"
    other_config.write_bytes(
        configs["candidate"] + (
            '\n# Test-only rule; never publish as production policy.\n'
            '[[rules]]\n'
            'id = "hamon-receiving-canary"\n'
            'description = "Independent rule-scoping control"\n'
            "regex = '''" + source_value + "'''\n"
        ).encode()
    )
    scan(
        "other-rule-same-allowed-identity",
        {exact_path: (json.dumps({"evidence_identity": source_value}) + "\n").encode()},
        other_config, [(exact_path, 1, "hamon-receiving-canary")],
    )

    # This unassigned offline detector probe is generated at runtime and never used for a request.
    probe = "gh" + "p_" + hashlib.sha256(b"offline default detector control").hexdigest()[:36]
    scan(
        "default-github-rule",
        {exact_path: (json.dumps({"probe": probe}) + "\n").encode()},
        args.candidate_config, [(exact_path, 1, "github-pat")],
    )

    for path, data in original.items():
        assert (args.source / path).read_bytes() == data
    assert args.current_config.read_bytes() == configs["current"]
    assert args.published_config.read_bytes() == configs["published"]
    assert args.candidate_config.read_bytes() == configs["candidate"]
    assert digest(args.scanner.read_bytes()) == bound["scanner_sha256"]
    receipt["all_original_inputs_unchanged"] = True
    receipt["free_after_bytes"] = shutil.disk_usage(args.work).free
    assert receipt["free_after_bytes"] > receipt["capacity_floor_bytes"]
    receipt["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    receipt["pass"] = all(group["pass"] for group in receipt["groups"])
    receipt["limits"] = [
        "Five source-bound finding files in synthetic local Git histories.",
        "Does not replace exact-head full-repository hosted scanning.",
        "Test-only canary configuration is not part of the proposed policy.",
        "No source/evidence payload or inherited policy allowance changed.",
    ]
    write_json(out / "receipt.json", receipt)
    print(json.dumps({
        "status": "PASS" if receipt["pass"] else "FAIL",
        "groups": len(receipt["groups"]),
        "receipt": str(out / "receipt.json"),
        "receipt_sha256": digest((out / "receipt.json").read_bytes()),
        "candidate_sha256": digest(configs["candidate"]),
        "candidate_git_blob": blob(configs["candidate"]),
    }), flush=True)
    return 0 if receipt["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
