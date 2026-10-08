# Retained career log receiving record

## Receiving source and scope

Original reproduction base: `f5f9b29393731d18aee2d66a31d89c315aa79c60` on `main`,
tree `fc78f333d2f97f81fa5537ea749f3d1c732d34bd`.
Receiving parent: `0b5cc2342cd93141beb461edf6e83998388e67b8`,
tree `80c18a2a9698c7c0a1b270e433ce6df8b3b90164`.
The publication-turn freshness check caught two intervening career-lab deliveries.
Their comparison, import-label and receiving-evidence changes were preserved in
the full receiving tree; none overlapped the log reader's source hunk.
The TypeScript check, production build, ten helper groups and all five native
browser assertion groups passed again on this composed receiving source.
Scope is recorded in [uma-sim #67](https://github.com/Jacob-Met/uma-sim/issues/67)
and coordinated with the separate browser/session owner in
[issue63 comment6056215668](https://github.com/Jacob-Met/uma-sim/issues/63#issuecomment-6056215668).

The existing `App.tsx` log call now passes `history={cs.log}`. `LogPanel.tsx`
displays those retained snapshot entries, uses the component-local `logView.ts`
search helper and `logPanel.css`, and retains the former current-state rendering
inside a separately labeled disclosure. The UI package README and focused tests
document and exercise this receiving path.

No store, session/request implementation, Rust engine/API, catalog, dependency,
workflow or release configuration is changed. Other App wiring is unchanged.

## Reproduced user problem

The prior `Event log` received `state.textLines`. The API's `handle_text` calls
`TextRenderer::render` with an empty event-lines slice, producing the short
current-state summary. Actual recorded outcomes already exist in `state.log`.

The real original React UI and Rust API reproduced this with seed 42. At turn 4,
the native snapshot retained career start, the debut race, successful speed
training and failed speed training. The visible old log instead contained the
five-line state summary. [baseline-receipt.json](baseline-receipt.json) preserves
the exact retained entries, visible text, source hashes and API binary hash.

The new reader keeps each retained entry and its position, including duplicates.
Literal search ignores case while preserving original Unicode text and distinguishes
matching entries from individual occurrences. Search and scrolling back pause
following. Resuming with a filter explicitly clears it. A changed retained
sequence resets a paused view to the beginning; ordinary appended entries preserve
the recorded scroll offset. Following tracks viewport resizing as well as new data.

## Verification

The original and final React sources pass the existing TypeScript check and Vite
production build. The ten independent helper-test groups cover exact original
text reconstruction, regular-expression characters as literal data, markup-like
text, duplicate entries, non-overlapping occurrences and empty/no-match states.
They include 2,000 deterministic mixed-Unicode cases against a scalar-token oracle
and 14,641 history-sequence comparisons against a serialized-prefix oracle. The
helper does not use either oracle. The tests run on Node 20+ using the package's
existing TypeScript compiler.

Independent review also challenged resize behavior. Chromium reproduced the
initial candidate leaving a following reader above the newest entry after a
desktop-to-mobile resize. [resize-counterexample.txt](resize-counterexample.txt)
retains the failed assertion and exact old component hash. The correction observes
the scroll viewport only while following and disconnects when paused or unmounted.
The independently reviewed final component is SHA-256
`19d398a0d5f004da56d73d04233e14218ac6af798efeff2f5e85ddc3ce6bd10f`.

The optional [browser test](../../packages/uma-sim-ui/test/career-log-browser.mjs)
uses the actual built React UI, real Rust API and pointer/keyboard input. It checks
native career entries against the visible rows, summary separation, search/focus,
paused append and resize, following at desktop/mobile widths, and replacement by
a fixture admitted through the existing native checkpoint library. That fixture
checks Unicode, literal metacharacters, markup safety and long identifiers.
The receipt is emitted only after this run's processes/profile/state are cleaned up.

The final native browser run **passed all five assertion groups and exited 0**.
[browser-receipt.json](browser-receipt.json) preserves the checked source hashes,
unchanged API binary hash, native seed/turn and browser versions. The inspected
[desktop](desktop.png) and [mobile](mobile.png) screenshots show the actual reader
at turn 36, with 40 retained native events.

The test host used Node 22.22.1 and Chromium 153.0.8010.47. Rust was built with the
locked dependency set and unchanged source. Temporary-filesystem quota refusals
withheld passing receipts; only this contribution's build target and temporary
test career state were moved to its own writable host directory. No existing
server, career, profile or resident worker was used. Setup retains its existing
catalog portrait URL behavior; the log reader adds no external requests.
The final harness starts the browser before warming the API catalogs to limit
startup memory peaks and keeps browser temporary files in its own temporary state
directory. Earlier host startup/transport failures did not produce passing receipts.

## Reproduce

From `packages/uma-sim-ui`, run:

```sh
npm ci
npm run typecheck
node --test test/log-view.test.mjs
npm run build
```

From the repository root, with Node 22+ and Chromium installed:

```sh
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_LOG_CHROMIUM=chromium node packages/uma-sim-ui/test/career-log-browser.mjs
```

Optional environment variables select the API binary, writable profile/state
parents and evidence directory; see the UI package README. The test starts fresh
loopback servers and does not attach to an existing service.

## Evidence limits

This is the engine's retained snapshot log. It can omit detailed narration and
some action responses. Entry positions are not timestamps or turn labels, and
prefix equality is not treated as proof of career/session identity. The reader
does not reconstruct missing events or create a separate durable history. Native
receiving checks demonstrate this source behavior; they do not assert a public
deployment or measured player benefit.
