# Training stat gain (Global)

Cross-validated from uma.guide / umareference / GameTora `support_effects.calc`.

## Formula (conceptual)

```
StatGain = floor(
  (BaseTraining + StatBonus)
  × ∏(1 + FriendshipBonus_i / 100)     # multiplicative across friendship cards
  × (1 + BaseMood × (1 + ΣMoodEffect/100))
  × (1 + ΣTrainingEffectiveness/100)   # additive within type, then mult across types
  × (1 + 0.05 × NumCharactersPresent)
  × (1 + UmaGrowth/100)
)
```

### Critical stacking rule

- **Friendship Bonus** stacks **multiplicatively with itself**.
- Other effect types are **additive within type**, then **multiplicative across types**.
- Encoded in GameTora `support_effects[].calc` (`mult` vs additive).

### Friendship training

Requires bond ≥ 80 **and** card on its specialty facility.

### Mood multiplier (BaseMood)

Approx ±0.1 per step from Normal (Great +20% … Awful −20%). Support **Mood Effect** amplifies this.

### Soft / hard caps (July 2026)

- Soft: gains above **1200** halved; in-race contribution above 1200 roughly halved.
- Hard: per-scenario caps (see `scenarios/*.md` and GameTora `hard_caps`).

### Facility level

Training level multiplies base values (Lv1 ≈ 1.0× … Lv5 ≈ 2.0× — confirm against charts when implementing). Energy cost rises with level.

### Bond gain (typical)

Regular +7 · Hint training +5 · Support event +5/+10.

## Implementation note

Bot decision engine should consume KB `support_card.payload.effects` + deck uncap level → breakpoint values, not hand-tuned sliders alone.
