# Interaction Latency Research (Phase 2)

Notes for safe minimum waits per screen class. Used to tune `AdaptiveLoading` without sacrificing OCR precision.

## Screen classes

| Class | Examples | Server round-trip? | Safe minimum settle |
|-------|----------|-------------------|---------------------|
| **LOCAL_UI** | Training tab switch, dialog dismiss, Back button | No | 80–150ms animation |
| **SERVER_SYNC** | Train confirm, race start, lesson purchase, rest/recreation | Yes | Loading label gone + 2 clear polls |
| **OCR_READ** | Turn-start stats, training analysis | No (read-only) | Header/template stable 1 frame |

## Uma Musume client behavior (Global)

- **Connecting** (top): server request in flight — must wait; false early exit causes wrong-screen actions.
- **Now Loading** (bottom): asset/scene load — same requirement.
- **Dialog dismiss**: title gradient gone ≈ 200–400ms on MuMu; fixed 500ms was conservative.
- **Training tab**: header icon animates ~100–200ms before failure-chance OCR is valid.
- **Lessons list** (Grand Live): list refresh after purchase 1–3s server-side; hardcoded 3–5s waits map here — candidate for element poll (token row / Learnable banner).

## Emulator (MuMu x86_64)

- MediaProjection capture: ~30–80ms per frame typical at 1080×1920.
- Each `checkLoading()` = 1 capture + 2 template checks — dominant cost in loading loops.
- Adaptive backoff reduces **sleep** between polls; capture count drops via 2-poll early exit when stable.

## v6.0 implementation mapping

- `AdaptiveLoading.BACKOFF_MS` — inter-poll sleep schedule.
- `CONSECUTIVE_CLEAR_TO_EXIT = 2` — conservative vs 1 (avoids single-frame flicker).
- Dialog poll cap = `3 × dialogWaitDelay` (min 1.2s, max 4s).
- `skill_list_confirmation` keeps dedicated 1.0s — multi-step skill UI close.

## Open follow-ups (post-baseline)

1. Grand Live lesson purchase waits → Learnable-banner / token OCR poll.
2. Single-frame-per-tick in `Campaign.process()` when screen class unchanged.
3. Profile `android-cv-automation-library` capture path on MuMu vs physical device.
