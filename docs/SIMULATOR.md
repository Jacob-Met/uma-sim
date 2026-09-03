# Uma Musume Offline Simulator

Deterministic, seedable, text-first career engine for Global scenarios.
**Authoritative engine:** Rust crate `uma-sim-core/` (`uma-sim` CLI + `uma-sim-api` REST).

## Status

| Phase | Status |
|-------|--------|
| 0 — KB + research | **Done** — canonical ingest + `research/*.json` |
| 1–5 — Engine | **Done in Rust** — 72-turn loop, training/events, scenarios, legacies; **R8 race physics default** |
| 6 — Bot harness | **Done** — adapter + golden seeds (200, re-frozen R8.8) + telemetry replay (≥90%) |
| 7 — REST/MCP | **Done** — Rust REST `:8765` + MCP/TUI + **embedded web UI** |
| 8 — Perf | **Done** — `tests/perf.rs` x20 / x100 gates |
| Calibration / GL fidelity | **R7.2 Grand Live complete** — evidence table below |
| R7 scenario / race / legacy fidelity | **Advanced** — TB/Unity/URA + inheritance; mid-run races use `uma-race-core` physics (R8) |
| R8 race physics | **Default `physics` (R8.8)** — see `docs/SIMULATOR_RACE_PLAN.md`, `research/R88_REFREEZE.md` |
| Live Android telemetry calibration | **Pass (sim proxy)** — `python scripts/calibrate_sim.py --runs-dir runs` median≤2, ≥80% within 2 |

## Product-ready checklist

| Criterion | Evidence |
|-----------|----------|
| Any scenario completes 72 turns | `tests/product_readiness.rs` |
| Same seed → same outcome | `tests/golden_seeds.rs` (200 fixtures) |
| x20 career &lt;10s / x100 &lt;3s | `tests/perf.rs` |
| Dialogue modes + speed 1–100 | CLI `--dialogue=`, `--speed=` |
| Bot ≥90% on replay fixtures | `tests/bot_parity.rs`, `telemetry_replay.rs`, `live_bot_telemetry.rs` |
| REST + MCP | `cargo run --bin uma-sim-api`, `packages/uma-sim-mcp` |
| Content packs without engine rewrite | `validate` CLI, REST `load_content_pack` |
| Live Android telemetry calibration | **Pass (sim export proxy)** — `calibrate_sim.py --runs-dir runs` |
| Kotlin → Rust test map | [SIMULATOR_RUST_PARITY.md](SIMULATOR_RUST_PARITY.md) (133 `@Test`, 0 missing) |

## Platforms (desktop-first)

| Target | Role | Status |
|--------|------|--------|
| **Rust (desktop)** | Primary — CLI, REST, goldens, perf | **Working** |
| **Node/JS** | MCP stdio bridge + TUI | **Working** (talks to Rust API) |
| **Android** | Bot uses `scoring-shared` only; sim is offline | Partial |

```powershell
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- start --seed=42 --policy=bot
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- serve --port=8765 --open
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim-api -- 8765
```

Web UI (dev): build with `--features embed-ui` after `packages/uma-sim-ui` `npm run build`, or run Vite (`npm run dev`) against `serve`.
## Quick start

```powershell
python knowledge/validate/validate.py

cd uma-sim-core
cargo test --tests --lib          # ≥143 tests

..\scripts\sim-harness.ps1 -Mode all

cd ..\packages\uma-sim-cli
npm run tui                       # sidebar TUI (starts Rust API if needed)

# Full CLI flags
cargo run --manifest-path ..\uma-sim-core\Cargo.toml --bin uma-sim -- start --seed=42 --scenario=grand_concert --speed=1 --dialogue=full --deck=support:10001,support:10002 --legacy=factor:blue:1@3 --policy=bot --trace-rng
cargo run --manifest-path ..\uma-sim-core\Cargo.toml --bin uma-sim -- fast --seed=42 --speed=100 --policy=bot
cargo run --manifest-path ..\uma-sim-core\Cargo.toml --bin uma-sim -- validate --path=content_packs/example.json
```

## Speed multiplier

Integer **1–100**. Presets: 1, 2, 5, 10, 20, 50, 100.

