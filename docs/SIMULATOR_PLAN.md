# Uma Musume Offline Simulator — Master Plan

> **Goal created:** autonomous execution proceeds phase-by-phase per this document.  
> **Vision:** deterministic, seedable, text-first career engine for all four Global scenarios at documented numeric fidelity — no visuals, full mechanics.

---

## Rust engine (`uma-sim-core`)

Production sim logic lives in Rust **`uma-sim-core/`**. Golden / RNG / turn-trace fixtures under
`uma-sim-core/tests/fixtures/` are the parity contract (historical Kotlin oracle is private and not shipped).

```powershell
cd uma-sim-core
cargo test                    # RNG + seed-42 URA golden
cargo test --test golden_seeds  # integration tests (200-fixture gate in progress)
```

Kotlin CLI/TUI/MCP remain shells until Rust is exposed via FFI/cdylib.

## Vision & success criteria

A **deterministic, seedable career engine** that replays the full training loop (72+ turns, all Global scenarios) without visuals: stats, energy, mood, bonds, events, races, skills, scenario economies, inheritance/sparks/legacies/hints — at **documented numeric fidelity**. The live Android bot becomes a **client** of the same rules (via shared `scoring-shared` + sim state adapters), not the source of truth for mechanics.

**Not in v1:** 3D race visuals, multiplayer, JP-only content without Global gating.

**Product-ready when:**
- Start any supported scenario with deck + trainee + legacies; finish a full career in text mode at **x20+** in under ~10s on a laptop (x100 turbo for batch bot testing)
- **Same seed → same outcome** (reproducible for bot A/B)
- Bot decisions fed from sim observations match live-bot decisions on ≥90% of turns when calibrated (telemetry harness)
- New Global card/scenario ships by adding canonical JSON + optional scenario plugin hook (no engine rewrite)
- MCP + REST text API: `GET state`, `POST action`, `POST auto_step`, `POST load_content_pack`

---

## Current assets (~35–40% toward full sim)

| Layer | Status |
|-------|--------|
| Canonical KB | 553 supports, 1886 skills, 3460 events, 402 races, GL/TB structured data — `knowledge/canonical/` |
| Decision math | `scoring-shared` — training/events/lessons/skills EV |
| Race schedule | Smart Race Solver (MILP) — planning only, not race physics |
| Telemetry | `TurnTelemetry.kt` + `scripts/analyze_run.py` |
| **Missing** | Turn RNG engine, factors/legacies canonical, training gain sampling, race execution, inheritance deck model |

Extend existing KB principles (`knowledge/README.md`): namespaced IDs, provenance, Global gating, payload-per-kind.

---

## Architecture

```
Content Layer          Engine (KMP sim-engine)           Interfaces
─────────────────      ─────────────────────────         ──────────────
knowledge/canonical →  CareerState + SeededRNG     →     uma-sim CLI/TUI
scenario_plugins    →  MechanicsRegistry         →     REST text API
content_packs       →  ScenarioController ×4     →     MCP server
                       ActionResolver              →     BotTestHarness
                              ↕
                       scoring-shared (decision oracle)
                       BotDecisionAdapter
```

**Tech choices (locked):**
- **Engine:** Kotlin Multiplatform module `uma-sim/sim-engine` beside `scoring-shared` (JVM + JS; shared types with bot)
- **CLI:** TypeScript `packages/uma-sim-cli` — v1 uses JVM engine subprocess for perf; KMP JS when hot paths stabilize
- **RNG:** explicit `SimRandom(seed)` wrapper; all rolls logged when `traceRng=true`
- **Content:** JSON from `knowledge/canonical/by_kind/*.json` + version manifest; scenario plugins implement `ScenarioPlugin`

---

## Phase 0 — Research program (2–3 weeks)

**Goal:** every mechanic has a **source, formula, and confidence tier** before coding.

### 0.1 Source inventory & extraction

| Domain | Primary sources | Deliverable |
|--------|-----------------|-------------|
| Training gains | GameTora `support_effects`, uma.guide, `training_stat_gain.md` | `research/training_gain_tables.json` |
| Failure rates | Energy curves, mood, facility level | `research/training_failure.json` |
| Mood / energy | Official + community charts | `research/mood_energy.json` |
| Bond / friendship | Support specialty rules | `research/bond_gain.json` |
| Events | 3460 event texts + GameTora reward objects | `research/event_reward_schema.json` |
| Event RNG | Support chain frequency, random branches | `research/event_probabilities.json` |
| Races | `race_instances`, fan rewards, goal criteria | `research/race_outcomes.json` (stub v1) |
| Skills / hints | `skills.json`, hint level progression | `research/hint_progression.json` |
| Sparks / factors | Raw `knowledge/raw/gametora/factors.*.json` | `factor.json` + inheritance rules |
| Legacies / parents | GameTora character-cards, factors | `research/legacy_deck_schema.json` |
| Inspiration | Scenario docs + community | `research/inspiration.json` |
| Scenario economies | `knowledge/scenarios/*.md`, UMAT assets | `scenario_rules.json` per scenario |
| Grand Live | songs, lessons, concerts, hype | Extend GL canonical |
| Unity Cup | spirit gauges, bursts | `research/unity_cup.json` |
| Trackblazer | shop, coins, irregular | `research/trackblazer.json` |
| URA | duels, Meek challenge | `research/ura_finale.json` |

