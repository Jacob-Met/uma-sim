# uma-sim

Unofficial Umamusume career simulator (Rust) with optional race physics,
embedded **web UI**, CLI, REST API, and MCP wrappers.

## Web UI

Interactive career play in the browser (run setup, turn view, events, races,
Grand Live panel, deck placement, auto/fast-forward).

### From a release zip

Download a platform zip from [Releases](https://github.com/Jacob-Met/uma-sim/releases),
extract, then:

```bash
./uma-sim serve --open
# Windows: uma-sim.exe serve --open
```

Keep `research/`, `knowledge/`, and `content_packs/` next to the binary so the
engine can load catalogs.

### From source (embedded UI)

```bash
cd packages/uma-sim-ui && npm ci && npm run build && cd ../..
cargo build --release --features embed-ui -p uma-sim-core
./target/release/uma-sim serve --open
```

### Dev (hot reload)

Terminal A:

```bash
cargo run -p uma-sim-core --bin uma-sim -- serve --port=8765
```

Terminal B:

```bash
cd packages/uma-sim-ui && npm run dev
```

Open the Vite URL (proxies `/v1` to the API).

## Crates

| Crate | Role |
|-------|------|
| `uma-sim-core` | Career engine, scenarios, scoring, REST/CLI bins + optional embedded UI |
| `uma-race-core` | Clean-room mid-run race physics |

## Quick start (CLI / API)

```bash
cargo build --release -p uma-sim-core
./target/release/uma-sim serve --port 8765
# or interactive CLI:
./target/release/uma-sim
```

Node packages (optional):

```bash
cd packages/uma-sim-cli && npm install && npm run tui
cd packages/uma-sim-mcp && npm install && node server.js
```

## Data layout

- `research/*.json` — formula / calibration constants loaded at runtime
- `knowledge/canonical/by_kind/` — catalogs the engine reads (events, cards, songs, …)
- `content_packs/` — optional event packs

Regenerate catalogs offline (private ingest tooling is not shipped here).
Ship-ready subset is the eight `by_kind` files listed in `docs/SIMULATOR.md`.

## External bot policy (optional)

Set `UMA_POLICY_CMD` to a policy-server binary if you want JVM scoring-shared
parity (`--policy=external`). Without it, the Rust scoring path is used.

## Tests

```bash
cargo test --workspace
python scripts/calibrate_grand_live.py --strict
cd packages/uma-sim-ui && npm run typecheck && npm run build
```

## License

GPL-3.0 — see `LICENSE` and `NOTICE`.
