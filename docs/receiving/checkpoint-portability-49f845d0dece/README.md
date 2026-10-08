# Portable checkpoint browser receiving

Receiving identity: `estate-49f845d0dece/product`

Base: `0ac14602addd1a4610aa8359896915a34cce5659`

Source issue: [#70](https://github.com/Jacob-Met/uma-sim/issues/70)

## Correct the import-name promise

The import field said a blank name defaults to the file's stored name. Actual browser receiving disproved that promise: a checkpoint named `portable-origin-muzb9v6z` downloaded through Export and uploaded into an empty library became `seed42-t11-ura`.

An exported file is the complete `RunSnapshot`; it has no library entry name. Native import generates a default from seed, turn, and scenario. The one-line change in [LibraryPanel.tsx](../../../packages/uma-sim-ui/src/components/LibraryPanel.tsx) describes that existing behavior accurately:

> Name (optional — generated from seed, turn, and scenario)

## Actual receiving result

Both runs used Chromium 153.0.8010.12 with Playwright 1.62.1, the built React UI, and the real native Rust API. Candidate execution started at `2026-10-08T09:06:34.563Z`.

| Actual browser/native check | Baseline | Candidate |
|---|---|---|
| Browser Export preserves the complete snapshot and exact RNG words | Pass | Pass |
| Browser Import into fresh storage and subsequent Export preserve the snapshot | Pass | Pass |
| Rendered name label describes the generated default | Fail | Pass |
| Persisted checkpoint files survive an API process restart unchanged | Pass | Pass |
| Browser Resume restores the full engine snapshot after restart | Pass | Pass |
| Next browser Auto step reproduces complete state and RNG position | Pass | Pass |
| **Recorded cases** | **5 passed, 1 failed** | **6 passed, 0 failed** |
| Browser page errors | 0 | 0 |

The candidate received `seed42-t11-ura` while showing the corrected label. All six RNG state words in the next-step result matched the reference exactly. RNG call count advanced from 53 to 57 in both executions.

## Three processes, two fresh storage roots

Each receiving run creates two new directories, A and B, and starts three separate native API processes. The candidate process IDs were 50114, 50247, and 50253; all three exited after use.

1. **Origin in A.** Start the seed-42 URA fixture through the browser, advance ten Auto steps, save a custom-named checkpoint, and click its actual Export link. Compare the downloaded snapshot to the full native engine state. Advance once more through the browser to capture the reference next state.
2. **Import in empty B.** Stop the origin process and start the same binary in fresh B. Require zero checkpoints and live sessions. Upload only the actual downloaded file through the file chooser with the name blank. Export the imported entry through the browser and require complete parsed-snapshot equality.
3. **Restart in B.** Stop the import process and start a third process in the same B directory. Require an empty live-session map and identical persisted checkpoint file hashes. Resume through the browser, then independently read `/v1/run/state`. Require full origin equality and execute the same browser Auto step, requiring full reference-next-state equality.

The `/v1/library/load` response is not used as the restoration oracle: the check reads the engine after Resume. The `meta`, `settings`, `state`, `rngSeed`, `rngCalls`, and all `rngState` fields participate in equality. Library metadata is recorded separately because import regenerates it.

Every process uses the same explicit `UMA_REPO_ROOT`, native binary and UI build. Each boot receives a fresh browser context. Servers bind to loopback, and browser requests to non-loopback hosts are blocked. The two generated storage roots remain preserved in the local receiving output.

## Exact evidence

[SHA256.json](SHA256.json) records sizes and SHA-256 hashes for all 15 preserved execution files.

| Artifact | Purpose |
|---|---|
| [Baseline receipt](baseline/receipt.json) and [candidate receipt](candidate/receipt.json) | Exact source/build hashes, process/storage identity, requests, naming observations, RNG words, case results, and page errors |
| `portable-checkpoint.json` in each run directory | Actual browser Export download from storage A |
| `imported-roundtrip.json` | Actual browser Export download after import into fresh B |
| `origin.json` and `restored.json` | Independently read full engine snapshots before transfer and after restart/Resume |
| `expected-next.json` and `actual-next.json` | Full native engine state after the same actual browser Auto step |
| [UI build log](ui-build.log) | Successful `tsc -b && vite build` of the one-line candidate |

The baseline and candidate receipts record base HEAD `0ac14602addd1a4610aa8359896915a34cce5659`; the candidate was tested before its one-line repair was committed. Each receipt records the actual source file hashes. Candidate `LibraryPanel.tsx` SHA-256 is `4ee64eea0e358402db5ac82ea20bd5567db00e0bba2f24efec32b31b6ff4664a`.

The native binary has SHA-256 `61298b37b1ecdf7dfaaca892ac51de4b03ba22b233b962f1fa1304f5f0a2fd66`. It is the previously qualified local binary; native source is unchanged from its build through this receiving base and candidate. The same binary was used for all six process boots. The candidate UI build completed successfully.

The snapshots establish same-build, fixed-fixture portability and persistence across process restart. They do not establish cross-version compatibility or the separately owned concurrent-session behavior.

## Repeat the check

The portable receiver is [checkpoint-portability.browser.mjs](../../../packages/uma-sim-ui/tests/checkpoint-portability.browser.mjs). From a checkout with native/UI dependencies available:

```sh
CARGO_BUILD_JOBS=2 cargo build --offline --locked -p uma-sim-core --bin uma-sim-api
npm --prefix packages/uma-sim-ui ci --ignore-scripts --no-audit --no-fund
npm --prefix packages/uma-sim-ui run build
UMA_LAB_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs \
UMA_LAB_CHROMIUM_EXECUTABLE=/absolute/path/to/chromium \
UMA_LAB_OUTPUT_ROOT=/absolute/path/to/portability-receiving \
node packages/uma-sim-ui/tests/checkpoint-portability.browser.mjs
```

Existing Playwright/Chromium can be selected with the two overrides shown above. Without them the script imports `playwright` and launches its default Chromium. `UMA_LAB_API_BINARY` can select an existing matching binary, and `UMA_LAB_REPO` can select the matching source/dist root. Output defaults to `.checkpoint-portability` under the repository.

The default stage is `candidate`, which requires all six cases to pass. `LAB_STAGE=baseline` deliberately expects the original incorrect label while still requiring all five portability controls. The receiver captures failures, retains its artifacts, and closes its browser, local proxy, and native API processes.

## Independent review

An independent worker reviewed the complete receiver, the exact one-line production diff, both receipts, all twelve snapshot/download artifacts, and current persisted files. The reviewer recomputed all seven source hashes for each stage against the base/current files, checked the current binary and candidate UI asset hashes, and verified the A/B/B process sequence, completed exits, full snapshot equalities and exact next-step RNG equality. No blocking findings were reported. This was a source-and-evidence review rather than a second execution. The historical baseline JavaScript asset is represented by its recorded receipt hash; rebuilding the candidate replaced that asset in the local dist directory.