### 0.2 Complete KB ingest

Extend `knowledge/ingest/gametora_fetch.py` normalizers for all `DEFAULT_DATASETS` (today only support/skill/scenario write canonical):

- `factor`, `race`, `trainee`, `nickname`, `status_effect`, `skill_condition`, `skill_effect_value`, `scenario_factor`
- Wire `consolidate_local.py` dedup + schema validation against `entity.schema.json`
- Add `knowledge/validate/` CI: schema + cross-ref integrity

### 0.3 Calibration loop

- Expand telemetry: every sim-relevant field on each turn
- `scripts/calibrate_sim.py`: fit parameters from `telemetry/*.jsonl`
- Acceptance: median absolute error on stat gains ≤ 2 pts for 80% of training turns

**Exit gate:** `research/` + full canonical coverage checklist; training gain + event reward schemas exist.

---

## Phase 1 — Core engine skeleton (weeks 4–6)

### CareerState model

Mirror bot types (`Trainee.kt`, `GameDate.kt`) plus:

- `RunMeta` — seed, scenario, objectiveProfile
- `DeckState` — 6 supports, bonds, specialties
- `LegacyState`, `SparkState`, `HintState`, `SkillState`
- `ScenarioState` — sealed per scenario
- `RunFlags` — injury, conditions, dating chain

### Turn loop

Implement `Campaign.decideNextAction` priority as pure `TurnScheduler`.

Actions: `Train`, `Rest`, `Recreation`, `Date`, `Race`, `Event`, `ScenarioSideAction`, `SkillShop`, `Inheritance`, `Advance`.

### Text rendering

- `TextRenderer`: `minimal` | `standard` | `full` (flavor dialogue from event corpus)
- Settings: `dialogueMode: off | choices_only | full`, `speedMultiplier: 1–100` (presets: 1, 2, 5, 10, 20, 50, 100)

### Speed multiplier

- **Range:** integer **1–100** (`speedMultiplier` clamped at 100). Presets in UI/CLI; arbitrary values allowed (e.g. x15).
- **Modes:**
  - **Interactive (x1–x10):** render text per step; optional dialogue per `dialogueMode`
  - **Fast (x11–x50):** suppress non-choice dialogue; batch-resolve consecutive auto steps; emit summary lines per N turns
  - **Turbo (x51–x100):** headless path — no text render unless paused; RNG + state only; for golden-seed sweeps and bot regression
- Logical turns advance one player/bot action at x1; higher multipliers run up to N **safe micro-steps** per wall-clock tick (training/rest/routine with `auto_policy`, not choice points)
- **Never skip choice points** unless `auto_policy` is set (bot/random/objective)
- `auto_play` with bot adapter until pause (concert, skill shop, event choice, mandatory race)
- At x20+, dialogue is forced off unless explicitly overridden (`dialogueMode=full` + `allowDialogueAtHighSpeed=true`)

**Exit gate:** URA shell — 72 turns with stub training + mandatory races; text CLI playable.

---

## Phase 2 — Training & events (weeks 7–10)

### TrainingResolver

Implement `training_stat_gain.md` in code:

- Sample base gain from facility×level table
- Friendship (multiplicative), mood, effectiveness, growth, presence, KB effects
- Failure roll + penalty distribution
- Bonds, rainbow, hints on facility
- Emit `TrainingObservation` identical to bot OCR output → feed `scoring-shared`

### Event system

- Load events from `event_local.json`
- Parse options to typed `EventEffect` (extend `EventScoring.kt` to **apply**, not just score)
- Weighted branch selection from `event_probabilities.json`
- Support chain / dating / inspiration triggers

### Hints & skill points

- Hint levels on hint training / events
- SP accrual from races/events/training

**Exit gate:** Training gains match telemetry within calibrated bounds.

---

## Phase 3 — Scenarios (weeks 11–16)

| Scenario | Plugin responsibilities |
|----------|-------------------------|
| **URA Finale** | Goal races, Happy Meek duel RNG + bias, finale races |
| **Unity Cup** | Spirit gauges, fill/burst/extreme burst, team race flow |
| **Trackblazer** | Shop coins, inventory, irregular training gate, item effects |
| **Grand Concert** | Tokens, lessons shop, hype, concerts, 18-song path |

Each plugin: `onTurnStart`, `sideActions`, `onTrainingComplete`, `onRaceComplete`, `resources()`.

**Exit gate:** Full career in all 4 scenarios (race outcomes stubbed win).

---

## Phase 4 — Legacies, sparks, inheritance (weeks 17–19)

