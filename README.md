# uma-sim

Unofficial Umamusume career simulator (Rust) with optional race physics,
CLI, REST API, and MCP wrappers.

## Crates

| Crate | Role |
|-------|------|
| `uma-sim-core` | Career engine, scenarios, scoring, REST/CLI bins |
| `uma-race-core` | Clean-room mid-run race physics |

## Quick start

```bash
cargo build --release -p uma-sim-core
./target/release/uma-sim serve --port 8765
# or interactive CLI:
./target/release/uma-sim
```

Node packages (optional):

```bash
cd packages/uma-sim-cli && npm install && node tui.js
cd packages/uma-sim-mcp && npm install && node server.js
```

## Data layout

- `research/*.json` â€” formula / calibration constants loaded at runtime
- `knowledge/canonical/by_kind/` â€” catalogs the engine reads (events, cards, songs, â€¦)
- `content_packs/` â€” optional event packs

Regenerate catalogs offline (private ingest tooling is not shipped here).
Ship-ready subset is the eight `by_kind` files listed in `docs/SIMULATOR.md`.

## External bot policy (optional)

Set `UMA_POLICY_CMD` to a policy-server binary if you want JVM scoring-shared
parity (`--policy=external`). Without it, the Rust scoring path is used.

## Tests

```bash
cargo test --workspace
python scripts/calibrate_grand_live.py --strict
```

## License

GPL-3.0 â€” see `LICENSE` and `NOTICE`.
