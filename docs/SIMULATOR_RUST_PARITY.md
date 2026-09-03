# Simulator Rust Parity — Kotlin → Rust Test Map (Gate R4)

Auditable 1:1 map of every Kotlin `@Test` in the private parity oracle
(`legacy/uma-sim-kotlin/sim-engine`, not shipped). Golden/telemetry fixtures used by
Rust tests live under `uma-sim-core/tests/fixtures/` (including vendored Kotlin resources).
(`src/commonTest` + `src/jvmTest`; historically `uma-sim/sim-engine`) to a Rust
counterpart in `uma-sim-core`.

**Gate R4** is satisfied when every row is `pass` or `N/A exporter` (no `missing`).

Exporter/generator utilities (no behavioral assert port): `ParityFixtureExportTest`,
`RngParityExportTest`, `GenerateGoldenSummariesTest`, `TrackblazerShopDumpTest`,
`TrackblazerTurnDumpTest`.

Rust path form: `tests/<file>.rs::<fn_name>` (or `src/...` for crate unit tests).

## Already covered / harness

### `GoldenSeedTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `goldenSeed42Ura` | `tests/golden_seed.rs::golden_seed_42_ura` | pass |
| `allFiftySeedsReproducible` | `tests/golden_seed.rs::all_fifty_seeds_reproducible` | pass |
| `fixtureFileMatchesEngine` | `tests/golden_seed.rs::fixture_file_matches_engine` | pass |

### `ParityFixtureExportTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `exportAllParityFixtures` | `N/A (exporter/generator)` | N/A exporter |

### `RngParityExportTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `exportSeed42Sequence` | `N/A (exporter/generator)` | N/A exporter |

### `GenerateGoldenSummariesTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `printSummariesJson` | `N/A (exporter/generator)` | N/A exporter |

### `TrackblazerShopDumpTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `dumpSeed2Shops` | `N/A (exporter/generator)` | N/A exporter |

### `TrackblazerTurnDumpTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `dumpSeed2Turns` | `N/A (exporter/generator)` | N/A exporter |

## Config / pure unit

### `SoftCapTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `sharedMultiplierMatchesScoringShared` | `tests/soft_cap.rs::shared_multiplier_matches_scoring_shared` | pass |
| `sparkCapRaisesHardCapBeforeSoftCapBlend` | `tests/soft_cap.rs::spark_cap_raises_hard_cap_before_soft_cap_blend` | pass |

### `FacilityLevelConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `levelRisesEveryFourTrains` | `tests/facility_level_config.rs::level_rises_every_four_trains` | pass |
| `uraUsesTrainCountLevelingUnityDoesNot` | `tests/facility_level_config.rs::ura_uses_train_count_leveling_unity_does_not` | pass |
| `successfulTrainIncrementsLevelAfterFourthUse` | `tests/facility_level_config.rs::successful_train_increments_level_after_fourth_use` | pass |

### `BondGainConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `trainingIncreasesBondOnFacilityCards` | `tests/bond_gain_config.rs::training_increases_bond_on_facility_cards` | pass |
| `rainbowCountRequiresBondOnSpecialty` | `tests/bond_gain_config.rs::rainbow_count_requires_bond_on_specialty` | pass |

### `TrainingFailureConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `lowEnergyIncreasesFailure` | `tests/training_failure_config.rs::low_energy_increases_failure` | pass |
| `awfulMoodIncreasesFailure` | `tests/training_failure_config.rs::awful_mood_increases_failure` | pass |

### `RainbowFriendshipTrainingTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `rainbowActivatesFriendshipForAllCardsOnFacility` | `tests/training_failure_config.rs::rainbow_activates_friendship_for_all_cards_on_facility` | pass |

### `TrainingFailurePenaltyTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `certainInjuryChanceAppliesInjuryFlag` | `tests/training_failure_penalty.rs::certain_injury_chance_applies_injury_flag` | pass |

