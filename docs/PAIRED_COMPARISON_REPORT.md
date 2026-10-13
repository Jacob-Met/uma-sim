# Read a saved paired comparison offline

`scripts/render_paired_comparison.py` turns an existing native paired-comparison JSON report into one self-contained HTML file. Open the file in a browser to inspect the complete cohort, search the recorded fields, filter wins/ties/losses, and order pairs by score difference. The command uses Python's standard library and does not run a career or launch a browser.

## Make a reader copy

First obtain a complete report using the existing [paired-batch comparator](PAIRED_BATCHES.md):

```sh
uma-sim compare-batches --input=changed.jsonl --baseline=original.jsonl --format=json --output=comparison.json
```

Keep the original batches and their simulator/configuration records. The paired report alone does not authenticate those details.

Compute the SHA256 of the saved file you intend to read:

```sh
python -c "import hashlib,pathlib; print(hashlib.sha256(pathlib.Path('comparison.json').read_bytes()).hexdigest())"
```

Then pass that exact digest and a new output filename:

```sh
python -B scripts/render_paired_comparison.py \
  --input comparison.json \
  --input-sha256 YOUR_64_HEX_DIGEST \
  --output comparison.html
```

The command returns one JSON success receipt with the input/output hashes, byte counts and pair count. Open `comparison.html` in your browser; it needs no server or internet connection. Move or share that single HTML file to share the complete reader copy, including the original JSON bytes. Treat it as containing all the source report's data.

Optional `--baseline-label "Earlier run" --input-label "Changed run"` changes the visible names. These are caller-supplied labels, not evidence that a particular policy or simulator version produced either arm. Labels must contain 1–128 UTF-8 bytes and no C0/C1 control characters. The calculation remains **Input minus Baseline**.

The command accepts Python 3.10 or later. It installs no packages. A modern browser supplies the optional search/filter/order controls. Without JavaScript, every original row, the complete-cohort summary and the JSON download are still present.

## Reading the result

The four headline values describe the **complete recorded cohort**: pair count, mean difference, median difference and population standard deviation. The outcome bar and its explicit counts classify positive, zero and negative score differences. The lowest/highest differences are also shown.

All pairs appear in a table, initially in the native identity order: scenario, trainee, then numeric signed seed. Every seed is rendered as exact decimal text, including both signed 64-bit endpoints. Browser sorting does not convert seeds to floating-point numbers. Scores and deltas are also shown as exact integers; the widened difference can span −4,294,967,295 to +4,294,967,295.

Search is a literal, case-insensitive substring search across the displayed row text. It does not interpret a regular expression. Outcome filtering and sorting affect only the table view; the match count says how many rows are shown, while the complete-cohort summary remains unchanged. Equal sort values retain native identity order. Reset restores every pair and that original order.

Grades are literal recorded strings, not ranked categories. Unicode, markup characters, quotation marks and whitespace are escaped/preserved for display. Nonprinting C0/C1 controls other than tab, line feed and carriage return are shown as `⟦U+XXXX⟧` markers. These markers may also occur literally in a source string, so the visual spelling is not a reversible encoding. **Download original comparison JSON** returns the exact original bytes for data processing, including control characters, formatting and full floating-point values.

The page rounds average/spread values to three decimal places for display. The downloadable report preserves their original values. HTML source includes an inline-script hash policy and no external assets or resource requests. All data text is escaped; source values are never inserted into executable JavaScript.

## What is checked before writing

The reader accepts exactly `uma-sim.paired-batches.v1`, direction `input_minus_baseline`, and identity fields `scenario`, `trainee`, `seed`. Its admission follows the native structures in `uma-sim-core/src/batch_pairs.rs` at qualification basis `cd852b3be106855524e789c42670d38f419d3bd5`:

