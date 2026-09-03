use crate::bot::scoring_auto_policy;
use crate::calendar::{TurnCalendar, CAREER_TURNS};
use crate::catalog::event::{BuiltinEventCatalog, EventCatalog};
use crate::config::{
    BondGainConfig, EventProbabilityConfig, FacilityLevelConfig, HintProgressionConfig,
    InspirationConfig, MoodEnergyConfig, RaceOutcomeConfig, RacePlacement, TrainingFailureConfig,
};
use crate::deck::DeckPlacement;
use crate::events::EventEffectApplier;
use crate::factory;
use crate::legacy::LegacyApplicator;
use crate::policy::default_auto_policy;
use crate::render::TextRenderer;
use crate::rng::SimRandom;
use crate::scenario::{
    scenario_plugin_for, GrandLiveDeckSupport, GrandLiveMechanics, ScenarioPlugin,
    TrackblazerMechanics, UnityCupMechanics, UraMechanics,
};
use crate::scoring::soft_cap_effectiveness_multiplier;
use crate::snapshot::RunSnapshot;
use crate::state::{
    default_facility_levels, CareerState, DeckState, RunMeta, SimAction, SimActionKind, SimChoice,
    SimDate, SimSettings, TrainingFacility, TurnPhase, downgrade_mood, upgrade_mood,
};
use crate::telemetry::SimTelemetry;
use crate::training::TrainingResolver;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SimStepResult {
    pub state: CareerState,
    pub text_lines: Vec<String>,
    pub choices: Vec<SimChoice>,
    pub career_ended: bool,
}

pub struct SimEngine {
    settings: SimSettings,
    training_resolver: TrainingResolver,
    event_catalog: Arc<dyn EventCatalog>,
    event_chance_override: Option<f64>,
    rng: SimRandom,
    state: CareerState,
    plugin: Box<dyn ScenarioPlugin>,
    telemetry: SimTelemetry,
    last_terminal: Option<crate::scoring::CareerTerminalRecord>,
}

impl SimEngine {
    /// Default engine — matches Kotlin `SimEngine()` (builtin events, default resolver, no KB load).
    pub fn new(settings: SimSettings) -> Self {
        Self {
            settings,
            training_resolver: TrainingResolver::default(),
            event_catalog: Arc::new(BuiltinEventCatalog),
            event_chance_override: None,
            rng: SimRandom::new(0),
            state: empty_state(),
            plugin: scenario_plugin_for("ura"),
            telemetry: SimTelemetry::new(),
            last_terminal: None,
        }
    }

    /// JVM-parity factory: loads research/*.json and canonical KB from detected repo root.
    pub fn create(settings: SimSettings) -> Self {
        factory::init_from_detected_repo(true);
        Self {
            settings,
            training_resolver: TrainingResolver::from_installed_tables(),
            event_catalog: crate::catalog::event::active_event_catalog(),
            event_chance_override: None,
            rng: SimRandom::new(0),
            state: empty_state(),
            plugin: scenario_plugin_for("ura"),
            telemetry: SimTelemetry::new(),
            last_terminal: None,
        }
    }

    /// Matches Kotlin `SimEngine(eventCatalog = …, eventChanceOverride = …)`.
    pub fn with_event_catalog(
        settings: SimSettings,
        event_catalog: Arc<dyn EventCatalog>,
        event_chance_override: Option<f64>,
    ) -> Self {
        Self {
            settings,
            training_resolver: TrainingResolver::default(),
            event_catalog,
            event_chance_override,
            rng: SimRandom::new(0),
            state: empty_state(),
            plugin: scenario_plugin_for("ura"),
            telemetry: SimTelemetry::new(),
            last_terminal: None,
        }
    }

    pub fn start(&mut self, meta: RunMeta) -> SimStepResult {
        self.rng = SimRandom::with_trace(meta.seed, self.settings.trace_rng);
        self.plugin = scenario_plugin_for(&meta.scenario_id);
        let cal = TurnCalendar::career_start();
        let cal_label = cal.label();
        let legacy = if meta.legacy_factors.is_empty() {
            Default::default()
        } else {
            LegacyApplicator::build_legacy(&meta.legacy_factors, meta.parent_names.clone())
        };
        let base_stats = LegacyApplicator::apply_pink_aptitude(
            crate::state::TraineeStats {
                speed: 100,
                stamina: 100,
                power: 100,
                guts: 100,
                wit: 100,
            },
            &legacy,
        );
        let scenario_resources = self.plugin.initial_scenario_resources(&meta);
        let deck = if meta.deck_supports.is_empty() {
            DeckState::default()
        } else {
            DeckState {
                slots: DeckPlacement::build_from_specs(&meta.deck_supports),
            }
        };
        self.state = CareerState {
            meta: meta.clone(),
            date: cal.date,
            turn: cal.turn,
            stats: base_stats,
            energy: 100,
            max_energy: 100,
            mood: crate::state::MoodLevel::Normal,
            fans: 0,
            skill_points: 0,
            career_complete: false,
            facility_levels: default_facility_levels(),
            scenario_resources,
            legacy,
            deck,
            log: vec![format!(
                "Career started: {} / {} / seed={}",
                meta.trainee_name, meta.scenario_id, meta.seed
            )],
            ..empty_state()
        };
        self.begin_turn();
        if self.settings.trace_telemetry {
            self.telemetry.on_run_start(&self.state.meta);
        }
        self.snapshot(vec![format!("=== {cal_label} ===")])
    }

