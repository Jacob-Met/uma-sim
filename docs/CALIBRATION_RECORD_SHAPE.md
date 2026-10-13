# Calibration telemetry record shape

`scripts/calibrate_sim.py` reads line-delimited telemetry records. Each record
must be a JSON object. Blank lines, malformed JSON and decoded nonobjects
(arrays, strings, numbers, booleans and null) are skipped. An array containing
training objects is not unpacked into records.

This narrow admission rule prevents an unrelated valid JSON value such as
`[1]` from aborting calibration of the surrounding training records. It changes
only `parse_telemetry_line`; it does not validate the nested telemetry schema
or change predictions, arithmetic, sample selection within dictionaries,
thresholds, or report formatting.

## Use a selected saved file

From the source root, using Python 3.10 or later:

```sh
python -B scripts/calibrate_sim.py --telemetry "path/to/saved telemetry.jsonl"
```

Implicit discovery remains available:

```sh
python -B scripts/calibrate_sim.py --runs-dir "path/to/runs"
```

The script loads `research/training_gain_tables.json` relative to its source
location. Keep that file with the script for the qualified behavior.
No third-party Python package is needed. The CLI prints its JSON result to
stdout; it does not manage an output file or make shell redirection transactional.

## Admission and exit behavior

| Input or result | Behavior |
| --- | --- |
| Top-level JSON object | Pass unchanged to the existing record loader; nested data is not normalized. |
| Empty object | The parser accepts it; the existing loader skips it because it is empty. |
| Blank line or malformed JSON | Skip, as before. |
| Top-level array, string, number, boolean or null | Skip; never unpack or coerce it. |
| Explicitly selected file with no usable training samples | Print count 0, median_error null, pct_within_2 0.0, pass false; warn on stderr; exit 0, as before. |
| Implicit scan with no loaded records | Print the existing no-records diagnostic on stderr; no JSON; exit 1. |
| Nonempty calibration that meets its existing thresholds | JSON report; exit 0. |
| Nonempty calibration that fails its existing thresholds | JSON report with pass false; exit 2. |
| Missing, directory, unreadable or undecodable explicit selection | Keep the accepted selection repair: targeted read diagnostic; no fallback to unrelated runs; no JSON; exit 1. |

Exit 0 alone does not establish a nonempty or passing calibration. Inspect
`count` and `pass`. A scan can load nontraining dictionaries and still reach
the existing empty-calibration report; that is different from loading no records.

This is top-level admission only. A dictionary with an unsupported nested value
can still fail in the existing extractor. The repair does not silently discard
such errors, impose a new nested schema, reject duplicate object keys, or change
Python's existing JSON numeric decoding. File-size, line-size and decoding
policies are unchanged. Use trusted local telemetry consistent with the
existing command's expectations.

## Maintained regression command

Run both modes from the source root:

```sh
python -B scripts/tests/test_calibration_record_shape.py
python -B -O scripts/tests/test_calibration_record_shape.py
```

The eight tests invoke actual isolated CLI children with fictional rows, an
unchanged copy of the research table, and an unrelated working directory.
Optimized test mode also launches optimized children. They cover arrays,
all scalar classes, existing skips and output parity, a genuine failing gain
comparison, the distinct selected/implicit empty paths, multi-file discovery,
and unchanged nested-dictionary failure. Each invocation checks source,
table and input bytes plus modification timestamps.

The earlier selection regression remains separately maintained:

```sh
python -B scripts/tests/test_calibration_input.py
```

That earlier suite's subprocess mode is its own existing contract; the new
suite does not modify it. Neither course runs the simulator, trains weights,
reads live telemetry, updates a corpus, or demonstrates scientific calibration
accuracy.

## Source lineage and integration status

This repair builds on [issue 152's accepted selection source](https://github.com/Jacob-Met/uma-sim/issues/152)
at `b3cc555b9d15f4b5f5192847c5466d0d14b7bc95`, branch
`fix/calibration-explicit-input-6e0637a29ce6`. Its parent main was
`cd852b3be106855524e789c42670d38f419d3bd5`.
The accepted script's Git blob is
`acb4a075e2dfea5b8bd97e171ede2209abe65013`; its SHA-256 is
`d51d8d1bca70d414afd0527ce322ae7c26d9a5e817c63def92c49c4116c2aacd`.
The unchanged table's SHA-256 is
`6dc5973bfe504569a976791830fa2a41cc962aa7711c5f327b7e72f14fdd9d60`.

[Issue 164](https://github.com/Jacob-Met/uma-sim/issues/164) owns only this parser
span plus this guide and the new regression test. Every other byte of the
accepted script and all other source-tree files are preserved. The issue 152
explicit-file repair, tests, evidence and owner attribution remain intact.

The original witness on both main and accepted issue 152 source used two
fictional rows. Healthy input and malformed JSON between those rows returned
the same passing two-record report; an interleaved `[1]` instead aborted with
`AttributeError` and no report. Original failure evidence is retained separately
from the source changes.

This remains reviewed source custody until an authorized integration route is
available. The repository's existing workflow hold prohibits a PR or main
update under the no-Actions/Pages requirement. An additive source branch, if
published by the receiving owner, is not a claim of main integration or CI
execution. Workflow files and the issue 152 owner's branch remain unchanged.
