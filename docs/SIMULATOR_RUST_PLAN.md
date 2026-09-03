# Uma Musume Offline Simulator — Rust Port & Completion Plan

**Status:** active. Supersedes the engine-language decision in `docs/SIMULATOR_PLAN.md`.
`SIMULATOR_PLAN.md` remains authoritative for **game-mechanics scope and fidelity targets**; this
document is authoritative for **language, architecture, sequencing, and exit gates**.

---

## 1. Decision

The authoritative simulator engine is the Rust crate **`uma-sim-core/`**.

Parity fixtures under `uma-sim-core/tests/fixtures/` (including vendored Kotlin resources) are the
contract for RNG / turn-trace / golden gates. Optional external policy via `UMA_POLICY_CMD`.

Rationale: the product is an offline simulator whose surfaces are already Python (calibration) and
TypeScript (CLI/TUI/MCP). Rust gives one native binary, a clean C ABI for the TS and Python layers, and
deterministic float/RNG behaviour that makes golden-seed parity provable.

**Non-goals of the port:** rewriting `packages/uma-sim-cli`, `packages/uma-sim-mcp`,
`scripts/*.py`, `knowledge/canonical/`, or `research/*.json`. Those are kept as-is and re-pointed at
the Rust core.

---

## 2. Measured current state

Counts taken from the working tree, not estimates.

| Codebase | Files | Lines |
|---|---|---|
| Kotlin `sim-engine` commonMain | 45 | ~4,430 |
| Kotlin `sim-engine` jvmMain | 11 | ~1,158 |
| Kotlin `sim-engine` tests (common+jvm) | 34 | ~2,250 |
| Kotlin `scoring-shared` (src+test) | 18 | ~2,580 |
| **Rust `uma-sim-core`** | **17** | **~3,630** |

Content assets (shared, not ported): `research/` 16 JSON files;
`knowledge/canonical/by_kind/` 18 JSON files (~11 MB, 9,377 entities, `validate.py` green).

Kotlin test suite: **127 tests green** (`.\gradlew.bat :sim-engine:jvmTest`).

### 2.1 Golden-seed parity today (seed 42, `defaultAutoPolicy`, speed x50)

| Scenario | Kotlin fixture (turn/fans/speed/sp) | Rust | Parity |
|---|---|---|---|
| `ura` | 72 / 4139 / 141 / 181 | 72 / 4139 / 141 / 181 | **match** |
| `grand_concert` | 72 / 4623 / 206 / 226 | 72 / 4623 / 206 / 226 | **match** |
| `unity` | 72 / 2934 / 164 / 136 | 72 / 2934 / 164 / 136 | **match** |
| `trackblazer` | 72 / 3061 / 171 / 136 | 72 / 2930 / 164 / 136 | **DIVERGES** |

Rust `SimRandom` is byte-exact against Kotlin 2.0.21 `XorWowRandom` (raw ints, doubles,
`nextInt(until)`, and `restore()`), verified by `uma-sim-core/tests/fixtures/rng_seed_42.json`
generated from `RngParityExportTest.kt`.

---

## 3. Port matrix

Legend: **DONE** = ported and parity-checked · **PARTIAL** = ported with known gaps ·
**TODO** = not started.

### 3.1 `sim-engine/commonMain` → `uma-sim-core/src`

