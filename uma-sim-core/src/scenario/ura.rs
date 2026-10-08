//! URA Finale Happy Meek duel — badge spawn, contest predictions, win/loss RNG.

use crate::rng::SimRandom;
use crate::state::{shift_mood, CareerState, ScenarioResources, StatName, TrainingFacility};
use serde_json::Value;
use std::sync::{LazyLock, Mutex};

static FACILITIES: [TrainingFacility; 5] = TrainingFacility::ALL;

#[derive(Clone)]
struct UraConfig {
    enabled: bool,
    spawn_chance_per_turn: f64,
    min_turn: i32,
    max_meek_level: i32,
    duel_failure_acceptable_pct: i32,
    duel_training_bias_moderate: f64,
    cap_raise_on_win: i32,
    skill_points_on_win: i32,
    energy_on_energy_contest_win: i32,
    loss_energy: i32,
    loss_mood_delta: i32,
    prediction_weights: [i32; 4],
    win_chance_by_prediction: [f64; 4],
    stat_gain_by_level: Vec<i32>,
}

impl Default for UraConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            spawn_chance_per_turn: 0.12,
            min_turn: 6,
            max_meek_level: 5,
            duel_failure_acceptable_pct: 40,
            duel_training_bias_moderate: 1.25,
            cap_raise_on_win: 50,
            skill_points_on_win: 15,
            energy_on_energy_contest_win: 30,
            loss_energy: 15,
            loss_mood_delta: -1,
            prediction_weights: [15, 30, 35, 20],
            win_chance_by_prediction: [0.92, 0.72, 0.45, 0.22],
            stat_gain_by_level: vec![15, 20, 25, 30, 35],
        }
    }
}

static CONFIG: LazyLock<Mutex<UraConfig>> = LazyLock::new(|| Mutex::new(UraConfig::default()));

fn cfg() -> UraConfig {
    CONFIG.lock().unwrap().clone()
}

pub struct UraMechanics;

impl UraMechanics {
    pub fn enabled() -> bool {
        cfg().enabled
    }

    pub fn set_enabled(v: bool) {
        CONFIG.lock().unwrap().enabled = v;
    }

    pub fn spawn_chance_per_turn() -> f64 {
        cfg().spawn_chance_per_turn
    }

    pub fn min_turn() -> i32 {
        cfg().min_turn
    }

    pub fn max_meek_level() -> i32 {
        cfg().max_meek_level
    }

    pub fn duel_training_bias_moderate() -> f64 {
        cfg().duel_training_bias_moderate
    }

    pub fn duel_failure_acceptable_pct() -> i32 {
        cfg().duel_failure_acceptable_pct
    }

    pub fn cap_raise_on_win() -> i32 {
        cfg().cap_raise_on_win
    }

    pub fn skill_points_on_win() -> i32 {
        cfg().skill_points_on_win
    }

    pub fn energy_on_energy_contest_win() -> i32 {
        cfg().energy_on_energy_contest_win
    }

    pub fn loss_energy() -> i32 {
        cfg().loss_energy
    }

    pub fn loss_mood_delta() -> i32 {
        cfg().loss_mood_delta
    }

    fn prediction_weights() -> [i32; 4] {
        cfg().prediction_weights
    }

    fn win_chance_by_prediction() -> [f64; 4] {
        cfg().win_chance_by_prediction
    }

    fn stat_gain_by_level() -> Vec<i32> {
        cfg().stat_gain_by_level
    }

