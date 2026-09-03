//! Trackblazer shop coins + Pro Shop.

use crate::rng::SimRandom;
use crate::state::{CareerState, ScenarioResources, TrainingFacility};
use serde_json::Value;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub effect_text: String,
    pub category: String,
    pub facility: Option<TrainingFacility>,
    pub training_bonus_pct: i32,
    pub training_bonus_turns: i32,
    pub zero_fail_turns: i32,
    pub max_energy_delta: i32,
}

struct TrackblazerConfig {
    shop_interval_turns: i32,
    min_coins_to_open: i32,
    coins_per_race: i32,
    coins_per_climax: i32,
    offers_per_shop: i32,
    /// List-price discount applied at purchase when `is_sale_turn`. Default 0 (parity-safe).
    sale_discount_pct: i32,
    /// Sale every N shop openings (turn % interval == 0 and shop_index % period == 0). 0 = never.
    sale_every_n_shops: i32,
    climax_victory_points: i32,
    catalog: Vec<ShopItem>,
}

impl Default for TrackblazerConfig {
    fn default() -> Self {
        Self {
            shop_interval_turns: 6,
            min_coins_to_open: 50,
            coins_per_race: 40,
            coins_per_climax: 80,
            offers_per_shop: 3,
            sale_discount_pct: 0,
            sale_every_n_shops: 0,
            climax_victory_points: 100,
            catalog: default_catalog(),
        }
    }
}

static CONFIG: LazyLock<Mutex<TrackblazerConfig>> = LazyLock::new(|| {
    Mutex::new(TrackblazerConfig::default())
});

pub struct TrackblazerMechanics;

