# Brighter Together! Our Grand Concert (bot alias: Grand Live)

**ID:** `scenario:grand_concert`  
**MDB scenario_id:** 3  
**Official Global:** Brighter Together! Our Grand Concert  
**JP:** つなげ、照らせ、ひかれ。私たちのグランドライブ  
**Global release:** 2026-07-22  
**Stat caps (July 2026 Global):** Speed 1600 · Stamina/Power/Wit 1300 · Guts 1500  
**Scenario spark:** Grand Concert (Speed + Guts)

Machine-readable spec: `research/grand_concert.json` (schema v2).  
Packet/API reference: community packet notes (formerly documented alongside the Python bot as `grand_live_api.md`; not shipped in this repo).

---

## Core loop

1. **Train** facilities → earn **Performance** tokens (five types, cap **200** each). Separate from stat gains.
2. **Lessons** (no turn cost): spend Performance on **Concert Techniques** and **Song Lessons**.
3. Each song learned adds to the **current concert cycle** (API `hype.current` / `next_live_id_array`).
4. Before each promo/grand concert, learn enough cycle songs to hit **Great Success** (typically **3**; MDB `great_success_required`).
5. After each concert, cycle resets; **Concert Bonuses** from cycle songs activate (`effected_live_id_array`).
6. **Six concerts:** Make Debut + four Promo Concerts + Grand Concert.
7. **Best unique path:** **18** cumulative songs + Great Success on Grand Concert → special *Girls' Legend U* → **I Wanna Win with You**.

---

## Hype / Great Success (critical — not a gauge)

Community guides sometimes describe a “hype gauge.” The **packet/API model** is:

| Concept | API field | Meaning |
|---------|-----------|---------|
| Cycle songs | `hype.current` / `song_progress.next_concert` | Songs learned **since the last concert** (`next_live_id_array` length) |
| Required | `hype.great_success_required` | MDB threshold for Great Success at the **upcoming** concert (usually **3**) |
| Ready | `hype.great_success_ready` | `cycle_songs >= required` — bot reads this as “Hype maxed” (Great Hype icon) |
| Total songs | `song_progress.learned_total` | All songs ever learned (`master_live_id_array`) |
| Activated | `song_progress.activated` | Songs whose concert bonuses have fired (`effected_live_id_array`) |

**Start:** *Make Debut!* (`song_list_id` 1) is pre-owned → cycle count starts at **1** (one-third of the typical requirement of 3).

**Per-concert limits (UMAT bot):** minimum **3**, maximum **4** songs per cycle before a concert.

---

## Performance types

| Packet type | API key | Display | Bot OCR |
|-------------|---------|---------|---------|
| 1 | dance | Dance | Da |
| 2 | passion | Passion | Pa |
| 3 | vocal | Vocal | Vo |
| 4 | visual | Visual | Vi |
| 5 | composure | Composure | Co |

Each training facility yields tokens in a **60 / 30 / 10** split (primary / secondary / tertiary type). More supports in the facility and higher facility level → more tokens. Cap starts at **200** per type and rises by **+50 per concert held** (success or Great Success), reaching ~450 by the Grand Concert. Live bot scoring reads OCR overlays, so it never consults this table; the sim does.

