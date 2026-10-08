use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use super::decision_context::{mood_ordinal, DecisionContext};
use super::types::StatName;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventEffectReading {
    pub energy_delta: i32,
    pub energy_is_range: bool,
    pub mood_delta: i32,
    pub stats: HashMap<StatName, i32>,
    pub random_stat_gain: i32,
    pub all_stats_gain: i32,
    pub skill_pts: i32,
    pub hints: Vec<String>,
    pub bond: i32,
    pub positive_statuses: Vec<String>,
    pub negative_statuses: Vec<String>,
    pub performance_tokens: HashMap<String, i32>,
    pub random_branch: bool,
    pub dating: bool,
    pub chain_end: bool,
    pub random_tagged: bool,
}

static ENERGY_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)energy\s*([+-]?\d+)(?:\s*/\s*([+-]?\d+))?").unwrap());
static MOOD_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)mood\s*([+-]?\d+)").unwrap());
static STAT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(speed|stamina|power|guts|wit|wisdom)\s*([+-]?\d+)").unwrap()
});
static RANDOM_STAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+)\s*random\s*stat\s*\+?\s*(\d+)").unwrap());
static ALL_STATS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)all\s*stats?\s*\+?\s*(\d+)").unwrap());
static SKILL_PTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)skill\s*points?\s*([+-]?\d+)").unwrap());
static BOND_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)bond\s*([+-]?\d+)").unwrap());
static HINT_NAMED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([A-Za-z][A-Za-z0-9' \-☆★]+?)\s+hint\s*\+?\s*\d+").unwrap());
static PERF_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(dance|passion|vocal|visual|composure|mental)\s*\+?\s*(\d+)").unwrap()
});
static RANDOMLY_EITHER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)randomly\s+either").unwrap());
static BRANCH_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-{5,}").unwrap());

const POSITIVE_STATUS_KEYWORDS: &[&str] = &[
    "practice perfect",
    "charming",
    "fast learner",
    "hot topic",
    "good condition",
];
const NEGATIVE_STATUS_KEYWORDS: &[&str] = &[
    "practice poor",
    "migraine",
    "night owl",
    "slow metabolism",
    "slacker",
    "gloom",
];

pub fn parse_event_reward_text(reward_text: &str) -> EventEffectReading {
    let lower = reward_text.to_lowercase();
    if lower.contains("randomly either")
        || (lower.contains("randomly") && reward_text.contains("----------"))
    {
        let branches = split_random_branches(reward_text);
        if branches.len() >= 2 {
            let readings: Vec<EventEffectReading> = branches
                .iter()
                .map(|b| parse_event_reward_text_leaf(b))
                .collect();
            let mut avg = average_readings(&readings);
            avg.random_branch = true;
            return avg;
        }
    }
    parse_event_reward_text_leaf(reward_text)
}

pub fn event_reward_branches(reward_text: &str) -> Vec<String> {
    let lower = reward_text.to_lowercase();
    if lower.contains("randomly either")
        || (lower.contains("randomly") && reward_text.contains("----------"))
    {
        let branches = split_random_branches(reward_text);
        if branches.len() >= 2 {
            return branches;
        }
    }
    vec![reward_text.to_string()]
}

pub fn sample_event_reward(
    reward_text: &str,
    branch_roll: f64,
    energy_roll: f64,
) -> EventEffectReading {
    let branches = event_reward_branches(reward_text);
    let branch_idx = (branch_roll * branches.len() as f64).floor() as i32;
    let branch_idx = branch_idx.clamp(0, branches.len() as i32 - 1) as usize;
    let branch_text = &branches[branch_idx];
    let leaf = parse_event_reward_text_leaf(branch_text);
    let energy = sample_energy_delta(branch_text, &leaf, energy_roll);
    EventEffectReading {
        energy_delta: energy,
        random_branch: branches.len() > 1,
        ..leaf
    }
}