| Field | Reader requirement |
| --- | --- |
| Whole input | Regular nonsymlink UTF-8 file, at most 64 MiB, exact caller-supplied SHA256 |
| JSON objects | No duplicate keys, nonfinite constants, missing or unknown schema fields |
| Complete rows | 1–100,000; unique identities already in native order |
| Scenario / trainee | Literal nonempty string, at most 256 UTF-8 bytes each |
| Baseline / input grade | Literal nonempty string, at most 64 UTF-8 bytes each |
| Seed | Exact integer in signed 64-bit range; bool and JSON `1.0` are refused |
| Scores | Exact signed 32-bit integers |
| Delta | Exact widened input score minus baseline score |
| Summary integers | Count, wins, ties, losses, minimum and maximum exactly recomputed |
| Summary real values | Mean, median and population standard deviation recomputed from all rows |

The 64 MiB reader limit is its own report-file boundary; it does not change the native comparator's separate 16 MiB limit for each input batch. Large accepted files may take more time and memory to render or manipulate in a browser. No responsiveness threshold for a 100,000-row page is claimed.

Mean and variance use the native sorted-delta accumulation order. Floating summary values must agree with relative tolerance `1e-12` or absolute tolerance `1e-9`, allowing small arithmetic differences while keeping every integer fact exact. This is an integrity check, not an assertion that a supplied report came from the native binary. Recomputing a file's hash likewise identifies the bytes you selected; it does not authenticate their origin.

## Failure and file custody

An invalid report, wrong digest, invalid label, missing parent directory or existing output is refused with exit 2 and one JSON error on stderr. There is no success stdout. CLI argument errors use the same boundary; `--help` retains normal usage behavior.

The command validates and renders the whole report before exclusively creating the output. It never overwrites an existing file, follows an input symlink, or replaces an existing output symlink. Using the input itself or an existing hard-link alias as output is refused. It checks input bytes and nanosecond modification time again before and after writing. An output-write failure or detected input change attempts to remove only the file created by that invocation, provided the destination still names its created inode. It does not delete a concurrently substituted file. Cleanup is best effort: a filesystem refusal during removal can leave an incomplete file, without a success receipt; inspect the chosen output after a filesystem error.

As with ordinary local file processing, this is not a lock against another process changing the input. A detected concurrent change causes refusal rather than a silently accepted mixed report. The command creates no directories, session, settings, telemetry or simulator output.

## Meaning and limits

This is descriptive pairing of recorded outcomes. It supplies no confidence interval, significance test, independent-sample guarantee or causal policy-benefit claim. Matching seeds and identities alone do not establish identical simulator versions, decks, policies or random-number consumption. Historical saved studies retain their original model limitations and source context; a new reader copy does not requalify the model or show installed adoption.

The existing single-career telemetry reader and the simulator's frontend remain separate. This command consumes an already produced complete paired report and does not alter either implementation.

## Maintained checks

```sh
python -B -m unittest discover -s scripts/tests -p test_paired_comparison_report_596c.py -v
```

The eight methods cover an actual CLI course with mixed outcomes, signed seed/score endpoints, Unicode/markup/control text and byte-exact embedded JSON; complete-summary reconciliation and rounding tolerance; duplicate/nonfinite/malformed JSON and UTF-8; identity order/range/type/delta refusals; native string bounds; digest/existing-file/alias/symlink custody; changed-input and output-write failure cleanup; and size/label/argument boundaries. Symlink controls run only where the host permits creating a symlink.

Independent command-line receiving completed 32 actual calls: one successful mixed fourteen-pair report and 31 malformed-input or file-custody refusals, with 125 recorded predicates. A separate complete static reconciliation received the author's existing historical sixteen-pair HTML without rendering it again. Every table row, exact seed/delta string, summary display, escaped field and embedded original JSON matched the fixed inputs; protected source/input/sentinel bytes and nanosecond timestamps remained unchanged.

Actual browser receiving is **unqualified**. The available browser rejected navigation to the synchronized local file under its security policy before loading either report and explicitly prohibited workarounds. No alternative browser route or server was used. Search, filtering, sorting, matched-count updates, actual browser download, JavaScript-disabled behavior and responsive layout have source review and static evidence only, not observed browser acceptance. The source-adjacent evidence retains that refusal separately from the successful CLI/static course. This limitation does not change the historical study's scientific scope, and no simulator was rerun.

Ownership and review scope: [issue165](https://github.com/Jacob-Met/uma-sim/issues/165). This increment adds the reader, its focused tests and guide; it changes no native comparison or simulator source.