    pub fn restore(&mut self, snapshot: RunSnapshot) -> SimStepResult {
        self.settings = snapshot.settings;
        self.rng = SimRandom::restore_with_trace(
            snapshot.rng_seed,
            snapshot.rng_calls,
            self.settings.trace_rng,
        );
        self.plugin = scenario_plugin_for(&snapshot.meta.scenario_id);
        self.state = snapshot.state;
        if self.settings.trace_telemetry {
            self.telemetry.on_run_start(&self.state.meta);
        }
        self.snapshot(vec![format!("=== Resumed turn {} ===", self.state.turn)])
    }

    pub fn export(&self) -> RunSnapshot {
        RunSnapshot {
            meta: self.state.meta.clone(),
            settings: self.settings.clone(),
            state: self.state.clone(),
            rng_seed: self.rng.seed(),
            rng_calls: self.rng.call_count(),
        }
    }

    pub fn state(&self) -> &CareerState {
        &self.state
    }

    pub fn assign_deck_slot(&mut self, support_id: &str, facility: TrainingFacility) -> bool {
        let Some(updated) = DeckPlacement::reassign(&self.state.deck.slots, support_id, facility) else {
            return false;
        };
        self.state.deck.slots = updated;
        true
    }

    pub fn telemetry_log(&self) -> &[crate::telemetry::TurnTelemetryRecord] {
        self.telemetry.records()
    }

    pub fn export_telemetry_json(&self) -> String {
        self.telemetry.export_json()
    }

    pub fn export_telemetry_jsonl(&self) -> String {
        self.telemetry.export_jsonl()
    }

    pub fn choices(&self) -> Vec<SimChoice> {
        if self.state.career_complete {
            return Vec::new();
        }
        if self.state.phase == TurnPhase::MandatoryRace.as_str() {
            return vec![SimChoice {
                id: "race".into(),
                label: format!(
                    "Enter mandatory race: {}",
                    self.state.pending_race_id.as_deref().unwrap_or("?")
                ),
            }];
        }
        if self.state.phase == TurnPhase::Event.as_str() && !self.state.pending_event_options.is_empty() {
            return self
                .state
                .pending_event_options
                .iter()
                .enumerate()
                .map(|(idx, opt)| {
                    let preview = opt.lines().next().unwrap_or("Option").chars().take(40).collect::<String>();
                    SimChoice {
                        id: format!("event_{idx}"),
                        label: preview,
                    }
                })
                .collect();
        }
        let mut choices = vec![
            SimChoice { id: "train_speed".into(), label: "Train Speed".into() },
            SimChoice { id: "train_stamina".into(), label: "Train Stamina".into() },
            SimChoice { id: "train_power".into(), label: "Train Power".into() },
            SimChoice { id: "train_guts".into(), label: "Train Guts".into() },
            SimChoice { id: "train_wit".into(), label: "Train Wits".into() },
            SimChoice { id: "rest".into(), label: "Rest".into() },
            SimChoice {
                id: "recreation".into(),
                label: if self.dating_available() {
                    "Pal Date".into()
                } else {
                    "Recreation".into()
                },
            },
            SimChoice { id: "race".into(), label: "Race (optional)".into() },
        ];
        choices.extend(self.plugin.extra_choices(&self.state));
        choices
    }

