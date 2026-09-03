use crate::config::{EventProbabilityConfig, HintProgressionConfig};
use crate::rng::SimRandom;
use crate::scoring::{
    sample_event_reward as scoring_sample_event_reward, EventEffectReading as ScoringReading,
};
use crate::state::{shift_mood, CareerState, StatName, TrainingFacility};

pub use crate::scoring::EventEffectReading;

fn to_sim_reading(r: ScoringReading) -> EventEffectReading {
    r
}

pub fn sample_event_reward(
    reward_text: &str,
    branch_roll: f64,
    energy_roll: f64,
) -> EventEffectReading {
    to_sim_reading(scoring_sample_event_reward(
        reward_text,
        branch_roll,
        energy_roll,
    ))
}

pub struct EventEffectApplier;

impl EventEffectApplier {
    pub fn apply(
        state: &CareerState,
        reward_text: &str,
        rng: &mut SimRandom,
    ) -> (CareerState, Vec<String>) {
        let branch_roll = rng.next_double();
        let energy_roll = rng.next_double();
        let mut reading = sample_event_reward(reward_text, branch_roll, energy_roll);
        if EventProbabilityConfig::matches_energy_variance(reward_text) && reading.energy_delta != 0
        {
            reading.energy_delta = EventProbabilityConfig::pick_energy_variance(rng);
        }
        Self::apply_reading(state, &reading, Some(rng))
    }

    pub fn apply_reading(
        state: &CareerState,
        reading: &EventEffectReading,
        mut rng: Option<&mut SimRandom>,
    ) -> (CareerState, Vec<String>) {
        let mut lines = Vec::new();
        let mut s = state.clone();

        if reading.dating {
            s.scenario_resources = s.scenario_resources.set("dating_unlocked", 1);
            lines.push("Dating unlocked".to_string());
        }
        if reading.energy_delta != 0 {
            s.energy = (s.energy + reading.energy_delta).clamp(0, s.max_energy);
            lines.push(format!("Energy {}", reading.energy_delta));
        }
        if reading.mood_delta != 0 {
            s.mood = shift_mood(s.mood, reading.mood_delta);
            lines.push(format!("Mood {}", reading.mood_delta));
        }
        for (stat, amt) in &reading.stats {
            if *amt == 0 {
                continue;
            }
            let fac = stat.to_facility();
            s.stats = s.stats.with_delta(fac, *amt);
            lines.push(format!("{stat:?} +{amt}"));
        }
        if reading.random_stat_gain > 0 {
            let pick = if let Some(ref mut rng) = rng {
                let idx = rng.next_int_until(5);
                StatName::ALL[idx as usize]
            } else {
                StatName::Speed
            };
            s.stats = s
                .stats
                .with_delta(pick.to_facility(), reading.random_stat_gain);
            lines.push(format!("Random stat +{}", reading.random_stat_gain));
        }
        if reading.all_stats_gain > 0 {
            for fac in TrainingFacility::ALL {
                s.stats = s.stats.with_delta(fac, reading.all_stats_gain);
            }
            lines.push(format!("All stats +{}", reading.all_stats_gain));
        }
        if reading.skill_pts != 0 {
            s.skill_points = (s.skill_points + reading.skill_pts).max(0);
            lines.push(format!("Skill points +{}", reading.skill_pts));
        }
        if !reading.hints.is_empty() {
            let mut hints = s.hint_levels.clone();
            let skills_event =
                crate::catalog::trainee::TraineeCatalog::lookup(&s.meta.trainee_name)
                    .map(|t| t.skills_event.clone())
                    .unwrap_or_default();
            for hint in &reading.hints {
                let key = crate::catalog::skill::resolve_hint_key(
                    hint,
                    &skills_event,
                    rng.as_deref_mut(),
                );
                let cur = hints.get(&key).copied().unwrap_or(0);
                hints.insert(key, HintProgressionConfig::apply_event_hint(cur));
            }
            s.hint_levels = hints;
            lines.push(format!("Hints: {}", reading.hints.len()));
        }
        if !reading.positive_statuses.is_empty() || !reading.negative_statuses.is_empty() {
            let mut st: Vec<String> = s
                .statuses
                .iter()
                .cloned()
                .chain(reading.positive_statuses.iter().cloned())
                .collect();
            st.retain(|x| !reading.negative_statuses.contains(x));
            st.sort();
            st.dedup();
            s.statuses = st;
            lines.push("Statuses updated".to_string());
        }
        if !reading.performance_tokens.is_empty() {
            let mut tokens = s.performance_tokens.clone();
            for (k, v) in &reading.performance_tokens {
                *tokens.entry(k.clone()).or_insert(0) += v;
            }
            let sum: i32 = reading.performance_tokens.values().sum();
            s.performance_tokens = tokens;
            lines.push(format!("Performance tokens +{sum}"));
        }

        (s, lines)
    }
}

#[derive(Debug, Clone)]
pub struct SimEventEntry {
    pub id: String,
    pub title: String,
    pub owner_kind: String,
    pub owner_name: String,
    pub options: Vec<String>,
}

pub trait EventCatalog: Send + Sync {
    fn pick_random(
        &self,
        trainee_name: &str,
        turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry>;
}

pub struct BuiltinEventCatalog;

impl EventCatalog for BuiltinEventCatalog {
    fn pick_random(
        &self,
        trainee_name: &str,
        _turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry> {
        let samples = vec![
            SimEventEntry {
                id: "event:trainee:Special Week:fan_letter".to_string(),
                title: "Fan Letter".to_string(),
                owner_kind: "trainee".to_string(),
                owner_name: "Special Week".to_string(),
                options: vec![
                    "Energy +10\nMood +1".to_string(),
                    "Speed +5\nSkill points +15".to_string(),
                ],
            },
            SimEventEntry {
                id: "event:trainee:Special Week:extra_training".to_string(),
                title: "Extra Training".to_string(),
                owner_kind: "trainee".to_string(),
                owner_name: "Special Week".to_string(),
                options: vec![
                    "Energy -10\nSpeed +15".to_string(),
                    "Energy -5\nStamina +10".to_string(),
                ],
            },
            SimEventEntry {
                id: "event:shared:failed_training".to_string(),
                title: "Failed Training (Get Well Soon!)".to_string(),
                owner_kind: "shared".to_string(),
                owner_name: String::new(),
                options: vec![
                    "Energy +20".to_string(),
                    "Randomly either\n----------\nEnergy +30\n----------\nMood +1".to_string(),
                ],
            },
        ];
        let pool: Vec<_> = samples
            .into_iter()
            .filter(|e| {
                e.owner_kind == "shared"
                    || e.owner_name.eq_ignore_ascii_case(trainee_name)
                    || trainee_name.contains("Special Week")
            })
            .collect();
        if pool.is_empty() {
            return None;
        }
        let idx = rng.next_int_until(pool.len() as i32) as usize;
        Some(pool[idx].clone())
    }
}

impl BuiltinEventCatalog {
    pub fn pick_random(
        trainee_name: &str,
        turn: i32,
        rng: &mut SimRandom,
    ) -> Option<SimEventEntry> {
        <Self as EventCatalog>::pick_random(&Self, trainee_name, turn, rng)
    }
}