- **x1–10:** interactive text (`--dialogue=full`)
- **x11–50:** fast (suppress non-choice dialogue unless `--allow-dialogue-at-high-speed`)
- **x51–100:** turbo headless

## CLI reference

| Command | Description |
|---------|-------------|
| `start` | New career; persists to `.uma-sim/session.json` |
| `state` | Print stats, phase, deck, legacy |
| `step [action]` | Single action (`train_speed`, `rest`, `race`, `event_0`) |
| `fast` | Auto-play to completion (default speed 20 if unset) |
| `export-telemetry` | Write Android-shaped JSONL under `runs/sim-telemetry/` |
| `validate` | Validate a content pack JSON file |
| `deck place` | Reposition a support onto a facility |
| `serve` | Start REST API (+ embedded UI when built with `embed-ui`) on `--port=` (default 8765); `--open` launches the browser |
| `clear` | Clear saved session |

Flags: `--seed`, `--scenario`, `--trainee`, `--speed=1-100`, `--dialogue=off|choices|full`, `--deck=id1,id2`, `--legacy=factor:...`, `--policy=default|bot|external`, `--race-model=stub|physics` (default **physics**), `--trace-rng`, `--trace-telemetry`, `--output=`. Env: `UMA_RACE_MODEL`.

## Mid-run races (R8)

| Item | Status | Evidence |
|------|--------|----------|
| Frame-stepped multi-horse physics | **true** | `uma-race-core`; oracle case1/case3/Virtual exact |
| Default `race_model=physics` | **true** (R8.8) | `RaceModel::default()`, `research/R88_REFREEZE.md` |
| Stub opt-in for legacy parity | **true** | `--race-model=stub`; Kotlin rng/turn traces pinned stub |
| Career seed → race seed, zero career-RNG draws | **true** | `derive_race_seed`; race uses own Prando |
| Genuine finish order (not win-by-default) | **true** | `tests/race_physics_r87.rs` |
| 200 golden summaries | **re-frozen** | `uma-sim-core/tests/fixtures/kotlin/golden/summaries.json` |
| Checkpoint sample | **16/16 ≤1 frame**; V3 **103/119 (~86.6%)** | `checkpoint_r85`, `checkpoint_v3` |
| Lead competition | **true** (Nige/Oonige) | `research/race_lead_competition.json`; cp_10 exact |
| NPC field | **bootstrap soft-cal** + V4 physics dist | `race_field_npc.json`, `race_v4_physics_dist.json`, `race_npc_r86` |
| Race perf ≤10ms / 18-horse | **true** (release p50≈1.3ms) | `cargo test -p uma-sim-core --release --test race_perf_r89` |

### External policy (optional)

Set ``UMA_POLICY_CMD`` to an external policy-server binary for JVM scoring parity
(``--policy=external``). Without it, Rust ``src/scoring/`` is used
(``--policy=bot`` / ``--policy=default``).

```powershell
$env:UMA_POLICY_CMD = "path\to\policy-server.bat"   # optional
cargo run -p uma-sim-core --bin uma-sim -- fast --seed=42 --policy=external
```

**Throughput (measured 2026-09-02, warm JVM, URA seed 42):** ~130 turns/sec engine-reported (~72 turns in ~0.55s); wall-clock for a single `fast` CLI career after warm start ≈ 1.1–1.8s including process overhead. Cold start pays an extra ~8–12s for JVM boot on first `ping`.

**Batch terminal eval (Phase 5):** `uma-sim batch --count=100 --policy=default` → **~2780 careers/min** with non-zero `U` on every record; `--policy=external` sample → **~460 careers/min** (above the ~100/min fitting floor). Output JSONL fields: `u`, `phi_blue`, `psi_grade`, `grade`, `score`, `sp_spent`, `brackets`.

**Fitted value function (Phase 7):** optional weights via `UMA_VALUE_WEIGHTS` / CEM (`python scripts/fit_value_function.py`). A/B over 200 matched seeds (`--policy=external`): baseline mean U **11.10** 95%CI [10.87, 11.32]; fitted **12.71** [12.63, 12.79]; delta **+1.61** [1.37, 1.86].