    pub fn step(&mut self, action: SimAction) -> SimStepResult {
        if self.state.career_complete {
            return self.snapshot(vec!["Career already complete.".to_string()]);
        }
        if self.state.phase == TurnPhase::MandatoryRace.as_str() && action.kind != SimActionKind::Race {
            return self.snapshot(vec!["Mandatory race pending — you must race this turn.".to_string()]);
        }
        if self.state.phase == TurnPhase::Event.as_str() && action.kind != SimActionKind::Choose {
            return self.snapshot(vec!["Event pending — choose an option (event_0, event_1, ...).".to_string()]);
        }

        let pre_state = self.state.clone();
        let mut lines = Vec::new();
        let mut advance_turn = true;
        match action.kind {
            SimActionKind::Train => {
                lines.push(self.do_train(&action));
                if self.state.phase == TurnPhase::Event.as_str() && self.state.awaiting_choice {
                    advance_turn = false;
                }
            }
            SimActionKind::Rest => lines.push(self.do_rest()),
            SimActionKind::Recreation => lines.push(self.do_recreation()),
            SimActionKind::Race => {
                lines.push(self.do_race(self.state.phase == TurnPhase::MandatoryRace.as_str()));
            }
            SimActionKind::Choose => {
                lines.push(self.do_event_choice(action.payload.as_deref()));
                advance_turn = true;
            }
            SimActionKind::Lesson => {
                if let Some(payload) = &action.payload {
                    if let Some((s, plugin_lines)) = self.plugin.apply_side_action(&self.state, payload) {
                        self.state = s;
                        lines.extend(plugin_lines);
                    } else {
                        lines.push("Invalid lesson action.".to_string());
                    }
                } else {
                    lines.push("Invalid lesson action.".to_string());
                }
                advance_turn = false;
            }
            SimActionKind::Advance => lines.push("No-op.".to_string()),
        }

        if advance_turn && !self.state.career_complete {
            self.end_turn(&mut lines);
        }

        if self.settings.trace_telemetry {
            self.telemetry.record(&self.state, &self.choices(), Some(&action));
            self.telemetry.record_transition(
                &pre_state,
                &self.state,
                &action,
                action.payload.as_ref().and_then(|p| p.parse().ok()),
            );
        }

        self.snapshot(lines)
    }

    pub fn auto_step_with_policy(
        &mut self,
        policy: impl FnOnce(&[SimChoice]) -> SimAction,
    ) -> SimStepResult {
        let ch = self.choices();
        if ch.is_empty() {
            return self.snapshot(Vec::new());
        }
        self.step(policy(&ch))
    }

    pub fn auto_step(&mut self) -> SimStepResult {
        self.auto_step_scoring()
    }

    pub fn auto_step_scoring(&mut self) -> SimStepResult {
        let choices = self.choices();
        let state = self.state.clone();
        let resolver = &self.training_resolver;
        let plugin = self.plugin.as_ref();
        let action = scoring_auto_policy(&choices, &state, resolver, plugin);
        self.step(action)
    }

    pub fn play_to_completion(&mut self, max_actions: i32) {
        self.play_to_completion_with_policy(max_actions, |choices| default_auto_policy(choices));
    }

    pub fn play_to_completion_scoring(&mut self, max_actions: i32) {
        let mut actions = 0;
        while !self.state.career_complete && actions < max_actions {
            let mult = self.settings.clamped_speed().max(1);
            for _ in 0..mult {
                if self.state.career_complete {
                    return;
                }
                let choices = self.choices();
                if choices.is_empty() {
                    return;
                }
                let state = self.state.clone();
                let action = scoring_auto_policy(
                    &choices,
                    &state,
                    &self.training_resolver,
                    self.plugin.as_ref(),
                );
                self.step(action);
                actions += 1;
            }
        }
    }

    /// Drive the career with the JVM policy server (`--policy=external` / `UMA_POLICY_CMD`).
    pub fn play_to_completion_external(&mut self, max_actions: i32) {
        let mut actions = 0;
        while !self.state.career_complete && actions < max_actions {
            let mult = self.settings.clamped_speed().max(1);
            for _ in 0..mult {
                if self.state.career_complete {
                    return;
                }
                let choices = self.choices();
                if choices.is_empty() {
                    return;
                }
                let state = self.state.clone();
                let action = crate::policy_external::external_auto_policy(
                    &choices,
                    &state,
                    &self.training_resolver,
                    self.plugin.as_ref(),
                );
                self.step(action);
                actions += 1;
            }
        }
    }

    pub fn play_to_completion_with_policy(
        &mut self,
        max_actions: i32,
        policy: impl Fn(&[SimChoice]) -> SimAction,
    ) {
        let mut actions = 0;
        while !self.state.career_complete && actions < max_actions {
            let mult = self.settings.clamped_speed().max(1);
            for _ in 0..mult {
                if self.state.career_complete {
                    return;
                }
                let choices = self.choices();
                if choices.is_empty() {
                    return;
                }
                self.step(policy(&choices));
                actions += 1;
                if self.state.career_complete {
                    return;
                }
            }
        }
    }

    pub fn run_summary(&self) -> crate::state::GoldenSummary {
        crate::state::GoldenSummary {
            seed: self.state.meta.seed,
            scenario: self.state.meta.scenario_id.clone(),
            turn: self.state.turn,
            fans: self.state.fans,
            speed: self.state.stats.speed,
            sp: self.state.skill_points,
        }
    }

