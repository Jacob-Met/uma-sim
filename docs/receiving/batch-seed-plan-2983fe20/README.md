# Batch count and consecutive-seed admission

Author: estate-production-2983fe20 / estate_production.

Code and maintained-test input: `a248e4c25dc50c6f28116a149d6aa7051933e91b`.
Original input: `6245d48132df3f07aaa314f5faa9cb31536449b1`.

## Why this changes

The existing batch command parses a malformed effective count as 100, accepts
zero/negative counts as an empty cohort, and eagerly collects unchecked
`seed_start + i` values. These are consequential before the simulator runs:
the command opens/truncates its chosen output file even for an empty request.
Optimized arithmetic can also wrap into a different signed seed.

The new private seed plan admits the effective positive count and complete
signed-i64 range before directory creation, output opening or career execution.
Consecutive values use an inclusive iterator, so its endpoint can equal
`i64::MAX` without a subsequent overflow or a whole-cohort vector allocation.
No new batch-size cap is imposed.

An explicit first `--seeds=` list retains its previous parser, ordered
duplicates, ignored empty comma fields, first-list precedence and exit 1 error.
It still overrides count, including an invalid ignored count. The first count
wins when several are supplied. Existing strict `--seed=` admission still
runs first. Batch output-failure handling, simulation and paired comparisons
are unchanged.

The old zero-count success is deliberately retired. The inherited seed test
that used an empty output to admit signed endpoints now runs one actual
career and checks its exact terminal seed. Other seed tests remain intact.

## Original native evidence

Both binaries were built from the original input with Rust/Cargo 1.93.1 on the
ThinkPad, in a new isolated clone with two build jobs. The initial offline
build lacked a cached `tiny_http`; the ordinary locked build fetched the
missing dependencies. Later candidate tests ran locked and offline.

The receiving script starts a real seed-7 fixture career, authors an output sentinel, then invokes the real
CLI against only its disposable files. It records raw stdout/stderr, exit
status, output hashes/seed strings and saved-career equality.

| Case | Original debug | Original optimized |
| --- | --- | --- |
| Malformed count | Began unintended 100-career batch, wrote 91 records before the 90-second receiver timeout | Success, 100 actual records |
| Empty count | Success, 100 actual records | Success, 100 actual records |
| Zero or negative count | Success, output erased | Success, output erased |
| Maximum seed plus count 2 | Overflow panic 101 before output opening | Success, records use MAX then MIN |
| Five supported control cases | Pass | Pass |

The supported controls cover ordinary consecutive seeds, explicit-list
precedence and repeated seeds, first-count precedence, a range ending at MAX,
and a one-record MIN batch. Every original receiving case retains the saved
career bytes. The timed-out debug case remains recorded as a timeout and is
not relabeled as a successful batch.

The original script's `rowCount` counts nonempty output lines, including
the authored sentinel on a refused command. For a preserved output,
`rowSeeds:[null]` is a sentinel line, not a produced career. The underlying
hashes, raw output and seed strings are retained unchanged.

## Candidate native qualification

The following command passes 39 tests on the exact code input:

```sh
CARGO_BUILD_JOBS=2 cargo test --locked --offline -p uma-sim-core \
  --bin uma-sim --test batch_seed_plan --test cli_seed_admission \
  --test batch_output_failures --test paired_batches_cli
```

- 8 helper tests: default 100, lazy maximum count, both signed endpoints,
  invalid count families, explicit/default overflow and precedence.
- 8 actual-CLI test groups: authored output and saved-career preservation,
  fresh parent noncreation, valid sequence ordering and first refusal.
- 15 inherited seed-admission tests, with the one positive-count boundary
  adaptation described above.
- 3 unchanged native output-failure tests.
- 5 unchanged paired-batch CLI tests.

Invalid-count regressions set an unavailable command-local external policy
only as a sentinel: if admission ever regresses, a first attempted workload
fails promptly instead of running 100 unintended careers. Correct admission
returns exit 2 before consulting that policy. Accepted cases run the ordinary
native default policy with the existing stub race mode.

The source changes do not modify formulas, engine RNG, terminal records,
catalogs, API/MCP/browser behavior, dependencies, workflows or installed
runtimes. This packet proves focused native source behavior. Full-workspace
CI, independent receiving, source integration and release deployment are
separate states and must retain their own exact input pins.

## Files

`manifest.json` records the files' hashes and exact source inputs.
The two original receipts and build logs are unmodified. The author test log
is complete. `receive-batch-plan.mjs` is the original witness, callable with
`node receive-batch-plan.mjs BINARY SOURCE_ROOT NEW_EVIDENCE_DIRECTORY`;
its complete malformed debug batch can exceed the script's 90-second child
timeout, as the retained original receipt demonstrates.

Native coordination was published before source edits at
`/srv/hamon-estate/coord/estate-production-2983fe20-uma-batch.json`.
Initial GitHub issue creation was refused by a secondary content-creation
rate limit; no claim was invented from that failed call. Normal publication
follows current branch checks, independent review and the repository's
CI-on-head requirement.

## Current-main qualification

The branch normally merges main
`e1ca9baaa89036103d783c6eb02b98deed48c69f` at composition
`2e6d1cb734e804a209db1fe3c392061e088ffb02`. Main adds the saved-career
training inspection feature; the batch seed-plan implementation and its
maintained tests remain exactly the original code input above. Against this
composition the same native command, with `--test training_inspection`
added, passes **46 tests** (39 prior plus seven inherited inspection tests).
`current-native-tests.log` is the complete locked, offline author run.

Independent receiving and hosted checks must qualify this current-main
composition or its source-identical documentation descendants before merge.
The original receipts keep their original input pins.
