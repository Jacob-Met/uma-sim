# Career-lab comparison identity repair

Receiving identity: `estate-49f845d0dece/product`

Base: `f5f9b29393731d18aee2d66a31d89c315aa79c60`

Executed: 2026-10-08, isolated macOS checkout with Chromium 153.0.8010.12 and Playwright 1.62.1.

## User-visible problem

After comparing branches A and B, selecting C as the next B choice left the A/B result visible but made both report links export A/C. The same confusion between the displayed result and draft radio selections caused two deletion defects. Deleting a draft-only branch cleared a valid displayed comparison; deleting a displayed-only branch could leave its obsolete result and download links visible. A delayed compare response also overwrote newer draft selections.

The actual baseline browser run records four failures. For the export contradiction, the displayed pair was `br-1791447747-0000` / `br-1791447747-0001`; the downloaded report used `br-1791447747-0000` / `br-1791447747-0002`.

## Repair

- [ComparePanel.tsx](../../../packages/uma-sim-ui/src/components/ComparePanel.tsx) constructs JSON and Markdown report links from the displayed comparison's `aId` and `bId`.
- [labStore.ts](../../../packages/uma-sim-ui/src/state/labStore.ts) preserves newer draft radio selections when a compare finishes. Successful deletion invalidates the displayed result using its own IDs, clears matching draft selections independently, and applies that state change before refreshing the branch list.

The production change is confined to those two files. Browser/session targeting has separate ownership in [issue #63](https://github.com/Jacob-Met/uma-sim/issues/63). The candidate receipt records native Rust API and career-lab engine source hashes; both match the pinned base source.

## Executed verification

The receiver drives the built React UI in a fresh actual Chromium context. It starts the native Rust API and a same-origin UI proxy on fresh loopback ports, generates a seeded career through the UI, and uses real checkpoint, fork, branch, compare, delete and report endpoints. It checks downloaded file content. One test deliberately holds a real comparison response while keyboard input changes the next radio selection.

| Browser check | Baseline | Candidate |
|---|---|---|
| Save a checkpoint through the React UI | Pass | Pass |
| Fork open preserves checkpoint and main session | Pass | Pass |
| Close fork and resume the saved checkpoint | Pass | Pass |
| Run branches, compare, and download the displayed pair | Pass | Pass |
| JSON report remains bound to displayed pair after radio change | Fail | Pass |
| Delayed compare preserves newer draft selection | Fail | Pass |
| Deleting a draft-only branch preserves displayed comparison | Fail | Pass |
| Deleting a displayed-only branch clears result and downloads | Fail | Pass |
| Markdown report remains bound to displayed pair after radio change | Not recorded | Pass |
| **Recorded cases** | **4 passed, 4 failed** | **9 passed, 0 failed** |
| Browser page errors | 0 | 0 |

The Markdown assertion was added before the candidate run. The current portable receiver includes it for future runs.

The candidate's initial JSON and post-selection JSON reports are byte-identical and identify `br-1791449071-0000` / `br-1791449072-0001`. Its Markdown report names the same two branches. The post-selection screenshot and transient native API log remain in the isolated receiving output; the source-controlled evidence below contains the raw downloaded reports and execution receipts.

Completed builds:

- `npm ci --ignore-scripts --no-audit --no-fund`
- `npm run build` (`tsc -b && vite build`)
- `CARGO_BUILD_JOBS=2 cargo build --offline --locked -p uma-sim-core --bin uma-sim-api`

An independent worker reviewed the exact two-file source diff, the complete portable receiver, both execution receipts, and candidate downloaded files. The reviewer found no blocking defect and independently confirmed the recorded counts and source hashes. This was a source-and-evidence review, not a second browser execution.

## Evidence

[SHA256.json](SHA256.json) records byte sizes and SHA-256 hashes of the preserved evidence files.

- [Baseline receipt](baseline-receipt.json), [initial JSON report](baseline-initial-comparison.json), and [post-selection JSON report](baseline-after-selection-report.json)
- [Candidate receipt](candidate-receipt.json), [initial JSON report](candidate-initial-comparison.json), [post-selection JSON report](candidate-after-selection-report.json), and [post-selection Markdown report](candidate-after-selection-report.md)
- [Candidate UI build log](candidate-ui-build.log) and [native API build log](api-build.log)

Both receipts record the Git HEAD at execution, browser version, generated checkpoint/branch identity, request bodies, case outcomes, and page errors. The candidate additionally records SHA-256 of the relevant source files, built UI asset hashes, and native binary hash. Its `source` field identifies native repair commit `e11fe89f8c3c544469b0c5628fb7f657109be6e8`; `sourceFiles` and `apiBinary` store explicit `sha256` metadata objects.

The original candidate run and raw receipts remain preserved in native commit `e11fe89f8c3c544469b0c5628fb7f657109be6e8`. Publication initially triggered two `generic-api-key` false positives on computed file digests. The receiver now records those digests as structured metadata. The refreshed candidate run completed all nine cases again with identical production source, native binary, and built UI asset hashes. No scanner configuration or production behavior changed.

| Candidate source | SHA-256 |
|---|---|
| `ComparePanel.tsx` | `5d1497dd64f0e43fb751bd9d2f1e46d94f2842da8a90421d78a56d669fdc853a` |
| `labStore.ts` | `aeccd1b983ff988f185b2bc5ce1cfffcbf1f9d7020929fce4e6117b0e21a926a` |
| Native `api.rs` | `3db7edc89536c1670c26c3a900de8654fde1b27d66c552d017c94dcc31e44f95` |
| Native `career_lab.rs` | `95cb00bd9418e889372b83eb998f8100887218d91b288af9180deff0843427c7` |

## Repeat the browser receiving check

Use an isolated checkout with the Rust and UI dependencies available. From the repository root:

```sh
CARGO_BUILD_JOBS=2 cargo build --offline --locked -p uma-sim-core --bin uma-sim-api
npm --prefix packages/uma-sim-ui ci --ignore-scripts --no-audit --no-fund
npm --prefix packages/uma-sim-ui run build
UMA_LAB_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs \
UMA_LAB_CHROMIUM_EXECUTABLE=/absolute/path/to/chromium \
UMA_LAB_OUTPUT_ROOT=/absolute/path/to/receiving \
node packages/uma-sim-ui/tests/lab-comparison.browser.mjs
```

The executable and module overrides allow reuse of an existing Playwright/Chromium installation. If omitted, the script imports `playwright` and uses its default Chromium executable. The recorded run used the versions listed above. `UMA_LAB_API_BINARY` optionally selects the built native binary; `UMA_LAB_REPO` optionally selects the matching source/dist root.

The default stage is `candidate`; a failed assertion exits nonzero. `LAB_STAGE=baseline` is available when deliberately reproducing the original mismatch on the base source. Output defaults to `.lab-receiving` under the repository when `UMA_LAB_OUTPUT_ROOT` is absent.

Every execution uses a new temporary native API working directory for generated career storage and a fresh browser context. The server binds to loopback and the browser aborts non-loopback network requests. Browser, proxy and native API processes close after the run. Generated fixture storage and receipts remain under the chosen output directory for inspection.

## Receiving boundary

This evidence establishes the comparison repair against the exact source and local native runtime recorded above. It is not a deployed-service acceptance result or a verification of the separately owned session-targeting changes. Publication and current integration checks are tracked in [PR #65](https://github.com/Jacob-Met/uma-sim/pull/65).