    pub fn evaluate_terminal(&self) -> crate::scoring::CareerTerminalRecord {
        let s = &self.state;
        crate::scoring::evaluate_career_terminal(
            s.meta.seed,
            &s.meta.scenario_id,
            &s.meta.trainee_name,
            s.stats.speed,
            s.stats.stamina,
            s.stats.power,
            s.stats.guts,
            s.stats.wit,
            s.skill_points,
            &[],
        )
    }

    pub fn last_terminal(&self) -> Option<&crate::scoring::CareerTerminalRecord> {
        self.last_terminal.as_ref()
    }

    pub fn take_terminal(&mut self) -> Option<crate::scoring::CareerTerminalRecord> {
        self.last_terminal.take().or_else(|| {
            if self.state.career_complete {
                Some(self.evaluate_terminal())
            } else {
                None
            }
        })
    }

    pub fn rng_call_count(&self) -> u32 {
        self.rng.call_count()
    }

    pub fn rng_trace_log(&self) -> Vec<String> {
        self.rng.trace_log()
    }

    fn capture_turn_snapshot(&self) -> crate::state::TurnSnapshot {
        let s = &self.state;
        crate::state::TurnSnapshot {
            turn: s.turn,
            energy: s.energy,
            mood: s.mood.kotlin_name().to_string(),
            fans: s.fans,
            skill_points: s.skill_points,
            speed: s.stats.speed,
            stamina: s.stats.stamina,
            power: s.stats.power,
            guts: s.stats.guts,
            wit: s.stats.wit,
            phase: s.phase.clone(),
            scenario_resources: s.scenario_resources.values.clone(),
            rng_call_count: self.rng.call_count(),
        }
    }

    pub fn run_turn_trace(&mut self, max_actions: i32) -> Vec<crate::state::TurnSnapshot> {
        let mut snapshots = Vec::new();
        snapshots.push(self.capture_turn_snapshot());
        let mut last_turn = self.state.turn;
        let mut actions = 0;
        while !self.state.career_complete && actions < max_actions {
            let choices = self.choices();
            if choices.is_empty() {
                break;
            }
            self.step(default_auto_policy(&choices));
            actions += 1;
            let snap = self.capture_turn_snapshot();
            if snap.turn != last_turn {
                snapshots.push(snap);
                last_turn = self.state.turn;
            }
        }
        snapshots
    }

    fn snapshot(&self, extra_lines: Vec<String>) -> SimStepResult {
        let renderer = TextRenderer::new(self.settings.clone());
        SimStepResult {
            state: self.state.clone(),
            text_lines: renderer.render(&self.state, &extra_lines),
            choices: self.choices(),
            career_ended: self.state.career_complete,
        }
    }

    fn begin_turn(&mut self) {
        let (mut next, _lines) = self.plugin.on_turn_start(&self.state);
        if next.phase == TurnPhase::MandatoryRace.as_str() || next.phase == TurnPhase::Event.as_str() {
            self.state = next;
            return;
        }
        if self.rng.next_boolean(EventProbabilityConfig::inspiration_chance_per_turn()) {
            let bonus = InspirationConfig::roll_bonus(&mut self.rng);
            next.phase = TurnPhase::Event.as_str().to_string();
            next.awaiting_choice = true;
            next.pending_event_title = Some("Inspiration!".to_string());
            next.pending_event_options = InspirationConfig::event_options(bonus);
            next.log.push("Inspiration struck!".to_string());
            self.state = next;
            return;
        }
        if let Some(event) = self
            .event_catalog
            .pick_random(&next.meta.trainee_name, next.turn, &mut self.rng)
        {
            let mut chance = self
                .event_chance_override
                .unwrap_or_else(|| EventProbabilityConfig::event_chance_for(&next));
            // Grand Live Support Chain concert bonuses raise chain-event frequency.
            let chain = next.scenario_resources.get("bonus_support_chain_pct");
            if chain > 0 {
                chance = (chance * (1.0 + 0.12 * chain as f64)).clamp(0.0, 0.95);
            }
            if self.rng.next_boolean(chance) {
                next.phase = TurnPhase::Event.as_str().to_string();
                next.awaiting_choice = true;
                next.pending_event_title = Some(event.title.clone());
                next.pending_event_options = event.options;
                next.log.push(format!("Event: {}", event.title));
            }
        }
        self.state = next;
    }

