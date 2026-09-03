pub mod grand_live;
pub mod grand_live_catalog;
pub mod grand_live_deck_support;
pub mod grand_live_lesson_board;
pub mod grand_live_lesson_scoring;
pub mod trackblazer;
pub mod unity;
pub mod ura;

pub use grand_live::{ConcertOutcome, GrandLiveMechanics, PERF_CODES};
pub use grand_live_catalog::{
    GrandLiveCalibrationLoader, GrandLiveCatalog, GrandLiveCatalogLoader, GrandLiveMasteryBonus,
};
pub use grand_live_deck_support::GrandLiveDeckSupport;
pub use grand_live_lesson_board::GrandLiveLessonBoard;
pub use grand_live_lesson_scoring::GrandLiveLessonScoring;
pub use trackblazer::TrackblazerMechanics;
pub use unity::UnityCupMechanics;
pub use ura::{DuelContest, DuelPrediction, UraMechanics};

use crate::deck::DeckPlacement;
use crate::events::EventEffectApplier;
use crate::rng::SimRandom;
use crate::state::{
    CareerState, MandatoryRace, RunMeta, ScenarioResources, SimAction, SimChoice, TrainingFacility,
    TurnPhase,
};

pub trait ScenarioPlugin: Send + Sync {
    fn scenario_id(&self) -> &str;
    fn stat_caps(&self) -> std::collections::HashMap<String, i32>;
    fn mandatory_races(&self) -> &[MandatoryRace];
    fn on_turn_start(&self, state: &CareerState) -> (CareerState, Vec<String>);
    fn on_training_complete(
        &self,
        state: &CareerState,
        _facility: TrainingFacility,
        _success: bool,
    ) -> (CareerState, Vec<String>) {
        (state.clone(), Vec::new())
    }
    fn on_race_complete(
        &self,
        state: &CareerState,
        _race_id: &str,
        _won: bool,
    ) -> (CareerState, Vec<String>) {
        (state.clone(), Vec::new())
    }
    fn initial_scenario_resources(&self, _meta: &RunMeta) -> ScenarioResources {
        ScenarioResources::new()
    }
    fn extra_choices(&self, _state: &CareerState) -> Vec<SimChoice> {
        Vec::new()
    }
    fn apply_side_action(&self, _state: &CareerState, _action_id: &str) -> Option<(CareerState, Vec<String>)> {
        None
    }
    fn apply_soft_cap(&self, _facility: TrainingFacility, _current: i32, raw_gain: i32) -> i32 {
        raw_gain
    }
    fn training_stat_multiplier(&self, _state: &CareerState) -> f64 {
        1.0
    }
    fn effective_facility_level(&self, _state: &CareerState, _facility: TrainingFacility) -> Option<i32> {
        None
    }
    fn songs_learned(&self, _state: &CareerState) -> i32 {
        0
    }
    fn on_action_complete(
        &self,
        state: &CareerState,
        _action: &SimAction,
    ) -> (CareerState, Vec<String>) {
        (state.clone(), Vec::new())
    }
}

fn pending_mandatory_race(state: &CareerState, races: &[MandatoryRace]) -> Option<MandatoryRace> {
    races.iter().find(|r| {
        !state.completed_races.contains(&r.id)
            && state.date.year == r.year
            && state.date.month == r.month
            && state.date.half == r.half
    }).cloned()
}

struct BaseScenario {
    id: &'static str,
    caps: std::collections::HashMap<String, i32>,
    races: Vec<MandatoryRace>,
}

impl BaseScenario {
    fn turn_start_base(&self, state: &CareerState) -> (CareerState, Vec<String>) {
        if let Some(race) = pending_mandatory_race(state, &self.races) {
            let mut s = state.clone();
            s.phase = TurnPhase::MandatoryRace.as_str().to_string();
            s.pending_race_id = Some(race.id.clone());
            (s, vec![format!("Mandatory race: {}", race.name)])
        } else {
            let mut s = state.clone();
            s.phase = TurnPhase::Free.as_str().to_string();
            s.pending_race_id = None;
            (s, Vec::new())
        }
    }
}

pub struct UraScenarioPlugin {
    base: BaseScenario,
}

