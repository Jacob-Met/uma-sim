# Explicit calibration telemetry receiving

## Result

An explicit missing telemetry file previously fell through to the runs-directory scan. In the original witness, a misspelled requested file produced a passing one-record calibration from unrelated logs. The revised command keeps the explicit selection and reports an unreadable file with exit 1, targeted stderr and no calibration JSON.

Only the selected-file read boundary in `scripts/calibrate_sim.py` changes: six inserted lines and two removed lines. Inverting that exact span restores the original source bytes. The training table, formulas, thresholds, record extraction, implicit discovery and stub precedence remain unchanged.

- Original main: `cd852b3be106855524e789c42670d38f419d3bd5`.
- Original script SHA-256: `21a84801ca74e674ef7299b5c62ba3d3ec14fdeb5a2ece3c36c2d0ea2873618b`.
- Approved script SHA-256: `d51d8d1bca70d414afd0527ce322ae7c26d9a5e817c63def92c49c4116c2aacd`.
- Unchanged table SHA-256: `6dc5973bfe504569a976791830fa2a41cc962aa7711c5f327b7e72f14fdd9d60`.
- Exact maintained test SHA-256: `60eaed67707dc9f306929f22b4538157a64d53fae07ad5a6ce8ca7b87cc504f3`.

## Author qualification

The same final maintained test exercises the actual command through fresh copied script/table layouts: original **4/8**, candidate **8/8**. Four original failures are retained, including a false successful fallback and untargeted directory/UTF-8 errors. The original and candidate raw stdout/stderr streams are byte-preserved beside `qualification.json`.

Run from the repository root with Python 3.10 or later:

```sh
python3 -I -B scripts/tests/test_calibration_input.py -v
```

Each case checks source, input and saved-sentinel hashes after the actual process. The final test passes Ruff; the narrow product/doc diff check passes. The unchanged synthetic stub is exercised in the maintained test and independent course. No native simulator suite was run for this Python read-boundary correction.

## Independent receiving

A different worker read the unchanged original and froze `uma-review-contract.json` before inspecting the candidate. Its independently chosen selected/fallback fixtures use different gains and multiple discovery paths. Original receiving passes **11/17** checks; candidate receiving passes **17/17**, with explicit approval in `uma-independent-approval.json`.

Six original failures cover a missing selected path with/without passing fallback logs, a dangling link, a directory, invalid UTF-8 and an explicitly injected `PermissionError`. All nine successful original process outcomes remain byte-identical: a selected four-row failing calibration (median error 3, 25% within 2), changed selected passing data, a mixed-condition literal prediction of 28.4592, three-file discovery, standalone and two selector-conflict stub controls, empty-file behavior and existing parse policy. Source/table/input/sentinel bytes remain unchanged.

The receiver is retained unchanged as `review_uma_selection.py.txt`. To replay from the repository root, choose fresh nonexistent work/receipt paths:

```sh
python3 -I -B docs/receiving/calibration-input-6e0637a29ce6/review_uma_selection.py.txt \
  --root . --work /tmp/uma-calibration-receive-new \
  --receipt /tmp/uma-calibration-receive-new.json
```

The receiver writes only its new synthetic work directory and receipt. A fresh original script/table root produces the expected failing receiver exit 1 (11/17); the approved source produces exit 0 (17/17). Source selection and calibration report bytes are checked through actual isolated CLI children. The read-denial seam is labeled as injected rather than native filesystem-permission evidence.

## Delivery and scope

See [the recipient guide](../../CALIBRATION_INPUT.md). This increment is prepared for source-branch publication with `[skip actions]`. A pull request is deliberately not opened: the repository has an existing `pull_request_target` workflow that skip markers cannot safely suppress. No workflow changes, main updates, tags or dispatches are part of this increment. A separate delivery receipt records the actual published commit and source-head Actions readback.

These synthetic controls establish the chosen-file behavior and preservation of existing computations. They do not establish simulator accuracy, native calibration, physical-device capture or workload quality. The CLI emits stdout and manages no output file; shell redirection can truncate a destination before Python starts. Empty readable files and malformed individual lines retain their existing behavior.