fn sample_energy_delta(text: &str, leaf: &EventEffectReading, roll: f64) -> i32 {
    if !leaf.energy_is_range {
        return leaf.energy_delta;
    }
    if let Some(m) = ENERGY_LINE.captures(text) {
        let a = m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        let b = m.get(2).and_then(|g| g.as_str().parse::<i32>().ok());
        if let Some(b_val) = b {
            return if roll < 0.5 { a } else { b_val };
        }
    }
    leaf.energy_delta
}

fn split_random_branches(text: &str) -> Vec<String> {
    let after = RANDOMLY_EITHER.split(text).nth(1).unwrap_or("");
    let parts: Vec<String> = BRANCH_SPLIT
        .split(after)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() >= 2 {
        parts
    } else {
        Vec::new()
    }
}

fn average_readings(readings: &[EventEffectReading]) -> EventEffectReading {
    if readings.is_empty() {
        return EventEffectReading::default();
    }
    if readings.len() == 1 {
        return readings[0].clone();
    }
    let n = readings.len() as f64;

    let mut stats = HashMap::new();
    for stat in StatName::ALL {
        let v = (readings
            .iter()
            .map(|r| r.stats.get(&stat).copied().unwrap_or(0))
            .sum::<i32>() as f64
            / n) as i32;
        if v != 0 {
            stats.insert(stat, v);
        }
    }

    let mut token_keys = HashSet::new();
    for r in readings {
        token_keys.extend(r.performance_tokens.keys().cloned());
    }
    let mut tokens = HashMap::new();
    for k in token_keys {
        let v = (readings
            .iter()
            .map(|r| r.performance_tokens.get(&k).copied().unwrap_or(0))
            .sum::<i32>() as f64
            / n) as i32;
        if v != 0 {
            tokens.insert(k, v);
        }
    }

    let mut hints: Vec<String> = readings.iter().flat_map(|r| r.hints.clone()).collect();
    hints.sort();
    hints.dedup();

    let mut positive: Vec<String> = readings
        .iter()
        .flat_map(|r| r.positive_statuses.clone())
        .collect();
    positive.sort();
    positive.dedup();

    let mut negative: Vec<String> = readings
        .iter()
        .flat_map(|r| r.negative_statuses.clone())
        .collect();
    negative.sort();
    negative.dedup();

    EventEffectReading {
        energy_delta: (readings.iter().map(|r| r.energy_delta).sum::<i32>() as f64 / n) as i32,
        energy_is_range: readings.iter().any(|r| r.energy_is_range),
        mood_delta: (readings.iter().map(|r| r.mood_delta).sum::<i32>() as f64 / n) as i32,
        stats,
        random_stat_gain: (readings.iter().map(|r| r.random_stat_gain).sum::<i32>() as f64 / n)
            as i32,
        all_stats_gain: (readings.iter().map(|r| r.all_stats_gain).sum::<i32>() as f64 / n) as i32,
        skill_pts: (readings.iter().map(|r| r.skill_pts).sum::<i32>() as f64 / n) as i32,
        hints,
        bond: (readings.iter().map(|r| r.bond).sum::<i32>() as f64 / n) as i32,
        positive_statuses: positive,
        negative_statuses: negative,
        performance_tokens: tokens,
        dating: readings.iter().any(|r| r.dating),
        chain_end: readings.iter().any(|r| r.chain_end),
        random_tagged: readings.iter().any(|r| r.random_tagged),
        random_branch: false,
    }
}

