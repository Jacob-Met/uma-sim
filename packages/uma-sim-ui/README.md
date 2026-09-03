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