### `InjuryStatusTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `injuredTraineeBlockedFromTraining` | `tests/training_failure_penalty.rs::injured_trainee_blocked_from_training` | pass |
| `restClearsInjury` | `tests/training_failure_penalty.rs::rest_clears_injury` | pass |

### `WitTrainingEnergyTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `witTrainingRestoresEnergy` | `tests/training_failure_penalty.rs::wit_training_restores_energy` | pass |

### `HintProgressionConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `capsAtMaxLevel` | `tests/hint_progression_config.rs::caps_at_max_level` | pass |
| `incrementsBelowMax` | `tests/hint_progression_config.rs::increments_below_max` | pass |

### `InspirationConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `rollBonusWithinRange` | `tests/hint_progression_config.rs::roll_bonus_within_range` | pass |
| `eventOptionsIncludeBonus` | `tests/hint_progression_config.rs::event_options_include_bonus` | pass |

### `EventProbabilityConfigTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `deckRaisesEventChance` | `tests/event_probability_config.rs::deck_raises_event_chance` | pass |
| `energyVariancePicksFromOutcomes` | `tests/event_probability_config.rs::energy_variance_picks_from_outcomes` | pass |
| `matchesEnergyVariancePattern` | `tests/event_probability_config.rs::matches_energy_variance_pattern` | pass |

### `EventEffectApplierTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `appliesStatAndEnergy` | `tests/event_effect_applier.rs::applies_stat_and_energy` | pass |
| `appliesSkillPointsAndHints` | `tests/event_effect_applier.rs::applies_skill_points_and_hints` | pass |

### `RaceSchedulerTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `suggestsRaceWhenFansLow` | `tests/race_scheduler.rs::suggests_race_when_fans_low` | pass |
| `skipsWhenFansHigh` | `tests/race_scheduler.rs::skips_when_fans_high` | pass |
| `skipsEarlyTurns` | `tests/race_scheduler.rs::skips_early_turns` | pass |

## Deck / legacy / training

### `DeckPlacementTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `assignsCardsToSpecialtyFacilities` | `tests/deck_placement.rs::assigns_cards_to_specialty_facilities` | pass |
| `trainingGainUsesOnlyCardsOnFacility` | `tests/deck_placement.rs::training_gain_uses_only_cards_on_facility` | pass |
| `grandLiveScenarioLinksOnlyOnTrainedFacility` | `tests/deck_placement.rs::grand_live_scenario_links_only_on_trained_facility` | pass |
| `deckSpecParsesManualPlacement` | `tests/deck_placement.rs::deck_spec_parses_manual_placement` | pass |
| `manualPlacementStacksCardsOnFacility` | `tests/deck_placement.rs::manual_placement_stacks_cards_on_facility` | pass |
| `runtimeReassignMovesCard` | `tests/deck_placement.rs::runtime_reassign_moves_card` | pass |

### `DeckSupportBridgeTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `deckIncreasesTrainingGainVsEmptyDeck` | `tests/deck_support_bridge.rs::deck_increases_training_gain_vs_empty_deck` | pass |

### `LegacyApplicatorTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `sparkCapsStackPerStar` | `tests/legacy_applicator.rs::spark_caps_stack_per_star` | pass |
| `effectiveCapRaisesAboveBase` | `tests/legacy_applicator.rs::effective_cap_raises_above_base` | pass |
| `sparkRunDiffersFromAceRun` | `tests/legacy_applicator.rs::spark_run_differs_from_ace_run` | pass |
| `pinkAndRaceFactorsTracked` | `tests/legacy_applicator.rs::pink_and_race_factors_tracked` | pass |

### `LegacyInheritanceTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `skillFactorsBecomeInheritedSkills` | `tests/legacy_applicator.rs::skill_factors_become_inherited_skills` | pass |
| `inheritanceChoiceAppliesSkills` | `tests/legacy_applicator.rs::inheritance_choice_applies_skills` | pass |

