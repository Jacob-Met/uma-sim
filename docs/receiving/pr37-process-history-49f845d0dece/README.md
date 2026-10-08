# PR #37 process-history regression receiving

Three native integration tests expose lost continuation history across CLI processes. They are intended to pass once the existing PR37 implementation preserves each prior command's recorded prefix.

## Native baseline

- Original PR37 head: `ef2b26729fd20231b47715bba904cc67c4f88649`; parent tree `99b25142b3fb358ba0f6e5898300333fbf5c7c3c`.
- Regression source: [career_lab_process_history.rs](../../../uma-sim-core/tests/career_lab_process_history.rs).
- Compiler and formatter succeeded. One baseline run produced **0 passed / 3 failed**, zero ignored or filtered, in **4.89 seconds**; the Rust test harness exited 101.
- [Test stdout](baseline.stdout.txt), [assertion failures](baseline.stderr.txt), and [source/native provenance plus commands](provenance.json) are retained.

| Regression | Observed failure |
| --- | --- |
| manual_steps_keep_the_previous_process_history | The second command replaced `Train:speed` at telemetry turn 3 with only `Rest` at turn 4. |
| manual_branches_report_their_first_different_action | Speed versus stamina first actions were followed by shared Rest. Comparison printed a one-step shared prefix and identical action sequences. |
| manual_then_play_keeps_the_manual_prefix | Full play began its saved history at turn 4 and discarded the manual `Train:power` record at turn 3. |

The fixture completes the mandatory debut before training. It then runs each operation through a fresh native CLI process. The comparison test verifies that branching preserves the source checkpoint bytes and that comparison preserves all durable state bytes before asserting the missing divergence. The prefix tests compare actual decoded records, including order, and check that save retains the complete accumulated continuation after the prefix behavior is fixed.

## Execution boundary

The original PR37 module, CLI, and six authored test files were fetched at the exact head and checked against their Git blob IDs. A receiving-only shim re-exports cached current native core and includes the unchanged PR37 module. The unchanged PR37 CLI links through that shim. Its six authored tests previously passed once with the current default physics model.

These new tests were compiled separately with `rustc --test`, the existing serde_json cache, and `CARGO_BIN_EXE_uma-sim` pointing to that already built native CLI. No product source or cached dependency was rebuilt for this baseline. They use the stub race model to bound process-history receiving, explicit current `UMA_REPO_ROOT`, and temporary directories inside the receiver's owned directory.

Current native core source reference: `0b5cc2342cd93141beb461edf6e83998388e67b8`. This is unchanged PR37 lab/CLI source exercised against cached current core; the untouched historical PR37 tree was not built. The existing architecture hold and author ownership remain in effect.

Portable invocation in the author's source tree once this test is adopted:

```sh
cargo test -p uma-sim-core --test career_lab_process_history -- --test-threads=1
```

The command above describes how Cargo discovers the added integration test. The recorded baseline used the direct rustc commands in provenance.json. A product correction has not been supplied by this test-only contribution.

## Scope and provenance

Publication adds only this test and these compact receiving records on a new branch parented directly by the original PR37 head. The author branch and implementation are preserved. The earlier diagnostic receiver's fixture-development attempts are excluded from this baseline.

Regression source SHA-256: `9f44f15862953d9549d1caf0c686068fc0c4f1b44aec50e278e725bcec252190`.
Regression Git blob SHA-1: `b6130d81d4effc39026eeb9818ef057b3e183fe8`.
Native CLI SHA-256: `ee9401204c9bd257fc0f4326328935e0511a70d475ddd2925eebcf7e7bf66229`.

Original PR37 source Git blobs: `1cb4d93260135ab4cc271cf181b732563f5f00f0` (lab), `fe6a82f970c99fe9e30b969cd2d8f5f8eb674722` (CLI), `3ff18fbe51956fc8228b8ddf52fc8537bfdfe7fe` (authored tests).

The separate generated branch-ID/result overwrite in landed main is tracked by [issue #76](https://github.com/Jacob-Met/uma-sim/issues/76). It belongs to main's different `run_branch` and `BranchStore` design.
