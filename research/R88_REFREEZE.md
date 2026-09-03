# R8.8 golden re-freeze (physics default)

**Date:** 2026-09-02

**Why:** Mid-run races now use `uma-race-core` frame-stepped physics
(`race_model=physics`). Stub win-by-default is no longer the product default.
Career summaries (fans / SP / speed) change because placements are real.

**Invariant preserved:** Race PRNG is derived from `(career_seed, turn, race_id)`
with **zero** draws on the career RNG stream — training RNG parity is unchanged.
Placement-dependent **post-race** career-RNG branches (scenario hooks that key
off win/lose) can diverge from stub; Kotlin `rng_trace_*` / `turn_trace_*`
fixtures therefore remain pinned to **`race_model=stub`** in
`run_rng_trace_fixture` / `run_turn_trace_fixture`.

**Command:** `cargo run -p uma-sim-core --example golden_refreeze_physics --release`

**Fixture:** `legacy/uma-sim-kotlin/.../golden/summaries.json`
(200 rows = 50 seeds × 4 scenarios).

**Rollback:** `--race-model=stub` / `UMA_RACE_MODEL=stub` for legacy stub
behaviour; stub path remains supported.
