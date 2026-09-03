# Race Model Audit (behaviour & constants only)

This document records **observable behaviour and independently sourced constants** for the
clean-room Rust race simulator. It must never contain transcribed GPL source from umalator /
`uma-skill-tools`.

Plan: `docs/SIMULATOR_RACE_PLAN.md`. Oracle quarantine: `C:\Programming\umalator-ref\` (outside this
repo). In-repo client: `scripts/race_oracle.ps1`.

## GPL boundary

| Location | Contents | License |
| --- | --- | --- |
| `C:\Programming\umalator-ref\uma-tools\` | Reference checkout (engine + UI) | GPL-3.0 |
| `C:\Programming\umalator-ref\oracle\` | JSON stdio sidecar that *links* the engine | GPL-3.0 (derivative) |
| This repository | Thin PowerShell client + clean-room Rust + research constants | Unencumbered (our code) |

Rules:

1. Do not copy TypeScript from the quarantine into this tree.
2. Constants in `research/` come from `knowledge/mechanics/race_model.md`, community docs, game
   `master.mdb`, and telemetry — the oracle **verifies**, it does not **supply**.
3. Where oracle and live-game telemetry disagree, telemetry wins; record the divergence here.

## R8.0 — Oracle sidecar

Status: **R8.0 gate met** (driver + 3 locked fixtures + in-repo client). Manual UI
eyeball of the same three configs in umalator-global is optional confirmation; engine-path
identity is the same `RaceSolverBuilder` + `mode=compare` + seed `2615953739`.

### Protocol

`RaceRequest` → stdout `RaceResponse` (single JSON object). See
`C:\Programming\umalator-ref\oracle\race_oracle.ts` header comment for the schema.

Notable behavioural flags observed via the oracle (not copied from source):

- `mode: "compare"` enables stamina/HP drain for finish times; other modes may omit HP (infinite HP).
- Default UI seed observed in the published tool: `2615953739`.
- Integration timestep used by the reference: `dt = 1/15` seconds.
- `posKeepMode`: `None` | `Approximate` | `Virtual`.

### Hand-checked fixtures

Run: `.\scripts\race_oracle.ps1 -Verify`

| ID | Course | Strategy | Skills | Locked median finish (s) |
| --- | --- | --- | --- | --- |
| case1_tokyo_1400_senkou | 10601 / 1400 turf | Senkou | none | 66.46666666666802 |
| case2_hanshin_2600_nige | 10205 / 2600 turf | Nige | none | 130.7999999999977 |
| case3_tokyo_dirt_1600_oikomi | 10611 / 1600 dirt | Oikomi | 200701 (type 31 accel) | 76.13333333333414 |

Evidence: `.\scripts\race_oracle.ps1 -Verify` → `All 3 fixtures OK` (2026-09-02).

**Effect-type note:** master.mdb / oracle `SkillType` use **31 = Accel**, **28 = LaneMovementSpeed**.
Skill 200501 (Lane Legerdemain) is type 28 and does not change finish time when lane movement is
off — do not use it as an accel differential gate. Use 200701 (or other type-31 skills) instead.

UI cross-check procedure (manual): enter the same stats/course/seed into
[umalator-global](https://kachi-dev.github.io/uma-tools/umalator-global/), mode Compare, confirm
finish time within one frame (1/15 s) of the locked median.

## Constants ledger (R8.1+)

Populate from our docs / telemetry; verify against oracle.

### HP (from `knowledge/mechanics/race_model.md`)

- `MaxHP = 0.8 × StrategyCoef × Stamina + CourseDistance`
- StrategyCoef (approx): Runaway 0.86 · Pace Chaser 0.89 · Front Runner 0.95 · End Closer 0.995 · Late Surger 1.0
- `HP/s = 20 × (CurrentSpeed − BaseSpeed + 12)² / 144 × StatusModifier × GroundModifier`
- `GutsModifier = 1 + 200/√(600 × Guts)` from late race / last spurt
- Spurt candidate acceptance ≈ `(15 + 0.05 × Wit)%`

### Speed / accel

Phase speed/accel multipliers transcribed from Global 5th Anniversary reference into
`research/race_model_constants.json` and `knowledge/mechanics/race_model.md`. Start dash
`+24 m/s²` until opening target (KuromiAK).

### Integrator progress (R8.1–R8.4)

| Fixture | Oracle | Ours | Δ |
| --- | --- | --- | --- |
| case1 Tokyo 1400 Senkou (None) | 66.4667 | 66.4667 | **exact** — R8.1 |
| case1 + Virtual default pacer | 66.6000 | 66.6000 | **exact** — R8.4 |
| case3 dirt 1600 + accel **200701** (type 31) | 76.1333 | 76.1333 | **exact** — R8.3 |

**Effect types:** `31=Accel`, `28=LaneMovement`, `27=TargetSpeed`, `21=CurrentSpeed`, `9=Heal`. Skill **200501** is type 28 (finish noop without lane model).

### Course coverage (R8.2)

`research/race_course_data.json`: **89/89** career `course_id`s. Metadata primary from a private `master.mdb` `race_course_set` extract (cross-check: 119 matched; patched 10301 `laneMax`, 11203 `courseSetStatus`). Geometry matches fork `course_data` aside from the 10301 mdb `laneMax` override (`research/race_course_fork_crosscheck.json`). JP courses 11605/11612 in fork but absent from this mdb revision.

### Keywords / effects (R8.3)

See `research/race_skill_effects.json` for modeled vs `not_modeled`.

### Field (R8.4 / R8.7)

Virtual pack PaceUp/Down + Nige SpeedUp/Overtake; `simulate_field_synced`. Career `race_model=physics` (default since R8.8) uses synced Virtual + placeholder NPCs (`research/race_field_npc.json`).

### Checkpoints (R8.5 sample + V3 expand)

- Hard sample: **16/16** within 1 frame (`research/race_checkpoint_sample.json` → `race_checkpoint_triage.json`).
- Expanded V3 sample: **120 curated cases**, soft gate ≥90% (current ≈94.1% — see `race_checkpoint_v3_triage.json`).
- Wide V3: **240 cases**, soft gate ≥80% (`checkpoint_v3_wide` / `race_checkpoint_v3_sample_240.json`). Full 1000 still open.
- Lead competition (CompeteTop) + default Virtual pacer speed mult + **rushed single 3s×55% clear**.
- Condition keywords: `distance_rate_after_random`, `blocked_*_continuetime`, `is_finalcorner_laterhalf`,
  oval **`corner==1..4`** indexing, **`is_basis_distance`** (distance % 400), Erlang sample for
  `change_order_onetime` / overtake / blocked / lane-near, `straight_random` / `compete_fight_count`,
  precondition clip from first pre-region start, Unique wisdom skip for master rarity 3–5,
  **`is_finalcorner` → course end**, **Oonige≡Nige** for `running_style`, exhausted speed clamp-by-side.

### Compete-fight (追い比べ)

Constants: `research/race_compete_fight.json`. Wired in `simulate_field_synced` for **n≥3** only (not 2-horse Virtual+pacer — deterministic wiki trigger over-sped cp_11 vs oracle). Lane gap deferred.

### Lead competition (R8.4/R8.5)

See `research/race_lead_competition.json`. Oracle cp_10: Virtual slowdown is lead-comp HP×1.4 (not pos-keep); `leadCompetition(false)` restores None finish.

### Rushed (kakari)

Single early-clear check at **3s** with probability **0.55**; else hold to **12s** max.
Evidence: oracle duration hist (~56% @ 3.0s / ~44% @ 12.0667s). See `research/race_rushed.json`.
(Previous every-3s×55% model incorrectly cleared cp_7/cp_8 at 6s.)

### Lane / Approximate

- Type **28** skills apply wiki `MoveLaneModifier = (0.0002×power)^0.5` to target while active (lateral geometry still deferred).
- `PosKeepMode::Approximate` uses the same pack machine as Virtual when a pacer is present.

### NPC / telemetry (R8.6)

Bootstrap NPCs soft-calibrated + mined win/not-first corpus: `research/race_telemetry_corpus.json` (410 races). Soft ordering gate `race_npc_r86`. Physics place/margin Monte-Carlo: `research/race_v4_physics_dist.json` (`race_v4_dist`). Live ordinal place + margins still open.

## Divergence log

| Date | Case | Oracle | Telemetry / docs | Resolution |
| --- | --- | --- | --- | --- |
| 2026-09-02 | case3 skill id | used 200501 as “accel” | type 28 = lane | switched gate to 200701 (type 31) |
| 2026-09-02 | cp_7/cp_8 rushed early clear | held ~12s max | our every-3s×55% cleared at 6s | **single** 3s×55% then 12s max (`race_rushed.json`) |
| 2026-09-02 | cp_10 Virtual Nige | 129.600 | leadCompetition off → 128.333 | implemented CompeteTop; exact |
| 2026-09-02 | v3_25 corner skills | skills hurt finish / missed 120351 | `corner==N` was Always | real corner geometry; v3_25 exact |
| 2026-09-02 | order≥2 skills in solo | fired via Always | place always 1 | `dynamics_ok` Order vs 1-based place |
| 2026-09-02 | v3_89 101101 dual accel | 99.2 | 96.4 (basis alt0) | `is_basis_distance` = dist%400; non-core uses alt1 |
| 2026-09-02 | v3_118 100871 | 64.2 | skill skipped | `overtake_target_time` keyword + Erlang |
| 2026-09-02 | v3_86 105201211 | 86.67 | 85.8 (no fire) | Erlang `change_order_onetime` trigger |
| 2026-09-02 | corner==3 on 6-seg courses | oval idx formula | used corners[2] | `len+n-5` step −4 |
| 2026-09-02 | Unique wisdom | rarity 3–5 skip | only rarity 5 skipped | match oracle SkillRarity remap |
| 2026-09-02 | post-HP TargetSpeed | finish unchanged | free speed via snap-up | speed clamp by side (oracle step) |
| 2026-09-02 | is_finalcorner | through course end | corner arc only | bounds = [fc.start, distance] |
| 2026-09-02 | running_style==1 + Oonige | matches Nige | exact enum only | Oonige≡Nige |