    pub fn load_research(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let Some(meek) = root.get("happy_meek").and_then(|v| v.as_object()) else {
            return;
        };

        let mut c = CONFIG.lock().unwrap();

        if let Some(v) = meek.get("enabled").and_then(|v| v.as_str()) {
            c.enabled = v != "false";
        }
        if let Some(v) = meek.get("spawn_chance_per_turn").and_then(|v| v.as_f64()) {
            c.spawn_chance_per_turn = v;
        }
        if let Some(v) = meek.get("min_turn").and_then(|v| v.as_i64()) {
            c.min_turn = v as i32;
        }
        if let Some(v) = meek.get("max_level").and_then(|v| v.as_i64()) {
            c.max_meek_level = v as i32;
        }
        if let Some(v) = meek
            .get("duel_failure_acceptable_pct")
            .and_then(|v| v.as_i64())
        {
            c.duel_failure_acceptable_pct = (v as i32).clamp(0, 100);
        }
        if let Some(v) = meek
            .get("duel_training_bias_moderate")
            .and_then(|v| v.as_f64())
        {
            c.duel_training_bias_moderate = v;
        }
        if let Some(rewards) = meek.get("win_rewards").and_then(|v| v.as_object()) {
            if let Some(v) = rewards.get("cap_raise").and_then(|v| v.as_i64()) {
                c.cap_raise_on_win = v as i32;
            }
            if let Some(v) = rewards.get("skill_points").and_then(|v| v.as_i64()) {
                c.skill_points_on_win = v as i32;
            }
            if let Some(v) = rewards
                .get("energy_on_energy_contest")
                .and_then(|v| v.as_i64())
            {
                c.energy_on_energy_contest_win = v as i32;
            }
            if let Some(arr) = rewards.get("stat_gain_by_level").and_then(|v| v.as_array()) {
                let parsed: Vec<i32> = arr
                    .iter()
                    .filter_map(|v| v.as_i64().map(|n| n as i32))
                    .collect();
                if !parsed.is_empty() {
                    c.stat_gain_by_level = parsed;
                }
            }
        }
        if let Some(loss) = meek.get("loss_penalty").and_then(|v| v.as_object()) {
            if let Some(v) = loss.get("energy").and_then(|v| v.as_i64()) {
                c.loss_energy = v as i32;
            }
            if let Some(v) = loss.get("mood").and_then(|v| v.as_i64()) {
                c.loss_mood_delta = v as i32;
            }
        }
        if let Some(w) = meek.get("prediction_weights").and_then(|v| v.as_object()) {
            c.prediction_weights = [
                w.get("great").and_then(|v| v.as_i64()).unwrap_or(15) as i32,
                w.get("good").and_then(|v| v.as_i64()).unwrap_or(30) as i32,
                w.get("bad").and_then(|v| v.as_i64()).unwrap_or(35) as i32,
                w.get("worst").and_then(|v| v.as_i64()).unwrap_or(20) as i32,
            ];
        }
        if let Some(w) = meek
            .get("win_chance_by_prediction")
            .and_then(|v| v.as_object())
        {
            c.win_chance_by_prediction = [
                w.get("great").and_then(|v| v.as_f64()).unwrap_or(0.92),
                w.get("good").and_then(|v| v.as_f64()).unwrap_or(0.72),
                w.get("bad").and_then(|v| v.as_f64()).unwrap_or(0.45),
                w.get("worst").and_then(|v| v.as_f64()).unwrap_or(0.22),
            ];
        }
    }

    pub fn meek_level(resources: &ScenarioResources) -> i32 {
        resources
            .get("happy_meek_level")
            .clamp(0, Self::max_meek_level())
    }

    pub fn badge_facility(resources: &ScenarioResources) -> Option<TrainingFacility> {
        let badge = resources.get("happy_meek_badge");
        if badge <= 0 {
            return None;
        }
        FACILITIES.get((badge - 1) as usize).copied()
    }

    pub fn cap_bonus(resources: &ScenarioResources, stat_key: &str) -> i32 {
        resources.get(&format!("ura_cap_bonus_{stat_key}"))
    }

    pub fn roll_badge_on_turn(state: &CareerState) -> ScenarioResources {
        let mut res = state.scenario_resources.set("happy_meek_badge", 0);
        if !Self::enabled() || state.turn < Self::min_turn() {
            return res;
        }
        let rng_seed = state.meta.seed ^ ((state.turn as i64) << 20);
        let mut rng = SimRandom::new(rng_seed);
        if !rng.next_boolean(Self::spawn_chance_per_turn()) {
            return res;
        }
        let idx = rng.next_int_until(FACILITIES.len() as i32);
        let facility = FACILITIES[idx as usize];
        res = res.set("happy_meek_badge", facility.ordinal() + 1);
        res
    }

    pub fn build_duel_options(rng: &mut SimRandom, meek_level: i32) -> Vec<String> {
        Self::build_contests(rng)
            .into_iter()
            .map(|c| Self::format_contest_option(&c, meek_level))
            .collect()
    }

