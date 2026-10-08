# uma-sim

Unofficial, deterministic career simulator for *Umamusume: Pretty Derby* (Global),
written in Rust. It plays the 72-turn career loop (training, events, races,
scenario mechanics, legacy inheritance) from community-researched data, with
frame-stepped race physics for mid-career races.

The same seed always gives the same career, so you can compare decks, policies
or rule changes run against run. You can drive it from:

- a **browser UI** (`uma-sim serve --open`)
- a **CLI** (`uma-sim start / step / fast / batch`)
- a **REST API** on `localhost:8765` (`/v1/*`)
- an **MCP stdio server** for AI agents (`packages/uma-sim-mcp`)

Scenarios: URA Finale (`ura`), Grand Live (`grand_concert`), Unity Cup
(`unity`), Trackblazer (`trackblazer`).

Fan project, not affiliated with Cygames. See [NOTICE](NOTICE) for data sources
and attribution.

## Quick start from source

Needs a stable Rust toolchain, 1.80 or newer (install with [rustup](https://rustup.rs)).
Node 20 is only needed for the browser UI and the Node wrappers.

```bash
git clone https://github.com/Jacob-Met/uma-sim.git
cd uma-sim
cargo build --release -p uma-sim-core

# Play one career to the end with the default policy (seed 42)
./target/release/uma-sim fast --seed=42
```

The last two lines should read:

```text
Ended: career=true turn=72 fans=1217 elapsed=… speed=x20 policy=default
Terminal: U=9.000 grade=F score=770 sp_spent=121 φ=4.50 ψ=4.50
```

`--policy=bot` plays the same seed with the built-in scoring bot and finishes
with a much stronger career (grade C, score 3734 at the time of writing).

### Browser UI

Build the UI once, then embed it into the binary:

```bash
cd packages/uma-sim-ui && npm ci && npm run build && cd ../..
cargo build --release --features embed-ui -p uma-sim-core
./target/release/uma-sim serve --open        # http://127.0.0.1:8765/
```

For UI work with hot reload, run `cargo run -p uma-sim-core --bin uma-sim -- serve`
in one terminal and `npm run dev` in `packages/uma-sim-ui` in another, then
open the Vite URL (it proxies `/v1` to port 8765).

## Install from a release zip

[Releases](https://github.com/Jacob-Met/uma-sim/releases) carry one zip per
platform: the binary (web UI embedded) plus the `research/`, `knowledge/` and
`content_packs/` folders it reads at run time. Download the zip for your
platform, extract it and run:

```bash
./uma-sim serve --open         # Windows: uma-sim.exe serve --open
./uma-sim fast --seed=42
```

| Zip | Platform | Requirement |
|-----|----------|-------------|
| `uma-sim-windows-x64.zip` | Windows 10+ on x86-64 | none |
| `uma-sim-linux-x64.zip` | Linux on x86-64 | glibc 2.28 or newer (Ubuntu 20.04+, Debian 10+, RHEL/Alma/Rocky 8+) |
| `uma-sim-macos-arm64.zip` | macOS on Apple Silicon (M1 or newer) | none. Intel Macs are not supported |

The Linux zip is built inside a `manylinux_2_28` (glibc 2.28) container, and CI
fails the build if the binary needs a newer glibc symbol. Older releases
(v0.2.1 and earlier) were built on Ubuntu 24.04 and need glibc 2.39. Every
zip is unzipped fresh in CI and run through the layout smoke test before it
is published.

Keep the three data folders next to the binary. It finds them there whatever
directory you start it from; set `UMA_REPO_ROOT` if you keep them elsewhere.

Use v0.2.1 or later. The v0.1.0 and v0.2.0 binaries looked for race data at
the path of the CI machine that built them and stop on the first race with
`read /home/runner/work/uma-sim/…/race_course_data.json: No such file or
directory`.

## CLI

Between commands, the current career is saved in `.uma-sim/session.json` in
the working directory.

```bash
# Step through a career yourself
uma-sim start --seed=7 --scenario=unity --trainee="Special Week"
uma-sim state                  # stats, phase, available choices
uma-sim step race              # or train_speed, rest, recreation, event_0, …
uma-sim clear

# Let a policy play whole careers
uma-sim fast --seed=7 --scenario=unity --policy=bot           # one new career, start to finish
uma-sim batch --count=100 --seed=1 --output=out/batch.jsonl   # one JSON line per career
uma-sim batch --seeds=10,5,7 --policy=bot --output=out/rematch.jsonl  # re-run exact seeds (e.g. from analyze --top)
uma-sim analyze --input=out/batch.jsonl                      # aggregate stats: score distribution, grade histogram, top careers
uma-sim analyze --input=new.jsonl --compare=old.jsonl        # delta of new batch vs baseline batch
```

`fast` always starts a new career and replaces the saved one.

For consecutive `batch` runs, `--count=<integer>` must be positive and fit a
signed 64-bit integer; omitting it keeps the default of 100. The complete seed
range, including its final seed, must fit signed 64-bit integers. Invalid,
empty, zero or negative counts and overflowing ranges exit with status 2 before
creating an output directory, opening or replacing the output file, or starting
a career. Consecutive seeds are streamed without allocating the whole range.

The first `--count=` is used when repeated. An explicit `--seeds=` list retains
its existing precedence: its first supplied list determines the ordered careers,
including repeated seeds, and the count is ignored. Valid batches still replace
the chosen output file; keep prior results under a different filename.

`start`, `fast`, `batch`, and `export-telemetry` accept `--seed=<integer>`
as a signed 64-bit seed; omitting it uses 42. These commands reject an empty,
malformed, or out-of-range value with status 2 before a career or output file
changes. Every supplied `--seed=` value must be valid, even when a later seed
or batch's `--seeds=` list overrides it. The last valid `--seed=` wins; a valid
`--seeds=` list still overrides `--seed` and `--count`.

| Flag | Values |
|------|--------|
| `--scenario` | `ura` (default), `grand_concert`, `unity`, `trackblazer` |
| `--policy` | `default`, `bot` (built-in scoring bot), `external` (see below) |
| `--race-model` | `physics` (default), `stub` (legacy parity traces) |
| `--speed` | 1–100; 1–10 prints full dialogue, above 50 runs headless |
| `--dialogue` | `off`, `choices`, `full` |
| `--deck` | support ids, e.g. `support:10001@speed:85,support:10002` |
| `--legacy` | inherited factors, e.g. `factor:blue:1@3` |

`uma-sim validate --path=content_packs/example.json` checks a content pack
(extra events merged in at run time without engine changes). The full command
reference is in [docs/SIMULATOR.md](docs/SIMULATOR.md).

## REST API and MCP

`uma-sim serve [--port=8765]` (or the bare `uma-sim-api [port]` binary) serves:

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/v1/health` | status and the data folder in use |
| GET | `/v1/catalog/{scenarios,trainees,supports,factors}` | catalogs |
| POST | `/v1/run/start` | `{"seed":"42","scenario":"ura","trainee":"Special Week"}` |
| GET | `/v1/run/{state,text,choices,telemetry}` | current career |
| POST | `/v1/run/action` | `{"action":"train_speed"}` |
| POST | `/v1/run/auto`, `/v1/run/fast` | one bot step / play to the end |
| POST | `/v1/run/deck/place`, `/v1/run/style`, `/v1/run/load_content_pack` | setup |
| GET | `/v1/sessions` | list career sessions and the active one |
| POST | `/v1/session/fork` | fork a session from a library checkpoint or another session |
| POST | `/v1/session/close` | close a session |
| POST | `/v1/session/activate` | make a session the active one |

```bash
curl -X POST localhost:8765/v1/run/start -d '{"seed":"42","scenario":"ura"}'
curl localhost:8765/v1/run/choices
curl -X POST localhost:8765/v1/run/fast -d '{}'
```

### Sessions

The server can hold several live careers at once. The legacy single-run
behavior is the default session `""` (labeled `main`). Every `/v1/run/*`
endpoint accepts an explicit `"session"` field in its POST body (or
`?session=` on GET); when omitted, requests target the active session. A
missing session is a 404 (`no such session '<id>'`); with no active run at
all it is a 404 `no active run`.

```bash
# Start a run, then fork an independent continuation to try another policy
curl -X POST localhost:8765/v1/run/start -d '{"seed":"7","scenario":"ura"}'
curl -X POST localhost:8765/v1/session/fork -d '{"id":"try-bot","label":"bot run"}'
curl -X POST localhost:8765/v1/run/fast -d '{"session":"try-bot","policy":"bot","multiplier":"20"}'
curl -X POST localhost:8765/v1/session/activate -d '{"session":""}'
curl localhost:8765/v1/sessions
curl -X POST localhost:8765/v1/session/close -d '{"session":"try-bot"}'
```

- `POST /v1/session/fork` starts a new session from a named library
  checkpoint (`{"checkpoint":"<name>"}`) or from another live session
  (`{"session":"<id>"}`; defaults to the active session). The new session is
  an independent engine: playing it can never mutate the checkpoint or a
  sibling session. Omit `"id"` and the server generates one; `"label"`
  defaults to the id. Duplicate ids are a 409, unknown sources a 404.
- `POST /v1/session/close` takes `{"session":"<id>"}` (400 if missing).
  Closing the active session falls back to the default session id, so the
  client is never left without an active session.
- `GET /v1/sessions` returns
  `{"sessions":[{id,label,turn,phase,careerComplete,seed,scenarioId,traineeName},...],"active":"<id>"}`.

### `/v1/run/fast` failure semantics

`/v1/run/fast` takes an optional `"multiplier"` (string, parsed as an integer
and clamped to 1–100; defaults to the server's speed setting) and `"policy"`
(`default`, `bot`, or `external`; defaults to the server default policy).

Nothing is committed until the run succeeds. If the request fails — an
unknown session (404) or an unreachable external policy server (503) — the
engine is rolled back to its pre-request snapshot and the API-level speed
setting is left unchanged: a failed fast run never changes the speed. A
successful run applies the requested multiplier, so a later `/v1/run/fast`
without `"multiplier"` keeps it.

The Node wrappers use only Node built-ins, so no `npm install` is needed:

```bash
node packages/uma-sim-mcp/mcp-stdio.js           # MCP stdio server: sim_start, sim_act, sim_fast_forward, …
node packages/uma-sim-cli/tui.js 42 ura          # text UI; starts target/release/uma-sim-api if no API is up
node packages/uma-sim-cli/run.js fast --seed=42  # runs the built CLI from the repo root
```

The MCP server talks to `UMA_SIM_API` (default `http://127.0.0.1:8765`), so
start `uma-sim serve` first.

## Repository layout

| Path | What it is |
|------|------------|
| `uma-sim-core/` | Career engine, scenarios, scoring bot, CLI (`uma-sim`) and REST (`uma-sim-api`) binaries |
| `uma-race-core/` | Clean-room frame-stepped race physics |
| `research/` | Formula and calibration tables loaded at run time, plus research notes |
| `knowledge/canonical/by_kind/` | Game catalogs (events, skills, supports, trainees, races, …) |
| `content_packs/` | Optional event packs |
| `packages/` | Web UI (Vite + React), TUI/CLI wrapper, MCP server |
| `docs/` | Design, parity and race-model notes |

The catalogs are generated offline from community sources; the ingest tooling
is not part of this repository. `python knowledge/validate/validate.py` checks
the shipped catalogs.

## Environment variables

| Variable | Effect |
|----------|--------|
| `UMA_REPO_ROOT` | Folder containing `research/` and `knowledge/`. Default search: working directory and its parents, then the binary's folder |
| `UMA_RACE_MODEL` | `physics` or `stub`, same as `--race-model` |
| `UMA_POLICY_CMD` | Command for an external policy server, used by `--policy=external` |
| `UMA_SIM_API` | API base URL for the Node wrappers |

## Tests

```bash
cargo test --workspace                          # about 300 tests, including 200 golden seeds
python scripts/calibrate_grand_live.py --strict
cd packages/uma-sim-ui && npm run typecheck && npm run build
```

CI also stages a release-layout folder and runs
`scripts/smoke_release_layout.sh` on it. The script hides the source tree's
data folders and starts the binary from an unrelated directory, so a data path
baked in at compile time fails the build instead of shipping.

## License

GPL-3.0: the scoring bot ports GPL-3.0 code from
[uma-android-automation](https://github.com/steve1316/uma-android-automation).
See [LICENSE](LICENSE) and [NOTICE](NOTICE).