fn parse_event_reward_text_leaf(reward_text: &str) -> EventEffectReading {
    let mut energy = 0;
    let mut energy_range = false;
    let mut mood = 0;
    let mut stats = HashMap::new();
    let mut random_stat = 0;
    let mut all_stats = 0;
    let mut skill_pts = 0;
    let mut hints = Vec::new();
    let mut bond = 0;
    let mut pos = Vec::new();
    let mut neg = Vec::new();
    let mut tokens = HashMap::new();
    let mut dating = false;
    let mut chain_end = false;
    let mut random_tagged = false;

    for raw_line in reward_text.split('\n') {
        let line = raw_line.trim();
        if line.is_empty() || (line.len() >= 5 && line.chars().take(5).all(|c| c == '-')) {
            continue;
        }
        let lower = line.to_lowercase();

        if lower.contains("can start dating") {
            dating = true;
        }
        if lower.contains("event chain ended") {
            chain_end = true;
        }
        if lower.contains("(random)") {
            random_tagged = true;
        }

        if let Some(m) = ENERGY_LINE.captures(line) {
            let a: i32 = m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            let b = m.get(2).and_then(|g| g.as_str().parse::<i32>().ok());
            if let Some(b_val) = b {
                energy += (a + b_val) / 2;
                energy_range = true;
            } else {
                energy += a;
            }
        }

        if let Some(m) = MOOD_LINE.captures(line) {
            mood += m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        }

        for m in STAT_LINE.captures_iter(line) {
            let name_raw = m
                .get(1)
                .map(|g| g.as_str().to_lowercase())
                .unwrap_or_default();
            let name = if name_raw == "wisdom" {
                "wit"
            } else {
                name_raw.as_str()
            };
            if let Some(stat) = StatName::from_name(name) {
                let amt: i32 = m.get(2).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
                *stats.entry(stat).or_insert(0) += amt;
            }
        }

        if let Some(m) = RANDOM_STAT.captures(line) {
            random_stat += m.get(2).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        }
        if let Some(m) = ALL_STATS.captures(line) {
            all_stats += m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        }
        if let Some(m) = SKILL_PTS.captures(line) {
            skill_pts += m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        }
        if let Some(m) = BOND_LINE.captures(line) {
            bond += m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        }

        if lower.contains("hint") {
            let named = HINT_NAMED
                .captures(line)
                .and_then(|m| m.get(1))
                .map(|g| g.as_str().trim().to_string())
                .unwrap_or_default();
            if !named.is_empty() && !named.eq_ignore_ascii_case("a skill") {
                hints.push(named);
            } else {
                hints.push(String::new());
            }
        }

        for kw in POSITIVE_STATUS_KEYWORDS {
            if lower.contains(kw) {
                pos.push((*kw).to_string());
            }
        }
        for kw in NEGATIVE_STATUS_KEYWORDS {
            if lower.contains(kw) {
                neg.push((*kw).to_string());
            }
        }

        for m in PERF_TOKEN.captures_iter(line) {
            let key = m
                .get(1)
                .map(|g| g.as_str().to_lowercase().replace("mental", "composure"))
                .unwrap_or_default();
            let amt: i32 = m.get(2).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            *tokens.entry(key).or_insert(0) += amt;
        }
    }

    pos.sort();
    pos.dedup();
    neg.sort();
    neg.dedup();

    EventEffectReading {
        energy_delta: energy,
        energy_is_range: energy_range,
        mood_delta: mood,
        stats,
        random_stat_gain: random_stat,
        all_stats_gain: all_stats,
        skill_pts,
        hints,
        bond,
        positive_statuses: pos,
        negative_statuses: neg,
        performance_tokens: tokens,
        dating,
        chain_end,
        random_tagged,
        random_branch: false,
    }
}

pub fn score_event_option(ctx: &DecisionContext, reward_text: &str) -> f64 {
    let reading = parse_event_reward_text(reward_text);
    score_event_reading(ctx, &reading)
}