| Kotlin | Rust target | State | Known gap |
|---|---|---|---|
| `SimRandom.kt` | `rng.rs` | **DONE** | — |
| `CareerState.kt`, `SimAction.kt`, `ScenarioResources.kt`, `InjuryStatus.kt` | `state.rs` | **DONE** | — |
| `TurnCalendar.kt` | `calendar.rs` | **DONE** | — |
| `defaultAutoPolicy` (in `SimEngine.kt`) | `policy.rs` | **DONE** | — |
| `ScenarioPlugin.kt` | `scenario/mod.rs` | **DONE** | — |
| `UraMechanics.kt` | `scenario/ura.rs` | **DONE** | replace `static mut` with `OnceLock`/`Mutex` |
| `UnityCupMechanics.kt` | `scenario/unity.rs` | **DONE** | — |
| `SimEngine.kt` | `engine.rs` | **PARTIAL** | no telemetry, no snapshot/restore, no content packs, no injected event catalog, no `assignDeckSlot` |
| `TrackblazerMechanics.kt` | `scenario/trackblazer.rs` | **PARTIAL** | seed-42 divergence (shop RNG / offer order) |
| `GrandLiveMechanics.kt` (414 ln) | `scenario/grand_live.rs` (189 ln) | **PARTIAL** | missing research/community JSON load, calibration hook, friendship bias, Light Hello, technique gates, cycle bonuses, `daysToConcert`, `tokenTotalsForBot` |
| `GrandConcertScenarioPlugin.kt` | `scenario/mod.rs` | **PARTIAL** | no lesson board, no song/technique purchase, no cycle-bonus activation, no consolation/perfect paths |
| `TrainingResolver.kt` | `training.rs` | **PARTIAL** | ignores deck support slices, `research/training_gain_tables.json`, sub-stat config, growth % |
| `TrainingFailureConfig.kt`, `EventProbabilityConfig.kt`, `RaceOutcomeConfig.kt`, `HintProgressionConfig.kt`, `BondGainConfig.kt`, `FacilityLevelConfig.kt`, `InspirationConfig.kt` | `config.rs` | **PARTIAL** | values hardcoded; no `research/*.json` loading |
| `EventEffectApplier.kt`, `EventCatalog.kt` | `events.rs` | **PARTIAL** | hand-rolled text parser; must match Kotlin regex semantics exactly |
| `LegacyApplicator.kt`, `LegacyDeckConfig.kt`, `LegacyFactorContext.kt` | `legacy.rs` | **PARTIAL** | no factor-catalog lookup, no blue/skill spark caps, no inherited skills |
| `BotDecisionAdapter.kt` | `bot.rs` | **TODO** | blocks all bot-parity work |
| `TrainingPreview.kt`, `TrainingGainContext.kt` | `training.rs` | **TODO** | |
| `DeckSpec.kt`, `DeckPlacement.kt`, `DeckSupportBridge.kt`, `DeckTrainingSignals.kt` | `deck.rs` | **TODO** | |
| `GrandLiveCatalog.kt`, `GrandLiveLessonBoard.kt`, `GrandLiveLessonScoring.kt`, `GrandLiveDeckSupport.kt` | `scenario/grand_live_*.rs` | **TODO** | |
| `SimTelemetry.kt`, `TelemetryReplayLoader.kt` | `telemetry.rs` | **TODO** | must emit Android-shaped JSONL for `scripts/calibrate_sim.py` |
| `TextRenderer.kt` | `render.rs` | **TODO** | dialogue modes off/choices/full |
| `RaceScheduler.kt`, `RunSnapshot.kt` | `race.rs`, `snapshot.rs` | **TODO** | |
| `ScenarioResearchConfig.kt`, `ContentPackRegistry.kt` | `content.rs` | **TODO** | |
| `GoldenSeedFixtures.kt` | `tests/golden_seeds.rs` | **PARTIAL** | 200-fixture gate `#[ignore]` |

### 3.2 `sim-engine/jvmMain` → Rust

All **TODO**. Rust has no filesystem/catalog/serving layer yet.

| Kotlin | Rust target |
|---|---|
| `EngineFactory.kt` | `factory.rs` (repo-root detect + research/KB load) |
| `TraineeCatalog.kt`, `SupportCatalog.kt`, `FactorCatalog.kt` | `catalog/*.rs` (serde over `knowledge/canonical/`) |
| `GrandLiveCatalogLoader.kt`, `GrandLiveCalibrationLoader.kt` | `scenario/grand_live_catalog.rs` |
| `FileEventCatalog.kt`, `ContentPackLoader.kt` | `content.rs` |
| `RunSession.kt` | `session.rs` (`.uma-sim/session.json`) |
| `SimCliMain.kt` | `src/bin/uma-sim.rs` |
| `SimApiServer.kt` | `src/bin/uma-sim-api.rs` (REST `:8765`) |

