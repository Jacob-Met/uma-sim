# Compare the same careers across saved batches

`compare-batches` reports how each recorded career's score changed between two
batch JSONL files. A career is identified by the exact **scenario, trainee, and
signed seed** recorded in the file. Input order does not matter. Every career
must have exactly one partner, so an omitted, extra, or duplicate career stops
the comparison before a report is printed.

Run from the repository root:

```sh
cargo run --locked -p uma-sim-core --bin uma-sim -- compare-batches \
  --input=uma-sim-core/examples/paired_batches/input.jsonl \
  --baseline=uma-sim-core/examples/paired_batches/baseline.jsonl
```

The included files contain three illustrative, authored outcomes. The result is
one win, one tie, one loss, a mean score change of `-3.333333`, and a median
change of `0.000000`. Positive deltas mean the input score is higher than its
baseline partner. The text report lists up to ten careers with the largest
absolute changes; tied magnitudes are ordered by scenario, trainee, then numeric
signed seed.

For new experiments, save both batches explicitly with the same career choices:

```sh
cargo run --locked -p uma-sim-core --bin uma-sim -- batch \
  --seeds=42,43,44 --scenario=ura --trainee='Special Week' \
  --policy=default --output=out/paired/default.jsonl
cargo run --locked -p uma-sim-core --bin uma-sim -- batch \
  --seeds=42,43,44 --scenario=ura --trainee='Special Week' \
  --policy=bot --output=out/paired/bot.jsonl
cargo run --locked -p uma-sim-core --bin uma-sim -- compare-batches \
  --input=out/paired/bot.jsonl --baseline=out/paired/default.jsonl --format=json
```

The command reads the two supplied paths relative to the current directory. It
does not start a simulation, load content packs, consult simulator settings, or
read or save a session. `--help` by itself prints the usage line. The accepted
syntax is `--input=FILE --baseline=FILE [--format=text|json|csv]`; each option may be
provided once. Text is the default. Missing, empty, repeated, and unknown options
are errors.

## Reading the result

The JSON report includes every matched pair, ordered by scenario, trainee, then
numeric signed seed:

```json
{
  "format": "uma-sim.paired-batches.v1",
  "direction": "input_minus_baseline",
  "identity_fields": ["scenario", "trainee", "seed"],
  "summary": {
    "count": 1,
    "wins": 1,
    "ties": 0,
    "losses": 0,
    "mean_score_delta": 7.0,
    "median_score_delta": 7.0,
    "min_score_delta": 7,
    "max_score_delta": 7,
    "population_stddev_score_delta": 0.0
  },
  "comparisons": [
    {
      "scenario": "ura",
      "trainee": "Special Week",
      "seed": 5,
      "baseline_score": 10,
      "input_score": 17,
      "score_delta": 7,
      "baseline_grade": "F",
      "input_grade": "F"
    }
  ]
}
```

This is a one-pair schema example, separate from the three-pair files above.
Wins, ties, and losses count positive, zero, and negative **score** deltas; grade
labels are retained literally and are not ranked. The median is the median of
the individual paired changes. Population standard deviation divides the sum
of squared deviations by the number of pairs. There is no sample correction,
confidence interval, or significance claim.

The text format has five header lines followed by zero to ten tab-separated
detail lines. A valid comparison always has at least one pair. Mean, median, and
population standard deviation use six decimal places. Detail rows contain
signed delta, baseline score, input score, and a compact JSON identity tuple:

```text
Paired batch comparison (input minus baseline)
Pairs: 1; wins: 1; ties: 0; losses: 0
Score delta: mean 7.000000; median 7.000000; min 7; max 7; population stddev 0.000000
Largest absolute score changes (up to 10; ties by scenario, trainee, seed):
delta	baseline	input	[scenario,trainee,seed]
+7	10	17	["ura","Special Week",5]
```

The gaps in the two final lines are literal tabs. Encoding the identity as JSON
keeps embedded tabs and newlines escaped on one output line. Text and JSON end
with a newline. JSON includes both the summary and every pair with exact integer
fields. CSV provides every pair as a table, without summary rows.

