# Race Model — Umalator Audit & Integration Plan

Goal: replace the career race stub (`RaceOutcomeConfig::fan_gain` + "win by default") with a
frame-stepped, multi-horse race simulation that produces a real finish order, finish time, and
margins for every mid-run race, deterministic under the career seed and gated by tests.

Reference target: the race engine behind
[umalator-global](https://kachi-dev.github.io/uma-tools/umalator-global/).

---

## 1. Reconnaissance (already done)

### 1.1 What "umalator" actually is

Two repos, both **GPL-3.0**:

| Repo | Role |
| --- | --- |
| `alpha123/uma-skill-tools` | Original engine (`RaceSolver`, condition parser, HP model). Copyright (C) 2022 pecan. |
| `kachi-dev/uma-tools` | Hosts the umalator UI; **vendors a heavily extended fork** of `uma-skill-tools` under `uma-skill-tools/`, plus `umalator/` and `umalator-global/` front-ends. |

The site is the fork. Audit the fork, not the original — the fork adds a virtual opponent field,
lane movement, a separate spurt calculator, and probabilistic other-uma conditions that the original
explicitly disclaims.

### 1.2 Core engine inventory (fork, `uma-skill-tools/`)

| File | Lines | Contents |
| --- | --- | --- |
| `RaceSolver.ts` | 1508 | Integration loop (`step`), phases, target speed, accel/decel, rushed (kakari), hills/downhill mode, position keep state machine (`None`/`Approximate`/**`Virtual`**), lane movement, compete-fight, lead competition, last-spurt transition, skill activation + wisdom checks, random-gold selection |
| `ActivationConditions.ts` | 1022 | Condition → course-region reduction for every skill condition keyword |
| `RaceSolverBuilder.ts` | 888 | Assembly: horse/course/params → solver, skill compilation, RNG wiring, sample-count loop |
| `SpurtCalculator.ts` | 330 | Last-spurt candidate enumeration and acceptance |
| `ActivationSamplePolicy.ts` | 245 | Immediate / random / distribution sample policies and their dominance rules |
| `ConditionParser.ts` | 224 | Tokenizer + parser for the `a==1&b>=2@c` condition DSL |
| `HpPolicy.ts` | 163 | Max HP, HP/s, guts modifier, status modifiers, `getLastSpurtPair` |
| `CourseData.ts` | 110 | Course schema (corners/straights/slopes/laneMax), phase boundaries, course-set-status speed modifier |
| `ApproximateConditions.ts` | 69 | Start-rate/continuation-rate Markov model for other-uma conditions |
| `Region.ts` | 62 | Region algebra used by the condition reducer |
| `SpecialConditions.ts` | 60 | Concrete approximations: `blocked_side`, `overtake` (per strategy) |
| `Random.ts` | 29 | PRNG wrapper over `prando` (MIT) |
| `HorseTypes.ts` | 28 | Stats, strategy, aptitude grades |
| `RaceParameters.ts` | 19 | Mood, ground condition, weather, season, time, grade, popularity, `numUmas` |

≈4,760 lines of engine. Drivers we also care about: `umalator/simulator.worker.ts` (212) and
`umalator/compare.ts` (621) show how a race is set up and sampled.

Data shipped with it: `umalator-global/course_data.json` (107 courses),
`uma-skill-tools/data/course_data.json` (121 courses, JP superset),
`courseeventparams/*.json` (~180 per-course raw Unity dumps),
`skill_data.json` (652 global skills), `umas.json` (64 characters),
`test/regression/checkpoints/*.json` (up to 17 MB of frozen outputs — a ready-made fixture corpus).

### 1.3 Formula anchors confirmed against our own notes

`HpPolicy.ts` matches `knowledge/mechanics/race_model.md` exactly:
`maxHp = 0.8 × strategyCoef × stamina + distance`, `hp/s = 20 × (v − baseSpeed + 12)² / 144 × modifiers`,
`gutsModifier = 1 + 200/√(600 × guts)` applied from phase 2, spurt acceptance
`(15 + 0.05 × wisdom)%` per candidate. `RaceSolver.ts` adds the pieces our notes do not have:
strategy×phase speed/accel coefficient tables, aptitude modifier tables, `PhaseDeceleration`,
position-keep thresholds with `courseFactor = 0.0008 × (distance − 1000) + 1`.

This is the strongest possible signal that our documented-mechanics route and the fork agree, which
makes clean-room reimplementation viable rather than speculative.

### 1.4 Data-fit checks run against our knowledge base

- Our 320 career races use **89 distinct `course_id`s**. 79 are in the global course data; the other
  10 (`11203`, `11302`, `11303`, `11402`, `11403`, `11404`, `11501`, `11502`, `11504`, `11612`) are
  all present in the JP set → **89/89 covered** once the two sets are merged.
- Our `skill.json` holds **1898 skills, all with `condition_groups`** (GameTora shape:
  `precondition` / `condition` / `effects[{type,value}]` / `base_time`). Upstream's shape is
  `alternatives[{precondition,condition,baseDuration,effects[{type,target,modifier}]}]` derived from
  `master.mdb`. **561 of upstream's 652 global skills overlap by id**; 91 upstream ids are absent
  from ours and need reconciliation.
- Effect type ids: upstream implements `{1,2,3,4,5,6,8,9,10,13,14,21,22,27,28,29,31,35,37}`; ours
  carry those **plus** `{32,38,41,42,48,49,501,502,503}`. Everything upstream models, we have data
  for; our extras are the ones upstream leaves unimplemented (scaling effects et al.) and become
  explicit `not_modeled` entries.

Conclusion: the data side is largely already in the repo. The work is engine behaviour, not scraping.

---

## 2. Fit assessment

### 2.1 What the fork gives us

Per-frame physics for a horse; the full skill-condition DSL compiled to activation regions; HP and
last-spurt decisions; rushed/kakari; hills and downhill mode; wisdom gating of skill procs; and —
critically, contrary to the upstream README — a **real field**: `initUmas()`,
`getUmaByDistanceDescending()`, `PosKeepMode.Virtual`, pacer tracking, compete-fight, lead
competition, and lane movement. Real `order` / `order_rate` conditions are therefore resolvable.

### 2.2 What it does not give us (the half we must build)

1. **Career opponent fields.** umalator is a comparison tool: you supply both umas. Career races need
   9–18 NPC entrants with stats, skills, strategies, and post positions generated per race grade and
   career turn. Nothing upstream does this.
2. **Career-side race context.** Popularity/odds, race-day ground/weather, mood and status effects at
   race time, scenario race modifiers (Grand Live concert buffs, Trackblazer/Unity/URA race hooks).
3. **Fidelity gaps upstream admits:** inner/outer lane distance differences, skill cooldowns
   (multi-proc skills fire once), scaling effect types, and `accumulatetime`-combined skills firing
   early. `blocked_side` and `overtake` remain probability models even in Virtual mode.

So "incorporate umalator" = port/reimplement the solver **and** build a career field model on top of
it, calibrated against real game telemetry (which this repo already has infrastructure to capture).

---

## 3. Decision gate D1 — licensing (blocks all code)

GPL-3.0 is copyleft. `uma-sim-core` is currently unencumbered. Three routes:

| Route | Mechanism | Consequence |
| --- | --- | --- |
| **A. Clean-room reimplementation** | Rust written from mechanics documentation (`knowledge/mechanics/race_model.md`, uma.moe / community writeups, our own telemetry); umalator used only as a **black-box oracle** (inputs → outputs), never read while writing | No license obligation on our code. Slowest; every constant must be independently sourced or fitted. §1.3 shows this is achievable for the HP/speed core. |
| **B. Direct port** | Translate the TS to Rust | Derivative work: `uma-sim-core` (and anything distributed with it) becomes GPL-3.0. Fastest and most accurate. Fine if this project is never distributed, or is happy to be GPL. |
| **C. Sidecar** | Run the unmodified TS in a separate process (node worker) called over stdio/JSON | Keeps a process boundary; still legally debated for tightly-coupled use. Node dependency at runtime, ~ms-per-call IPC. |

Recommendation: **stand up C immediately as the audit oracle** (it is needed for differential
testing under any route, and running an unmodified GPL program is unrestricted), then implement
**A** for production, falling back to **B** only if a specific subsystem cannot be reconstructed from
documentation. Reference sources stay in a scratch directory outside the repo
(`%TEMP%\umalator-audit`, already populated) and are never committed.

**Nothing in phases R8.1+ starts until D1 is answered**, because it determines whether audit notes
may contain code-shaped detail or only behavioural specifications.

---

## 4. Audit workstreams

Every workstream produces (a) a section in `docs/RACE_MODEL_AUDIT.md`, (b) a machine-readable
constants file under `research/`, and (c) Rust code plus tests. Under route A, (a) records
*behaviour and constants*, not transcribed code.

| # | Subject | Upstream surface | Our artifact | Validation |
| --- | --- | --- | --- | --- |
| A1 | Integration loop, phases, target/base speed, accel, decel, start dash | `RaceSolver.step`, `baseSpeed`, `baseTargetSpeed`, `lastSpurtSpeed`, `baseAccel`, `PhaseDeceleration` | `research/race_model_constants.json`, `uma-race-core/src/physics.rs` | Constant-for-constant unit tests; no-skill finish time vs oracle within tolerance |
| A2 | HP model | `HpPolicy` | same constants file, `hp.rs` | HP curve vs oracle per frame; stamina-out behaviour |
| A3 | Last spurt | `SpurtCalculator`, `getLastSpurtPair` | `spurt.rs` | Spurt start position + speed vs oracle across a stat sweep |
| A4 | Course model | `CourseData`, `course_data.json` (both sets), `courseeventparams` | `research/race_course_data.json` (generated), `course.rs` | 89/89 course-id coverage test; geometry round-trip vs source |
| A5 | Condition DSL | `ConditionParser`, `Region` | `condition/parser.rs` | Parse every distinct `precondition`/`condition` string in our 1898 skills, zero failures |
| A6 | Condition → regions | `ActivationConditions` (1022 lines — largest single item) | `condition/regions.rs`, per-keyword coverage table | Keyword-by-keyword differential vs oracle; `all_conditions_implemented`-style test |
| A7 | Sample policies | `ActivationSamplePolicy` | `condition/sample.rs` | Distribution tests; dominance rules |
| A8 | Field dynamics | position keep state machine, pacer, compete-fight, lead competition, lane movement, rushed, hills/downhill, wisdom checks | `field.rs` | Order-trace differential vs oracle in Virtual mode |
| A9 | Other-uma approximations | `ApproximateConditions`, `SpecialConditions` | `research/race_position_keep.json` | Rate parity; replaced by real field state where our field allows |
| A10 | RNG | `Random.ts` (prando) | `rng.rs` (prando-compatible stream, MIT-licensed algorithm) | Bit-exact stream match — prerequisite for all differential tests |

A10 first: without an identical PRNG, differential testing degrades from exact to statistical.

---

## 5. Data workstreams

- **D-1 Course geometry.** Merge global (107) + JP (121) course sets into
  `research/race_course_data.json` keyed by our `payload.course_id`. Provenance choice: regenerate
  from the game's master database using private emulator/adb ingest (clean
  provenance, preferred), or transcribe the fork's JSON (game-derived facts, but sourced from a GPL
  repo). Decide with D1.
- **D-2 Skill adapter.** GameTora `condition_groups` → engine skill definition. Includes the
  `effects[].type` table (19 modelled, 9 recorded `not_modeled`), `target` semantics upstream carries
  that GameTora does not, rarity/evolution handling, and reconciliation of the 91 upstream-only ids.
- **D-3 Race parameters.** Map our race payload (`grade`, `distance`, `terrain`, `direction`,
  `season`, `entries`, `course_id`, `track`) onto `RaceParameters` (mood, ground, weather, season,
  time, grade, popularity, numUmas). Fill gaps: ground/weather roll per race day, popularity from
  fans/stats.
- **D-4 Uma/NPC catalog.** `umas.json` is 64 characters — not a career NPC field. Feeds §6.

---

## 6. The career field model (not obtainable from umalator)

This is what makes mid-run results *factually accurate*, and it must be researched from the game:

1. **Capture corpus.** Use the existing Android automation to record real career races: entrant list,
   each entrant's visible attributes, our stats/skills/mood/strategy, finish order, times, and
   margins. Land as `runs/*/race-telemetry/*.jsonl` with a schema in `research/race_field_npc.json`.
2. **Fit an NPC generator.** Per (scenario, turn, grade) produce entrant stat distributions, strategy
   mix, and skill loadouts that reproduce observed finish-order and margin distributions.
3. **Gate on distribution match, not single races.** A race is stochastic; the acceptance criterion is
   that simulated win rate / average placement / margin distribution match telemetry within a stated
   tolerance for matched stat profiles.

Until step 2 has a corpus, the field can be bootstrapped from grade-based NPC stat tables in
`research/` and flagged `provisional` in the run log — but the gate stays open.

---

## 7. Target architecture

```
uma-race-core/            # new crate: race physics only, no career concepts
  src/physics.rs  hp.rs  spurt.rs  course.rs  field.rs  rng.rs
  src/condition/{parser.rs, regions.rs, sample.rs}
  src/lib.rs              # RaceInput -> RaceResult
uma-sim-core/
  src/race.rs             # career-side adapter: scheduling (existing) + field generation + call
  src/engine.rs           # do_race() consumes RaceResult
```

```rust
pub struct RaceInput { course_id: u32, params: RaceParameters, entrants: Vec<Entrant>, seed: u64 }
pub struct RaceResult { finish: Vec<Finisher>, /* order, time_ms, margin, spurt info, skill procs */ }
```

Integration rules:

- **`do_race` switches on config**, `race_model = "stub" | "physics"`, default `stub` until R8.8
  passes. The 200 Kotlin golden-seed fixtures are a hard constraint: they must keep passing on
  `stub`, and get re-frozen deliberately (with a documented reason) when `physics` becomes default.
- **Determinism without perturbing the career RNG stream.** Derive the race seed from
  `(career_seed, turn, race_id)` via a hash, and run the race on its own PRNG instance. Zero new
  draws on the career RNG → existing parity fixtures survive.
- **Placement feeds existing machinery.** `RaceResult` → `RacePlacement` → `fan_gain_placed`,
  `skill_points_for`, `grant_epithet`, and the scenario `on_race_complete` hooks (Trackblazer VP,
  Unity legs, GL concerts) already accept a win/lose flag; widen them to placement.
- **Bot/search fast path.** `bot_scoring` / lookahead cannot afford a full field sim per candidate;
  keep a cheap analytic estimator (current `rank_estimate`) for search and use the full sim only for
  the race actually run. Document the asymmetry.

---

## 8. Validation ladder

| Level | Test | Gate |
| --- | --- | --- |
| V1 | Formula unit tests vs `research/race_model_constants.json` and `knowledge/mechanics/race_model.md` | Exact |
| V2 | Differential vs TS oracle: N randomized (course, stats, aptitudes, strategy, skills, seed) cases | Finish time within tolerance (target: ≤1 frame, 1/15 s) and identical skill-proc positions |
| V3 | Replay upstream `test/regression/checkpoints` as fixtures | Within V2 tolerance across the corpus |
| V4 | Career telemetry distribution match (§6) | Win rate and mean placement within stated tolerance per stat profile |
| V5 | Career golden fixtures re-frozen on `physics`, `cargo test` green | 0 failures, 0 ignored |
| V6 | Perf gates (§9) | Below budget |

V2/V3 are the reason the sidecar oracle (route C) is built first regardless of the production route.

---

## 9. Perf budget

At `dt = 1/15 s`, a race is ~900 frames (1200 m) to ~3000 frames (3600 m) per horse; an 18-horse
field is ~16k–54k solver steps, and a career is ~12–24 races. Targets:

- ≤10 ms per 18-horse 2000 m race, single-threaded (p50), skills included.
- ≤250 ms of race time per full career run.
- 200-fixture golden suite stays under a 2× wall-clock regression vs today.

If missed: field-level parallelism, region precompilation cache per course, and a coarse `dt` for
non-focus entrants are the escape hatches (each must be justified against V2).

---

## 10. Phases and gates

| Phase | Content | Gate |
| --- | --- | --- |
| **R8.0** | Answer D1. Vendor reference sources to a gitignored scratch path. Build the node sidecar oracle with a JSON stdio protocol + `scripts/race_oracle.ps1`. | Oracle reproduces the published umalator UI result for 3 hand-checked cases |

**R8.0 status (2026-09-02):** D1 = clean-room. Quarantine at `C:\Programming\umalator-ref\`.
Oracle at `umalator-ref\oracle\` (GPL). Client: `scripts/race_oracle.ps1`. Three fixtures locked;
`.\scripts\race_oracle.ps1 -Verify` → All 3 fixtures OK. Audit skeleton: `docs/RACE_MODEL_AUDIT.md`.

**R8.1 status (2026-09-02):** `uma-race-core` — Prando bit-exact; HP/phase/start-dash/hills/section-Wiz/rushed/course-set-status. case1 finish within **1 frame** of oracle (`cargo test case1_finish` green).

**R8.2 status (2026-09-02):** `research/race_course_data.json` covers **89/89** career `course_id`s. Metadata primary from a private `master.mdb` extract (`race_course_set` cross-check green aside from JP-only 11605/11612). Geometry matches fork `course_data` (intentional 10301 `laneMax` mdb override). Cross-checks: `research/race_course_mdb_crosscheck.json`, `research/race_course_fork_crosscheck.json`.

**R8.3 status (2026-09-02):** Condition DSL parser + region reduction + Immediate/Random sample policies + skill adapter. Effect types **9/21/22/27/31** applied in-race (`28` = lane-move, deferred to R8.4). Gate skill **200701** (not 200501) within **1 frame** of oracle (`case3_skill_finish_within_one_frame_of_oracle` green; accel beats no-skill baseline).

**R8.4 status (2026-09-02):** `HorseRunner` frame stepper; Virtual + default Nige pacer within **1 frame** of oracle; `simulate_field_synced` (pacer-first then pos-desc) with pack PaceUp/Down + Nige SpeedUp/Overtake; order-trace deterministic + Virtual couples vs None. Career `physics` path uses synced Virtual with placeholder NPCs. Approximate Markov A9 (`blocked_side`/`overtake`) in `special_conditions.rs` + lane movement. Lucky-pace mutation / multi-entrant oracle still open. Constants: `research/race_position_keep.json`.

**R8.5 status (2026-09-02):** Hard **16/16**. Curated V3 **119/119 (100%)**. Wide 240 **~95.1%**. Expanded greens-inclusive extract **679 scored / ~92.0%** (`checkpoint_v3_1000`, soft ≥80%). Fixes: green wisdom-skip, scenario ×1.2, `is_lastspurt` phase clip. Residual Δ mostly &lt;0.5s; unmodeled types 8/32/35/37 still exclude full upstream 1000.

**R8.6 status (2026-09-02):** NPC soft-cal + `race_npc_r86`. Live win prior `race_telemetry_corpus.json`. Physics place/margin MC `race_v4_physics_dist.json` + career log harvest `race_v4_career_place.json` (ordinal place + margins). Live-game ordinal place capture still open. Evidence: `research/R8_DONE_CRITERIA_EVIDENCE.md`.

**R8.7 status (2026-09-02):** `race_model = stub|physics` on `SimSettings`; CLI `--race-model=` / env `UMA_RACE_MODEL`. Physics path derives seed from `(career_seed, turn, race_id)` with **zero** career-RNG draws, maps finish order → `RacePlacement` → fans/SP/epithet/`on_race_complete`. Field = trainee + placeholder NPCs via `simulate_field_synced` + `PosKeepMode::Virtual`. Course ids from `race.json` for numeric races; symbolic `debut`/finales mapped. Full-career physics smoke in `tests/race_physics_r87.rs`.

**R8.8 status (2026-09-02):** Default flipped to **`physics`**. 200 golden summaries re-frozen (`research/R88_REFREEZE.md`). Kotlin RNG/turn traces pin `race_model=stub` so post-race career-RNG branches stay parity-stable. Done criterion #9 (physics default + `cargo test` green) **true**.

**R8.9 status (2026-09-02):** `docs/SIMULATOR.md` Mid-run races section + status table; `tests/race_perf_r89.rs` gates ≤10ms p50 (18-horse 2000m) and career wall proxy ≤250ms; `research/race_outcomes.json` / `grand_concert.json` updated (SmartRaceSolver mid-run gap retired). Residual: R8.5 full V3 + R8.6 telemetry V4.
| **R8.1** | A10 RNG, A1 physics core, A2 HP, A3 spurt. No skills, no field. | V1 exact; V2 on single-horse no-skill cases |
| **R8.2** | A4 course model + D-1 data pipeline | 89/89 course coverage test; geometry round-trip |
| **R8.3** | A5 parser + A6 regions + A7 sample policies + D-2 skill adapter | Every condition string in our 1898 skills parses; per-keyword differential green; `not_modeled` effect types recorded in `research/` |
| **R8.4** | A8 field dynamics + A9 approximations, Virtual pos-keep | Order-trace differential on multi-horse cases |
| **R8.5** | V3 regression-checkpoint corpus | Corpus within tolerance; failures triaged as ours vs upstream-known-issue |
| **R8.6** | §6 telemetry capture + NPC field generator | Corpus landed; generator reproduces telemetry distributions (V4) |
| **R8.7** | Engine integration: `RaceInput/RaceResult`, placement plumbing, scenario hooks, `race_model` flag | `physics` runs a full career end-to-end; `stub` fixtures untouched |
| **R8.8** | Flip default to `physics`, re-freeze golden fixtures, V5 | `cargo test` green, 0 ignored; re-freeze rationale documented |
| **R8.9** | V6 perf, `docs/SIMULATOR.md` race section rewritten with cited evidence, `research/*.json` `not_modeled` lists accurate | All gates cited with command output |

Working rules from `docs/SIMULATOR_RUST_PLAN.md` §5 carry over unchanged — in particular *oracle
first, never tune outputs to hide a divergence*, and *first-divergence debugging* (here: first
diverging frame, not first diverging turn).

---

## 11. Risks and open questions

- **D1 unresolved** blocks everything; route A costs weeks more than route B.
- **`ActivationConditions.ts` (1022 lines) is the long pole.** Mitigation: drive coverage from the
  conditions our 1898 skills actually use, not from upstream's full keyword set.
- **Upstream is not ground truth.** It carries known bugs and approximations (§2.2). Where V2 and V4
  disagree, telemetry wins and the divergence is recorded rather than "fixed" toward umalator.
- **Field model is genuinely new research.** The NPC generator is the least constrained part of this
  plan and the most likely to need iteration.
- **Golden-fixture churn.** Flipping to `physics` invalidates career fixtures by design; R8.8 exists
  so that happens once, deliberately.
- **JP vs global divergence.** Our knowledge base spans both; the fork ships separate global/JP data.
  Every course/skill lookup must be server-aware.
