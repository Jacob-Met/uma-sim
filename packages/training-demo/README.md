# UMA training-turn demo

An **unofficial, recorded-fixture browser demo** of one training turn. Pick a seed and checkpoint, then compare seven training and recovery choices from the same native state. The page shows the complete original simulator response, before/after changes, current status, and the exact preparation actions that produced the checkpoint.

Open [`out/index.html`](out/index.html) directly in a browser. It is a single self-contained HTML file: React, styles, and the complete recorded fixture are included. It needs JavaScript and makes no API requests. To serve the same file locally:

```sh
python3 -m http.server 8765 --directory packages/training-demo/out
```

Then open `http://127.0.0.1:8765/`. The receiving helper below installs this same page at an isolated `training-demo/` route. This package does not change the repository's shared UI, landing pages, or deployment configuration.

## What the controls do

- **Seed:** choose 1, 7, or 42. Changing the seed keeps the selected preparation depth and resets the comparison.
- **Checkpoint:** choose the first free training turn or the checkpoint after three or six free training choices. Mandatory races and events encountered during preparation are included in the recorded history.
- **Choices:** five training actions, rest, and recreation. Each button shows a native outcome captured from the same checkpoint. Keys **1–7** perform the same actions; form controls, modifier keys, and repeated keydown events are excluded.
- **Reset comparison:** return the displayed status to the selected checkpoint.
- **Replay this recording:** show the identical captured outcome again. The screen-reader announcement counts comparisons viewed at the checkpoint.

Choosing a second action compares an alternative; it does not continue from the previous result. The result's next legal choices are shown as information. This finite fixture does not provide arbitrary seeds, a full playable career, or a new simulator implementation.

All captures use Special Week, URA, the shipped **physics** race model, and the **default** internal policy. Preparation uses actual legal `race`, `event_0`, and `train_speed` actions. Each alternative is captured after a fresh seeded start and replay of the entire preparation history.

| Seed | First training checkpoint | After 3 training choices | After 6 training choices |
| --- | --- | --- | --- |
| 1 | Turn 2, energy 100 | Turn 5, energy 40 | Turn 9, energy 20 |
| 7 | Turn 3, energy 100 | Turn 7, energy 40 | Turn 10, energy 5 |
| 42 | Turn 2, energy 100 | Turn 5, energy 10 | Turn 8, energy 10 |

The nine checkpoints contain 63 complete native outcomes, including low energy, injury, and recovery. Their differing turn numbers are native results of the actual preparation, including intervening events.

## Rebuild from the existing UI toolchain

Use Node.js 20+ and the existing UI's lockfile. From the repository root:

```sh
npm ci --prefix packages/uma-sim-ui
node packages/training-demo/build.mjs
```

The builder imports the existing `StatsPanel`, `ChoicePanel`, UI types, and stylesheet unchanged. Its HTML entry retains the existing UI shell structure, with a demo title, description, application marker, and no-JavaScript explanation. It type-checks the actual TypeScript imports, verifies the exact reused Git blobs and fixture hash, builds with Vite, and inlines the single script and stylesheet into `out/index.html`. All temporary build files are created and removed inside this package.

`out/build-receipt.json` records the input pins, locked tool versions, builder hash, and final page bytes/hash. Its `sourceCommit` and `sourceTree` identify the qualified UI input baseline: commit `d85c09556eedbbed78a5d21204e239ef3e1900d3`, tree `178ef23920a19cf2b58117e2c683c1f001a17fe9`. The receipt records the actual package metadata separately, while `nativeFixtureSource` preserves the simulator revision below.

The builder pins the five directly reused UI source blobs, the TypeScript configuration used for typechecking, and the exact lockfile. It validates every remaining `package.json` field with Node's strict structural comparison, including the complete field set, value types, object/array distinctions, and dependency declarations, while ignoring only the top-level `scripts` field. It invokes TypeScript and Vite directly and does not run those project scripts. This permits unrelated UI test-script changes without accepting dependency or other manifest changes. `inputUiBlobs` records the actual package Git blob; `nonScriptPackageSHA` records the qualified non-script manifest SHA-256 (`4956fe167824c184fd52951070ea1f3d9dede8cf471d893f1a22d99e32e910ae`).

A pre-existing dependency cache may be supplied read-only with `--toolchain /path/to/uma-sim-ui`; the six direct compiler/runtime/type versions must match the canonical UI lock. The demo build does not import the UI's separate Playwright test dependency.

### Check the build input boundary

Use the included Node receiver with an already-installed UI toolchain and a new output directory:

```sh
node packages/training-demo/tests/receive-build.mjs \
  --toolchain packages/uma-sim-ui \
  --output ../uma-training-build-receipt
```

The receiver builds a fresh input copy from the current package and from a scripts-only variant whose commands would exit with an error if invoked. Both must reproduce the qualified page below. It then rejects dependency and manifest-field changes, as well as changes to the lockfile, consumed source, and TypeScript configuration, while verifying that prior output bytes remain intact. The independent receiving run passed all 11 groups with this builder against the merged UI inputs. The receiver retains its input manifest, build receipts, and raw logs, and verifies that original source inputs remain unchanged.

| Artifact | SHA-256 |
| --- | --- |
| `fixture.json` · 430,479 bytes | `906e6876f2485b012677ffda82295858ad5afda818c33b57a39fece9f992d9de` |
| `out/index.html` · 348,711 bytes | `56f75bd888cc20b5c71ea007a76430736c0668eb89805dc867b81fb053056b85` |

## Reproduce the native outcomes

The native source is pinned to commit **`0bc58cb83e89f07c6af3056dd7c37436906d994f`**, tree **`7c1b145124486f2a7911df3cecb620558abf84e1`**. Build that exact clean revision in an isolated worktree:

```sh
git worktree add --detach ../uma-training-native 0bc58cb83e89f07c6af3056dd7c37436906d994f
cargo build --locked --manifest-path ../uma-training-native/Cargo.toml -p uma-sim-core --bin uma-sim-api
node packages/training-demo/tests/verify-native.mjs \
  --fixture packages/training-demo/fixture.json \
  --fixture-sha256 906e6876f2485b012677ffda82295858ad5afda818c33b57a39fece9f992d9de \
  --source-root ../uma-training-native \
  --binary ../uma-training-native/target/debug/uma-sim-api \
  --output ../uma-training-native-receipt
```

The independent verifier starts an isolated native API, replays every actual preparation and action, and compares all nine checkpoint states, legal-choice arrays, and 63 complete action responses. It recursively sorts object keys while preserving array order and every value. It also exercises two actual counterexamples: omitted preparation training and substitution of a training result for rest. It records the source and binary identities, preserves its receipt in the new output directory, and stops only its own API process.

The included unchanged [`tests/native-receipt.json`](tests/native-receipt.json) records the qualified 495-request native replay and both rejected counterexamples; its SHA-256 is `d81aa48df751fb4c5d45f684dc84422fb96bf09b8c594c04f2ceeb8fdbd7b519`. [`tests/native-counterexamples.json`](tests/native-counterexamples.json) retains the actual differing states and responses. These historical receipts preserve their receiving host paths; the portable verifier accepts caller paths and does not depend on those locations.

The frozen fixture was captured and independently replayed using a native macOS ARM64 binary with SHA-256 `57ff25f252bb1474bca9a3ad8da4f220b47c119cdd4e3251e33f67660607e6cb`. A binary rebuilt on another platform can have different bytes; the verifier records that identity and still requires exact behavioral agreement. `--binary-sha256` optionally enforces a particular binary's byte identity.

`capture.mjs` is the original capture program. To create a separate capture for inspection, explicitly start the pinned native API from its source worktree and pass its loopback URL, exact source checkout, binary, and a new output filename:

```sh
node packages/training-demo/capture.mjs \
  --base http://127.0.0.1:PORT \
  --source-root ../uma-training-native \
  --binary ../uma-training-native/target/debug/uma-sim-api \
  --output ../uma-training-new-fixture.json
```

The capture checks native source identity, starts only its own explicitly named sessions, confirms each replayed baseline, and records actual API responses. The qualified fixture used by the page remains pinned by the builder; a refreshed fixture requires a deliberate source update and independent native receiving.

## Re-run the independent browser receiving

The unchanged [`tests/receive-browser.mjs`](tests/receive-browser.mjs) checks the actual rendered page against the native fixture, without importing app modules or reading React state. Supply an already-installed Playwright package and Chromium executable; it installs nothing. All six flags are required. The output directory must be new with an existing parent, and the temporary directory must be an owned location.

```sh
node packages/training-demo/tests/receive-browser.mjs \
  --page packages/training-demo/out/index.html \
  --fixture packages/training-demo/fixture.json \
  --out ../uma-training-browser-receipt \
  --playwright /path/to/node_modules/playwright \
  --chromium /path/to/chromium \
  --tmp ../uma-training-browser-tmp
```

The accepted run used Playwright 1.62.0 and Chromium 151.0.7922.34 on macOS, with an explicit `en-US` browser locale. All 82 groups / 2,047 assertions passed: 63 complete rendered outcomes, nine baselines, exact preparation and response text, stats/facilities/deltas, same-checkpoint alternatives, reset/replay/seed/depth controls, numeric shortcuts and focus exclusions, keyboard navigation, mobile injury/recovery, and the no-JavaScript explanation. There were no JavaScript errors or external requests. The receiver writes its receipt and desktop/mobile screenshots into the caller's output directory and closes its fresh browser afterward.

## Install, read back, and roll back

The Node.js standard-library helper touches only `index.html` under an explicitly named `training-demo` directory. Its parent directory and the private receipt directory must already exist. It records immutable before/after bytes and permissions outside the served route, rejects foreign pages and symlinks, preserves neighboring files, and refuses rollback over a page changed after installation.

Use new receipt filenames for each installation:

```sh
node packages/training-demo/receive.mjs install \
  --source packages/training-demo/out/index.html \
  --sha256 56f75bd888cc20b5c71ea007a76430736c0668eb89805dc867b81fb053056b85 \
  --target /existing/public/training-demo \
  --receipt /existing/private/uma-install.json
node packages/training-demo/receive.mjs readback --receipt /existing/private/uma-install.json
node packages/training-demo/receive.mjs rollback --receipt /existing/private/uma-install.json
node packages/training-demo/receive.mjs readback --receipt /existing/private/uma-install.json
node packages/training-demo/receive.mjs install \
  --source packages/training-demo/out/index.html \
  --sha256 56f75bd888cc20b5c71ea007a76430736c0668eb89805dc867b81fb053056b85 \
  --target /existing/public/training-demo \
  --receipt /existing/private/uma-final-install.json
node packages/training-demo/receive.mjs readback --receipt /existing/private/uma-final-install.json
```

This exercises installation, exact restoration, and a final installed/readback state. Run the sequence in a disposable receiving scope before an intended deployment. `node packages/training-demo/receive.mjs --help` documents the complete boundary.

## Source and attribution

The package is part of the GPL-3.0-only repository. It reuses the existing status and choice components without altering their source or engine behavior. The compiled page preserves React's license notices and links to the repository's [LICENSE](../../LICENSE) and [NOTICE](../../NOTICE).

Umamusume: Pretty Derby and related trademarks belong to Cygames, Inc. This is an unofficial fan simulator and is not affiliated with Cygames.