## Export all pairs as CSV

```sh
cargo run --locked -p uma-sim-core --bin uma-sim -- compare-batches \
  --input=out/paired/bot.jsonl --baseline=out/paired/default.jsonl --format=csv \
  > comparison.csv
```

The first record is this unquoted header, followed by every matched pair in
scenario, trainee, then numeric signed seed order:

```csv
scenario,trainee,seed,baseline_score,input_score,score_delta,baseline_grade,input_grade
"ura","Special Week",5,10,17,7,"F","F"
```

This one-pair illustration is the same schema example as above. The actual file
uses UTF-8 without a byte-order mark and CRLF (`\r\n`) record terminators,
including after the final record. All four string fields are always quoted;
embedded double quotes are doubled. Literal commas, tabs, whitespace, Unicode,
carriage returns and newlines remain inside the quoted value. A record can span
multiple physical lines: use a CSV parser, not line splitting.

| Columns | CSV representation |
| --- | --- |
| `scenario`, `trainee`, `baseline_grade`, `input_grade` | Quoted literal strings; no trimming or rewriting |
| `seed` | Unquoted exact signed 64-bit decimal integer |
| `baseline_score`, `input_score` | Unquoted exact signed 32-bit decimal integers |
| `score_delta` | Unquoted exact signed 64-bit input-minus-baseline difference |

The exporter performs no new comparison or aggregation. Every admitted pair is
included, even when the text report's ten-detail-row limit would omit it. There
are no empty/null substitutions or summary records.

When importing into a spreadsheet, select **text** for the seed and literal
string columns. Automatic type inference can change long seeds or interpret
formula-looking values. CSV quoting preserves field boundaries; it does not
force a spreadsheet cell type. The exporter retains the saved values literally
and does not encode them as spreadsheet expressions. A CSV parser that leaves
fields as strings can retain these values exactly.

## Input admission and failure behavior

Each file must be UTF-8 JSONL, at most **16 MiB (16,777,216 bytes)**, with at most
**100,000 nonblank records**. Blank lines are ignored. Every nonblank line must
be a JSON object with these fields:

| Field | Required value |
| --- | --- |
| `seed` | JSON integer in the signed 64-bit range |
| `scenario` | Nonempty string, at most 256 decoded UTF-8 bytes |
| `trainee` | Nonempty string, at most 256 decoded UTF-8 bytes |
| `score` | JSON integer in the signed 32-bit range |
| `grade` | Nonempty string, at most 64 decoded UTF-8 bytes |

Numeric strings and floating-point representations such as `1.0` do not satisfy
the integer fields. Duplicate required fields are invalid. Other terminal
fields, such as stats and utility, are ignored. Scenario, trainee, and grade are
literal strings: whitespace-only nonempty strings are accepted, and values are
neither trimmed nor Unicode-normalized. The string limits count decoded UTF-8
bytes, while the whole-file limit counts the input bytes as read. Subtraction is
widened before arithmetic, so both signed 32-bit score endpoints are supported.

Malformed records are not skipped. Empty files, repeated identities, unmatched
cohorts, invalid fields, read errors, and resource-limit violations return exit
status `1`, with no comparison bytes written to stdout. Options, both complete
files, and the full pairing are admitted before report output starts. Errors are
reported on stderr when that stream is writable. A failed stdout write or flush
also returns `1`; bytes already delivered to an external pipe cannot be recalled.
Success returns `0`.

Ordinary shell redirection opens its destination before starting the command.
To replace an existing report only after successful comparison, write a temporary
report and move it after a zero exit status:

```sh
cargo run --locked -p uma-sim-core --bin uma-sim -- compare-batches \
  --input=out/paired/bot.jsonl --baseline=out/paired/default.jsonl --format=json \
  > comparison.pending.json && mv comparison.pending.json comparison.json
```

Keep the original batch files and record the code revision, content, policy,
deck, and other experimental settings separately. Matching recorded identities
does not authenticate those settings or establish equal random-number
consumption between runs. This report describes saved outcomes. The existing
`analyze` command continues to provide its aggregate batch summaries.