Source: [GameTora Our Grand Concert](https://gametora.com/umamusume/our-grand-concert) (2026-07-22). Confirmed against Condor run logs (Stamina → Vo, Guts → Da).

| Facility | 60% (primary) | 30% (secondary) | 10% (fixed tertiary stand-in) |
|----------|---------------|-----------------|--------------------------------|
| Speed | Dance | **Visual** | Passion |
| Stamina | Passion | **Vocal** | Visual |
| Power | Vocal | **Composure** | Dance |
| Guts | Visual | **Dance** | Passion |
| Wit | Composure | **Passion** | Vocal |

GameTora models the 10% bucket as a random other type; the sim uses the fixed tertiary above for deterministic runs.

### Level-1 base training (no supports, no growth)

Sim applies this table for `grand_concert` L1 via `research/training_gain_tables.json` → `scenario_overrides.grand_concert.l1_base` (mood/support/growth multipliers still apply on top).

| Facility | Stat gains | Tokens | Energy |
|----------|------------|--------|--------|
| Speed | +8 Spd, +4 Pwr, +4 SP | 10 | −19 |
| Stamina | +8 Sta, +6 Guts, +4 SP | 10 | −20 |
| Power | +4 Sta, +9 Pwr, +4 SP | 10 | −20 |
| Guts | +2 Spd, +2 Pwr, +7 Guts, +4 SP | 10 | −20 |
| Wit | +2 Spd, +6 Wit, +5 SP | 6 | **+5** |

Facility levels: start Lv1, **+1 every 4 uses**, up to Lv5.

---

## Concert schedule

| Turn | Event | Calendar | live_type† |
|------|-------|----------|------------|
| **1** | Junior Make Debut | Y1 Jun Late | — |
| **24** | Promo Concert 1 | Y2 Jun Late | 1 |
| **36** | Promo Concert 2 | Y2 Dec Late | 2 |
| **48** | Promo Concert 3 | Y3 Jun Late | 3 |
| **60** | Promo Concert 4 | Y3 Dec **Early** | 4 |
| **72** | Grand Concert | Y3 Dec Late | 5 |

† `live_type` values inferred from API examples; confirm via MDB `single_mode_live_live_data`.

**Grand Concert** also checks `total_song_requirement` (**18** for best unique).

---

## Lessons

- **No turn cost** — side action from main screen, concert screen, or career end.
- **3 slots** per refresh (`next_square_info_array` → API `lesson_choices`).
- **Categories:** stat (1), skill_hint (2), recovery (3), song (4).
- **Techniques** (248 in KB) must be learned before new **songs** reliably appear on the board (runtime gate; sim approximates with technique-count thresholds).
- **Affordability:** every cost component must be ≤ current Performance balances.
- **Specialty Priority Up** (concert bonuses) raises specialty facility weight on daily deck placement.
- **Closer Together** (Senior Early Nov, ≥16 songs): scenario-link skill hints; gold if trainee or support is Falcon/Bourbon/Suzuka/Tachyon.
- **21-song technique pivot** (before Grand Concert): do not force a song slot; prefer techniques when songs are weak.
- **Make Debut!** granted ~4 turns after career start (sim turn 5): all Performance +10 mastery, fills 1/3 first-promo hype; Specialty Priority activates after Promo 1.
- **Character debut race** does not reset the concert cycle.

### Song catalog (24 total, 21 purchasable)

Canonical: `knowledge/canonical/by_kind/song.json`.

| Part | When | song_list_ids |
|------|------|---------------|
| 1 | Junior | 3, 4, 5, 8, 9, 11, 12, 23 |
| 2 | Classic | 2, 13, 15 |
| 3 | Senior (before Dec Late) | 6, 7, 10, 19 |
| 4 | Senior Dec Late | 14, 16, 17, 18, 20, 21 |
| 5 | Non-purchasable | 1 (Make Debut!), 22 (Normal GLU), 24 (Special GLU) |

Each purchasable song has:
- **Mastery Bonus** (immediate on purchase — stats, SP, etc.)
- **Concert Bonus** (after next concert — Friendship Training Effectiveness, Specialty Rate Up, or Support Chain Event Frequency; typically +5%)

Techniques: `knowledge/canonical/by_kind/lesson.json` — 93 stat, 140 skill_hint, 15 recovery.

---

## Key songs & uniques

| song_list_id | live_id | Name | How obtained |
|--------------|---------|------|--------------|
| 1 | 1006 | Make Debut! | Career start |
| 22 | 1036 | Girls' Legend U (Normal) | Auto Y3 Dec Early; consolation unique |
| 24 | 1029 | Girls' Legend U (Special) | 18 songs + Grand Great Success |

| Outcome | Unique skill |
|---------|--------------|
| 18 songs + Grand Great Success | **I Wanna Win with You** |
| Otherwise | **On the Way to Our Dream** |

Both scale with fans at career end via GameTora `MultiplyFanCount` tiers
(`[0,20k)→0.8`, `[20k,50k)→0.9`, `[50k,100k)→1.0`, `[100k,160k)→1.1`, `[160k,∞)→1.2`)
in `GrandLiveMechanics::unique_skill_power`.

---

## Scenario links

Light Hello, Smart Falcon, Agnes Tachyon, Silence Suzuka, Mihono Bourbon.  
**Light Hello:** grants repeatable Performance of your **least-owned** type.

---

## Soft cap (Global July 2026)

Gains above **1200** are halved; in-race effect of stats above 1200 is roughly halved. Scenario hard caps still apply.

---

## Bot / sim alignment

| Bot field | Sim source |
|-----------|------------|
| `tokenTotals` (Da/Pa/Vo/Vi/Me) | `perf_*` resources |
| `daysToConcert` | turns until 24/36/48/60/72 |
| `songsLearned` | `songs_learned` |
| `isHypeMaxed` | `cycle_songs >= great_success_required` |
| `objectiveProfile` | `scenario_clear_grand_concert` |

Automation picker alias: **`Grand Live`**. KB + sim use `grand_concert`.

---

## Lesson technique → song pattern (deterministic)

Resets after each live. Carrying a song across a live into the next segment counts as one step of the next initial pattern.

| Window | Initial | Looping |
|--------|---------|---------|
| Before 1st promo | 1-2-3 | 4-4-2-2 |
| Before 2nd/3rd/4th promo | 2-2-2 | 4-5-2-2 |
| Before Grand Concert | 2-2-2 | 4-3-2-2 |

Lives also award **5 SP per technique lesson + 25 SP per song lesson** since the previous live, and raise each token pool cap by **50**.

---

## Known gaps (need MDB / packet ingest)

- Exact `performance_gains` per facility level ≥2 and support-count scaling  
- `live_bonus_type` numeric → effect mapping  
- Exact `next_square_info_array` board weights (sim: 3-slot pool with song preference, category diversity, freeze, technique tiers, 21-song pivot)  
- Full `single_mode_live_live_data` schedule table  
- Historical per-concert activated song groupings (requires packet diff)
- Dating-starts proc rate vs exact support-event table (sim: 12%/turn, Light Hello bond≥40)

`live_type` (0–5) and `result_state` (0 fail / 1 normal / 2 great success) are modeled from KUC captures.
`training_bonuses.target_type` is mirrored (friendship=1, specialty=2, support_chain=6) pending full MDB confirm.

See `research/grand_concert.json` → `sim_implementation_status` and `gaps_requiring_mdb_ingest`.