impl UraScenarioPlugin {
    pub fn new() -> Self {
        Self {
            base: BaseScenario {
                id: "ura",
                caps: std::collections::HashMap::from([
                    ("speed".into(), 1400),
                    ("stamina".into(), 1400),
                    ("power".into(), 1400),
                    ("guts".into(), 1400),
                    ("wit".into(), 1400),
                ]),
                races: vec![
                    MandatoryRace { id: "debut".into(), name: "Junior Make Debut".into(), year: 1, month: 6, half: 2 },
                    MandatoryRace { id: "finale_qualifier".into(), name: "URA Finale Qualifier".into(), year: 3, month: 11, half: 1 },
                    MandatoryRace { id: "finale_semifinal".into(), name: "URA Finale Semifinal".into(), year: 3, month: 11, half: 2 },
                    MandatoryRace { id: "finale_finals".into(), name: "URA Finale Finals".into(), year: 3, month: 12, half: 1 },
                ],
            },
        }
    }
}

impl Default for UraScenarioPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioPlugin for UraScenarioPlugin {
    fn scenario_id(&self) -> &str {
        self.base.id
    }
    fn stat_caps(&self) -> std::collections::HashMap<String, i32> {
        self.base.caps.clone()
    }
    fn mandatory_races(&self) -> &[MandatoryRace] {
        &self.base.races
    }
    fn on_turn_start(&self, state: &CareerState) -> (CareerState, Vec<String>) {
        // Match Kotlin UraScenarioPlugin: inheritance event before badge roll.
        if state.turn == 13
            && !state.legacy.inheritance_complete
            && !state.meta.legacy_factors.is_empty()
            && state.phase != TurnPhase::MandatoryRace.as_str()
        {
            let mut s = state.clone();
            s.phase = TurnPhase::Event.as_str().to_string();
            s.awaiting_choice = true;
            s.pending_event_title = Some("Inheritance".into());
            s.pending_event_options = vec![
                "Accept inherited skills\nSkill points +30".into(),
                "Focus on spark stats\nSpeed +20\nStamina +20".into(),
            ];
            return (s, vec!["Inheritance event — choose succession bonus".into()]);
        }
        let with_badge = {
            let mut s = state.clone();
            s.scenario_resources = UraMechanics::roll_badge_on_turn(state);
            s
        };
        let (mut base, mut lines) = self.base.turn_start_base(&with_badge);
        if let Some(badge) = UraMechanics::badge_facility(&base.scenario_resources) {
            lines.push(format!("Happy Meek duel badge on {}", badge.key()));
        }
        base.scenario_resources = with_badge.scenario_resources;
        (base, lines)
    }
    fn on_training_complete(
        &self,
        state: &CareerState,
        facility: TrainingFacility,
        success: bool,
    ) -> (CareerState, Vec<String>) {
        if !success {
            return (state.clone(), Vec::new());
        }
        let badge = UraMechanics::badge_facility(&state.scenario_resources);
        if badge != Some(facility) {
            return (state.clone(), Vec::new());
        }
        let mut rng = crate::rng::SimRandom::new(
            state.meta.seed ^ ((state.turn as i64) << 28) ^ facility.ordinal() as i64,
        );
        let level = UraMechanics::meek_level(&state.scenario_resources);
        let options = UraMechanics::build_duel_options(&mut rng, level);
        let mut s = state.clone();
        s.phase = TurnPhase::Event.as_str().to_string();
        s.awaiting_choice = true;
        s.pending_event_title = Some("Happy Meek's Challenge!".to_string());
        s.pending_event_options = options;
        s.scenario_resources = state.scenario_resources.set("happy_meek_pending", 1);
        (s, vec!["Happy Meek's Challenge! — pick a contest".to_string()])
    }
}

pub struct GrandConcertScenarioPlugin;

impl Default for GrandConcertScenarioPlugin {
    fn default() -> Self {
        Self
    }
}

