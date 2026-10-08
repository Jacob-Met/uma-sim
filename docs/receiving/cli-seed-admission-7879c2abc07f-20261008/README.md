# CLI seed admission — native qualification

Issue [#87](https://github.com/Jacob-Met/uma-sim/issues/87), production PR [#90](https://github.com/Jacob-Met/uma-sim/pull/90). This packet preserves the actual original failure and the unchanged-test candidate success. It adds receiving evidence only; its parent is the exact candidate source.

## Player outcome and repair

A malformed explicit seed could replace the current saved career with a different seed, or truncate an authored batch export while returning success. The repair changes only the existing `--seed=` branch in `parse_flags`: parse as `i64`, otherwise print a seed-specific error and exit 2 before command effects.

The accepting commands are `start`, `fast`, `batch`, and `export-telemetry`. Default 42, valid signed values, last-valid duplicate behavior and valid explicit-list priority are preserved. A malformed value is refused even when a later option would override it. Bare `--seed`, other flags and output/save implementations remain outside this change.

## Actual maintained runtime

| Stage | Actual source and run | Outcome |
| --- | --- | --- |
| Initial format attempt | head `e06d1ffc`; [37777034470](https://github.com/Jacob-Met/uma-sim/actions/runs/37777034470) | Rustfmt stopped at one call layout; no native regression ran |
| Original CLI | head `43d39044`, checkout `a54e31ad`; [37777355371](https://github.com/Jacob-Met/uma-sim/actions/runs/37777355371/job/113311939020) | **5 passed / 10 intended failures / 0 ignored**, 2.68s |
| Candidate | head `e12db23f`, checkout `6c9820f2`; [37777881604](https://github.com/Jacob-Met/uma-sim/actions/runs/37777881604/job/113313655859) | **15 passed / 0 failed / 0 ignored**, 2.14s; complete CI success |

The formatting correction wrapped one call and added its trailing comma. It changed no inputs, assertions or execution semantics. The final regression Git blob `465bf36edb3752021bb09f0f3fd94e9891d7aadc` is exact on the original and candidate native runs.

These are actual Cargo-built CLI processes, using owned temporary directories and command-local cwd/environment. Refusal assertions first capture status and actual post-command file bytes. No local build/install, external policy process, live account or provider request was used.

### Concrete original effects

- Malformed `batch --count=0`: exit 0, authored output **40 → 0 bytes**.
- Malformed `start`: exit 0, **2,758-byte saved seed 7 → 2,763-byte saved seed 42**.
- Malformed duplicates before/after valid seed 99: prior career replaced by seed 99.
- Malformed first start: new session directory and a 2,763-byte seed-42 career created.
- Malformed seed overridden by a valid list: output **40 → 590 bytes**, with two careers written.

The candidate's same ten refusal methods all pass: status 2, the seed-specific diagnostic, no stdout, and exact prior bytes retained or no first career created. The five positive methods preserve default, ordinary signed seeds, duplicate precedence, signed-i64 admission boundaries and ordered explicit-list priority.

## Source composition and ownership

Candidate [`e12db23f6ce61df700cbe375e6c4fdb26d541b28`](https://github.com/Jacob-Met/uma-sim/commit/e12db23f6ce61df700cbe375e6c4fdb26d541b28) has tree `06f43399a86ddcff758bc321160f9920eb73445f`. Its two-parent composition retains current main `09df4c25cbd86c12d16a121481c01d1633a5efb6` as a parent.

Every leaf in both complete, non-truncated trees was compared by path, type, mode and Git blob. The result is **678 candidate leaves**: **675 unrelated current-main leaves exact**, only the CLI/README changed, and one test added; zero deletions. All three authored bodies were read back exactly. Checkpoint PR #88's source and evidence are preserved; PR #82's output-error owner retains its separate implementation and integration. [Native coordination](https://github.com/Jacob-Met/uma-sim/pull/82#issuecomment-6059790102) identifies the disjoint scopes.

The independent receiver reproduced the full parser delta, reviewed all 15 methods, and independently checked original raw results and current complete-tree preservation. [Independent final acceptance](https://github.com/Jacob-Met/uma-sim/pull/90#issuecomment-6060058064) also confirms the actual candidate checkout and passing hosted logs. Merge-time gates remain separate. This packet seals candidate qualification, not an additional runtime or a deployment.

## Contents and custody

- [QUALIFICATION.json](QUALIFICATION.json): exact commits, trees, source SHA-256/Git identities, runs, outcomes, complete-tree result and limits.
- [native-results.log](native-results.log): readable actual test sections with timestamps; only ANSI color sequences removed.
- [hosted-logs.json](hosted-logs.json): all three complete tool-decoded UTF-8 job logs, each preserved losslessly as gzip plus base64, with decoded/compressed sizes and hashes.

To recover one complete log after downloading `hosted-logs.json`:

```python
import base64, gzip, json
from pathlib import Path

packet = json.loads(Path("hosted-logs.json").read_text())
entry = next(log for log in packet["logs"] if log["name"] == "original-runtime")
raw = gzip.decompress(base64.b64decode(entry["gzip_base64"]))
assert len(raw) == entry["raw_bytes"]
print(raw.decode("utf-8"), end="")
```

The gzip stream may contain concatenated members; standard `gzip.decompress` reconstructs the complete log. Compression and hashing used existing Python in memory only. No source checkout, cache or runtime artifact was written on the storage-constrained local machine.

Native execution here is Ubuntu/Linux. Boundary controls qualify extreme-seed admission with zero careers; they do not claim a complete extreme-seed simulation. Ordinary merge still requires current-base ancestry, fresh review/PR checks, exact current CI and expected-head protection.