    pub fn build_contests(rng: &mut SimRandom) -> Vec<DuelContest> {
        let mut contests: Vec<DuelContest> = FACILITIES
            .iter()
            .map(|&fac| DuelContest {
                facility: Some(fac),
                prediction: Self::roll_prediction(rng),
            })
            .collect();
        contests.push(DuelContest {
            facility: None,
            prediction: Self::roll_prediction(rng),
        });
        contests
    }

    pub fn format_contest_option(contest: &DuelContest, meek_level: i32) -> String {
        let label = contest
            .facility
            .map(|f| format!("Contest of {}!", capitalize(&f.key().to_string())))
            .unwrap_or_else(|| "Contest of energy!".to_string());
        let odds = match contest.prediction {
            DuelPrediction::Great => "Great odds",
            DuelPrediction::Good => "Good odds",
            DuelPrediction::Bad => "Bad odds",
            DuelPrediction::Worst => "Worst odds",
        };
        let reward = contest
            .facility
            .map(|f| {
                let gain = Self::stat_gain_for_level(meek_level);
                format!("{} +{gain}", capitalize(&f.key().to_string()))
            })
            .unwrap_or_else(|| format!("Energy +{}", Self::energy_on_energy_contest_win()));
        format!("{label}\n{reward} ({odds})")
    }

    pub fn parse_contest(option: &str) -> DuelContest {
        let lower = option.to_lowercase();
        let facility = FACILITIES.iter().find(|f| lower.contains(f.key())).copied();
        let prediction = if lower.contains("great odds") {
            DuelPrediction::Great
        } else if lower.contains("good odds") {
            DuelPrediction::Good
        } else if lower.contains("bad odds") {
            DuelPrediction::Bad
        } else {
            DuelPrediction::Worst
        };
        DuelContest {
            facility,
            prediction,
        }
    }

    /// True when implied failure % is within `duel_failure_acceptable_pct`.
    pub fn prediction_failure_acceptable(prediction: DuelPrediction) -> bool {
        let win = Self::win_chance_by_prediction()[prediction as usize];
        let fail_pct = ((1.0 - win) * 100.0).round() as i32;
        fail_pct <= Self::duel_failure_acceptable_pct()
    }

    pub fn choose_duel_contest_index(options: &[String], priorities: &[StatName]) -> i32 {
        if options.is_empty() {
            return 0;
        }
        let contests: Vec<(i32, DuelContest)> = options
            .iter()
            .enumerate()
            .map(|(i, o)| (i as i32, Self::parse_contest(o)))
            .collect();
        let good_odds: Vec<(i32, DuelContest)> = contests
            .iter()
            .filter(|(_, c)| {
                c.facility.is_some()
                    && priorities.contains(&c.facility.unwrap().to_stat_name())
                    && c.prediction as i32 <= DuelPrediction::Good as i32
            })
            .cloned()
            .collect();
        // When no Good+ priority target: accept Bad/Worst only if failure % ≤ research pct.
        let acceptable: Vec<(i32, DuelContest)> = contests
            .iter()
            .filter(|(_, c)| {
                c.facility.is_some()
                    && priorities.contains(&c.facility.unwrap().to_stat_name())
                    && Self::prediction_failure_acceptable(c.prediction)
            })
            .cloned()
            .collect();
        let pool: &Vec<(i32, DuelContest)> = if !good_odds.is_empty() {
            &good_odds
        } else if !acceptable.is_empty() {
            &acceptable
        } else {
            &contests
        };
        pool.iter()
            .min_by(|a, b| {
                a.1.prediction
                    .cmp(&b.1.prediction)
                    .then_with(|| {
                        let rank_a =
                            a.1.facility
                                .map(|f| {
                                    priorities
                                        .iter()
                                        .position(|s| *s == f.to_stat_name())
                                        .unwrap_or(usize::MAX)
                                })
                                .unwrap_or(usize::MAX);
                        let rank_b =
                            b.1.facility
                                .map(|f| {
                                    priorities
                                        .iter()
                                        .position(|s| *s == f.to_stat_name())
                                        .unwrap_or(usize::MAX)
                                })
                                .unwrap_or(usize::MAX);
                        rank_a.cmp(&rank_b)
                    })
                    .then(a.0.cmp(&b.0))
            })
            .map(|(i, _)| *i)
            .unwrap_or(0)
    }