impl ScenarioPlugin for GrandConcertScenarioPlugin {
    fn scenario_id(&self) -> &str {
        "grand_concert"
    }
    fn stat_caps(&self) -> std::collections::HashMap<String, i32> {
        std::collections::HashMap::from([
            ("speed".into(), 1600),
            ("stamina".into(), 1300),
            ("power".into(), 1300),
            ("guts".into(), 1500),
            ("wit".into(), 1300),
        ])
    }
    fn mandatory_races(&self) -> &[MandatoryRace] {
        static RACES: std::sync::OnceLock<Vec<MandatoryRace>> = std::sync::OnceLock::new();
        RACES.get_or_init(|| {
            vec![
                MandatoryRace { id: "debut".into(), name: "Junior Make Debut".into(), year: 1, month: 6, half: 2 },
                MandatoryRace { id: "promo_1".into(), name: "Promo Concert 1".into(), year: 2, month: 6, half: 2 },
                MandatoryRace { id: "promo_2".into(), name: "Promo Concert 2".into(), year: 2, month: 12, half: 2 },
                MandatoryRace { id: "promo_3".into(), name: "Promo Concert 3".into(), year: 3, month: 6, half: 2 },
                MandatoryRace { id: "promo_4".into(), name: "Promo Concert 4".into(), year: 3, month: 12, half: 1 },
                MandatoryRace { id: "grand_concert".into(), name: "Grand Concert".into(), year: 3, month: 12, half: 2 },
            ]
        })
    }
    fn initial_scenario_resources(&self, _meta: &RunMeta) -> ScenarioResources {
        GrandLiveMechanics::initial_resources()
    }
    fn songs_learned(&self, state: &CareerState) -> i32 {
        state.scenario_resources.get("songs_learned")
    }
    fn on_turn_start(&self, state: &CareerState) -> (CareerState, Vec<String>) {
        let mut lines = Vec::new();
        let mut s = state.clone();
        // Make Debut! ~4 turns after career start (uma.guide); fills 1/3 of first-promo hype.
        if s.turn >= crate::scenario::grand_live::MAKE_DEBUT_GRANT_TURN
            && !GrandLiveMechanics::owns_song(&s.scenario_resources, 1)
        {
            let (res, grant_lines) =
                GrandLiveMechanics::grant_make_debut_song(&s.scenario_resources);
            s.scenario_resources = res;
            lines.extend(grant_lines);
        }
        if let Some(race_id) = GrandLiveMechanics::concert_race_id(s.turn) {
            if !s.completed_races.iter().any(|r| r == race_id) {
                // Song saving: snapshot unpaid song board before the concert turn locks FREE.
                let slots = GrandLiveLessonBoard::current_slots(&s);
                if slots.iter().any(|slot| slot.is_song) {
                    let ids: Vec<String> = slots.into_iter().map(|slot| slot.action_id).collect();
                    s.scenario_resources =
                        GrandLiveMechanics::freeze_lesson_board(&s.scenario_resources, &ids);
                    lines.push("Song board saved across concert".to_string());
                }
                s.phase = TurnPhase::MandatoryRace.as_str().to_string();
                s.pending_race_id = Some(race_id.to_string());
                return (
                    s,
                    vec![format!(
                        "Mandatory: {}",
                        GrandLiveMechanics::concert_race_name(race_id)
                    )]
                    .into_iter()
                    .chain(lines)
                    .collect(),
                );
            }
        }
        if s.date.year >= 3 && s.date.month == 12 && s.date.half == 1
            && !GrandLiveMechanics::owns_song(&s.scenario_resources, 22)
        {
            s.scenario_resources =
                GrandLiveMechanics::mark_song_owned(&s.scenario_resources, 22, false);
            s.log.push("Received Girls' Legend U".to_string());
            lines.push("Girls' Legend U added to setlist".to_string());
        }
        // Daily specialty-weighted deck placement (Specialty Priority Up from concert bonuses).
        if !s.deck.slots.is_empty()
            && s.phase == TurnPhase::Free.as_str()
            && !s.awaiting_choice
        {
            let specialty_bonus = s.scenario_resources.get("bonus_specialty_pct");
            let mut rng = SimRandom::new(s.meta.seed * 53 + s.turn as i64 * 29);
            s.deck.slots =
                DeckPlacement::roll_for_turn(&s.deck.slots, &mut rng, specialty_bonus);
        }
        // Senior Early November: Closer Together (16+ songs → scenario-link skill hint).
        if !s.awaiting_choice
            && s.phase == TurnPhase::Free.as_str()
            && s.date.year >= 3
            && s.date.month == 11
            && s.date.half == 1
            && s.scenario_resources.get("closer_together_done") == 0
            && s.scenario_resources.get("songs_learned") >= 16
        {
            s.phase = TurnPhase::Event.as_str().to_string();
            s.awaiting_choice = true;
            s.pending_event_title = Some("Closer Together".to_string());
            s.pending_event_options = closer_together_options(&s);
            s.scenario_resources = s.scenario_resources.set("closer_together_done", 1);
            lines.push("Closer Together (16-song scenario reward)".to_string());
            return (s, lines);
        }
        // Light Hello dating-starts: real support event (not a fixed every-N turn outing).
        // After unlock, dating is a recreation/Pal Date action — see engine do_recreation.
        if !s.awaiting_choice
            && s.phase == TurnPhase::Free.as_str()
            && s.turn > 4
            && !GrandLiveMechanics::dating_unlocked(&s.scenario_resources)
            && GrandLiveDeckSupport::any_light_hello_in_deck(&s)
            && GrandLiveDeckSupport::light_hello_bond(&s) >= 40
        {
            let mut rng = SimRandom::new(s.meta.seed * 41 + s.turn as i64 * 23);
            if rng.next_boolean(GrandLiveMechanics::dating_start_event_chance()) {
                s.phase = TurnPhase::Event.as_str().to_string();
                s.awaiting_choice = true;
                s.pending_event_title = Some("Embrace Those Emotions!".to_string());
                s.pending_event_options = vec![
                    "Energy +21\nMood +1\nSpeed +8\nGuts +8\nLight Hello bond +5\nCan start dating"
                        .to_string(),
                    "Mood +1\nWit +10\nLight Hello bond +5".to_string(),
                ];
                lines.push("Light Hello dating-starts event".to_string());
            }
        }
        (s, lines)
    }
    fn apply_soft_cap(&self, _facility: TrainingFacility, current: i32, raw_gain: i32) -> i32 {
        GrandLiveMechanics::soft_cap_gain(current, raw_gain)
    }
    fn training_stat_multiplier(&self, state: &CareerState) -> f64 {
        GrandLiveMechanics::training_stat_multiplier(&state.scenario_resources)
    }
    fn on_training_complete(
        &self,
        state: &CareerState,
        facility: TrainingFacility,
        success: bool,
    ) -> (CareerState, Vec<String>) {
        if !success {
            return (state.clone(), Vec::new());
        }
        let mut rng = SimRandom::new(
            state.meta.seed * 37 + state.turn as i64 * 19 + facility.ordinal() as i64,
        );
        let (gains, extra) = GrandLiveMechanics::resolve_training_perf_gains(state, facility, &mut rng);
        let res = GrandLiveMechanics::add_perf_tokens(&state.scenario_resources, &gains);
        let parts: Vec<String> = gains.iter().map(|(k, v)| format!("{k} +{v}")).collect();
        (
            {
                let mut s = state.clone();
                s.scenario_resources = res;
                s
            },
            [vec![format!("Performance {}", parts.join(", "))], extra].concat(),
        )
    }
    fn on_race_complete(
        &self,
        state: &CareerState,
        race_id: &str,
        won: bool,
    ) -> (CareerState, Vec<String>) {
        if !won && race_id == "debut" {
            return (state.clone(), vec!["Make Debut failed — retry".to_string()]);
        }
        let mut lines = Vec::new();
        let mut res = state.scenario_resources.clone();
        let mut stats = state.stats.clone();
        let cycle_songs = GrandLiveMechanics::cycle_songs(&res);
        let cycle_techniques = res.get("cycle_techniques");
        let outcome = GrandLiveMechanics::concert_outcome_with_race(race_id, &res, won);
        let great_success = outcome == crate::scenario::grand_live::ConcertOutcome::GreatSuccess;
        let failed = outcome == crate::scenario::grand_live::ConcertOutcome::Failure;
        // Character debut is not a concert — skip live packet fields.
        if race_id != "debut" {
            res = res
                .set("last_live_result", GrandLiveMechanics::result_state_for_outcome(outcome))
                .set("last_live_type", GrandLiveMechanics::live_type_for_race(race_id))
                .set(
                    "member_ready_count",
                    GrandLiveMechanics::member_ready_count(state),
                );
        }

        match race_id {
            "debut" => {
                // Character debut race is not a concert: keep Make Debut! in the first-promo cycle.
                lines.push("Make Debut race complete".to_string());
                let mut next = state.clone();
                next.scenario_resources = res;
                next.stats = stats;
                return (next, lines);
            }
            _ if failed => {
                let required = GrandLiveMechanics::great_success_required_for_race(race_id);
                lines.push(format!(
                    "Concert FAILED ({cycle_songs}/{} cycle songs, avg perf {})",
                    required,
                    GrandLiveMechanics::average_perf_tokens(&res)
                ));
            }
            _ if great_success => {
                let required = GrandLiveMechanics::great_success_required_for_race(race_id);
                lines.push(format!("Great Success! ({cycle_songs}/{required} cycle songs)"))
            }
            _ => {
                let required = GrandLiveMechanics::great_success_required_for_race(race_id);
                lines.push(format!("Concert complete ({cycle_songs}/{required} cycle songs)"))
            }
        }

        {
            let mut next_mood = state.mood;
            if failed {
                next_mood = crate::state::downgrade_mood(state.mood);
                lines.push("Concert failure: mood dropped".to_string());
            }
            let stat_bonus = GrandLiveMechanics::concert_stat_bonus_for_outcome(outcome);
            stats = stats.add_all(stat_bonus);
            lines.push(format!(
                "Concert +{stat_bonus} all stats ({})",
                outcome.label()
            ));
            let sp_gain = if failed {
                GrandLiveMechanics::sp_between_concerts(cycle_techniques, cycle_songs) / 2
            } else {
                GrandLiveMechanics::sp_between_concerts(cycle_techniques, cycle_songs)
            };
            if sp_gain > 0 {
                lines.push(format!(
                    "Between-concert SP +{sp_gain} ({cycle_techniques} techniques, {cycle_songs} songs)"
                ));
            }
            if cycle_songs > 0 && !failed {
                res = activate_cycle_bonuses(&res, great_success, &mut lines);
            }
            for (target_type, effect_value) in GrandLiveMechanics::training_bonuses_packet(&res) {
                res = res.set(&format!("training_bonus_target:{target_type}"), effect_value);
            }
            if !failed {
                res = GrandLiveMechanics::raise_perf_cap_after_concert(&res);
                lines.push(format!(
                    "Performance cap raised to {}",
                    GrandLiveMechanics::perf_max(&res)
                ));
            }
            res = GrandLiveMechanics::reset_cycle_after_concert(&res);
            let mut next = state.clone();
            next.scenario_resources = res.clone();
            next.stats = stats;
            next.mood = next_mood;
            if sp_gain > 0 {
                next.skill_points += sp_gain;
            }

            if race_id == "grand_concert" {
                let songs = res.get("songs_learned");
                let unique_power = GrandLiveMechanics::unique_skill_power(state.fans);
                if songs >= crate::scenario::grand_live::TARGET_SONGS && great_success {
                    res = res
                        .add("grand_live_perfect", 1)
                        .set("unique_skill_power", unique_power);
                    if !GrandLiveMechanics::owns_song(&res, 24) {
                        res = GrandLiveMechanics::mark_song_owned(&res, 24, false);
                        lines.push(format!(
                            "Special Girls' Legend U unlocked — I Wanna Win with You path (power {unique_power})"
                        ));
                    }
                } else {
                    if !GrandLiveMechanics::owns_song(&res, 22) {
                        res = GrandLiveMechanics::mark_song_owned(&res, 22, false);
                    }
                    res = res.set("unique_skill_power", unique_power);
                    lines.push(format!(
                        "On the Way to Our Dream (consolation unique, power {unique_power})"
                    ));
                }
                next.scenario_resources = res;
            }

            return (next, lines);
        }
    }
    fn extra_choices(&self, state: &CareerState) -> Vec<SimChoice> {
        // Board may include unaffordable cards (API parity); only affordable ones are actionable.
        GrandLiveLessonBoard::current_slots(state)
            .into_iter()
            .filter(|slot| slot.affordable)
            .map(|slot| SimChoice {
                id: slot.action_id,
                label: slot.label,
            })
            .collect()
    }
    fn apply_side_action(
        &self,
        state: &CareerState,
        action_id: &str,
    ) -> Option<(CareerState, Vec<String>)> {
        if GrandLiveLessonBoard::find_slot(state, action_id).is_some() {
            if action_id.starts_with("gl_song_") {
                return purchase_song(state, action_id.trim_start_matches("gl_song_"));
            }
            if action_id.starts_with("gl_tech_") {
                return purchase_technique(state, action_id.trim_start_matches("gl_tech_"));
            }
        }
        if action_id.starts_with("gl_song_") {
            return purchase_song(state, action_id.trim_start_matches("gl_song_"));
        }
        if action_id.starts_with("gl_tech_") {
            return purchase_technique(state, action_id.trim_start_matches("gl_tech_"));
        }
        None
    }
}