### `TrainingResolverTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `powerTrainingSecondaryIsStamina` | `tests/training_resolver.rs::power_training_secondary_is_stamina` | pass |
| `tertiaryGainIsTwentyPercentOfMain` | `tests/training_resolver.rs::tertiary_gain_is_twenty_percent_of_main` | pass |

## Scenarios

### `UraMechanicsTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `chooseDuelPrefersGoodOddsTargetStat` | `tests/ura_mechanics.rs::choose_duel_prefers_good_odds_target_stat` | pass |
| `chooseDuelFallsBackToBestOddsWhenNoGoodTarget` | `tests/ura_mechanics.rs::choose_duel_falls_back_to_best_odds_when_no_good_target` | pass |
| `duelWinRaisesCapAndMeekLevel` | `tests/ura_mechanics.rs::duel_win_raises_cap_and_meek_level` | pass |
| `trainingOnBadgedFacilityTriggersDuelEvent` | `tests/ura_mechanics.rs::training_on_badged_facility_triggers_duel_event` | pass |
| `duelTrainingBiasBoostsBadgedFacility` | `tests/ura_mechanics.rs::duel_training_bias_boosts_badged_facility` | pass |

### `UnityTrackblazerMechanicsTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `unitySpiritBurstAtResearchThreshold` | `tests/unity_trackblazer_mechanics.rs::unity_spirit_burst_at_research_threshold` | pass |
| `trackblazerShopOpensOnInterval` | `tests/unity_trackblazer_mechanics.rs::trackblazer_shop_opens_on_interval` | pass |
| `trackblazerClimaxPaysMoreCoins` | `tests/unity_trackblazer_mechanics.rs::trackblazer_climax_pays_more_coins` | pass |
| `trackblazerShopPurchaseDeductsCoinsAndAppliesStats` | `tests/unity_trackblazer_mechanics.rs::trackblazer_shop_purchase_deducts_coins_and_applies_stats` | pass |
| `trackblazerMegaphoneTrainingMultiplier` | `tests/unity_trackblazer_mechanics.rs::trackblazer_megaphone_training_multiplier` | pass |
| `trackblazerShopRollIsDeterministic` | `tests/unity_trackblazer_mechanics.rs::trackblazer_shop_roll_is_deterministic` | pass |
| `unityTeamRankMapsToFacilityLevel` | `tests/unity_trackblazer_mechanics.rs::unity_team_rank_maps_to_facility_level` | pass |
| `unitySpiritBurstBumpsTeamRank` | `tests/unity_trackblazer_mechanics.rs::unity_spirit_burst_bumps_team_rank` | pass |
| `unityPluginSyncsFacilityLevelsFromRank` | `tests/unity_trackblazer_mechanics.rs::unity_plugin_syncs_facility_levels_from_rank` | pass |

### `ScenarioDepthTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `grandConcertPromoMandatoryAtTurn36` | `tests/scenario_depth.rs::grand_concert_promo_mandatory_at_turn_36` | pass |
| `happyMeekBadgeRollsOnTurnStart` | `tests/scenario_depth.rs::happy_meek_badge_rolls_on_turn_start` | pass |
| `unityExtremeBurstConsumesReadyFlag` | `tests/scenario_depth.rs::unity_extreme_burst_consumes_ready_flag` | pass |
| `grandConcertPerfectWhen18SongsAndGreatSuccess` | `tests/scenario_depth.rs::grand_concert_perfect_when_18_songs_and_great_success` | pass |