### 3.3 `scoring-shared` → `uma-sim-core/src/scoring/`

The hard part. Only `FormulaGain.kt` + `softCapEffectivenessMultiplier` are partially ported into
`scoring.rs`.

| Kotlin | Lines | Rust target | State |
|---|---|---|---|
| `FormulaGain.kt` | 97 | `scoring/formula_gain.rs` | **PARTIAL** |
| `Scoring.kt` | 386 | `scoring/training.rs` | **TODO** |
| `EventScoring.kt` | 350 | `scoring/event.rs` | **TODO** |
| `RankEstimate.kt` | 341 | `scoring/rank.rs` | **TODO** |
| `SkillScoring.kt` | 273 | `scoring/skill.rs` | **TODO** |
| `Types.kt` | 243 | `scoring/types.rs` | **TODO** |
| `SupportEffects.kt` | 137 | `scoring/support.rs` | **TODO** |
| `LessonScoring.kt` | 110 | `scoring/lesson.rs` | **TODO** |
| `ObjectiveProfiles.kt` | 80 | `scoring/objectives.rs` | **TODO** |
| `DecisionContext.kt` | 57 | `scoring/context.rs` | **TODO** |

Kotlin `scoring-shared` keeps its own tests; the Android bot keeps consuming the Kotlin module
until §7 gate 6. Rust must reproduce the same numbers, verified by exported fixtures (§4, R0).

---

## 4. Phases

Each phase has a **hard exit gate**. Do not start phase N+1 until phase N's gate passes.
Regenerate golden/fixture files only when a deliberate mechanics change justifies it, and say so.

### R0 — Parity harness (foundation)

Build the machinery that makes every later phase provable.

1. Extend `RngParityExportTest.kt` into a general **fixture exporter** in Kotlin that writes, to
   `uma-sim-core/tests/fixtures/`:
   - `rng_seed_42.json` (exists)
   - `rng_trace_<seed>.json` — full ordered RNG call trace (`SimSettings.traceRng`) for seeds
     1, 42, 7 × 4 scenarios
   - `scoring_vectors.json` — inputs+outputs for every public `scoring-shared` function
     (`calculateRawTrainingScore`, `scoreEventOption`, `parseEventRewardText`,
     `sampleEventReward`, `softCapEffectivenessMultiplier`, `applyTrainingMultipliers`,
     skill/lesson/rank scorers), ≥50 vectors per function spanning edge cases
   - `event_parse_vectors.json` — every distinct option text reachable from
     `BuiltinEventCatalog` + `knowledge/canonical/by_kind/event_local.json` sample of 500,
     with the parsed `EventEffectReading`
   - `turn_trace_<seed>_<scenario>.json` — per-turn state snapshots (stats, energy, mood, fans,
     sp, phase, scenario resources) for the same seeds
2. Add `cargo test --test parity` consuming all of the above.
3. Add `scripts/parity.ps1` that runs Kotlin export → `cargo test` → prints a matrix.

**Gate R0:** `scripts/parity.ps1` runs clean; `rng_trace` tests pass for all 12 seed×scenario
combinations; `turn_trace` tests exist and report per-turn first-divergence turn number for any
mismatch (they may still fail at this stage — the harness must *detect* divergence, not hide it).

### R1 — Config, content, and catalog layer

Rust must read the same JSON the Kotlin engine reads. No more hardcoded constants.