pub struct UnityCupScenarioPlugin {
    base: BaseScenario,
}

impl Default for UnityCupScenarioPlugin {
    fn default() -> Self {
        Self {
            base: BaseScenario {
                id: "unity",
                caps: std::collections::HashMap::from([
                    ("speed".into(), 1300),
                    ("stamina".into(), 1300),
                    ("power".into(), 1300),
                    ("guts".into(), 1300),
                    ("wit".into(), 1800),
                ]),
                races: vec![
                    MandatoryRace { id: "debut".into(), name: "Junior Make Debut".into(), year: 1, month: 6, half: 2 },
                    MandatoryRace { id: "unity_preseason".into(), name: "Unity Preseason".into(), year: 2, month: 6, half: 2 },
                    MandatoryRace { id: "unity_finals".into(), name: "Unity Finals".into(), year: 3, month: 12, half: 2 },
                ],
            },
        }
    }
}

impl ScenarioPlugin for UnityCupScenarioPlugin {
    fn scenario_id(&self) -> &str {
        self.base.id
    }
    fn stat_caps(&self) -> std::collections::HashMap<String, i32> {
        self.base.caps.clone()
    }
    fn mandatory_races(&self) -> &[MandatoryRace] {
        &self.base.races
    }
    fn initial_scenario_resources(&self, _meta: &RunMeta) -> ScenarioResources {
        UnityCupMechanics::initial_resources()
    }
    fn effective_facility_level(&self, state: &CareerState, facility: TrainingFacility) -> Option<i32> {
        Some(UnityCupMechanics::facility_level_for(&state.scenario_resources, facility))
    }
    fn on_turn_start(&self, state: &CareerState) -> (CareerState, Vec<String>) {
        let (mut base, lines) = self.base.turn_start_base(state);
        base.facility_levels = UnityCupMechanics::facility_levels_from_resources(&base.scenario_resources);
        (base, lines)
    }
    fn on_training_complete(
        &self,
        state: &CareerState,
        facility: TrainingFacility,
        success: bool,
    ) -> (CareerState, Vec<String>) {
        if !success {
            return (state.clone(), Vec::new());
        }
        let level = UnityCupMechanics::facility_level_for(&state.scenario_resources, facility);
        let gain = UnityCupMechanics::spirit_gain(level);
        let (res, lines, _) =
            UnityCupMechanics::apply_training_spirit(&state.scenario_resources, gain, facility);
        let mut s = {
            let mut t = state.clone();
            t.scenario_resources = res.clone();
            t
        };
        s.facility_levels = UnityCupMechanics::facility_levels_from_resources(&res);
        let (after, extreme_lines) = UnityCupMechanics::consume_extreme_burst(&s, facility);
        s = after;
        s.facility_levels = UnityCupMechanics::facility_levels_from_resources(&s.scenario_resources);
        (s, [lines, extreme_lines].concat())
    }
    fn on_race_complete(
        &self,
        state: &CareerState,
        race_id: &str,
        won: bool,
    ) -> (CareerState, Vec<String>) {
        if !won || race_id == "debut" {
            return (state.clone(), Vec::new());
        }
        let (mut res, mut lines) = UnityCupMechanics::bump_all_team_ranks(&state.scenario_resources);
        // Abstract 5-leg team race: a win counts as sweeping all five legs.
        if race_id.contains("unity") || race_id.contains("team") {
            let legs = if won { 5 } else { 0 };
            res = res.add("unity_legs_won", legs).add("unity_team_races_done", 1);
            lines.push(format!("Team race legs +{legs}"));
            // 4th team-race win upgrades Zenith finals (research / unity.md).
            if res.get("unity_team_races_done") >= 4 {
                res = res.set("unity_zenith_upgraded", 1);
                lines.push("Team Zenith upgraded".to_string());
            }
        }
        let mut s = {
            let mut t = state.clone();
            t.scenario_resources = res.clone();
            t
        };
        s.facility_levels = UnityCupMechanics::facility_levels_from_resources(&res);
        (s, lines)
    }
}

