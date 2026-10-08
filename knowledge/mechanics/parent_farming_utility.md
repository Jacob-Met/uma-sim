# Parent-farming utility (what a career run is actually worth)

Terminal value of a career run, for runs whose purpose is producing inheritance factors ("genes" / "sparks") rather than
winning a specific race. This is the objective the training scorer should maximize.

Primary source: *Crazyfellow's Parenting & Gene guide* (extracted to `knowledge/raw/references/parenting_genes_crazyfellow.md`),
which cites `umamusustation.com/blue_factor_analysis.html` and `aoneko-uma.fanbox.cc` for the underlying sample data.

---

## Two independent levers

The single most important structural fact: **blue genes and white/green genes are driven by different things.**

| Gene colour | What it is | Driven by | NOT driven by |
|-------------|-----------|-----------|---------------|
| **Blue** | One of the 5 stats | The value of the *selected* stat, in bands | Run grade/score |
| **Red** | Aptitude (distance/surface/style) | Aptitude at run end | Run grade/score |
| **White** | Learned skills | **Run grade (score)** | Individual stat values |
| **Green** | Unique skill | **Run grade (score)** | Individual stat values |

Confirmed explicitly: *"Completing a run where your end grade is SS or above will increase the star QUALITY of your
green/white genes. Blue and Red genes are not affected by this mechanic"* (L1911). And *"the increased gene quality stops
at SS grade for the Uma and SS for stats"* (L1919) — pushing a stat past 1200 does nothing for blue genes.

So the objective needs **two terms**: a per-stat bracket term, and a total-score term. Optimizing only stat totals (what
the legacy scorer did) conflates them.

---

## Blue genes: three regimes with cliffs at 600 and 1100

At career end the game picks **one stat uniformly at random (20% each)**, then rolls star quality from that stat's band.

Raw observation counts, and the derived rates:

| Stat band | 1★ obs | 2★ obs | 3★ obs | n | 1★ | 2★ | **3★** |
|-----------|--------|--------|--------|------|------|------|--------|
| 1200 (SS+) | 287 | 1033 | 153 | 1473 | 19.5% | 70.1% | **10.39%** |
| 1150–1199 | 254 | 820 | 128 | 1202 | 21.1% | 68.2% | **10.65%** |
| 1100–1149 | 218 | 740 | 118 | 1076 | 20.3% | 68.8% | **10.97%** |
| 1050–1099 | 451 | 400 | 40 | 891 | 50.6% | 44.9% | 4.49% |
| 1000–1049 | 482 | 444 | 51 | 977 | 49.3% | 45.4% | 5.22% |
| 900–999 | 750 | 669 | 97 | 1516 | 49.5% | 44.1% | 6.40% |
| 800–899 | 802 | 731 | 98 | 1631 | 49.2% | 44.8% | 6.01% |
| 700–799 | 792 | 740 | 106 | 1638 | 48.4% | 45.2% | 6.47% |
| 600–699 | 1081 | 1033 | 123 | 2237 | 48.3% | 46.2% | 5.50% |
| 500–599 | 2232 | 225 | **0** | 2457 | 90.8% | 9.2% | **0%** |
| 400–499 | 3119 | 330 | **0** | 3449 | 90.4% | 9.6% | **0%** |
| 300–399 | 2875 | 329 | **0** | 3204 | 89.7% | 10.3% | **0%** |
| ≤299 | 590 | 70 | **0** | 660 | 89.4% | 10.6% | **0%** |

Collapsing to the three regimes the data actually shows:

| Regime | 1★ | 2★ | 3★ |
|--------|------|------|------|
| **< 600** | ~90% | ~10% | **0%** |
| **600 – 1099** | ~49% | ~45% | ~6% |
| **≥ 1100** | ~20% | ~69% | ~10.7% |

Two hard cliffs, and **flat in between**. Within 600–1099 the 3★ rate wanders between 4.5% and 6.5% with no trend — a
stat at 1050 is worth no more than a stat at 650. Above 1100 it is flat again (10.4–11.0%), so 1100 is the target and
1200 is not better.

A 3★ blue is only reachable at **B rank or higher, i.e. above 600** (L1609). Below that it is a literal zero across
~9,770 observations.

---

## Consequence: the marginal value of a stat point is a two-spike function

Value accrues **only** at the moment a stat crosses 600 or 1100. Using the blue-factor stat-uncap magnitudes as the star
values (1★ = +4, 2★ = +9, 3★ = +16 uncap points, from the Grand Live inheritance section):

| Regime | Expected uncap value |
|--------|---------------------|
| < 600 | 0.90(4) + 0.10(9) = **4.5** |
| 600–1099 | 0.49(4) + 0.45(9) + 0.06(16) = **6.97** |
| ≥ 1100 | 0.20(4) + 0.69(9) + 0.107(16) = **8.72** |

- Crossing **600**: **+2.47**
- Crossing **1100**: **+1.75**

Value density per stat point invested:

- 0 → 600: 2.47 / 600 = **0.0041 per point**
- 600 → 1100: 1.75 / 500 = **0.0035 per point**
- anywhere else: **0**

These two densities are nearly equal, which is *why* the community heuristic "get 2–3 stats past 1100 and the rest to
600" is close to optimal: the two cliffs buy value at almost the same rate, so it doesn't matter much which you chase —
only that you never leave a stat parked in the flat middle of a band.