    pub fn resolve_duel(
        state: &CareerState,
        option: &str,
        rng: &mut SimRandom,
    ) -> (CareerState, Vec<String>) {
        let contest = Self::parse_contest(option);
        let level = Self::meek_level(&state.scenario_resources);
        let win_chance = Self::win_chance_by_prediction()[contest.prediction as usize];
        let won = rng.next_boolean(win_chance);
        let mut lines = Vec::new();
        let mut res = state
            .scenario_resources
            .set("happy_meek_badge", 0)
            .set("happy_meek_pending", 0);
        let mut s = state.clone();
        s.scenario_resources = res.clone();

        if won {
            let new_level = (level + 1).min(Self::max_meek_level());
            res = res.set("happy_meek_level", new_level);
            s.scenario_resources = res.clone();
            s.skill_points += Self::skill_points_on_win();
            lines.push(format!("Happy Meek duel WON (L{level}→L{new_level})"));
            // Beating max-level Meek unlocks Past My Limits (URA.md).
            if level >= Self::max_meek_level() {
                let skill = "skill:past_my_limits";
                if !s.learned_skill_ids.iter().any(|id| id == skill) {
                    s.learned_skill_ids.push(skill.to_string());
                    lines.push("Unlocked Past My Limits".to_string());
                }
            }
            if let Some(facility) = contest.facility {
                let gain = Self::stat_gain_for_level(level);
                let key = facility.key();
                s.stats = s.stats.with_delta(facility, gain);
                res = res.add(&format!("ura_cap_bonus_{key}"), Self::cap_raise_on_win());
                // Racing Spirit family hint (URA.md) — one level per facility duel win.
                let hint_key = format!("racing_spirit_{key}");
                let hint_lv = s.hint_levels.get(&hint_key).copied().unwrap_or(0) + 1;
                s.hint_levels.insert(hint_key.clone(), hint_lv);
                s.scenario_resources = res;
                lines.push(format!(
                    "{} +{gain}, cap +{}, hint {hint_key} L{hint_lv}",
                    facility.name(),
                    Self::cap_raise_on_win()
                ));
            } else {
                s.energy = (s.energy + Self::energy_on_energy_contest_win()).min(s.max_energy);
                s.scenario_resources = res;
                lines.push(format!("Energy +{}", Self::energy_on_energy_contest_win()));
            }
        } else {
            lines.push("Happy Meek duel LOST".to_string());
            s.energy = (s.energy - Self::loss_energy()).max(0);
            s.mood = shift_mood(s.mood, Self::loss_mood_delta());
            s.scenario_resources = res;
            lines.push(format!("Energy -{}", Self::loss_energy()));
        }
        (s, lines)
    }

    pub fn apply_duel_training_bias(
        base_score: f64,
        facility: TrainingFacility,
        resources: &ScenarioResources,
        failure_chance_pct: i32,
        max_failure_chance: i32,
    ) -> f64 {
        if base_score <= 0.0 {
            return base_score;
        }
        let Some(badge) = Self::badge_facility(resources) else {
            return base_score;
        };
        if badge != facility {
            return base_score;
        }
        if !(0..=max_failure_chance).contains(&failure_chance_pct) {
            return base_score;
        }
        base_score * Self::duel_training_bias_moderate()
    }

    fn stat_gain_for_level(level: i32) -> i32 {
        let gains = Self::stat_gain_by_level();
        let idx = level.min(gains.len() as i32 - 1).max(0) as usize;
        *gains.get(idx).unwrap_or_else(|| gains.last().unwrap())
    }

    fn roll_prediction(rng: &mut SimRandom) -> DuelPrediction {
        let weights = Self::prediction_weights();
        let total: i32 = weights.iter().sum();
        let mut roll = rng.next_int_until(total);
        for (idx, weight) in weights.iter().enumerate() {
            roll -= weight;
            if roll < 0 {
                return match idx {
                    0 => DuelPrediction::Great,
                    1 => DuelPrediction::Good,
                    2 => DuelPrediction::Bad,
                    _ => DuelPrediction::Worst,
                };
            }
        }
        DuelPrediction::Worst
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DuelPrediction {
    Great = 0,
    Good = 1,
    Bad = 2,
    Worst = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelContest {
    pub facility: Option<TrainingFacility>,
    pub prediction: DuelPrediction,
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