pub struct TrackblazerScenarioPlugin {
    base: BaseScenario,
}

impl Default for TrackblazerScenarioPlugin {
    fn default() -> Self {
        Self {
            base: BaseScenario {
                id: "trackblazer",
                caps: std::collections::HashMap::from([
                    ("speed".into(), 1200),
                    ("stamina".into(), 1900),
                    ("power".into(), 1200),
                    ("guts".into(), 1200),
                    ("wit".into(), 1500),
                ]),
                races: vec![
                    MandatoryRace { id: "debut".into(), name: "Junior Make Debut".into(), year: 1, month: 6, half: 2 },
                    MandatoryRace { id: "climax_1".into(), name: "Twinkle Star Climax 1".into(), year: 3, month: 11, half: 1 },
                    MandatoryRace { id: "climax_3".into(), name: "Twinkle Star Climax 3".into(), year: 3, month: 12, half: 2 },
                ],
            },
        }
    }
}

impl ScenarioPlugin for TrackblazerScenarioPlugin {
    fn scenario_id(&self) -> &str {
        self.base.id
    }
    fn stat_caps(&self) -> std::collections::HashMap<String, i32> {
        self.base.caps.clone()
    }
    fn mandatory_races(&self) -> &[MandatoryRace] {
        &self.base.races
    }
    fn training_stat_multiplier(&self, state: &CareerState) -> f64 {
        TrackblazerMechanics::training_stat_multiplier(&state.scenario_resources)
    }
    fn on_turn_start(&self, state: &CareerState) -> (CareerState, Vec<String>) {
        let decayed = TrackblazerMechanics::decay_turn_buffs(state);
        let (mut base, mut lines) = self.base.turn_start_base(&decayed);
        if base.phase == TurnPhase::MandatoryRace.as_str() {
            return (base, lines);
        }
        if TrackblazerMechanics::should_open_shop(base.turn, base.scenario_resources.get("tb_coins")) {
            let options = TrackblazerMechanics::roll_shop_options(&base);
            base.phase = TurnPhase::Event.as_str().to_string();
            base.awaiting_choice = true;
            base.pending_event_title = Some("Trackblazer Pro Shop".to_string());
            base.pending_event_options = options;
            lines.push("Pro Shop open".to_string());
        }
        (base, lines)
    }
    fn on_race_complete(
        &self,
        state: &CareerState,
        race_id: &str,
        won: bool,
    ) -> (CareerState, Vec<String>) {
        if !won {
            return (state.clone(), Vec::new());
        }
        let coins = TrackblazerMechanics::race_coin_gain(race_id);
        let vp = TrackblazerMechanics::climax_victory_points(race_id, won);
        let mut res = state.scenario_resources.add("tb_coins", coins);
        let mut lines = vec![format!("Shop coins +{coins}")];
        if vp > 0 {
            res = res.add("tb_victory_points", vp);
            lines.push(format!("Victory Points +{vp}"));
        }
        (
            {
                let mut s = state.clone();
                s.scenario_resources = res;
                s
            },
            lines,
        )
    }
}

