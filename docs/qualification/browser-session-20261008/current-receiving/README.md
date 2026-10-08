# Browser-session receiving history

## Composition after checkpoint import landed

Receiving parent: `0ad4bd4d2ca0a133fe91080b6b7818efbaedb2fa`; session parent: `63aca264ee8b47df14bcecf47cf1b390b244852f`. The incoming PR114 exact-JSON import is retained. Only `withSession` and the package test command overlap: the client keeps all incoming bytes except the reviewed empty-session condition, and both maintained test files are included.

Actual native `npm run build` on the composed application passed **116 tests, zero failed or skipped**, then TypeScript and Vite succeeded (69 modules). All 65 measured UI inputs remained unchanged. `post-import-native-ui-build.json` records the complete command output and source/build hashes. The first cache-path ENOSPC refusal and corrected child-only cache configuration are recorded separately; no shared cache or source was removed. Required hosted gates on the newly published composition remain the integration gate.

The appended scanner exception identifies one historical qualification SHA256 checksum, independently recomputed from its pinned API source. It retains the four existing fingerprint exceptions and every scanner workflow/rule. `scanner-checksum-review.json` records the exact finding and source-byte proof.

`hosted-ci-receipt.json` and the raw UI/Rust logs below qualify the earlier published tree `ebf0e447639c24b30d364dea0c5a92a5f6d9966b`: 101 UI tests and 414 native Rust tests passed, with one native test ignored, plus browser/trace, terminal recovery, MCP, release and calibration gates. The 39-case wrapper receiver made 60 inert HTTP requests; its complete source and output are retained. These records keep their original source attribution.

## Earlier receiving at 8d74413

Receiving parent: `8d74413d7dbc3caf1745779c92d56b6eb7feea42`. Original qualified session head: `fa8711b8baeb48bb2ba2732a7d47084bff86bf7b`. The merge keeps every incoming product, test, and dependency path except the explicitly recorded session changes.

The native API differs from the receiving parent only in the three reviewed session selector functions. The run store and API client keep the earlier qualified correction. App composition retains the newer Conditions and Skills panels, retained LogPanel history, RacePanel history and current lab behavior. The complete incoming npm test list and Playwright dependency survive; the regression list receives the session test file. The renderer dependency is locked at 18.3.1.

### Actual receiving results

- Native `npm run build`: 101 tests passed, zero failed/skipped; TypeScript and Vite succeeded, 68 modules transformed. The receipt preserves all 63 input hashes and three generated dist hashes. Inputs remained unchanged across execution.
- Independent actual-React receiver: 20 tests passed, zero failed/skipped, with complete 46-file source/test/config manifest. It includes delayed action/selection flows through the current App and verifies that the new panels receive the selected career's state/history. Stylesheet contents are outside these fixture claims.
- Separate TypeScript `tsc --noEmit --incremental false`: exit 0 on the receiving source.
- The original 344-test Rust workspace output is preserved in the parent qualification directory and explicitly attributed to its historical tree. Current Rust and hosted browser receiving remain CI gates.

The first shared-filesystem test attempt stopped at ENOSPC while compiling the new session suite; 88 other tests passed. The full build then executed in one private ephemeral copy without moving or deleting the source. Both outcomes are retained. The ephemeral build directory does not survive its invocation; the full output and generated-file hashes do.

`review-receiving.mjs` is the exact independently executed in-memory receiver, with its original source/dependency path parameters retained as execution evidence. The shipping regression suite is `packages/uma-sim-ui/tests/run-session.test.mjs` and runs through the native package scripts. This source integration does not claim an installed runtime change.