pub fn score_event_reading(ctx: &DecisionContext, reading: &EventEffectReading) -> f64 {
    let w = ctx.objective_weights().normalized();
    let mut score = 0.0;

    if reading.dating {
        score += 1000.0;
    }
    if reading.chain_end {
        score += if ctx.dating_schedule_enabled && ctx.dating_chain_complete {
            -50.0
        } else {
            -300.0
        };
    }
    if reading.random_tagged {
        score -= 10.0;
    }
    if reading.random_branch {
        score += 25.0;
    }

    if reading.energy_delta != 0 {
        let energy_score = if ctx.prioritize_energy {
            reading.energy_delta as f64 * 100.0
        } else {
            let mult = match ctx.energy {
                e if e < 30 => 4.0,
                e if e < 50 => 3.0,
                e if e < 70 => 2.0,
                e if e >= 90 && reading.energy_delta > 0 => 0.0,
                _ => 1.0,
            };
            reading.energy_delta as f64 * mult
        };
        score += energy_score;
    }

    if reading.mood_delta != 0 {
        if reading.mood_delta < 0 {
            score += if ctx.mood_ordinal <= mood_ordinal::BAD {
                -200.0
            } else {
                -150.0
            };
        } else {
            let mood_gain = match ctx.mood_ordinal {
                mood_ordinal::AWFUL => 150.0,
                mood_ordinal::BAD => 120.0,
                mood_ordinal::NORMAL => 100.0,
                mood_ordinal::GOOD => 80.0,
                _ => 0.0,
            };
            score += mood_gain * reading.mood_delta as f64;
        }
    }

    score += match reading.bond.cmp(&0) {
        std::cmp::Ordering::Greater => 20.0,
        std::cmp::Ordering::Less => -20.0,
        std::cmp::Ordering::Equal => 0.0,
    };

    score += reading.positive_statuses.len() as f64 * 100.0;
    score += reading.negative_statuses.len() as f64 * -25.0;

    score += reading.skill_pts as f64 * (0.5 + 0.5 * (w.career_score + w.pvp_raceability));

    for hint in &reading.hints {
        let key = hint.to_lowercase();
        let key = key.trim();
        score += if key.is_empty() {
            25.0
        } else if ctx
            .preferred_skill_hints
            .iter()
            .any(|pref| pref.contains(key) || key.contains(pref.as_str()))
        {
            80.0
        } else {
            35.0
        };
    }

    let priority = if ctx.event_choice_stat_priority.is_empty() {
        &ctx.stat_prioritization
    } else {
        &ctx.event_choice_stat_priority
    };
    for (stat, amt) in &reading.stats {
        let discounted = ctx.soft_cap_discount(*stat, *amt);
        let idx = priority
            .iter()
            .position(|s| s == stat)
            .map(|i| i as i32)
            .unwrap_or(-1);
        let bonus = match idx {
            0 => 50.0,
            1 => 40.0,
            2 => 30.0,
            3 => 20.0,
            4.. => 10.0,
            _ => 0.0,
        };
        score += (discounted + bonus) * (w.stat_targets + w.career_score + 0.5 * w.pvp_raceability);
    }

    if reading.random_stat_gain > 0 {
        score += reading.random_stat_gain as f64 * 0.8 * (w.stat_targets + w.career_score);
    }
    if reading.all_stats_gain > 0 {
        score += reading.all_stats_gain as f64 * 5.0 * (w.stat_targets + w.career_score);
    }

    let token_sum: i32 = reading.performance_tokens.values().sum();
    if token_sum > 0 {
        score += token_sum as f64 * 8.0 * (w.scenario_completion + 0.5 * w.spark_quality);
    }

    score
}

pub fn choose_best_event_option(ctx: &DecisionContext, rewards: &[String]) -> (usize, Vec<f64>) {
    if rewards.is_empty() {
        return (0, Vec::new());
    }
    let scores: Vec<f64> = rewards.iter().map(|r| score_event_option(ctx, r)).collect();
    let best = scores
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    (best, scores)
}

pub fn owner_match_boost(
    owner_name: &str,
    trainee_name: &str,
    deck_support_names: &[String],
) -> f64 {
    let owner = owner_name.trim().to_lowercase();
    if owner.is_empty() {
        return 0.0;
    }
    let trainee = trainee_name.trim().to_lowercase();
    if !trainee.is_empty()
        && (owner == trainee || owner.contains(&trainee) || trainee.contains(&owner))
    {
        return 0.05;
    }
    for support in deck_support_names {
        let s = support.trim().to_lowercase();
        if !s.is_empty() && (owner == s || owner.contains(&s) || s.contains(&owner)) {
            return 0.04;
        }
    }
    0.0
}
