# Race HP / speed model (reference constants)

Documented from community race-mechanics writeups. Use to **validate** ports; do not copy GPL sources.

## Max HP

`MaxHP = 0.8 × StrategyCoef × Stamina + CourseDistance`

StrategyCoef (approx): Runaway 0.86 · Pace Chaser 0.89 · Front Runner 0.95 · End Closer 0.995 · Late Surger 1.0.

Styles convert stamina→HP differently; they do not inherently “drain” at different rates.

## HP consumption /s

`20 × (CurrentSpeed − BaseSpeed + 12)² / 144 × StatusModifier × GroundModifier`

StatusModifier examples: rushed 1.6× · pace-down 0.6× · downhill accel 0.4×.

## Guts (late race / last spurt HP)

`GutsModifier = 1 + 200/√(600 × Guts)` applied to late-race / last-spurt HP consumption.

## Last spurt (high level)

Last spurt speed combines phase-2 target speed, Speed, distance proficiency, and a Guts term (post-1st-anniversary). Wit affects spurt candidate acceptance ≈ `(15 + 0.05 × Wit)%` per candidate.

## Base speed

`BaseSpeed = 20 − (CourseDistance − 2000) / 1000` (m/s).

## Phases

Opening `0 .. 1/6`, Middle `1/6 .. 2/3`, Final `2/3 .. 5/6`, Last Spurt `5/6 .. 1` of distance.

## Strategy phase speed multipliers

From Global 5th Anniversary reference (Great Escape / Runner / Leader / Betweener / Chaser ≈ Oonige / Nige / Senkou / Sasi / Oikomi):

| Strategy | Opening | Middle | Final+Spurt |
| --- | --- | --- | --- |
| Great Escape | 1.063 | 0.962 | 0.95 |
| Runner | 1.0 | 0.98 | 0.962 |
| Leader | 0.978 | 0.991 | 0.975 |
| Betweener | 0.938 | 0.998 | 0.994 |
| Chaser | 0.931 | 1.0 | 1.0 |

## Strategy phase acceleration multipliers

| Strategy | Opening | Middle | Final+Spurt |
| --- | --- | --- | --- |
| Great Escape | 1.17 | 0.94 | 0.956 |
| Runner | 1.0 | 1.0 | 0.996 |
| Leader | 0.985 | 1.0 | 0.996 |
| Betweener | 0.975 | 1.0 | 1.0 |
| Chaser | 0.945 | 1.0 | 0.997 |

## Target speed (community / KuromiAK)

Distance proficiency (S→G): `1.05, 1.0, 0.9, 0.8, 0.6, 0.4, 0.2, 0.1`.

Opening / middle (phases 0–1): Speed does not enter;  
`BaseTargetSpeed = BaseSpeed × StrategyPhaseSpeedCoef`.

Final / last-spurt phase target (before choosing last-spurt speed):  
`BaseTargetSpeed = BaseSpeed × StrategyPhaseSpeedCoef + √(500 × Speed) × DistProf × 0.002`.

Last-spurt speed (post-1st-anniversary guts term):  
`LastSpurtSpeed = (BaseTargetSpeed_phase2 + 0.01 × BaseSpeed) × 1.05
 + √(500 × Speed) × DistProf × 0.002
 + (450 × Guts)^0.597 × 0.0001`.

## Acceleration

`BaseAccel = 0.0006` (uphill `0.0004`).  
`Accel = BaseAccel × √(500 × Power) × AccelPhaseCoef × GroundApt × DistApt`.

Ground aptitude (S→G): `1.05, 1.0, 0.9, 0.8, 0.7, 0.5, 0.3, 0.1`.  
Distance aptitude for accel (S→G): `1.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.5, 0.4`.

Phase decelerations when above target: about `-1.2 / -0.8 / -1.0` m/s² (opening / middle / final).  
Gate delay is uniform in `[0, 0.1]` s.

## Target speed modifiers

Uphill penalty: `(slope/10000) × 200 / Power` (not √(500×Power)).  
Downhill accel mode bonus: `0.3 + slope/100000` (slope negative on downhill); HP ×0.4 while active.  
Enter downhill mode: each second on a downhill, chance `Wisdom × 0.0004`; 20%/s to end.

Course-set-status (threshold stats on the course): average of per-stat bonuses  
`(+0.05/+0.10/+0.15/+0.20` for ≤300 / ≤600 / ≤900 / >900), then  
`adjustedSpeed *= (1 + average)`. Cap input stats at 901 for the threshold check.

Section Wiz randomness (24 equal sections; not applied in last spurt):  
`max = (Wiz/5500)×log10(Wiz×0.1)`, `min = max − 0.65` (percent),  
`bonus = BaseSpeed × (min + U(0,1)×(max−min)) / 100`.

Start dash ends at `0.85 × BaseSpeed` (cap while dashing); +24 m/s² while active.  
Minimum speed: `0.85 × BaseSpeed + √(200 × Guts) × 0.001`.

## Skills

Evaluate GameTora `condition_groups` / `static/skill_conditions` expressions inside this model for PvP raceability objective.

## External technical reference

KuromiAK race mechanics writeup (community): https://docs.google.com/document/d/15VzW9W2tXBBTibBRbZ8IVpW6HaMX8H0RP03kq6Az7Xg/edit  
Local export: `knowledge/raw/references/kuromiak_race_mechanics.txt` (formulas in the Google Doc are often images; tables above are transcribed from the export + Global 5th Anniversary reference).