1. `content.rs` + `factory.rs`: repo-root detection, load all 16 `research/*.json`.
2. Rewrite `config.rs` so every constant is JSON-driven with the Kotlin defaults as fallback.
3. `catalog/`: `trainee.rs`, `support.rs`, `factor.rs`, `event.rs` over
   `knowledge/canonical/by_kind/*.json` with serde + lazy indices. Must reproduce
   `TraineeCatalogTest` / `SupportCatalogTest` / `TraineeCatalog` char-vs-card-name behaviour.
4. `scenario/grand_live_catalog.rs`: songs, techniques, costs, calibration table.
5. Port `KnowledgeValidateTest` as a Rust test that shells `knowledge/validate/validate.py` and
   asserts 9,377 entities / OK.

**Gate R1:** `cargo test` green; a Rust test asserts each of the 16 `research/*.json` files is
loaded and at least one non-default value from each is observable in engine behaviour; catalog tests
match the Kotlin catalog tests' assertions.

### R2 — `scoring-shared` port

Port all 10 files in §3.3 into `uma-sim-core/src/scoring/`. Keep Kotlin function names as Rust
`snake_case` equivalents and keep the module layout 1:1 so review is mechanical.

Order: `types.rs` → `context.rs` → `objectives.rs` → `formula_gain.rs` → `support.rs` →
`training.rs` → `event.rs` → `skill.rs` → `lesson.rs` → `rank.rs`.

Event text parsing must use the `regex` crate with patterns transcribed from `EventScoring.kt`
(`ENERGY_LINE`, `MOOD_LINE`, `STAT_LINE`, `RANDOM_STAT`, `ALL_STATS`, `SKILL_PTS`, `BOND_LINE`,
`HINT_NAMED`, `PERF_TOKEN`), including `splitRandomBranches` and `averageReadings` semantics.
Replace the hand-rolled parser in `events.rs`.

**Gate R2:** every vector in `scoring_vectors.json` and `event_parse_vectors.json` passes exactly
(integers equal; floats within 1e-9). No `#[ignore]`.

### R3 — Engine completion

1. `deck.rs`: `DeckSpec` parsing (`support:10001@speed:85`), placement, facility caps,
   `DeckSupportBridge` slices, `DeckTrainingSignals`.
2. `training.rs`: full `TrainingResolver` — research tables, deck slices, growth %, sub-stat
   config, `TrainingPreview`.
3. `legacy.rs`: factor-catalog lookup, blue/pink/race/skill sparks, spark caps, inherited skills,
   inheritance event + `auto_policy`.
4. `engine.rs`: telemetry hooks, `RunSnapshot` export/restore (RNG call-count restore),
   injected event catalog, content packs, `assign_deck_slot`, side actions.
5. `scenario/grand_live*.rs`: complete `GrandLiveMechanics` (all 414 lines of behaviour),
   lesson board, lesson scoring, song/technique purchase, cycle bonuses, concert paths.
6. `scenario/trackblazer.rs`: fix the seed-42 divergence.
7. `race.rs`, `render.rs`, `snapshot.rs`, `telemetry.rs`.
8. `bot.rs`: `BotDecisionAdapter` + `bot_scoring_policy`.

**Gate R3 (the big one):**
- `turn_trace` parity passes for all 12 seed×scenario combinations, turn-for-turn.
- `tests/golden_seeds.rs::all_kotlin_golden_summaries` passes with `#[ignore]` **removed** — all
  200 fixtures (50 seeds × 4 scenarios) match Kotlin exactly.
- A Rust test reproduces `BotParityTest` and `TelemetryReplayTest` assertions at **≥90%** match.

### R4 — Test-suite port

Port all 127 Kotlin tests to Rust, preserving names so the mapping is auditable. Include
`GrandLiveSimulationTest` (450 lines, 24 tests), `ProductReadinessTest`, `PerfTest`,
`ScenarioDepthTest`, `UraMechanicsTest`, `UnityTrackblazerMechanicsTest`, `DeckPlacementTest`,
`LegacyApplicatorTest`, `SoftCapTest`, the config tests, and the loader tests.