- Canonical `factor.json` from GameTora (~865 entries)
- `LegacyState`: parents, factor slots, inherited skills, spark stat bonuses, cap raises
- Inheritance event with player choice or `auto_policy`
- Inspiration triggers
- Spark stat caps in `softCapEffectivenessMultiplier`

**Exit gate:** Spark-farming run differs measurably from ace run in caps + inheritances.

---

## Phase 5 — Races & goals (weeks 20–22)

- Integrate `SmartRaceSolver` as optional auto-scheduler
- **Outcomes v1:** fan gain + SP + epithet from win/place stub (`race_model.md`)
- **Outcomes v2 (later):** optional full race sim
- Goal criteria: `character_objectives.json`, fan classes, G1 requirements

---

## Phase 6 — Bot harness & validation (weeks 23–25)

### BotDecisionAdapter

- Maps `CareerState` → bot `Trainee`/`GameDate`/scenario fields
- Maps sim training results → `TrainingOption[]`
- Invokes same Kotlin decision paths as live bot

### Regression suite

- **Golden seeds:** 50 fixed seeds × 4 scenarios; store summary stats
- **Telemetry replay:** recorded OCR observations → assert same bot action
- **Diff report:** `sim-harness compare --run-id X`

---

## Phase 7 — MCP & text API (weeks 26–27)

### REST (localhost)

```
POST /v1/run/start     { scenario, deck, trainee, legacies, seed, settings }
GET  /v1/run/state
GET  /v1/run/text      ?mode=standard
POST /v1/run/action    { action, payload }
POST /v1/run/auto      { policy: bot|random|objective }
POST /v1/run/fast      { multiplier: 1-100, until: choice|turn|career_end }
```

### MCP server (`packages/uma-sim-mcp`)

Tools: `sim_start`, `sim_state`, `sim_choices`, `sim_act`, `sim_fast_forward`, `sim_export_telemetry`.

Resources: current run JSON, event log text.

---

## Phase 8 — UI & performance (weeks 28–30)

- **TUI:** split pane — state sidebar + event log + choice prompt
- **Web (v1.1):** replay viewer for telemetry JSONL
- **Perf:** cache KB lookups; batch JSON at startup; full career **≤10s @ x20**, **≤3s @ x100** (turbo, no render)
- **Settings:** dialogue, speed (1–100), auto-policy, trace RNG, objective profile, turbo/render overrides

---

## Phase 9 — Content scalability (ongoing)

- **`content_packs/`** format: `{ version, scenario?, kinds: { ... } }`
- **`sim content validate`** CLI merges pack into runtime catalog
- Release workflow in `knowledge/RELEASE.md`
- Scenario-only plugins as JAR/JS module

---

## Repository layout (new)

```
packages/
  uma-sim-cli/          # TS CLI + TUI
  uma-sim-mcp/          # MCP server wrapping REST
uma-sim/
  sim-engine/           # KMP core
  sim-scenarios/        # URA, Unity, TB, GL plugins
  sim-content/          # Loaders + validation
research/               # Phase 0 artifacts
scripts/
  calibrate_sim.py
  sim-harness.ps1
docs/
  SIMULATOR.md          # User guide + MCP docs
```

---

## Relationship to Android bot

| Bot component | Simulator role |
|---------------|------------------|
| `scoring-shared` | Decision oracle on sim observations |
| Smart Lessons/Events/Skills (v5.9+) | Validated via harness |
| OCR pipeline | Bypassed; optional noise injection later |
| TurnTelemetry | Golden calibration input |

---

## Goal execution order

Execute phases **sequentially**; do not skip Phase 0 research gates.

1. Phase 0.2 — complete KB ingest + validation CI
2. Phase 0.1 — training gain + event probability research artifacts
3. Phase 1 — engine skeleton + CLI minimal
4. Phase 2 — training + events
5. Phase 3 — four scenario plugins (URA → GL → Unity → TB)
6. Phase 4 — legacies/sparks
7. Phase 5 — races stub
8. Phase 6 — bot harness + golden seeds
9. Phase 7 — MCP
10. Phase 8 — TUI polish + perf

Each phase ends with: unit tests, 3 golden seed replays, update `docs/SIMULATOR.md`.

---

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Unknown RNG tables | Telemetry calibration + confidence tiers; mark `estimated` in content |
| JP/Global drift | Strict `server` + `available_en` gating |
| Scope creep (full race sim) | Stub wins v1; race HP module optional v2 |
| Bot/sim drift | Shared `scoring-shared`; harness regression on every PR |
| Performance | KMP JVM engine; avoid per-turn full JSON parse |

---

## Immediate next actions (Phase 0 start)

1. Create `uma-sim/sim-engine` KMP module scaffold + `CareerState` + `SimRandom`
2. Extend `gametora_fetch.py` to normalize `factors` + `race_instances`
3. Author `research/training_gain_tables.json` from GameTora + formula doc
4. Minimal `uma-sim` CLI: `start`, `state`, `step` (stub gains)
5. Add `docs/SIMULATOR.md` with MCP contract draft
