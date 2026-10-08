# uma-sim-ui

Browser SPA for interactive `uma-sim` careers (Vite + React + TypeScript).

## Scripts

```bash
npm ci
npm run dev        # http://localhost:5173 — proxies /v1 -> :8765
npm run typecheck
npm run build      # writes dist/ (embedded by Rust with --features embed-ui)
```

Requires `uma-sim serve` (or `uma-sim-api`) on port 8765 for API calls.

See the root [README](../../README.md) for release / embed instructions.

## Reading a career

The **Career event log** shows entries retained in the current career snapshot,
including recorded training, event choices and race outcomes. Numbers show entry
positions. The engine records selected outcomes; this is not a complete transcript
of every action or narration. **Current state summary** expands the existing
summary of the current turn, stats and scenario resources.

Use **Search retained events** to find literal text without case sensitivity.
Matching entries keep their original positions and wording. The status reports
both matching entries and individual occurrences. Repeated entries stay separate.

Scrolling back or searching pauses automatic following. **Follow latest** returns
to the newest retained entry. When a search is active, the button explicitly clears
the search before following. Loading another retained history replaces the displayed
entries; the reader does not create an additional history store.

## Log reader checks

After `npm ci`, run `node --test test/log-view.test.mjs` with Node 20 or newer.
This uses the existing TypeScript compiler and needs no additional dependencies.
`npm run typecheck` and `npm run build` also cover the receiving React component.

The optional browser check uses a built UI, Node 22+, an installed Chromium and the
real `uma-sim-api` binary. From the repository root:

```bash
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_LOG_CHROMIUM=chromium node packages/uma-sim-ui/test/career-log-browser.mjs
```

The check starts isolated loopback servers, its own browser profile and temporary
career/checkpoint state, then removes its state after finishing. Set
`UMA_LOG_API_BINARY`, `UMA_LOG_PROFILE_ROOT`, `UMA_LOG_STATE_ROOT` or
`UMA_LOG_EVIDENCE_DIR` for an alternate binary, writable browser profile parent,
temporary career state parent or evidence folder. It never connects to
an existing uma-sim server. See [the receiving record](../../verification/career-log/README.md)
for the exact source baseline, comparison and browser results.