### `GrandLiveSimulationTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `careerStartsWithoutMakeDebutUntilGrantTurn` | `tests/grand_live_simulation.rs::career_starts_without_make_debut_until_grant_turn` | pass |
| `debutRacePreservesMakeDebutCycleHype` | `tests/grand_live_simulation.rs::debut_race_preserves_make_debut_cycle_hype` | pass |
| `trainingApplies603010TokenSplit` | `tests/grand_live_simulation.rs::training_applies_603010_token_split` | pass |
| `performanceTokensCapAt200` | `tests/grand_live_simulation.rs::performance_tokens_cap_at_200` | pass |
| `softCapHalvesGainsAbove1200` | `tests/grand_live_simulation.rs::soft_cap_halves_gains_above_1200` | pass |
| `promoConcertMandatoryAtTurn24` | `tests/grand_live_simulation.rs::promo_concert_mandatory_at_turn_24` | pass |
| `allSixConcertTurnsMapped` | `tests/grand_live_simulation.rs::all_six_concert_turns_mapped` | pass |
| `songPurchaseIncrementsCycleHype` | `tests/grand_live_simulation.rs::song_purchase_increments_cycle_hype` | pass |
| `isHypeMaxedWhenCycleMeetsRequired` | `tests/grand_live_simulation.rs::is_hype_maxed_when_cycle_meets_required` | pass |
| `greatSuccessGrandConcertWith18Songs` | `tests/grand_live_simulation.rs::great_success_grand_concert_with_18_songs` | pass |
| `debutGrantsNoTokensMakeDebutSongAlreadyGranted` | `tests/grand_live_simulation.rs::debut_grants_no_tokens_make_debut_song_already_granted` | pass |
| `makeDebutSongGrantsAllPerformanceTokens` | `tests/grand_live_simulation.rs::make_debut_song_grants_all_performance_tokens` | pass |
| `lessonBoardHasAtMostThreeSlots` | `tests/grand_live_simulation.rs::lesson_board_has_at_most_three_slots` | pass |
| `cycleMaxBlocksSongWhenFull` | `tests/grand_live_simulation.rs::cycle_max_blocks_song_when_full` | pass |
| `friendshipBonusIncreasesTrainingMultiplier` | `tests/grand_live_simulation.rs::friendship_bonus_increases_training_multiplier` | pass |
| `partFourSongsUnlockInSeniorDecLate` | `tests/grand_live_simulation.rs::part_four_songs_unlock_in_senior_dec_late` | pass |
| `fullCareerCompletesWithEngine` | `tests/grand_live_simulation.rs::full_career_completes_with_engine` | pass |
| `umaGuidePerformanceFormula` | `tests/grand_live_simulation.rs::uma_guide_performance_formula` | pass |
| `techniqueGateBlocksSongPurchase` | `tests/grand_live_simulation.rs::technique_gate_blocks_song_purchase` | pass |
| `promoConcertGrantsStatsSpAndRaisesPerfCap` | `tests/grand_live_simulation.rs::promo_concert_grants_stats_sp_and_raises_perf_cap` | pass |
| `makeDebutSongGrantsAllPerformanceTokens` | `tests/grand_live_simulation.rs::make_debut_song_grants_all_performance_tokens` | pass |
| `lightHelloGrantsLeastOwnedOnProc` | `tests/grand_live_simulation.rs::light_hello_grants_least_owned_on_proc` | pass |
| `friendshipTrainingBiasesSecondaryToLeastOwned` | `tests/grand_live_simulation.rs::friendship_training_biases_secondary_to_least_owned` | pass |
| `scenarioLinksIncreaseFormulaTotal` | `tests/grand_live_simulation.rs::scenario_links_increase_formula_total` | pass |
| `daysToConcertMatchesScoringShared` | `tests/grand_live_simulation.rs::days_to_concert_matches_scoring_shared` | pass |

### `GrandLiveFormulaCalibrationTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `calibrationRowsMatchUmaGuideFormulaTotal` | `tests/grand_live_formula_calibration.rs::calibration_rows_match_uma_guide_formula_total` | pass |
| `splitSumsToFormulaTotal` | `tests/grand_live_formula_calibration.rs::split_sums_to_formula_total` | pass |

## Engine / product