pub fn scenario_plugin_for(id: &str) -> Box<dyn ScenarioPlugin> {
    let normalized = id.to_lowercase().replace(' ', "_");
    match normalized.as_str() {
        "ura" | "ura_finale" => Box::new(UraScenarioPlugin::default()),
        "grand_concert" | "grand_live" | "gl" => Box::new(GrandConcertScenarioPlugin),
        "unity" | "unity_cup" => Box::new(UnityCupScenarioPlugin::default()),
        "trackblazer" | "tb" => Box::new(TrackblazerScenarioPlugin::default()),
        _ => Box::new(UraScenarioPlugin::default()),
    }
}

fn closer_together_options(state: &CareerState) -> Vec<String> {
    let has = |needle: &str| {
        let trainee = state.meta.trainee_name.to_lowercase().replace(['_', '-'], " ");
        if trainee.contains(needle) {
            return true;
        }
        state.deck.slots.iter().any(|s| {
            let n = s.support_id.to_lowercase().replace(['_', '-'], " ");
            n.contains(needle)
        })
    };
    vec![
        if has("smart falcon") || has("30210") || has("30017") {
            "Full Speed! hint +1".into()
        } else {
            "Full Tilt hint +1".into()
        },
        if has("mihono bourbon") || has("bourbon") {
            "Concentration hint +1".into()
        } else {
            "Focus hint +1".into()
        },
        if has("silence suzuka") || has("suzuka") {
            "Trackblazer hint +1".into()
        } else {
            "Rosy Outlook hint +1".into()
        },
        if has("agnes tachyon") || has("tachyon") {
            "Come What May hint +1".into()
        } else {
            "All I've Got hint +1".into()
        },
        "Lane Legerdemain hint +1".into(),
    ]
}