    fn end_turn(&mut self, lines: &mut Vec<String>) {
        if self.state.career_complete {
            return;
        }
        let cal = TurnCalendar {
            date: self.state.date.clone(),
            turn: self.state.turn,
        }
        .advance();
        self.state.date = cal.date.clone();
        self.state.turn = cal.turn;
        self.state.phase = TurnPhase::Free.as_str().to_string();
        self.state.pending_event_title = None;
        self.state.pending_event_options.clear();
        self.state.awaiting_choice = false;
        if self.state.turn >= CAREER_TURNS {
            self.state.career_complete = true;
            self.state.phase = TurnPhase::Complete.as_str().to_string();
            self.state.log.push("Career complete".to_string());
            lines.push(format!("=== Career complete (turn {CAREER_TURNS}) ==="));
            let terminal = self.evaluate_terminal();
            lines.push(format!(
                "Terminal U={:.3} grade={} score={} (shop +{}) φ={:.2} ψ={:.2} brackets={}/5@600 {}/5@1100",
                terminal.u,
                terminal.grade,
                terminal.score,
                terminal.score - terminal.score_before_shop,
                terminal.phi_blue,
                terminal.psi_grade,
                terminal.brackets.at_or_above_600,
                terminal.brackets.at_or_above_1100,
            ));
            self.last_terminal = Some(terminal);
            return;
        }
        self.begin_turn();
        lines.push(format!("→ {}", cal.label()));
    }

    fn do_event_choice(&mut self, payload: Option<&str>) -> String {
        let idx: i32 = payload
            .map(|p| p.trim_start_matches("event_"))
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let options = self.state.pending_event_options.clone();
        if options.is_empty() {
            return "No event options.".to_string();
        }
        let title = self
            .state
            .pending_event_title
            .clone()
            .unwrap_or_else(|| "Event".to_string());
        let choice_idx = idx.clamp(0, options.len() as i32 - 1) as usize;
        let choice = options[choice_idx].clone();
        let mut lines = Vec::new();

        let mut after = self.state.clone();
        if title == "Trackblazer Pro Shop" {
            let (s, purchase_lines) = TrackblazerMechanics::apply_purchase(&after, &choice);
            after = s;
            lines.extend(purchase_lines);
            // Same RNG draws as legacy path (EventEffectApplier on choice text).
            let (s, effect_lines) =
                TrackblazerMechanics::apply_item_effects(&after, &choice, &mut self.rng);
            after = s;
            lines.extend(effect_lines);
        } else if title.to_lowercase().contains("happy meek") {
            let (s, duel_lines) = UraMechanics::resolve_duel(&after, &choice, &mut self.rng);
            after = s;
            lines.extend(duel_lines);
        } else {
            let (s, effect_lines) = EventEffectApplier::apply(&after, &choice, &mut self.rng);
            after = s;
            lines.extend(effect_lines);
        }

        after.phase = TurnPhase::Free.as_str().to_string();
        after.awaiting_choice = false;
        after.pending_event_title = None;
        after.pending_event_options.clear();
        after.log = self
            .state
            .log
            .iter()
            .cloned()
            .chain(std::iter::once(format!("Event '{title}' choice {idx}")))
            .collect();
        if title == "Inheritance" {
            after = LegacyApplicator::apply_inheritance_choice(&after, idx);
            after.legacy.inheritance_complete = true;
        }
        self.state = after;
        format!("Event: {title} → {}", lines.join("; "))
    }

