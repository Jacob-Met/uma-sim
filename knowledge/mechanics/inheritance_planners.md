# Inheritance / parent aptitude planners

External tools for assembling a 2-parent + 4-grandparent inheritance tree, checking compatibility, and
predicting how **red (aptitude) factors** raise starting aptitudes. Complementary to the end-of-run gene
generation rates in `parent_farming_utility.md`.

---

## [ウマ娘設計図](https://design.u-ma.org/) (Parent Aptitude Planner)

Unofficial JP fan tool (“Uma Musume Blueprint”). Builds one screen with:

- Trainee + 2 parents + 4 grandparents
- Blue / red factor stars on each node
- Compatibility score for the tree
- **Predicted aptitude rank after inheritance** from red-factor star totals
- Browser save, shareable URL, image export
- Related modules: factorization probability calculator, CM / LoH race-record log

Usage guide: https://design.u-ma.org/guide (updated 2026-08-13)

### Red-factor → aptitude rise (start of career)

Aptitude gain is determined by the **sum of ★ of matching-type red factors** across parents + grandparents
(same aptitude line: e.g. all Dirt, all Mile, all Pace). From the site’s documented rule:

| Matching ★ total | Aptitude rank-ups |
|------------------|-------------------|
| 1–3 | +1 |
| 4–6 | +2 |
| 7–9 | +3 |
| ≥ 10 | +4 |

Rules:

- Rise is **capped at A** — red factors alone **cannot** push A → S (S only comes mid-run).
- Example: Dirt G + 3×3★ Dirt factors (★9) → +3 → **D**, not A. Reaching A from G needs more than red inheritance.
- Borders at 3/4, 6/7, 9/10 are the planning cliffs (“one more ★ unlocks another rank”).

This matches Crazyfellow’s qualitative guidance (e.g. ~10★ lineage to push G → C at start) with a clean
step table.

---

## Related tools

| Tool | URL | Role |
|------|-----|------|
| ウマ娘相性ツール | https://www.umamusumeaisyou.com/ | Compatibility matrix, “魔改造” success simulator, factor-farm loop planner |
| Crazyfellow race planner | https://uma.pwnation.net/ | Race rotation / hidden gene conditions |
| GameTora compatibility | https://gametora.com/umamusume/compatibility | Official-data compatibility calculator |

---

## Bot / KB relevance

| Concern | Where it lands |
|---------|----------------|
| End-of-run **generation** of red/blue/white/green genes | `parent_farming_utility.md` |
| Mid-run **inheritance procs** (compatibility × base rates) | same + Crazyfellow tables |
| **Pre-run** tree design: which parents to borrow, which ★ totals unlock target aptitudes | this page / design.u-ma.org |
| Career training `buildTargets` | distance/CM floors — not this planner |

The bot does not need to reimplement the UI. The ★-sum table above is the mechanical piece worth encoding if we
ever validate OCR’d aptitudes at career start against expected inheritance, or recommend parent borrows.
