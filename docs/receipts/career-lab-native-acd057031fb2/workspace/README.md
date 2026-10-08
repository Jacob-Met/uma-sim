# Native workspace receiving

The exact native candidate **97754679972a3128b5b84cde2159b43a4d85c56e**, tree **da863f9b1df928ed5bd1ac85642ee67bfd1f4cdc**, passes the three delegated native workspace gates. It descends from accepted main **0ac14602addd1a4610aa8359896915a34cce5659**, tree **e7d10088a713d8fbd7391c86050a92898652bdd6**. The independent checkout was clean before and after execution.

| Gate | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Pass, exit 0 | `accepted-977-fmt.log.gz` |
| `cargo clippy --workspace --all-targets --locked --offline` | Pass, exit 0 | `accepted-977-clippy.log.gz` |
| `cargo test --workspace --locked --offline` | Pass, exit 0; **354 passed, 0 failed, 0 ignored, 0 filtered** | `accepted-977-workspace-tests.log.gz` |

The full test run contains 71 result blocks: 67 nonempty test binaries, two empty binary targets and two empty doc-test targets. It includes all 45 core-library tests, the 14 existing career-lab integration tests, nine renderer regressions and six ended-branch regressions. `workspace-suites.json` records every result block and its count. No test filter was used for the accepted workspace run.

## Source scope

The production change is confined to Markdown rendering and the two shorter-branch length references in `uma-sim-core/src/career_lab.rs`. Three integration-test paths carry the existing report assertion adjustment and the two focused regression suites. One assertion in the API module's `#[cfg(test)]` block also requires the new legal Markdown escapes. The API module's complete production prefix is byte-identical to accepted main; its exact test-only diff and both prefix hashes are recorded in `source-scope.json`.

At the native gate commit there are zero changes to UI packages, Cargo manifests or lockfile, API/CLI entry points, catalog data, race-core source or workflows. The manifest lists all 182 changed paths, exact native SHA256/Git blob pins, unchanged scope checks and every Clippy diagnostic's source-equivalence basis. The warning in `career_lab.rs` belongs to the unchanged `sweep_stale_tmp` function, not the renderer or count correction. Clippy warnings are retained, matching the repository's CI command without a warnings-as-errors override. This is source comparison of diagnostic locations, not a separate baseline Clippy run.

The manifest deliberately stops at this gate commit. Later browser-receiving and documentation commits have their own scope: the final publication may include a Markdown-oracle adjustment to the existing UI browser test while preserving UI runtime source and manifests. That later test-file change must not be described as a zero-diff claim for every UI-package path.

## Preserved negative results

The first composed candidate, `f6bbea9f1480a1ed6db2a7b5f974743e94bc2e2f`, passed formatting and Clippy but stopped during the core-library tests: an API download test still asserted the unescaped source heading `e2e-rest vs e2e-train`. Its eight subsequent failures were poisoned-lock cascades. The exact same single API test passes on accepted main `0ac14602` (one passed, 44 filtered), establishing an introduced source-format expectation mismatch. Root corrected only that embedded test assertion in `9775467`. The complete initial failure and baseline-control logs are preserved.

Two further receiving attempts encountered shared storage exhaustion. The first retained only 4,096 bytes ending mid-line, so it has no accepted test summary. The streamed attempt preserved explicit OS error 28 during checkpoint and policy-stub writes, followed by shared-lock poison failures. Neither is counted as the final qualification. `capacity-attempts.json` preserves their metadata and recovery details; both raw outputs remain available.

The successful run used the same committed source after generated-cache cleanup and writable owned runtime directories. There were no additional production changes, skipped tests or assertion relaxations.

## Reproduction and evidence boundaries

Rust and Cargo were 1.99.0; exact toolchain versions and absolute runtime/cache paths are in `receipt.json`. Dependencies were already cached and all Cargo work was locked/offline. Builds used debug information level 0, incremental compilation disabled and two jobs. The accepted test rebuild additionally stripped symbols to reduce storage use; it remained the unoptimized default-feature test profile. Formatting and Clippy used the declared compact environment without that final strip override. Cargo manifests and the lockfile were unchanged.

Run the three commands in the table from the exact source checkout with an available Rust toolchain and sufficient writable target/runtime space. For the constrained environment used here, set `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2` and an owned `CARGO_TARGET_DIR`/`TMPDIR`; the successful workspace tests also used `CARGO_PROFILE_DEV_STRIP=symbols` and `CARGO_PROFILE_TEST_STRIP=symbols`.

This receipt qualifies the delegated native formatting, Clippy and workspace-test gates. Release embedding/layout, standalone calibration, UI build/browser receiving and MCP checks have separate evidence. Each compressed log is losslessly verified against its recorded uncompressed SHA256; the accepted test log is the complete concatenated native stdout/stderr tool capture. `receipt.json` is the authoritative index for final and historical attempts.