    fn do_train(&mut self, action: &SimAction) -> String {
        if self.state.is_injured() {
            return "Injured — rest to recover before training.".to_string();
        }
        let facility = parse_facility(action.payload.as_deref()).unwrap_or(TrainingFacility::Speed);
        let key = facility.key();
        let level = self.facility_level(facility);
        let energy_before = self.state.energy;
        let outcome = self.training_resolver.resolve(
            facility,
            level,
            self.state.mood,
            &mut self.rng,
            Some(&self.state),
            None,
        );
        let scenario = self.state.meta.scenario_id.to_lowercase();
        let fail_pct = if (scenario == "unity" || scenario == "unity_cup")
            && UnityCupMechanics::zero_failure_when_burst_ready(&self.state.scenario_resources)
        {
            0
        } else if (scenario == "trackblazer" || scenario == "tb")
            && TrackblazerMechanics::zero_failure_ready(&self.state.scenario_resources)
        {
            0
        } else {
            TrainingFailureConfig::failure_chance_pct(
                (energy_before - outcome.energy_cost.max(0)).max(0),
                self.state.max_energy,
                self.state.mood,
                level,
            )
        };
        if outcome.energy_cost > 0 && energy_before < outcome.energy_cost {
            return format!(
                "Not enough energy to train ({} required, {} available).",
                outcome.energy_cost, energy_before
            );
        }
        if self.rng.next_boolean(fail_pct as f64 / 100.0) {
            let fail = TrainingFailureConfig::resolve_failure(
                energy_before,
                outcome.energy_cost.max(0),
                self.state.max_energy,
                &mut self.rng,
            );
            let new_statuses = if fail.injured && !self.state.is_injured() {
                let mut st = self.state.statuses.clone();
                st.push(crate::state::INJURED.to_string());
                st
            } else {
                self.state.statuses.clone()
            };
            self.state.energy = fail.energy;
            if fail.mood_dropped {
                self.state.mood = downgrade_mood(self.state.mood);
            }
            self.state.statuses = new_statuses;
            self.state.log.push(format!("Training failed at {}.", facility.name()));
            let (after, fail_lines) = self.plugin.on_training_complete(&self.state, facility, false);
            self.state = after;
            let injury_note = if fail.injured { " Injury sustained!" } else { "" };
            let extra = if fail_lines.is_empty() {
                String::new()
            } else {
                format!(" {}", fail_lines.join(" "))
            };
            return format!("Training FAILED at {}.{injury_note}{extra}", facility.name());
        }
        let scaled_gain =
            (outcome.main_gain as f64 * self.plugin.training_stat_multiplier(&self.state)) as i32;
        let mastery_stat = if self
            .state
            .meta
            .scenario_id
            .eq_ignore_ascii_case("grand_concert")
        {
            GrandLiveMechanics::mastery_train_stat_bonus(&self.state.scenario_resources, facility)
        } else {
            0
        };
        let capped_gain = self.apply_stat_cap(facility, scaled_gain + mastery_stat);
        let sec_fac = self.training_resolver.secondary_facility(facility);
        let ter_fac = self.training_resolver.tertiary_facility(facility);
        let capped_secondary = self.apply_stat_cap(
            sec_fac,
            (outcome.secondary_gain as f64 * self.plugin.training_stat_multiplier(&self.state)) as i32,
        );
        let capped_tertiary = self.apply_stat_cap(
            ter_fac,
            (outcome.tertiary_gain as f64 * self.plugin.training_stat_multiplier(&self.state)) as i32,
        );
        let hint_gain = if self.rng.next_boolean(HintProgressionConfig::training_hint_chance()) {
            1
        } else {
            0
        };
        let mastery_sp = if self
            .state
            .meta
            .scenario_id
            .eq_ignore_ascii_case("grand_concert")
        {
            GrandLiveMechanics::mastery_train_sp_bonus(&self.state.scenario_resources)
        } else {
            0
        };
        let new_hint_level = if hint_gain > 0 {
            HintProgressionConfig::apply_training_hint(self.state.hint_levels.get(key).copied().unwrap_or(0))
        } else {
            self.state.hint_levels.get(key).copied().unwrap_or(0)
        };
        let (new_levels, new_counts) = if FacilityLevelConfig::uses_train_count_leveling(&self.state.meta.scenario_id) {
            FacilityLevelConfig::apply_successful_train(
                facility,
                &self.state.facility_levels,
                &self.state.facility_train_counts,
            )
        } else {
            (self.state.facility_levels.clone(), self.state.facility_train_counts.clone())
        };
        let old_level = level;
        let new_level = new_levels.get(key).copied().unwrap_or(level);
        self.state.stats = self
            .state
            .stats
            .with_delta(facility, capped_gain)
            .with_delta(sec_fac, capped_secondary)
            .with_delta(ter_fac, capped_tertiary);
        self.state.skill_points += hint_gain + mastery_sp;
        if hint_gain > 0 {
            self.state.hint_levels.insert(key.to_string(), new_hint_level);
        }
        self.state.deck = BondGainConfig::apply_training_bond(&self.state.deck, facility);
        self.state.facility_levels = new_levels;
        self.state.facility_train_counts = new_counts;
        self.state.energy = apply_energy_after_training(
            energy_before,
            outcome.energy_cost,
            self.state.max_energy,
        );
        self.state.log.push(format!("Trained {} +{capped_gain}", facility.name()));
        let (after, plugin_lines) = self.plugin.on_training_complete(&self.state, facility, true);
        self.state = after;
        let energy_line = if outcome.energy_cost < 0 {
            format!("energy +{}", -outcome.energy_cost)
        } else {
            format!("energy -{}", outcome.energy_cost)
        };
        let level_line = if new_level > old_level {
            format!(" | {} facility → Lv{new_level}", facility.name())
        } else {
            String::new()
        };
        let extra = if plugin_lines.is_empty() {
            String::new()
        } else {
            format!(" | {}", plugin_lines.join(" "))
        };
        format!(
            "Trained {}: +{capped_gain} (Lv{old_level}, {energy_line}){level_line}{extra}",
            facility.name()
        )
    }

