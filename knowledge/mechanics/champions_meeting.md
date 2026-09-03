# Champions Meeting targets (ingest stub)

Per-CM build targets for \(U_{\text{race}}\) — the lookup that should drive `buildTargets` when a run is aimed at
PvP rather than parent farming.

## Status

**Not yet ingested.** Neither community Word doc we extracted contains a structured CM table:

- `reference_global.md` § "Current CM Parameters" is a pointer to YouTube guides.
- `reference_5th_anniversary.md` has format rules and a partial JP month schedule, not Global cup rows.

Career race calendar (`knowledge/canonical/by_kind/race.json`, 320 races) is **not** the CM calendar.

## Intended schema

```json
{
  "cm_id": 18,
  "name": "Libra Cup",
  "server": "global",
  "distance_m": 1600,
  "distance_band": "mile",
  "surface": "turf",
  "course": "Hanshin",
  "direction": "right",
  "season": null,
  "baselines": {
    "graded": { "speed": 1500, "stamina": 800, "power": 1100, "guts": 400, "wit": 1100 },
    "open_a_plus": { "speed": 1200, "stamina": 600, "power": 900, "guts": 400, "wit": 800 }
  },
  "notes": "recovery skills / debuffer environment"
}
```

## Sources to scrape

| Priority | Source | Notes |
|----------|--------|-------|
| 1 | [uma.guide Champions Meeting](https://uma.guide/) + per-cup guides (e.g. Libra Cup CM18) | Global-facing, current cups |
| 2 | Game8 CM archives | Per-cup baselines (e.g. Virgo CM17 Open League `1200 / 750+ / 900 / 500 / 800`) |
| 3 | JP historical cups | Prior for Global's accelerated timeline (~JP delay ÷ 3); confidence decays with distance |

## Wiring once ingested

```
targets(t) = baseline(next_contestable_cm(trainee, t), league)
```

where `next_contestable_cm` skips cups the trainee's distance aptitude cannot reach (e.g. mile Condor skips a 3000m
long). Parent-farm runs use `parent_farming_utility.md` instead of this table.
