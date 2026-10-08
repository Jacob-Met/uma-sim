# Skill valuation (community spreadsheet)

Ingest plan for end-of-career / pre-finals skill purchasing. Source extract:
`knowledge/raw/references/skills_spreadsheet.md` (from *Uma Musume Skills Spreadsheet.xlsx*).

Survey: agent transcript for sheet inventory and extraction gaps.

---

## What to ingest (canonical)

Prefer the **current category sheets**, not the `(old)` ones:

| Sheet | Role |
|-------|------|
| Speed | Speed skills — Team Trials + **CM9** ranks |
| Accel | Acceleration — TT + CM6 |
| Other 🟧 | Lane / FoV / start / style-change — TT + CM6 |
| Stamina | Recovery — TT + CM6 |
| Debuff | Opponent debuffs — TT + CM4 |
| Greens | Track/season/weather greens — TT + CM |

Shared columns (category sheets):

```
Skill Name | Rank (Team Trials) | Rank (CM*) | Score/SP | Base Cost
| Ground/Distance/Style | Base Duration | Effect… | Precondition(s) | Condition(s) | Why?
```

**Primary ranking key for the bot:** `Score/SP` (numeric efficiency).  
**Secondary filter:** tier symbol for the active mode (`⍟ ◎ ◯ ▲ △ ✕` — must-have → never).

Style/distance filter sheets (`Front` / `Pace` / `Late` / `End` / `Sprint` / `Mile` / `Medium` / `Long`) and master `Tierlist` duplicate the same skills with dual TT/CM(PvP) columns — useful for context-aware reweighting once we know running style + target distance, not as the first ingest.

Skip `alldata` (formula spill, ~3770 empty rows, duplicated blocks). Skip all `(old)` sheets.

---

## Rank symbols (README)

| Symbol | Meaning (community) |
|--------|---------------------|
| ⍟ | Highest priority / must-have in that mode |
| ◎ | Strong |
| ◯ | Good |
| ▲ | Situational |
| △ | Weak / niche |
| ✕ | Avoid |

CM column version labels differ by sheet (CM4 / CM6 / CM9) — treat as “CM / PvP” until a re-extract unifies them.

---

## Example efficiencies (illustrative)

| Skill | Score/SP | Cost | Notes |
|-------|----------|------|-------|
| Lone Wolf | ~7.14 | 70 | Green; easy proc |
| Early Lead | ~4.17 | 120 | Front Runner must-have |
| [Run Style] Corners ◯ | ~3.85 | 130 | Cheap consistent |
| Hesitant [Style]s | ~3.85 | 130 | Speed debuff |
| Corner Recovery ◯ | ~2.94 | 170 | Consistent heal |
| Risky Business | ~4.17 | 120 | ✕ — stamina suicide |

Buy order under a fixed SP budget: sort affordable skills by `Score/SP` descending, subject to style/distance tags and aptitude multipliers from `run_grade.md` (A/S ×1.1, …).

---

## Wiring to the bot

1. Parse category sheets → `knowledge/canonical/by_kind/skill_valuation.json` (or extend `skill.json` payload).
2. Map OCR skill names → valuation rows (aliases / official names).
3. Replace “skill auto-buy: no plans enabled” with a knapsack over `Score/SP` × remaining SP, gated by:
   - running style + distance tags
   - tier floor (e.g. skip ✕ / △ unless Open League purple-score gaming)
   - gold vs white: prefer gold when `Score/SP` and condition reliability win
4. Feed purchased skills’ **run-grade points** (see `run_grade.md`) into \(U\) so SP and training turns share units.

---

## Re-extract caveats

Text dump loses: cell colors, merged header structure on Tierlist/style sheets, formula provenance, parent/child gold↔white row pairing (empty cells inherit from the ◯ row). For production ingest, prefer openpyxl / a dedicated parser on the original `.xlsx` rather than the markdown dump.

---

## Related

- `knowledge/mechanics/run_grade.md` — skill → career score points
- `knowledge/mechanics/parent_farming_utility.md` — white-gene appearance needs skills *learned*
- Bot: `SkillPlan.kt` / careerComplete + preFinals plans
