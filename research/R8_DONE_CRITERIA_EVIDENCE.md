# R8 done-criteria evidence (working audit)

Date: 2026-09-02. Goal: `docs/SIMULATOR_RACE_PLAN.md` R8.0–R8.9.

| # | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Oracle JSON stdio, 3 fixtures | **Met** | `.\scripts\race_oracle.ps1 -Verify` → All 3 fixtures OK |
| 2 | Prando bit-exact | **Met** | `uma-race-core` rng tests / case1 exact |
| 3 | Physics/HP/spurt + solo ≤1 frame | **Met** | case1/case3 exact (`cargo test -p uma-race-core --lib`) |
| 4 | 89/89 course_ids | **Met** | Coverage 89/89; mdb + fork cross-checks green (10301 `laneMax` mdb override intentional); JP-only 11605/11612 absent from this mdb revision |
| 5 | Skill condition parse + not_modeled | **Met** | Conditions + effects; `random_lot`/`hp_per` dynamics; season/weather/time/grade defaults; see `research/race_skill_effects.json` |
| 6 | Multi-horse field (Virtual, pacer, compete-fight, lead, lane, rushed) | **Met** | Virtual+synced field; compete/lead/rushed; lane + Approximate Markov `blocked_side`/`overtake` (`uma-race-core/src/special_conditions.rs`, `research/race_position_keep.json`). Skill-keyword sampling remains Erlang (Virtual parity) |
| 7 | Upstream checkpoints within tolerance | **Partial** | Hard 16/16; curated **119/119**; wide **225/225 (100%)**; expanded **677/679 (~99.7%)** soft ≥80%. Plan V3 full corpus ≤1 frame still open (**2 fails**: `v3_394`, `v3_690`) |
| 8 | NPC generator + telemetry distribution | **Partial** | Soft NPC + MC + career place/margin harvest green; live-game corpus still win/not_first only (no ordinal place from game capture) |
| 9 | physics default, career E2E, cargo test green | **Met** | `RaceModel::default()==Physics`; `race_physics_r87`; user notified |
| 10 | Perf ≤10ms p50 / ≤250ms career | **Met** | `cargo test --release --test race_perf_r89`: p50≈3.2ms, career wall≈20ms |
| 11 | SIMULATOR.md + audit + not_modeled | **Met** | Mid-run races section + audit + skill effects lists current |

**Do not mark goal complete** until #7 full V3 tolerance and #8 live place telemetry (or explicit soft-V4 acceptance) are settled with evidence.

### Recent deltas (this session)
- Green skills (effect types 1–5) skip wisdom checks → expanded V3 **87% → 90%**.
- A9 Approximate Markov for lane `blocked_side`/`overtake`; release perf still under budget.
- Scenario skill ×1.2 patch; `is_lastspurt` phase clip; `is_finalcorner==0` region fix.
- Heal re-triggers last-spurt eval (umalator Recovery) → **~93.2%**.
- Erlang reconcile keeps right DistUniform/Erlang; OR region union → **~94.4%**; fixed `v3_272`/`v3_359`.
- `change_order_up_*` phase/corner clips → **~94.6%**; `100191` trigger ~oracle (2709).
- Type 28 LaneMove: lateral-only + MoveLaneModifier only while changing lanes (was always boosting target speed) → **~95.7%** expanded, wide **97.3%**; `v3_141` exact.
- `up_slope_random` Random on uphill slopes (was Immediate noop) → **~96.3%**; `v3_705` exact.
- Default virtual pacer: copy focus as Nige (removed fitted 1.125 speed mult; matches oracle `useDefaultPacer`).
- Start-dash: no accel while decelerating; clamp `min(target, 0.85*base)`; do not raise PaceDown target to `min_spd` (fixes Soft/pacer `v3_488`).
- `post_number`: real `gateBlock(gateRoll, numUmas)` (was Always) — fixes green `105601211` false positives.
- `is_used_skill_id`: dynamic used-set check so dual-alt uniques (e.g. `111041`) place heal-gated first alt and skip TargetSpeed second (umalator `buildSkillData`).
- `is_finalcorner` on empty-corner courses (e.g. 10301) → empty regions (was whole-course Always) → expanded **672/679 (~99.0%)**.
- **Start-dash / min_spd order:** end startDash *after* minSpeed floor (umalator order). Fixing same-frame snap restored PaceDown re-enter (`v3_590`).
- **Section mods after gate greens:** roll Wiz section modifiers with post-green wisdom. Expanded **677/679 (~99.7%)**, wide **225/225**.
- Residuals: `v3_394` (`200352` corner sample ≠ oracle → wisdom shift → `202662` miss); `v3_690` (procs match, still 2 frames fast).