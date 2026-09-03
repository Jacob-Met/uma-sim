pub mod api;
pub mod bot;
pub mod calendar;
pub mod catalog;
pub mod config;
pub mod content;
pub mod deck;
pub mod engine;
pub mod events;
pub mod factory;
pub mod legacy;
pub mod mid_run_inheritance;
pub mod policy;
pub mod policy_external;
pub mod race;
pub mod render;
pub mod rng;
pub mod scenario;
pub mod scoring;
pub mod session;
pub mod snapshot;
pub mod state;
pub mod telemetry;
pub mod training;

pub use bot::{scoring_auto_policy, BotDecisionAdapter};
pub use calendar::{TurnCalendar, CAREER_TURNS};
pub use catalog::support::{SupportCardMeta, SupportCatalog};
pub use catalog::trainee::{TraineeCatalog, TraineeMeta};
pub use config::{
    BondGainConfig, EventProbabilityConfig, FacilityLevelConfig, HintProgressionConfig,
    InspirationConfig, RaceOutcomeConfig, RacePlacement, ScenarioResearchConfig,
    TrainingFailureConfig,
};
pub use content::{ContentPackLoader, ContentPackRegistry};
pub use deck::{DeckPlacement, DeckSpec, DeckSupportBridge, DeckTrainingSignals};
pub use engine::{
    run_career_summary, run_rng_trace_fixture, run_turn_trace_fixture, SimEngine, SimStepResult,
};
pub use events::{
    BuiltinEventCatalog, EventCatalog, EventEffectApplier, EventEffectReading, SimEventEntry,
};
pub use factory::{
    detect_repo_root, init_engine_resources, init_from_detected_repo, load_training_tables,
    RESEARCH_FILES,
};
pub use legacy::{
    blue_cap_bonus, blue_starting_stat_bonus, pink_aptitude_rank_ups, raise_aptitude_letter,
    raise_aptitude_letter_uncapped, LegacyApplicator, LegacyDeckConfig, LegacyFactorContext,
    LegacyFactorMeta,
};
pub use policy::default_auto_policy;
pub use race::{
    derive_race_seed, horse_input_from_career, horse_input_from_career_on_course,
    race_effective_stat, run_physics_race, RaceModel, RaceScheduler,
};
pub use render::TextRenderer;
pub use rng::SimRandom;
pub use scenario::{
    scenario_plugin_for, ConcertOutcome, DuelContest, DuelPrediction, GrandConcertScenarioPlugin,
    GrandLiveCalibrationLoader, GrandLiveCatalog, GrandLiveCatalogLoader, GrandLiveDeckSupport,
    GrandLiveLessonBoard, GrandLiveLessonScoring, GrandLiveMasteryBonus, GrandLiveMechanics,
    ScenarioPlugin, TrackblazerMechanics, TrackblazerScenarioPlugin, UnityCupMechanics,
    UnityCupScenarioPlugin, UraMechanics, UraScenarioPlugin, PERF_CODES,
};
pub use scoring::soft_cap_effectiveness_multiplier;
pub use scoring::{
    calculate_raw_training_score, choose_best_event_option, score_lesson_option, DecisionContext,
    TrainingConfig, TrainingOption,
};
pub use session::{parse_sim_action, RunSession};
pub use snapshot::{RunSnapshot, RunSnapshotCodec};
pub use state::{
    default_facility_levels, AncestorSparks, CareerState, DeckSlot, DeckState, DialogueMode,
    GoldenSummary, LegacyState, LegacyTree, MoodLevel, RngTraceFixture, RunMeta, ScenarioResources,
    SimAction, SimActionKind, SimChoice, SimDate, SimSettings, SparkSlot, StatName, TraineeStats,
    TrainingFacility, TurnPhase, TurnSnapshot, TurnTraceFixture, INJURED,
};
pub use telemetry::{SimReplayLine, SimTelemetry, TelemetryReplayLoader};
pub use training::{TrainingGainContext, TrainingOutcome, TrainingPreview, TrainingResolver};
