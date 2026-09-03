use crate::state::MoodLevel;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SupportEffectSlice {
    pub friendship_bonus_pct: f64,
    pub mood_effect_pct: f64,
    pub training_effectiveness_pct: f64,
    pub on_specialty: bool,
}

pub fn apply_training_multipliers(
    base_plus_stat_bonus: f64,
    supports: &[SupportEffectSlice],
    mood: MoodLevel,
    num_characters_present: i32,
    uma_growth_pct: f64,
) -> i32 {
    let mut friendship_prod = 1.0;
    let mut mood_effect_sum = 0.0;
    let mut training_eff_sum = 0.0;
    for s in supports {
        if s.on_specialty && s.friendship_bonus_pct != 0.0 {
            friendship_prod *= 1.0 + s.friendship_bonus_pct / 100.0;
        }
        mood_effect_sum += s.mood_effect_pct;
        training_eff_sum += s.training_effectiveness_pct;
    }
    let mood_mult = 1.0 + mood.base_mood() * (1.0 + mood_effect_sum / 100.0);
    let training_eff_mult = 1.0 + training_eff_sum / 100.0;
    let presence_mult = 1.0 + 0.05 * num_characters_present as f64;
    let growth_mult = 1.0 + uma_growth_pct / 100.0;
    (base_plus_stat_bonus * friendship_prod * mood_mult * training_eff_mult * presence_mult
        * growth_mult)
        .floor() as i32
}

impl MoodLevel {
    pub fn base_mood(self) -> f64 {
        match self {
            MoodLevel::Great => 0.2,
            MoodLevel::Good => 0.1,
            MoodLevel::Normal => 0.0,
            MoodLevel::Bad => -0.1,
            MoodLevel::Awful => -0.2,
        }
    }

    pub fn from_scoring_name(value: &str) -> Option<MoodLevel> {
        match value.to_uppercase().as_str() {
            "GREAT" => Some(MoodLevel::Great),
            "GOOD" => Some(MoodLevel::Good),
            "NORMAL" => Some(MoodLevel::Normal),
            "BAD" => Some(MoodLevel::Bad),
            "AWFUL" => Some(MoodLevel::Awful),
            _ => None,
        }
    }
}

pub fn expected_value_under_failure(
    gain_score: f64,
    failure_chance_percent: i32,
    fail_penalty: f64,
) -> f64 {
    let clamped = if failure_chance_percent < 0 {
        0
    } else {
        failure_chance_percent.clamp(0, 100)
    };
    let p_fail = clamped as f64 / 100.0;
    (1.0 - p_fail) * gain_score - p_fail * fail_penalty
}

pub fn mood_adjust_score(raw_score: f64, mood: MoodLevel) -> f64 {
    raw_score * (1.0 + mood.base_mood())
}