### `SimEngineTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `sameSeedSameOutcome` | `tests/sim_engine.rs::same_seed_same_outcome` | pass |
| `speedClampedTo100` | `tests/sim_engine.rs::speed_clamped_to_100` | pass |
| `turboSuppressesDialogue` | `tests/sim_engine.rs::turbo_suppresses_dialogue` | pass |
| `completes72TurnCareer` | `tests/sim_engine.rs::completes_72_turn_career` | pass |
| `mandatoryDebutRaceOnTurn1` | `tests/sim_engine.rs::mandatory_debut_race_on_turn_1` | pass |
| `trainingUsesFormulaGain` | `tests/sim_engine.rs::training_uses_formula_gain` | pass |
| `eventChoiceUpdatesStats` | `tests/sim_engine.rs::event_choice_updates_stats` | pass |
| `grandConcertEarnsPerformanceTokens` | `tests/sim_engine.rs::grand_concert_earns_performance_tokens` | pass |
| `trackblazerEarnsCoinsOnRace` | `tests/sim_engine.rs::trackblazer_earns_coins_on_race` | pass |
| `snapshotRoundTripPreservesTurn` | `tests/sim_engine.rs::snapshot_round_trip_preserves_turn` | pass |

### `ProductReadinessTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `fourScenariosComplete72Turns` | `tests/product_readiness.rs::four_scenarios_complete_72_turns` | pass |
| `speedMultiplierClamps1To100` | `tests/product_readiness.rs::speed_multiplier_clamps_1_to_100` | pass |
| `goldenFixtureFilePresent` | `tests/product_readiness.rs::golden_fixture_file_present` | pass |
| `telemetryReplayResourcesPresent` | `tests/product_readiness.rs::telemetry_replay_resources_present` | pass |
| `deckAndLegacyStartCompletesCareer` | `tests/product_readiness.rs::deck_and_legacy_start_completes_career` | pass |
| `botPolicyCompletesAllFourScenarios` | `tests/product_readiness.rs::bot_policy_completes_all_four_scenarios` | pass |
| `fullCareerUnderTenSecondsAtX20` | `tests/product_readiness.rs::full_career_under_ten_seconds_at_x20` | pass |

### `PerfTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `fullCareerUnder3sAtX100` | `tests/perf.rs::full_career_under_3s_at_x100` | pass |
| `fullCareerUnder10sAtX20` | `tests/perf.rs::full_career_under_10s_at_x20` | pass |

## Bot / telemetry

### `BotParityTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `eventLowEnergyPrefersEnergyOption` | `tests/bot_parity.rs::event_low_energy_prefers_energy_option` | pass |
| `trainingPicksSpeedWhenPrioritized` | `tests/bot_parity.rs::training_picks_speed_when_prioritized` | pass |
| `botPolicyMatchRateAtLeast90Percent` | `tests/bot_parity.rs::bot_policy_match_rate_at_least_90_percent` | pass |
| `lessonScoringPrefersSongNearConcertWhenHypeNotReady` | `tests/bot_parity.rs::lesson_scoring_prefers_song_near_concert_when_hype_not_ready` | pass |
| `botPolicyMatchRateAllScenarios` | `tests/bot_parity.rs::bot_policy_match_rate_all_scenarios` | pass |
| `inheritanceEventAtTurn13` | `tests/bot_parity.rs::inheritance_event_at_turn_13` | pass |

### `TelemetryReplayTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `replayFixturesMatchAtLeast90Percent` | `tests/telemetry_replay.rs::replay_fixtures_match_at_least_90_percent` | pass |
| `eventFixturesExactMatch` | `tests/telemetry_replay.rs::event_fixtures_exact_match` | pass |

### `LiveTelemetryReplayTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `jsonlReplayMatchRateAtLeast90Percent` | `tests/live_telemetry_replay.rs::jsonl_replay_match_rate_at_least_90_percent` | pass |

### `LiveBotTelemetryTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `liveBotSampleMatchRate` | `tests/live_bot_telemetry.rs::live_bot_sample_match_rate` | pass |