**Gate R4:** `cargo test` reports **≥127 tests, 0 failures, 0 ignored**. A checklist in
`docs/SIMULATOR_RUST_PARITY.md` maps every Kotlin test to its Rust counterpart with no gaps.

### R5 — Interfaces

1. `src/bin/uma-sim.rs` — full CLI parity with `SimCliMain.kt`: `start`, `state`, `step`, `fast`,
   `export-telemetry`, `validate`, `content validate`, `deck place`, `serve`, `clear`; flags
   `--seed --scenario --trainee --speed=1-100 --dialogue=off|choices|full --deck --legacy
   --policy=default|bot --trace-rng --trace-telemetry --output`.
2. `src/bin/uma-sim-api.rs` — REST parity with `SimApiServer.kt` on `:8765`, same routes and JSON
   shapes (`/v1/run/start`, `/state`, `/choices`, `/act`, `/auto`, `/telemetry`, …).
3. `cdylib` + C ABI (`ffi.rs`) so `packages/uma-sim-cli` and `packages/uma-sim-mcp` can call the
   core directly; keep the REST path as the default transport so the TS layer needs no rewrite.
4. Re-point `packages/uma-sim-cli` (incl. `tui.js`) and `packages/uma-sim-mcp` at the Rust binary.
   TUI polish: legacy/deck in sidebar.
5. `content_packs/` loading through the Rust CLI.

**Gate R5:** every documented CLI command and REST route responds identically to the Kotlin
version for seed 42 (captured as fixture files); `npm run tui` works against the Rust binary;
MCP tools `sim_start`/`sim_state`/`sim_choices`/`sim_act`/`sim_auto`/`sim_fast_forward`/
`sim_export_telemetry`/`sim_load_content_pack`/`sim_deck_place` all succeed.

### R6 — Kotlin decommission

1. Move `uma-sim/` to `legacy/uma-sim-kotlin/` (keep the parity exporter runnable). **Done.**
2. **Choice recorded:** keep `scoring-shared` as the Android bot's only Kotlin dependency (no FFI
   this turn). `legacy/uma-sim-kotlin/sim-engine` is oracle-only, not production.
3. Update `scripts/parity.ps1`, `scripts/sim-harness.ps1`, `scripts/export_sim_telemetry.ps1`, and
   `docs/SIMULATOR.md` to Rust for normal ops; parity exporter path → `legacy/uma-sim-kotlin`.

**Gate R6:** no *production* build path depends on `uma-sim/sim-engine` (now under
`legacy/uma-sim-kotlin`); `scripts/parity.ps1` still runs the exporter from the archived module;
`docs/SIMULATOR.md` describes only Rust commands.

### R7 — Resume `SIMULATOR_PLAN.md` fidelity work (in Rust)

This is where the port stops and the *simulator* work resumes. Drive from
`docs/SIMULATOR_PLAN.md` Phases 2–8 and its "not_modeled" lists.

1. **Telemetry calibration (Phase 2 exit gate):** `uma-sim export-telemetry` writes Android-shaped
   JSONL; `python scripts/calibrate_sim.py --telemetry <file>` passes (median abs error ≤2 pts on
   ≥80% of training turns). Obtain or synthesise `runs/*/telemetry/*.jsonl` and make
   `--runs-dir runs` pass.
2. **Grand Live full realisation** — the explicit user checklist:
   - `research/grand_concert.json` `not_modeled` emptied or each item covered by a parity test
   - token gains within ≤2 pts median error on ≥80% of GL training turns vs telemetry
   - bot adapter ≥90% on **GL-specific** replay fixtures
   - all three concert-cycle paths verified (Great Success / normal / consolation)
   - lesson board vs `next_square_info_array`, `blocked_performance_types`, concert failure,
     `member_states`, fan-scaled uniques, dating/scenario-link events
   - `docs/SIMULATOR.md` GL section marked complete with test evidence
