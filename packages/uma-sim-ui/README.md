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

## Session behavior and regression checks

The Run tab identifies the displayed session (`main` is the empty session
ID). Actions and exported telemetry continue to target that career if
another client changes the server's active session. Use Library & lab to
select and load another session. An unavailable selection clears the old
playable view and reports the error.

Switching sessions or choosing New run discards obsolete view responses;
it does not cancel an action that the server has already accepted. The
usual busy controls and a synchronous request guard prevent duplicate
run submissions from a repeated click or key press.

`npm test` exercises the real React store and App with a deterministic
in-memory HTTP fixture and deferred responses. It covers main/named
session targeting, competing response completion, errors, reset, duplicate
input, visible choice shortcuts and telemetry export. The fixture is not
live gameplay evidence. `npm run build` runs these tests before the
TypeScript/Vite build, including in the existing UI CI job.