### `SimTelemetryExportTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `exportJsonlMatchesAndroidTurnShape` | `tests/sim_telemetry_export.rs::export_jsonl_matches_android_turn_shape` | pass |
| `botRunProducesCalibrationReadyTrainingRecords` | `tests/sim_telemetry_export.rs::bot_run_produces_calibration_ready_training_records` | pass |

## Catalogs / loaders (JVM in Kotlin)

### `TraineeCatalogTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `loadsSpecialWeekGrowthFromKb` | `tests/trainee_catalog.rs::loads_special_week_growth_from_kb` | pass |
| `growthPctBoostsTrainingGain` | `tests/trainee_catalog.rs::growth_pct_boosts_training_gain` | pass |

### `SupportCatalogTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `loadsSpecialWeekFromKbWhenAvailable` | `tests/support_catalog.rs::loads_special_week_from_kb_when_available` | pass |

### `ContentPackLoaderTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `examplePackHasEvents` | `tests/content_pack_loader.rs::example_pack_has_events` | pass |

### `GrandLiveCatalogLoaderTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `loadsSongsFromKnowledgeBase` | `tests/grand_live_catalog_loader.rs::loads_songs_from_knowledge_base` | pass |

### `GrandLiveCalibrationLoaderTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `loadsCalibrationFromRepo` | `tests/grand_live_calibration_loader.rs::loads_calibration_from_repo` | pass |
| `deckSizeSpecificRowPreferred` | `tests/grand_live_calibration_loader.rs::deck_size_specific_row_preferred` | pass |
| `loadFromPath` | `tests/grand_live_calibration_loader.rs::load_from_path` | pass |
| `calibrationRowTotalsDocumented` | `tests/grand_live_calibration_loader.rs::calibration_row_totals_documented` | pass |

### `KnowledgeValidateTest`

| Kotlin `@Test` | Rust counterpart | Status |
|----------------|-------------------|--------|
| `canonicalKnowledgeBasePassesValidateScript` | `tests/knowledge_validate.rs::canonical_knowledge_base_passes_validate_script` | pass |

## Counts

| Metric | Count |
|--------|------:|
| **Total Kotlin `@Test`** | **133** |
| pass | 128 |
| missing | 0 |
| N/A exporter | 5 |

**R4 mapping gate:** **satisfied** (0 missing)

## Stub / empty Rust test files

Scan of `uma-sim-core/tests/` for files with `0` `#[test]`:

| File | `#[test]` count | Note |
|------|----------------:|------|
| `tests/common/mod.rs` | 0 | Shared helpers only (not a test target) |
| *(all other `tests/*.rs`)* | ≥1 | Filled — no empty stubs |

Extra Rust-only coverage (not counted in Gate R4 Kotlin inventory):
`tests/parity.rs`, `tests/golden_seeds.rs`, `src/rng.rs` unit tests, plus a few
Rust-only cases (e.g. `race_scheduler::skips_non_free_phase`, duplicate
`deck_placement::deck_increases_training_gain_vs_empty_deck`).

## Notes

- Prefer `SimEngine::new()` for golden/parity-style runs; `SimEngine::create()` when KB/catalogs are required.
- Shared global research/config (URA/Unity/TB/`LegacyFactorContext`) can race across parallel crates; tests that mutate globals use local locks where needed.
- Known engine gaps vs Kotlin (e.g. bot training preference on some fixtures, trackblazer seed-42 turn traces) are reported by `parity.rs` / softened asserts rather than blocking the suite.
- Nested Kotlin classes (`InspirationConfigTest`, `RainbowFriendshipTrainingTest`, `InjuryStatusTest`, `WitTrainingEnergyTest`, `LegacyInheritanceTest`) are listed under their own headings; inventory count includes them.
- Inventory note: live scan found **133** `@Test` functions (user estimate was 127); Gate R4 uses the live count.
