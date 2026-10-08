# R8.8 golden re-freeze (physics default)

**Date:** 2026-09-02

**Why:** Mid-run races now use `uma-race-core` frame-stepped physics (`race_model=physics`). Stub win-by-default is no longer the product default. Career summaries (fans / SP / speed) change because placements are real.

**Invariant preserved:** Race PRNG is derived from `(career_seed, turn, race_id)` with **zero** draws on the career RNG stream — training RNG parity is unchanged; only race outcomes and hooks that depend on placement diverge from stub.

**Command:** `cargo run -p uma-sim-core --example golden_refreeze_physics`

**Fixture:** `uma-sim-core/tests/fixtures/kotlin/golden/summaries.json` (200 rows = 50 seeds × 4 scenarios).

**Elapsed:** 2.6432952s

**Rollback:** set `race_model=stub` (CLI `--race-model=stub` / env `UMA_RACE_MODEL=stub`) for legacy stub behaviour; stub path remains supported.