3. **Trackblazer:** victory points, shop sales depth.
4. **Unity Cup:** remaining `research/unity_cup.json` gaps.
5. **URA:** remaining `research/ura_finale.json` gaps.
6. **Legacies/sparks/inheritance:** full factor parity, `auto_policy`.
7. **Race model:** deepen beyond the stub per Phase 5.
8. **Perf gates:** Phase 8 targets audited in Rust (`cargo bench` or timed tests).
9. **`docs/SIMULATOR.md`:** rewrite for Rust; complete the product-ready checklist with evidence.

**Gate R7:** every numbered item above has passing test or command output cited in
`docs/SIMULATOR.md`; `ProductReadinessTest` equivalent passes in Rust; the Grand Live checklist is
fully true (**notify the user when this specific item flips true** — it is an explicit standing
request).

---

## 5. Working rules

- **Parity oracle first.** Never "fix" Rust to match a guess. Export a Kotlin fixture, then match it.
- **First-divergence debugging.** When a golden seed mismatches, use `rng_trace` and `turn_trace` to
  find the first diverging RNG call or turn, and fix that cause — do not tune outputs.
- **RNG call order is load-bearing.** Any added/removed/reordered RNG draw changes every downstream
  value. Preserve Kotlin's exact draw sequence, including draws inside failure rolls, hint rolls,
  event branch/energy rolls, and scenario hooks.
- **Integer semantics.** Kotlin `Int` division truncates toward zero; `Double.toInt()` truncates;
  `floor()` in `FormulaGain` is explicit. Mirror exactly with `i32` and `.floor() as i32`.
- **No `#[ignore]` at a gate.** An ignored test is an unmet gate.
- **No `static mut`.** Use `OnceLock`, `LazyLock`, or `Mutex`.
- **Keep it warning-clean.** `cargo clippy -- -D warnings` before each gate.
- **Content is shared, not copied.** Read `research/` and `knowledge/canonical/` in place.
- **Commit per gate**, with the gate's evidence in the message. Do not commit unless asked.

## 6. Commands

```powershell
# Rust core (production)
cd uma-sim-core
cargo test                       # all tests
cargo test --test golden_seeds   # 200-fixture parity gate
cargo test --test parity         # RNG / scoring / turn-trace fixtures
cargo clippy -- -D warnings
cargo run --bin uma-sim -- start --seed=42 --scenario=ura
cargo run --bin uma-sim-api      # REST :8765

# Kotlin parity oracle (archived — reference only)
cd legacy\uma-sim-kotlin
.\gradlew.bat :sim-engine:jvmTest
.\gradlew.bat :sim-engine:jvmTest --tests <RngParityExportTest>

# Content + calibration
python knowledge\validate\validate.py
python scripts\calibrate_grand_live.py
python scripts\calibrate_sim.py --telemetry runs\sim-telemetry\sim-42-ura.jsonl
.\scripts\parity.ps1
```

## 7. Definition of done

1. `cargo test` — ≥127 tests, 0 failures, 0 ignored.
2. `tests/golden_seeds.rs::all_kotlin_golden_summaries` — all 200 fixtures match Kotlin, not ignored.
3. All `scoring_vectors.json` / `event_parse_vectors.json` / `rng_trace` / `turn_trace` fixtures pass.
4. Bot parity ≥90% on training, event, and GL-specific replay fixtures.
5. CLI, REST, MCP, and TUI run on the Rust core with parity fixtures captured.
6. No build path depends on `uma-sim/sim-engine`; decommission recorded.
7. `python scripts/calibrate_sim.py` passes on exported sim telemetry.
8. Grand Live checklist (§R7.2) fully true — **user notified**.
9. `docs/SIMULATOR.md` product-ready checklist complete, every claim backed by cited output.
10. `docs/SIMULATOR_RUST_PARITY.md` maps all 127 Kotlin tests to Rust counterparts, no gaps.