Rust `src/scoring/` remains the world-model / fixture math (including `terminal_utility.rs` + `estimate_rank` at career end + a flat 3.3 pts/SP skill-shop stub).

## REST API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/v1/health` | `{ ok, version, repoRoot }` |
| GET | `/v1/catalog/scenarios` | Compact scenario list |
| GET | `/v1/catalog/trainees` | Compact trainee list |
| GET | `/v1/catalog/supports` | Compact support list (`id`, `name`, `type`, `rarity`) |
| GET | `/v1/catalog/factors` | Compact factor list (`id`, `name`, `kind`) |
| POST | `/v1/run/start` | `{ seed, scenario, trainee, speed, deckSupports, legacyFactors, dialogue, raceModel, policy, traceTelemetry }` |
| GET | `/v1/run/state` | Full run snapshot JSON |
| GET | `/v1/run/text` | Rendered text |
| GET | `/v1/run/choices` | Available actions |
| POST | `/v1/run/action` | `{ action }` → `{ text, careerEnded, state, choices }` |
| POST | `/v1/run/auto` | `{ policy }` → `{ text, careerEnded, state, choices }` |
| POST | `/v1/run/fast` | `{ multiplier, policy }` |
| GET | `/v1/run/telemetry` | Turn telemetry JSON |
| POST | `/v1/run/deck/place` | `{ supportId, facility }` |
| POST | `/v1/run/load_content_pack` | `{ path: "content_packs/example.json" }` |
| GET | `/` (+ SPA assets) | Embedded web UI when built with `--features embed-ui` |

## MCP bridge

```powershell
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim-api
cd packages/uma-sim-mcp; npm run mcp
```

Tools: `sim_start`, `sim_state`, `sim_choices`, `sim_act`, `sim_auto`, `sim_fast_forward`, `sim_export_telemetry`, `sim_load_content_pack`, `sim_deck_place`.

## Calibration

```powershell
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- export-telemetry --seed=42 --scenario=ura --policy=bot
python scripts/calibrate_sim.py --telemetry runs/sim-telemetry/sim-42-ura.jsonl
python scripts/calibrate_sim.py --stub
python scripts/calibrate_grand_live.py
python scripts/calibrate_grand_live.py --strict
```

## Grand Live / URA / Unity / Trackblazer

Mechanics live in `uma-sim-core/src/scenario/`. Research sources: `research/grand_concert.json`, `ura_finale.json`, `unity_cup.json`, `trackblazer.json`.

### Grand Live — R7.2 complete

| Checklist item | Status | Evidence |
|----------------|--------|----------|
| `not_modeled` emptied or parity-covered | **true** | Mid-run race physics shipped (R8); residual GL approximations listed in `research/grand_concert.json` |
| Token gains exact types+amounts (`--strict`) | **true** | `python scripts/calibrate_grand_live.py --strict` + `tests/grand_live_token_labels.rs` |
| Bot adapter ≥90% on GL fixtures | **true** | `tests/grand_live_bot_replay.rs` + `tests/fixtures/grand_live_replay/fixtures.json` |
| Great Success / normal / consolation | **true** | `tests/grand_live_r7.rs` + `tests/grand_live_simulation.rs` |
| Lesson board / blocked / failure / members / fan uniques / dating | **true** (MDB square weights still approximate; board includes unaffordable slots) | `grand_live.rs`, `grand_live_r7.rs`; status in `research/grand_concert.json` |

Quick check:

```powershell
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- start --seed=42 --scenario=grand_concert --dialogue=full
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- step gl_song_3
cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim -- step train_speed
cargo test --manifest-path uma-sim-core/Cargo.toml --test grand_live_r7 --test grand_live_bot_replay --test grand_live_simulation
```

### Trackblazer / Unity / URA (R7.3–R7.5)