impl TrackblazerMechanics {
    pub fn load_research(json_text: Option<&str>) {
        let Some(text) = json_text.filter(|s| !s.trim().is_empty()) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let mut cfg = CONFIG.lock().unwrap();
        // Partial test JSON must not leave prior sale discounts sticky across parallel tests.
        cfg.sale_discount_pct = 0;
        cfg.sale_every_n_shops = 0;
        if let Some(shop) = root.get("shop").and_then(|v| v.as_object()) {
            if let Some(v) = shop.get("interval_turns").and_then(|v| v.as_i64()) {
                cfg.shop_interval_turns = v as i32;
            }
            if let Some(v) = shop.get("min_coins_to_open").and_then(|v| v.as_i64()) {
                cfg.min_coins_to_open = v as i32;
            }
            if let Some(v) = shop.get("coins_per_optional_race").and_then(|v| v.as_i64()) {
                cfg.coins_per_race = v as i32;
            }
            if let Some(v) = shop.get("coins_per_climax_race").and_then(|v| v.as_i64()) {
                cfg.coins_per_climax = v as i32;
            }
            if let Some(v) = shop.get("offers_per_shop").and_then(|v| v.as_i64()) {
                cfg.offers_per_shop = v as i32;
            }
            if let Some(v) = shop.get("sale_discount_pct").and_then(|v| v.as_i64()) {
                cfg.sale_discount_pct = (v as i32).clamp(0, 90);
            }
            if let Some(v) = shop.get("sale_every_n_shops").and_then(|v| v.as_i64()) {
                cfg.sale_every_n_shops = (v as i32).max(0);
            }
        }
        if let Some(climax) = root.get("climax").and_then(|v| v.as_object()) {
            if let Some(v) = climax.get("victory_points_per_win").and_then(|v| v.as_i64()) {
                cfg.climax_victory_points = v as i32;
            }
        }
        if let Some(arr) = root.get("shop_items").and_then(|v| v.as_array()) {
            let parsed: Vec<ShopItem> = arr
                .iter()
                .filter_map(|el| {
                    let o = el.as_object()?;
                    let id = o.get("id")?.as_str()?.to_string();
                    let name = o
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&id)
                        .to_string();
                    let cost = o.get("cost")?.as_i64()? as i32;
                    let effect_text = o
                        .get("effect")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let category = o
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Stats")
                        .to_string();
                    let mechanics = o.get("mechanics").and_then(|v| v.as_object());
                    let facility = mechanics
                        .and_then(|m| m.get("facility"))
                        .and_then(|v| v.as_str())
                        .and_then(parse_facility);
                    Some(ShopItem {
                        id,
                        name,
                        cost,
                        effect_text,
                        category,
                        facility,
                        training_bonus_pct: mechanics
                            .and_then(|m| m.get("training_bonus_pct"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32,
                        training_bonus_turns: mechanics
                            .and_then(|m| m.get("training_bonus_turns"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32,
                        zero_fail_turns: mechanics
                            .and_then(|m| m.get("zero_fail_turns"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32,
                        max_energy_delta: mechanics
                            .and_then(|m| m.get("max_energy"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32,
                    })
                })
                .collect();
            if !parsed.is_empty() {
                cfg.catalog = parsed;
            }
        }
    }

    pub fn shop_catalog() -> Vec<ShopItem> {
        CONFIG.lock().unwrap().catalog.clone()
    }

    pub fn race_coin_gain(race_id: &str) -> i32 {
        let cfg = CONFIG.lock().unwrap();
        if race_id.contains("climax") {
            cfg.coins_per_climax
        } else {
            cfg.coins_per_race
        }
    }

    /// Victory points awarded for a climax race win (Twinkle Star Climax).
    pub fn climax_victory_points(race_id: &str, won: bool) -> i32 {
        if !won || !race_id.contains("climax") {
            return 0;
        }
        CONFIG.lock().unwrap().climax_victory_points
    }

    /// Shop openings are on turns divisible by `shop_interval_turns`. Sale when
    /// `sale_every_n_shops > 0` and the opening index (turn/interval) is a multiple.
    pub fn is_sale_turn(turn: i32) -> bool {
        let cfg = CONFIG.lock().unwrap();
        if cfg.sale_discount_pct <= 0 || cfg.sale_every_n_shops <= 0 {
            return false;
        }
        if turn <= 1 || turn % cfg.shop_interval_turns != 0 {
            return false;
        }
        let shop_index = turn / cfg.shop_interval_turns;
        shop_index % cfg.sale_every_n_shops == 0
    }

    pub fn sale_discount_pct(turn: i32) -> i32 {
        if Self::is_sale_turn(turn) {
            CONFIG.lock().unwrap().sale_discount_pct
        } else {
            0
        }
    }

    /// Effective coin cost after optional sale discount (list price unchanged in UI text).
    pub fn effective_cost(list_cost: i32, turn: i32) -> i32 {
        let pct = Self::sale_discount_pct(turn);
        if list_cost <= 0 || pct <= 0 {
            return list_cost;
        }
        (list_cost * (100 - pct) / 100).max(1)
    }

    pub fn should_open_shop(turn: i32, coins: i32) -> bool {
        let cfg = CONFIG.lock().unwrap();
        turn > 1 && turn % cfg.shop_interval_turns == 0 && coins >= cfg.min_coins_to_open
    }

    pub fn format_option(item: &ShopItem) -> String {
        if item.cost <= 0 {
            format!("{}\n{}", item.name, item.effect_text)
        } else {
            format!("{} ({} coins)\n{}", item.name, item.cost, item.effect_text)
        }
    }

    pub fn roll_shop_options(state: &CareerState) -> Vec<String> {
        let cfg = CONFIG.lock().unwrap();
        let coins = state.scenario_resources.get("tb_coins");
        let mut rng = SimRandom::new(state.meta.seed ^ ((state.turn as i64) << 16));
        let affordable: Vec<_> = cfg
            .catalog
            .iter()
            .filter(|it| (1..=coins).contains(&it.cost))
            .cloned()
            .collect();
        let take = cfg.offers_per_shop.min(affordable.len() as i32) as usize;
        let picked = shuffled(&affordable, &mut rng);
        let mut options: Vec<String> = picked.into_iter().take(take).map(|i| Self::format_option(&i)).collect();
        options.push(Self::format_option(&ShopItem {
            id: "skip".to_string(),
            name: "Skip".to_string(),
            cost: 0,
            effect_text: "Energy +10".to_string(),
            category: "Skip".to_string(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        }));
        options
    }

    pub fn item_for_option(option: &str) -> Option<ShopItem> {
        let cfg = CONFIG.lock().unwrap();
        if option.starts_with("Skip") {
            return cfg
                .catalog
                .iter()
                .find(|i| i.id == "skip")
                .cloned()
                .or_else(|| {
                    Some(ShopItem {
                        id: "skip".to_string(),
                        name: "Skip".to_string(),
                        cost: 0,
                        effect_text: "Energy +10".to_string(),
                        category: "Skip".to_string(),
                        facility: None,
                        training_bonus_pct: 0,
                        training_bonus_turns: 0,
                        zero_fail_turns: 0,
                        max_energy_delta: 0,
                    })
                });
        }
        let name = option.split(" (").next()?.trim();
        cfg.catalog.iter().find(|i| i.name == name).cloned()
    }

    pub fn purchase_cost(option: &str) -> i32 {
        Self::item_for_option(option).map(|i| i.cost).unwrap_or(0)
    }

    pub fn apply_purchase(state: &CareerState, option: &str) -> (CareerState, Vec<String>) {
        let Some(item) = Self::item_for_option(option) else {
            return (state.clone(), Vec::new());
        };
        if item.id == "skip" {
            return (state.clone(), Vec::new());
        }
        let mut lines = Vec::new();
        let mut res = state.scenario_resources.clone();
        let mut facility_levels = state.facility_levels.clone();
        let mut max_energy = state.max_energy;
        let charged = Self::effective_cost(item.cost, state.turn);

        if charged > 0 {
            res = res.add("tb_coins", -charged);
            if charged < item.cost {
                lines.push(format!(
                    "Coins -{charged} (sale −{}%)",
                    Self::sale_discount_pct(state.turn)
                ));
            } else {
                lines.push(format!("Coins -{charged}"));
            }
        }
        if let Some(facility) = item.facility {
            let key = facility.key().to_string();
            let level = facility_levels.get(&key).copied().unwrap_or(1);
            facility_levels.insert(key.clone(), (level + 1).min(5));
            lines.push(format!("{} facility +1", facility.name()));
        }
        if item.max_energy_delta != 0 {
            max_energy = (max_energy + item.max_energy_delta).min(150);
            lines.push(format!("Max energy +{}", item.max_energy_delta));
        }
        if item.training_bonus_pct > 0 && item.training_bonus_turns > 0 {
            res = res
                .set("tb_training_bonus_pct", item.training_bonus_pct)
                .set("tb_training_bonus_turns", item.training_bonus_turns);
            lines.push(format!(
                "Training bonus +{}% ({} turns)",
                item.training_bonus_pct, item.training_bonus_turns
            ));
        }
        if item.zero_fail_turns > 0 {
            res = res.set("tb_zero_fail_turns", item.zero_fail_turns);
            lines.push(format!("Zero fail ({} turn)", item.zero_fail_turns));
        }

        let mut s = state.clone();
        s.scenario_resources = res;
        s.facility_levels = facility_levels;
        s.max_energy = max_energy;
        (s, lines)
    }

    /// Apply list-price effect text (stats / energy / mood). Caller should pass career RNG.
    pub fn apply_item_effects(
        state: &CareerState,
        option: &str,
        rng: &mut SimRandom,
    ) -> (CareerState, Vec<String>) {
        let Some(item) = Self::item_for_option(option) else {
            return (state.clone(), Vec::new());
        };
        if item.effect_text.is_empty() {
            return (state.clone(), Vec::new());
        }
        crate::events::EventEffectApplier::apply(state, &item.effect_text, rng)
    }

    pub fn decay_turn_buffs(state: &CareerState) -> CareerState {
        let mut res = state.scenario_resources.clone();
        let bonus_turns = res.get("tb_training_bonus_turns");
        if bonus_turns > 0 {
            let next = bonus_turns - 1;
            res = if next <= 0 {
                res.set("tb_training_bonus_turns", 0)
                    .set("tb_training_bonus_pct", 0)
            } else {
                res.set("tb_training_bonus_turns", next)
            };
        }
        let fail_turns = res.get("tb_zero_fail_turns");
        if fail_turns > 0 {
            res = res.set("tb_zero_fail_turns", (fail_turns - 1).max(0));
        }
        state.with_resources(res)
    }

    pub fn training_stat_multiplier(resources: &ScenarioResources) -> f64 {
        let pct = resources.get("tb_training_bonus_pct");
        if pct > 0 {
            1.0 + pct as f64 / 100.0
        } else {
            1.0
        }
    }

    pub fn zero_failure_ready(resources: &ScenarioResources) -> bool {
        resources.get("tb_zero_fail_turns") > 0
    }
}

fn parse_facility(raw: &str) -> Option<TrainingFacility> {
    match raw.to_lowercase().as_str() {
        "speed" => Some(TrainingFacility::Speed),
        "stamina" => Some(TrainingFacility::Stamina),
        "power" => Some(TrainingFacility::Power),
        "guts" => Some(TrainingFacility::Guts),
        "wit" => Some(TrainingFacility::Wit),
        _ => None,
    }
}

fn shuffled(items: &[ShopItem], rng: &mut SimRandom) -> Vec<ShopItem> {
    let mut copy = items.to_vec();
    for i in (1..copy.len()).rev() {
        let j = rng.next_int_range(0, i as i32 + 1) as usize;
        copy.swap(i, j);
    }
    copy
}

fn default_catalog() -> Vec<ShopItem> {
    vec![
        ShopItem {
            id: "speed_charm".into(),
            name: "Speed Charm".into(),
            cost: 50,
            effect_text: "Speed +20".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "stamina_charm".into(),
            name: "Stamina Charm".into(),
            cost: 50,
            effect_text: "Stamina +20".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "speed_scroll".into(),
            name: "Speed Scroll".into(),
            cost: 30,
            effect_text: "Speed +15".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "stamina_scroll".into(),
            name: "Stamina Scroll".into(),
            cost: 30,
            effect_text: "Stamina +15".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "power_scroll".into(),
            name: "Power Scroll".into(),
            cost: 30,
            effect_text: "Power +15".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "vita_40".into(),
            name: "Vita 40".into(),
            cost: 55,
            effect_text: "Energy +40".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "plain_cupcake".into(),
            name: "Plain Cupcake".into(),
            cost: 30,
            effect_text: "Motivation +1".into(),
            category: "Stats".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "coaching_megaphone".into(),
            name: "Coaching Megaphone".into(),
            cost: 40,
            effect_text: "Training bonus +20% for 4 turns".into(),
            category: "Training Effects".into(),
            facility: None,
            training_bonus_pct: 20,
            training_bonus_turns: 4,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "good_luck_charm".into(),
            name: "Good-Luck Charm".into(),
            cost: 40,
            effect_text: "Training failure rate set to 0% (One turn)".into(),
            category: "Training Effects".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 1,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "speed_training_app".into(),
            name: "Speed Training Application".into(),
            cost: 150,
            effect_text: "Speed Training Level +1".into(),
            category: "Training Facilities".into(),
            facility: Some(TrainingFacility::Speed),
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
        ShopItem {
            id: "energy_drink_max_ex".into(),
            name: "Energy Drink MAX EX".into(),
            cost: 50,
            effect_text: "Maximum energy +8".into(),
            category: "Energy and Motivation".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 8,
        },
        ShopItem {
            id: "skip".into(),
            name: "Skip".into(),
            cost: 0,
            effect_text: "Energy +10".into(),
            category: "Skip".into(),
            facility: None,
            training_bonus_pct: 0,
            training_bonus_turns: 0,
            zero_fail_turns: 0,
            max_energy_delta: 0,
        },
    ]
}