Since the stats are not equally cheap to raise (deck specialty and rainbows make Speed/Wit cheap; Guts and Stamina cost
the most energy per point), the optimal split falls out naturally: **push the cheap stats to 1100, drag the expensive
ones just over 600, and stop.** Which stats are cheap is a property of the deck, not a fixed preference.

### Worked failure case

El Condor Pasa, Grand Live, 2026-09-02 (`runs/20260902T151735Z-86196c2a`), pre-finale statline:

| Stat | Final | Regime | Expected value |
|------|-------|--------|---------------|
| Speed | 1215 | ≥1100 | 8.72 |
| Stamina | **596** | **<600** | **4.5** |
| Power | 731 | 600–1099 | 6.97 |
| Guts | 634 | 600–1099 | 6.97 |
| Wit | 767 | 600–1099 | 6.97 |

Stamina finished **4 points short of the 600 cliff**. Every stamina point from 409 (turn 56) to 596 earned **zero**
blue-gene value; the next 4 were worth +2.47. Guts at 634 cleared its cliff and then kept going — the points from 600 to
634 were also worth zero.

---

## White / green genes: driven by run grade, not stats

Star *quality* bands (Crazyfellow L1931–1945; green score bands in the same doc L2717–2725 match):

| End grade | Score | 1★ | 2★ | 3★ |
|-----------|-------|------|------|------|
| Below B | < 6,500 | 90% | 10% | 0% |
| Below SS | 6,500 – 17,499 | 50% | 45% | 5% |
| Above SS, below Ue | ≥ 17,500 | 20% | 70% | 10% |
| Ue and above | ≥ 28,800 | 17.5% | 70% | 12.5% |

**Ub and above give no further increase** — Ue is the ceiling (L1945). Full letter table and raw-stat→score
decomposition: `knowledge/mechanics/run_grade.md`. Score is **stats + skills only** (fans/wins/titles do not feed it).

The Condor farm scored **~11,400 (A)** — in the middle band. Reaching SS (17,500) would have doubled the 3★ white/green
rate. With ~2,056 unspent SP, skill purchases were the cheapest remaining path across that cliff and the bot bought almost
none.

### White-gene *appearance* (per learned skill)

Independent of run grade. Each skill learned during the run rolls once:

| Skill type | Base | Cap | +per lineage holder | Model |
|------------|------|-----|---------------------|-------|
| White | 20% | 35% | +2.5% | linear; or `20% × 1.1^N` |
| White ◎ | 25% | 40% | +2.5% | linear; or `25% × 1.1^N` |
| Gold (learned) | 40% | 70% | +5% | linear; or `40% × 1.1^N` |
| Race (G1 win) | 20% | 35% | +2.5% | same as white |

Flat **5%** of generated white/race genes are 3★ before the grade quality table is applied. Must learn the skill in-run;
gold versions cannot be passed as white genes.

Green (unique) genes: **guaranteed** to generate for 3★+ characters; star quality follows the same grade table.

### Red genes

- Only from aptitudes at **A or higher**; one aptitude chosen uniformly among all A+ aptitudes (S does not increase
  selection weight or ★ rarity).
- Fixed ★ at generation: **10% / 70% / 20%** for 3★ / 2★ / 1★.
- Fewer native A aptitudes → higher P(target red). Mid-run inheritance uses a hidden **+1…+5** value roll.

### Compatibility (mid-run inheritance, not end-of-run generation)

```
inheritance_odds = base_odds_per_star × (1 + individual_compatibility_score / 100)
```

◎ = >150 pts, 〇 = 51–150, △ = <50. Target **>300**, optimal **~500**. Post-2.0 Global: G1 overlaps only, **+3** per
shared G1. Base proc rates at 0 compatibility: blue 70/80/90%, green 5/10/15%, white/scenario 3/6/9%, race 1/2/3%,
red 1/3/5% for 1★/2★/3★.

---

## Objective function

\[
U = \underbrace{\tfrac{1}{5}\sum_s \Phi_{\text{blue}}(x_s)}_{\text{blue genes}}
  + \underbrace{\Psi_{\text{grade}}\big(\text{score}(x,\ \text{skills})\big)}_{\text{white + green quality}}
  + \underbrace{\sum_{\text{skills}} P_{\text{appear}}(s)}_{\text{white appearance}}
  + \underbrace{\Theta_{\text{red}}(\text{aptitudes})}_{\text{red genes}}
  + U_{\text{scenario}}
  - \underbrace{B(x)}_{\text{career-clear barrier}}
\]

\(\Phi_{\text{blue}}\) has cliffs at **600** and **1100**. \(\Psi_{\text{grade}}\) has cliffs at **6,500 / 17,500 /
28,800**. Under uncertainty the usable derivative is a sum of bumps at those points — not a curve monotone in
percent-of-cap.

Tension the scorer must hold: blue genes treat points between cliffs as waste; run grade rewards raw total (and
accelerating score returns above 900), so padding past 1100 still buys white/green quality. Skill SP and training turns
must share the same ΔU units via `run_grade.md`.

---

## Related

- `knowledge/mechanics/run_grade.md` — score components and letter thresholds
- `knowledge/mechanics/inheritance_planners.md` — pre-run red-factor ★ totals → aptitude rank-ups ([design.u-ma.org](https://design.u-ma.org/))
- `knowledge/raw/references/parenting_genes_crazyfellow.md` — full source extract
- `knowledge/raw/references/reference_5th_anniversary.md` — score tables, training formula