    fn do_rest(&mut self) -> String {
        let healed = self.state.is_injured();
        let gain = MoodEnergyConfig::rest_energy_gain();
        self.state.energy = (self.state.energy + gain).min(self.state.max_energy);
        if self.rng.next_boolean(MoodEnergyConfig::rest_mood_upgrade_chance()) {
            self.state.mood = upgrade_mood(self.state.mood);
        }
        self.state.statuses = CareerState::without_injury(&self.state.statuses);
        self.state.log.push(if healed {
            "Rested (injury healed)".to_string()
        } else {
            "Rested".to_string()
        });
        if healed {
            format!("Rested: +{gain} energy, injury healed")
        } else {
            format!("Rested: +{gain} energy")
        }
    }

    fn dating_available(&self) -> bool {
        self.state.meta.scenario_id.eq_ignore_ascii_case("grand_concert")
            && GrandLiveMechanics::dating_unlocked(&self.state.scenario_resources)
            && GrandLiveDeckSupport::any_scenario_link_in_deck(&self.state)
    }

    fn do_recreation(&mut self) -> String {
        if self.dating_available() {
            // Pal Date (recreation submenu) — replaces generic outing after dating unlock.
            let gain = MoodEnergyConfig::recreation_energy_gain() + 5;
            self.state.energy = (self.state.energy + gain).min(self.state.max_energy);
            self.state.mood = upgrade_mood(self.state.mood);
            for slot in &mut self.state.deck.slots {
                if GrandLiveDeckSupport::is_scenario_link(&slot.support_id) {
                    slot.bond = (slot.bond + 5).min(100);
                }
            }
            self.state.log.push("Pal Date".to_string());
            return format!("Pal Date: mood up, +{gain} energy, link bond +5");
        }
        let gain = MoodEnergyConfig::recreation_energy_gain();
        self.state.energy = (self.state.energy + gain).min(self.state.max_energy);
        if self.rng.next_boolean(MoodEnergyConfig::recreation_mood_upgrade_chance()) {
            self.state.mood = upgrade_mood(self.state.mood);
        }
        self.state.log.push("Recreation".to_string());
        format!("Recreation: mood up, +{gain} energy")
    }

    fn do_race(&mut self, mandatory: bool) -> String {
        let race_id = self
            .state
            .pending_race_id
            .clone()
            .unwrap_or_else(|| "optional".to_string());

        // Stub = always First (golden parity). Physics = uma-race-core field; seed is
        // derived off-stream so career RNG draw count is unchanged vs stub.
        let (placement, place, physics_note) = match self.settings.race_model {
            crate::race::RaceModel::Stub => (RacePlacement::First, 1usize, None),
            crate::race::RaceModel::Physics => {
                let out = crate::race::run_physics_race(&self.state, &race_id);
                (
                    out.placement,
                    out.place,
                    Some(format!(
                        "physics t={:.3}s course={} seed={} field={} margin_win={:.3}s margin_ahead={:.3}s",
                        out.finish_time,
                        out.course_id,
                        out.seed,
                        out.field_size,
                        out.margin_to_winner_s,
                        out.margin_ahead_s
                    )),
                )
            }
        };
        let won = crate::race::placement_counts_as_win(placement);

        let fan_gain =
            RaceOutcomeConfig::fan_gain_placed(mandatory, &race_id, placement, &mut self.rng);
        let sp_gain = RaceOutcomeConfig::skill_points_for(mandatory, placement)
            + if won {
                LegacyApplicator::race_win_skill_bonus(&self.state.legacy)
            } else {
                0
            };
        let mut completed = self.state.completed_races.clone();
        if mandatory {
            completed.push(race_id.clone());
        }
        self.state.fans += fan_gain;
        self.state.skill_points += sp_gain;
        self.state.completed_races = completed.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
        self.state.phase = TurnPhase::Free.as_str().to_string();
        self.state.pending_race_id = None;
        let mut epithet_note = String::new();
        if won {
            if let Some(ep) = RaceOutcomeConfig::grant_epithet(&mut self.state.statuses, &race_id) {
                epithet_note = format!(" | {ep}");
                self.state.log.push(format!("Epithet unlocked: {ep}"));
            }
        }
        // Stub log line stays Kotlin-identical (`Race {id} +{fans} fans`).
        let log_line = match &physics_note {
            None => format!("Race {race_id} +{fan_gain} fans"),
            Some(phys) => {
                let place_txt = crate::race::place_label(place);
                format!("Race {race_id} {place_txt} +{fan_gain} fans [{phys}]")
            }
        };
        self.state.log.push(log_line);
        let (after, race_lines) = self.plugin.on_race_complete(&self.state, &race_id, won);
        self.state = after;
        let extra = if race_lines.is_empty() {
            String::new()
        } else {
            format!(" | {}", race_lines.join(" "))
        };
        let place_txt = crate::race::place_label(place);
        let phys = physics_note
            .as_ref()
            .map(|s| format!(" [{s}]"))
            .unwrap_or_default();
        if mandatory {
            format!(
                "Mandatory race ({race_id}): {place_txt}! +{fan_gain} fans{extra}{epithet_note}{phys}"
            )
        } else {
            format!("Race: {place_txt}! +{fan_gain} fans{extra}{epithet_note}{phys}")
        }
    }

