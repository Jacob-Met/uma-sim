# Grand Concert decision policy (v1)

## What changed

[`GrandLiveTraining.scoreTraining`](../../uma-android-automation/android/app/src/main/java/com/steve1316/uma_android_automation/bot/campaigns/GrandLiveTraining.kt) now:

1. Computes the existing heuristic score (friendship early / raw later).
2. Applies **mood multiplier** (`MoodLevel` from KB formula).
3. Converts to **expected value under failure** (`(1-p)·score − p·penalty`).
4. Adds **Performance token** value as scenario-action score.
5. Combines via **`ObjectiveProfiles`** (default `scenario_clear_grand_concert`).

Canonical math lives in `scoring-shared`:
- [`FormulaGain.kt`](../../uma-android-automation/android/scoring-shared/src/commonMain/kotlin/com/steve1316/uma_scoring/FormulaGain.kt)
- [`ObjectiveProfiles.kt`](../../uma-android-automation/android/scoring-shared/src/commonMain/kotlin/com/steve1316/uma_scoring/ObjectiveProfiles.kt)

TS mirror: [`objectiveProfiles.ts`](../../uma-android-automation/src/lib/training/objectiveProfiles.ts)

## Setting

`general.objectiveProfile` — one of:
`spark_farming` | `pvp_ace` | `career_score` | `stat_total` | `scenario_clear_grand_concert` (default for Grand Live)

Blend profiles later by extending settings to two names + mix weight.

## Skill spend

End-of-career skill knapsack already exists as `SkillPlan.SpendingStrategy.OPTIMIZE_SKILLS` / `OPTIMIZE_RANK`. Prefer `OPTIMIZE_SKILLS` with community tiers from KB skill export for PvP / score profiles.

## Next iterations (telemetry-driven)

- Feed real deck `SupportEffectSlice` from KB support_card effects (uncap breakpoints).
- Lesson EV as first-class action vs train (already partially in GrandLive lesson policy).
- A/B profiles using `scripts/analyze_run.py` disagreement + objective outcome tags.
