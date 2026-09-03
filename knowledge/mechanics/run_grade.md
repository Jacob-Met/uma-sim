# Career run grade (score → letter)

Terminal letter grade is a function of **stats + skills only**. Aptitudes, win count, and titles do not feed score
directly. Source: *Umamusume Reference (5th Anniversary)* (`knowledge/raw/references/reference_5th_anniversary.md`),
cross-checked against Crazyfellow's gene-quality bands.

---

## Letter thresholds

| Grade | Min score | Grade | Min score |
|-------|-----------|-------|-----------|
| G+ | 300 | SS | **17,500** |
| F | 600 | SS+ | 19,200 |
| F+ | 900 | Ug | 19,600 |
| E | 1,300 | Uf | 23,900 |
| E+ | 1,800 | **Ue** | **28,800** |
| D | 2,300 | Ud | 34,400 |
| D+ | 2,900 | Uc | 40,700 |
| C | 3,500 | Ub | 47,600 |
| C+ | 4,900 | Ua | 55,200 |
| **B** | **6,500** | Us | 63,400 |
| B+ | 8,200 | Us9 | 71,400+ |
| A | 10,000 | LG+ | ?? |
| A+ | 12,100 | | |
| S | 14,500 | | |
| S+ | 15,900 | | |

Gene-quality cliffs that matter for parenting (see `parent_farming_utility.md`):

- **B (6,500)** — white/green leave the 90/10/0 band
- **SS (17,500)** — white/green jump to 20/70/10
- **Ue (28,800)** — white/green ceiling at 17.5/70/12.5 (Ub+ adds nothing)

---

## Raw stat → score points

Per-stat contribution (raw, before soft-cap effects on race performance — score uses the displayed raw):

| Raw | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 1000 | 1100 | 1200 |
|-----|-----|-----|-----|-----|-----|-----|-----|------|------|------|
| Points | 352 | 577 | 847 | 1143 | 1463 | 1808 | 2209 | 2653 | 3171 | 3841 |
| Δ from −100 | — | +225 | +270 | +296 | +320 | +345 | +401 | +444 | +518 | +670 |

Note the accelerating returns above 900 — the same 100 points that buy +296 score from 500→600 buy +670 from 1100→1200.
That is why padding past the blue-gene cliffs still matters for **run grade**, even though it buys nothing for blue genes.

Calculators cited in the doc: https://umsatei.com/ , https://yonkim.azurewebsites.net/

---

## Skill → score

| Base SP cost | White | Gold | Evolved |
|--------------|-------|------|---------|
| 70 | 129 | — | — |
| 90 | 129 | 174 | 508 |
| 100 | 239 | 288 | 696 |
| 120 | 239 | 367 | 433 |
| 170 | 239 | 559 (508) | 696 (633) |
| 200 | 289 | 696 | — |

- Aptitude multiplier on skill score: A/S **×1.1**; B/C **×0.9**; G **×0.7**; else **×0.8**
- Unique skill: **170 pts/level** (120 for 1★/2★ uma); Lv6 unique ≈ **+1050**
- Inherited unique: **180 pts**
- Purple (debuff) skills: **−129** (50-cost) or **−262** (100-cost) — used deliberately for Open League A+ builds

---

## Implication for the bot

A skill purchase and a training turn must be scored in the **same units** (expected ΔU). Score points from a gold skill
(~288–696) can move a run across a gene-quality cliff when the current estimated score sits near 6,500 / 17,500 /
28,800. The Condor farm that finished ~11,400 (A) was ~6,100 short of SS — buying skills from the unused ~2,056 SP
pool was the cheapest remaining path to the white/green cliff, and the bot bought almost none.