fn activate_cycle_bonuses(
    resources: &ScenarioResources,
    great_success: bool,
    lines: &mut Vec<String>,
) -> ScenarioResources {
    let mut res = resources.clone();
    for song_id in GrandLiveMechanics::cycle_song_ids(resources) {
        let Some(song) = GrandLiveCatalog::find_song(&song_id.to_string()) else {
            continue;
        };
        res = GrandLiveMechanics::activate_cycle_song_bonuses(&res, &song, great_success);
        if let Some(bonus) = &song.concert_bonus {
            lines.push(format!(
                "Activated: {} — {} +{}",
                song.name, bonus.effect, bonus.value
            ));
        }
    }
    res
}

fn purchase_song(state: &CareerState, song_key: &str) -> Option<(CareerState, Vec<String>)> {
    let song = GrandLiveCatalog::find_song(song_key)?;
    if !GrandLiveCatalog::can_afford(&state.scenario_resources, &song.costs) {
        return None;
    }
    if GrandLiveMechanics::owns_song(&state.scenario_resources, song.song_list_id) {
        return None;
    }
    if !GrandLiveMechanics::can_add_cycle_song(&state.scenario_resources) {
        return None;
    }
    let frozen_ok = GrandLiveLessonBoard::frozen_contains(&state.scenario_resources, &format!(
        "gl_song_{}",
        song.song_list_id
    ));
    if !GrandLiveMechanics::songs_unlocked_on_board(state) && !frozen_ok {
        return None;
    }
    let mut res = GrandLiveCatalog::pay_costs(&state.scenario_resources, &song.costs);
    res = GrandLiveMechanics::mark_song_owned(&res, song.song_list_id, true);
    res = GrandLiveMechanics::clear_frozen_board(&res);
    res = GrandLiveMechanics::bump_lesson_refresh(&res);
    let (res, stats, mastery_lines) =
        GrandLiveMechanics::apply_song_mastery(&res, &song.mastery, &state.stats);
    let mut next = state.clone();
    next.stats = stats;
    next.scenario_resources = res.clone();
    if let sp @ 1.. = res.get("pending_mastery_sp") {
        next.skill_points += sp;
        next.scenario_resources = next.scenario_resources.set("pending_mastery_sp", 0);
    }
    // Flat mastery already applied; EventEffectApplier only for leftover readable text.
    let mut effect_lines = mastery_lines;
    if matches!(
        song.mastery,
        crate::scenario::grand_live_catalog::GrandLiveMasteryBonus::None
    ) && !song.purchase_bonus_text.is_empty()
    {
        let mut rng = SimRandom::new(state.meta.seed + state.turn as i64);
        let (after_effect, lines) =
            EventEffectApplier::apply(&next, &song.purchase_bonus_text, &mut rng);
        next = after_effect;
        effect_lines.extend(lines);
    }
    let cycle = GrandLiveMechanics::cycle_songs(&next.scenario_resources);
    let required = GrandLiveMechanics::great_success_required(&next.scenario_resources, None);
    let ready = if GrandLiveMechanics::is_hype_maxed(&next.scenario_resources) {
        " (Great Success ready)"
    } else {
        ""
    };
    Some((
        next,
        [
            vec![format!(
                "Learned song: {} — cycle {cycle}/{required}{ready}",
                song.name
            )],
            effect_lines,
        ]
        .concat(),
    ))
}

fn purchase_technique(state: &CareerState, tech_key: &str) -> Option<(CareerState, Vec<String>)> {
    let tech = GrandLiveCatalog::find_technique(tech_key)
        .or_else(|| GrandLiveCatalog::find_technique(&format!("lesson:{tech_key}")))?;
    if !GrandLiveCatalog::can_afford(&state.scenario_resources, &tech.costs) {
        return None;
    }
    let mut res = GrandLiveCatalog::pay_costs(&state.scenario_resources, &tech.costs);
    res = GrandLiveMechanics::record_technique_purchase(&res);
    res = GrandLiveMechanics::clear_frozen_board(&res);
    res = GrandLiveMechanics::bump_lesson_refresh(&res);
    let mut next = state.clone();
    next.scenario_resources = res.clone();
    let mut rng = SimRandom::new(state.meta.seed + state.turn as i64 + 1);
    let (after_effect, effect_lines) =
        EventEffectApplier::apply(&next, &tech.effect_text, &mut rng);
    next = after_effect;
    next.scenario_resources = res;
    Some((
        next,
        [vec![format!("Technique: {}", tech.name)], effect_lines].concat(),
    ))
}
