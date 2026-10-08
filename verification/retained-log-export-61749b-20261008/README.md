# Retained career-log download receiving

## Product and source

This adds a local **Download all N entries** control to the existing career event
log. Plain text is a numbered, readable transcript. JSON uses
`uma-sim-retained-log/1` and preserves the exact string array, including order,
duplicates, empty strings, controls and isolated UTF-16 surrogates after parsing.
The button states the full retained entry count; the search filter never selects
what is downloaded. A click synchronously prepares the current history before
requesting a browser download. Previously prepared bytes remain unchanged when
the displayed history subsequently changes.

This is retained history, which can omit earlier events. It is not a checkpoint
or a complete-career claim. The file carries no inferred session/run identity.

The source base is `09df4c25cbd86c12d16a121481c01d1633a5efb6`, tree
`8352de08ded783d6d23030b312c36c5b9a4b0fe1`. All 48 selected UI/operations inputs
were matched to their Git blobs before use. Of those inherited inputs, 44 remain
exact; only LogPanel, its CSS, the test command and README change. Three new files
provide the child control, serializer/download helper and focused test. The
LogPanel change is one import and one child element. Its original retained
reader is credited to `estate-product-e652aa3341b6` / PR80; that reader, search
implementation and existing tests remain preserved. The active RacePanel #86
owner retains that separate feature and its App/native boundaries.

`frozen-source.json` lists the seven exact product paths. `source-manifest.json`
is the original selected source closure, not a full checkout. The original
contract was frozen before implementation; the separate boundary amendment
records root's instruction to leave browser acceptance unestablished. No browser
carrier, alternate surface or delegated browser operation was created.

## Executed qualification

| Gate | Actual result | Boundary |
| --- | --- | --- |
| Maintained focused tests | 8 top-level methods and 4 contained failure cases; Node reports 12/12, zero failures/skips | Existing TypeScript compiler, native React SSR, genuine Node Blob, simulated DOM/URL dispatch for helper faults |
| Original source receiving | Retained search works; no download control exists | Actual original LogPanel through native React TestRenderer |
| Candidate event receiving | 7/7 cases; 6 local Blob requests; zero fetch calls | Actual React hooks/callbacks and Node Blob/object-URL registry/timers; simulated document/anchor dispatch |
| Existing typecheck | Exit 0 | `npm run typecheck` |
| Existing production build | Exit 0; 60 modules transformed | `npm run build`; output hashes retained |

The event carrier demonstrates one-match and zero-match search exporting all
entries, append/replacement history at the next click, immutable prior Blob bytes,
plain-text selection, preparation refusal and explicit retry, empty-log disabling,
unchanged reader/input state and complete temporary URL cleanup. Saved files in
`authored/native-events-candidate/` are the exact bytes of the actual Node Blobs;
they are not claimed as browser downloads or native simulator output. Inputs are
explicitly authored strings.

Tests ran with Node 24.19.0, TypeScript 5.9.3 and React 18.3.1. The event-only
receiver used an existing matching React TestRenderer 18.3.1 installation, recorded
by exact path/entry hashes in its receipt. It is a QA dependency outside the
product manifest; no package, lockfile or runtime dependency was added. Existing
locked package inputs were reused read-only from the earlier verified local
cache. The normal build's regenerated tracked tsbuildinfo was hash-recorded then
restored to its incoming bytes, so it is absent from the contribution.

Actual browser save-dialog/download completion, visual layout, keyboard delivery,
public deployment and simulator operation are **unestablished by this packet**.
The app requests a local download and deliberately does not announce that the
file was saved. No API, native engine, session, runStore, App, RacePanel, library,
dependency or workflow source is changed.

## Reproduction

With the UI's existing locked dependencies available, run from
`packages/uma-sim-ui`:

```sh
node --test test/log-export.test.mjs
npm run typecheck
npm run build
```

For the supplementary Node-only event receiver, supply a fresh output directory
and an existing React TestRenderer package directory matching the UI's exact React
version:

```sh
node verification/retained-log-export-61749b-20261008/native_log_export_events.mjs \
  packages/uma-sim-ui OUTPUT candidate /path/to/node_modules/react-test-renderer
```

The same carrier's `baseline` mode uses the UI at the pinned original commit and
reproduces the absence while confirming its original reader still works. It
transpiles the exact TS/TSX modules with the package compiler, resolves their
imports and omits stylesheet imports only. DOM dispatch is explicitly simulated;
it does not launch or stand in for a browser. Output creation is exclusive.

Raw command logs, returned statuses, source identities and Blob bytes are retained
under `authored/`. This packet adds new feature qualification; it does not repeat
or relabel PR80's earlier native/browser receiving as current execution.

## Independent acceptance

The separate `estate_runtime/61749b0088e2` reviewer accepts all seven frozen
source paths without a requested correction. The original reader minus only the
new import/child hook is byte-identical; CSS/README preserve their old prefixes,
and the package edit only appends the new test entry.

The reviewer independently exercised one additional native React case with seven
assertion groups: anchor click refusal after real Blob/URL allocation shows the
inline alert, preserves the reader, removes the anchor and later revokes the real
URL, performs no automatic retry, and permits an explicit retry using only a
replacement career's current history. The JSON control includes every C0 control,
separate lone high/low surrogates, duplicate and empty entries. Its exact sources,
raw receipt and files are retained unchanged under `independent/`.

This supplements the author's gates; it is not a rerun of those gates and does
not establish browser download completion. Review receipt SHA-256:
`0494499e65b1df118e178098c61c4d719381e71983d06864164a10fa7c3bccb1`.

## Current receiving parent

Current main `f27829a266bb39fb1a4f298e2c2b3b32f380af0f`, tree `7ce4a2c5f68898de6256745d119a061c852b444c`,
has 679 leaf entries. All 48 qualified input blobs/modes/types remain exact,
with no new UI source file. `current-base-receiving.json` retains that comparison.
The newer parent changes are preserved through the full base tree; unchanged
source tests were not repeated. Final remote contribution and all unowned-parent
custody are checked separately after publication.