    fn apply_stat_cap(&self, facility: TrainingFacility, gain: i32) -> i32 {
        if gain <= 0 {
            return 0;
        }
        let key = facility.key();
        let base_cap = self.plugin.stat_caps().get(key).copied().unwrap_or(1400);
        let hard_cap = LegacyApplicator::effective_stat_cap(base_cap, key, &self.state.legacy)
            + UraMechanics::cap_bonus(&self.state.scenario_resources, key);
        let current = self.state.stats.get(facility);
        let after_soft = if self.state.meta.scenario_id.contains("grand") {
            self.plugin.apply_soft_cap(facility, current, gain)
        } else {
            let mult = soft_cap_effectiveness_multiplier(current, gain, hard_cap);
            (gain as f64 * mult) as i32
        };
        let room = (hard_cap - current).max(0);
        after_soft.min(room)
    }

    fn facility_level(&self, facility: TrainingFacility) -> i32 {
        if let Some(l) = self.plugin.effective_facility_level(&self.state, facility) {
            return l;
        }
        self.state
            .facility_levels
            .get(facility.key())
            .copied()
            .unwrap_or(1)
    }
}

fn apply_energy_after_training(before: i32, cost: i32, max_energy: i32) -> i32 {
    if cost < 0 {
        (before - cost).min(max_energy)
    } else {
        (before - cost).max(0)
    }
}

fn parse_facility(payload: Option<&str>) -> Option<TrainingFacility> {
    match payload?.to_lowercase().as_str() {
        "speed" | "train_speed" => Some(TrainingFacility::Speed),
        "stamina" | "train_stamina" => Some(TrainingFacility::Stamina),
        "power" | "train_power" => Some(TrainingFacility::Power),
        "guts" | "train_guts" => Some(TrainingFacility::Guts),
        "wit" | "wits" | "train_wit" => Some(TrainingFacility::Wit),
        _ => None,
    }
}

fn empty_state() -> CareerState {
    CareerState {
        meta: RunMeta::new(0, "ura", ""),
        date: SimDate { year: 1, month: 6, half: 2 },
        turn: 0,
        stats: Default::default(),
        energy: 0,
        max_energy: 100,
        mood: crate::state::MoodLevel::Normal,
        fans: 0,
        skill_points: 0,
        career_complete: false,
        awaiting_choice: false,
        pending_event_title: None,
        pending_race_id: None,
        phase: TurnPhase::Free.as_str().to_string(),
        completed_races: Vec::new(),
        facility_levels: default_facility_levels(),
        facility_train_counts: Default::default(),
        pending_event_options: Vec::new(),
        hint_levels: Default::default(),
        statuses: Vec::new(),
        performance_tokens: Default::default(),
        scenario_resources: Default::default(),
        legacy: Default::default(),
        learned_skill_ids: Vec::new(),
        deck: Default::default(),
        log: Vec::new(),
    }
}

pub fn run_career_summary(seed: i64, scenario: &str) -> crate::state::GoldenSummary {
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 50,
        ..Default::default()
    });
    engine.start(RunMeta::new(seed, scenario, "Special Week"));
    engine.play_to_completion(500);
    engine.run_summary()
}

pub fn run_turn_trace_fixture(seed: i64, scenario: &str) -> crate::state::TurnTraceFixture {
    // Kotlin parity traces were recorded under stub (always-win) placement branches.
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 1,
        race_model: crate::race::RaceModel::Stub,
        ..Default::default()
    });
    engine.start(RunMeta::new(seed, scenario, "Special Week"));
    let snapshots = engine.run_turn_trace(500);
    crate::state::TurnTraceFixture {
        seed,
        scenario: scenario.to_string(),
        snapshots,
    }
}

pub fn run_rng_trace_fixture(seed: i64, scenario: &str) -> crate::state::RngTraceFixture {
    // Pin stub: physics placement changes which post-race career-RNG branches run
    // (fan rolls / scenario hooks). Race physics itself draws zero career RNG.
    let mut engine = SimEngine::new(SimSettings {
        speed_multiplier: 1,
        trace_rng: true,
        race_model: crate::race::RaceModel::Stub,
        ..Default::default()
    });
    engine.start(RunMeta::new(seed, scenario, "Special Week"));
    engine.play_to_completion(500);
    crate::state::RngTraceFixture {
        seed,
        scenario: scenario.to_string(),
        entries: engine.rng_trace_log(),
    }
}