| Scenario | Status | Evidence |
|----------|--------|----------|
| **Trackblazer** VP + shop sales + consumable effects | **true** (climax_2 / rivals / epithets / inventory still `not_modeled` + parity tests) | `tests/unity_trackblazer_mechanics.rs` (`trackblazer_climax_awards_victory_points`, `trackblazer_shop_sale_*`, `trackblazer_speed_charm_*`); `research/trackblazer.json` |
| **Unity Cup** abstracted 5-leg wins + Zenith + Ignited Spirit hints | **true** (per-leg sim / teammate share still `not_modeled` + parity tests) | `unity_team_race_win_counts_five_legs_and_zenith`, `unity_extreme_burst_grants_ignited_spirit_hint`; `research/unity_cup.json` |
| **URA** Racing Spirit hints, bad-odds accept, Past My Limits | **true** (finale distance/surface still `not_modeled` + parity test) | `duel_win_grants_racing_spirit_hint`, `duel_accepts_bad_odds_when_failure_within_pct`, `max_level_meek_win_unlocks_past_my_limits`; `research/ura_finale.json` |

### Legacies / race outcomes (R7.6–R7.7)

| Item | Status | Evidence |
|------|--------|----------|
| Inheritance choice 0 (skills) + choice 1 (spark stats +20/+20) | **true** | `tests/legacy_applicator.rs::inheritance_choice_1_boosts_spark_stats` |
| Pink aptitude ★-sum rank-ups (cap A) + blue start 5/12/21 | **true** | `legacy.rs` + `inheritance_planners.md` / GameTora |
| Race place/show multipliers (Outcomes v1) | **true** | `tests/race_outcomes.rs`; physics placements feed same multipliers |
| Epithet stubs on G1 / climax / finale wins | **true** | `race_outcomes.rs`; granted into `CareerState.statuses` |
| Mid-run race physics (`uma-race-core`) | **true** (default) | R8 plan; SmartRaceSolver gap retired for career mid-run |

Other scenario tests: `ura_mechanics.rs`, `unity_trackblazer_mechanics.rs`. Full map: [SIMULATOR_RUST_PARITY.md](SIMULATOR_RUST_PARITY.md).

## §7 Definition of done (Rust plan)

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `cargo test` ≥127, 0 fail, 0 ignored | **true** | `cargo test --tests --lib` (green; 0 ignored) |
| 2 | 200 golden summaries match Kotlin | **true** | `tests/golden_seeds.rs` |
| 3 | scoring / event_parse / rng_trace / turn_trace fixtures | **true** | `tests/parity.rs` |
| 4 | Bot ≥90% training, event, GL replay | **true** | `bot_parity.rs`, `telemetry_replay.rs`, `grand_live_bot_replay.rs` |
| 5 | CLI / REST / MCP / TUI on Rust + seed-42 fixtures | **true** | `tests/fixtures/cli_rest/`, `packages/uma-sim-cli`, `packages/uma-sim-mcp` |
| 6 | Production path is Rust-only | **true** | this doc; Kotlin oracle private / fixtures vendored |
| 7 | `calibrate_sim.py` passes | **true** | `--runs-dir runs` → pass (median≤2, ≥80% within 2) |
| 8 | Grand Live checklist R7.2 fully true | **true** | section above; user notified |
| 9 | Product-ready checklist complete with citations | **true** | this document |
| 10 | Kotlin→Rust test map, 0 missing | **true** | [SIMULATOR_RUST_PARITY.md](SIMULATOR_RUST_PARITY.md) (133 / 0 missing) |

Remaining intentional gaps (parity-covered, not blockers): R8.5 full V3 checkpoint + R8.6 telemetry NPC V4 still open; TB rivals / epithet catalog / inventory / climax_2; Unity per-leg + teammate share; URA finale distance from history.

## Architecture

```
knowledge/canonical/  →  uma-sim-core (Rust)  →  CLI / REST / MCP
         ↓
   research/*.json
```

Optional: set `UMA_POLICY_CMD` for an external JVM scoring policy.

## Parity fixtures

Golden / telemetry / RNG / turn-trace fixtures ship under `uma-sim-core/tests/fixtures/`
(including a vendored Kotlin-resource subset at `tests/fixtures/kotlin/`).
The historical Kotlin oracle used to regenerate them is not part of this repository.

## Content packs

Add JSON under `content_packs/` (see `example.json`). Validate with `uma-sim validate --path=...`. Release workflow: `knowledge/RELEASE.md`.
