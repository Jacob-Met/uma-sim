# Choose the calibration telemetry explicitly

The training-gain calibration command keeps an explicit file selection:

```sh
python scripts/calibrate_sim.py --telemetry runs/sim-telemetry/sim-42-ura.jsonl
```

If that file is missing, is a directory, cannot be opened, or cannot be decoded
as UTF-8, the command prints `Cannot read telemetry <path>: <reason>` to stderr,
exits **1**, and emits no calibration JSON. It does not scan `runs/` or a supplied
`--runs-dir` in place of the selected file. Check the path or export the intended
telemetry before trying again.

To request discovery deliberately, omit `--telemetry`:

```sh
python scripts/calibrate_sim.py --runs-dir runs
```

The existing discovery paths, duplicate-file handling and `files_scanned` field
are unchanged. A readable explicitly selected file keeps priority over a runs
directory. `--stub` retains its existing precedence over both selectors and runs
the built-in synthetic self-test.

The correction changes source selection and read-error reporting. Calibration
formulas, record extraction and result thresholds are unchanged. A readable
file with no accepted training observations retains the existing zero-count
result, warning and exit **0**; that result contains `"pass": false`. A completed
nonempty calibration that misses the existing threshold still exits **2**.
Malformed individual JSON lines retain the existing parser behavior.

The script reads inputs and writes its report to stdout; it does not manage an
output file. Shell redirection can create or truncate a destination before
Python starts. Receive the exit status and output before replacing a retained
calibration artifact. This repair does not make shell redirection transactional.

## Receiving

Run the focused stdlib command-line tests from the repository root:

```sh
python -I -B scripts/tests/test_calibration_input.py -v
```

Each case copies the exact script and training table into a fresh temporary
layout, executes the actual entry point, and verifies that all source, input
and saved sentinel files remain unchanged. The tests contrast a selected
failing calibration with unrelated passing logs, and cover explicit/default
discovery, read failures, empty files and stub precedence.

The [source-bound receiving evidence](receiving/calibration-input-6e0637a29ce6/README.md)
retains original failures and candidate results. These are local synthetic
Python calibration checks; no career simulation, physical-device capture or
external data source is part of this qualification.
