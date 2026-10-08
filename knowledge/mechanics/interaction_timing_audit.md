# Interaction Timing Audit (Phase 1)

Baseline audit for the **Interaction Speed & Robustness v6.0** goal. Instrumentation ships in v5.9.2 (`InteractionTimer`, `interaction_span` telemetry, `scripts/analyze_timing.py`).

## Timing stack (inner → outer)

| Layer | Location | Default cost | Precision role |
|-------|----------|--------------|----------------|
| Fixed sleep | `Game.wait()` | `general.waitDelay` (0.5s) per call | Animation / server settle |
| Loading poll | `Game.checkLoading()` | 1× screenshot + 2 template checks | Detect Connecting / Now Loading |
| Loading loop | `Game.waitForLoading()` | Poll until clear; each poll = `waitDelay` sleep + checkLoading | Blocks until server sync done |
| Post-tap | `Game.tap()` | 0.2s + `waitForLoading()` | Server actions after gesture |
| Dialog close | `DialogHandler.handleDialogs()` | **+0.5s fixed** after every handled dialog | Dialog dismiss animation |
| Dialog batch | `Task.tryHandleAllDialogs()` | Up to 15s loop | Pop-up storms at turn start |
| Turn OCR | `Campaign.performTurnStartUpdates()` | 8–10 threads, 10s latch | Stat / mood / energy reads |
| Training scan | `Training.analyzeTrainings()` | 5 tab switches × parallel OCR/YOLO | Failure chance + gains |
| Scenario hooks | e.g. `GrandLive.runLessonPurchases()` | Many hardcoded 1–5s waits | Lessons list refresh |

## Tap path split (robustness risk)

| API | Loading wait | Used by |
|-----|--------------|---------|
| `Game.tap()` | Yes (0.2s + waitForLoading) | Recovery taps, some flows |
| `Component.click()` | **No** | Most buttons; caller must wait |

**Recommendation (Phase 3):** unified `waitPolicy` on click helpers (`LOCAL_UI` vs `SERVER_SYNC`).

## Screenshot budget hotspots

1. **`checkLoading()`** — new capture every poll (dominates loading loops).
2. **`getSourceBitmap()`** — counted per tick via `CustomImageUtils` override (v5.9.2+).
3. **`findImage(tries=3)`** — up to 3 captures when no shared bitmap passed.
4. **`updateEnergy()`** — separate thread capture while main thread holds frame.
5. **Training** — 5 facility tabs, each with parallel sub-reads.

## Grand Live hardcoded waits (sample)

File: `GrandLive.kt`

| Wait | Context |
|------|---------|
| 3–5s | After lesson purchase (list refresh) |
| 2s | Lessons navigation |
| 1–3s | Concert / Grand Concert flow |
| `dialogWaitDelay` | Confirm dialogs |

These are prime candidates for **element-present / element-absent polls** once Phase 0 baselines exist.

## External dependency

`android-cv-automation-library` (`ImageUtils.getSourceBitmap`, MediaProjection capture) owns capture latency. Phase 2 should profile on MuMu x86_64 vs 1080×1920 reference.

## Instrumentation spans (v5.9.2+)

| Span | Source |
|------|--------|
| `dialog_pass` | `Task.tryHandleAllDialogs` |
| `fixed_sleep` | `Game.wait` sleep portion |
| `loading_wait` | `Game.waitForLoading` |
| `turn_start_ocr` | `Campaign.performTurnStartUpdates` |
| `training_analyze` | `Training.analyzeTrainings` |
| `scenario_hook` | `GrandLive.checkCampaignSpecificConditions` |

Counters: `screenshotCount`, `loadingPollCount` per process tick.

## Ranked ROI (Phase 3 implementation order)

1. Adaptive loading (backoff + consecutive-clear early exit) — high savings, medium risk.
2. Dialog dismiss poll vs blind 0.5s — medium savings, low risk.
3. Single-frame-per-tick in `Campaign.process` — high savings, medium risk.
4. Training: drop redundant `waitForLoading` inside parallel threads — medium savings, low risk.
5. Grand Live lesson waits → template polls — high savings on GL careers, medium risk.
6. Lower `waitDelay` only with post-condition checks — tune last using 5-run baseline.

## Success metrics (from goal)

- Median **TRAIN** tick `totalMs` ↓ ≥15% vs baseline.
- No increase in OCR timeouts or wrong-screen recovery taps.
- Logs / telemetry explain skipped waits and extra polls (Phase 4+).
