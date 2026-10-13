## Purpose and ownership

Owner: estate_product-596c, with existing team peers providing independent source and reader receiving. Reserve new-only `scripts/render_paired_comparison.py`, `scripts/tests/test_paired_comparison_report_596c.py`, `docs/PAIRED_COMPARISON_REPORT.md`, and a unique compact evidence/example directory. Current basis is `cd852b3be106855524e789c42670d38f419d3bd5`. No existing Rust, simulator, career-report (#128), frontend, calibration, workflow, or other owner path changes.

The native `compare-batches --format=json` already emits every complete matched pair. This increment makes that saved report readable as one standalone offline HTML document, including the existing historical sixteen-pair study. It does not rerun or reinterpret a simulation.

## Frozen command and admission contract

`python -B scripts/render_paired_comparison.py --input comparison.json --input-sha256 SHA256 --output comparison.html`

Optional `--baseline-label` and `--input-label` are user-supplied labels, default Baseline and Input; the document explicitly treats them as labels, not authenticated policy/version provenance. Python standard library only.

Read a regular nonsymlink UTF-8 JSON file of at most 64 MiB; require its exact supplied SHA256, reject duplicate keys/nonfinite values and unsupported schema/unknown keys. Accept exactly the maintained `uma-sim.paired-batches.v1` shape and `input_minus_baseline` direction. Require 1–100000 complete rows, exact signed-i64 seeds, signed-i32 scores, widened exact score deltas, native string byte bounds, and unique rows already ordered by scenario/trainee/signed seed. Independently recompute count/wins/ties/losses/min/max/mean/median/population standard deviation; float summary comparison permits only documented relative 1e-12 / absolute 1e-9 rounding tolerance.

Finish all admission/rendering before exclusive creation of a new output; refuse existing/symlink outputs and input aliases. Preserve input bytes and nanosecond mtime. On refusal return exit2 with a concise JSON error on stderr, no success stdout and no new partial output; on success return exit0 with JSON receipt binding input/output bytes and hashes. No source imports, simulator, settings/session, network, browser launch or installation.

## Reader behavior

One standalone HTML with no external assets: complete-cohort summary cards, wins/ties/losses bar and labelled counts, every pair in a table, exact decimal seed strings, signed input-minus-baseline deltas, and literal scenario/trainee/grade display. All original JSON bytes are embedded for a local download, so unusual control characters can be displayed visibly without changing the retained data.

Optional local JavaScript adds literal case-insensitive text search, All/Wins/Ties/Losses filtering, and Identity/Delta descending/Delta ascending/Absolute-delta order. Identity ties use original row order; signed seeds never pass through a JavaScript Number. The matched-row count is separate from the unchanged complete-cohort summary. Without JavaScript the complete static table and summary remain readable.

Use clear Baseline/Input labels and descriptive-only wording. No confidence interval, causal policy claim, independent-sample assumption, simulator-version authentication, or scientific significance inference.

## Receiving and publication

Freeze independent changed-input acceptance before implementation: the exact historical 16-pair report, a mixed authored report with signed-i64 endpoints and literal Unicode/markup/control fields, complete retained data, sorting/filtering/empty matches and disabled-script reading; malformed/mismatched summary/duplicate identity/digest/output-custody refusals. Review source separately; no Rust build or simulator course.

Publish a coherent reviewed branch with skip markers and verify remote source/tree and zero Actions. No PR while the inherited pull_request_target workflow would execute. Existing no-Actions removal owner #162 remains untouched. Deliver a small source/example kit through the existing UMA Applications route.
