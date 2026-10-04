/** Hand-written mirrors of uma-sim-core snapshot / catalog JSON (camelCase). */

export type MoodLevel = "AWFUL" | "BAD" | "NORMAL" | "GOOD" | "GREAT";

import type { LegacyTree } from "../components/LegacyPanel";

export interface CatalogItem {
  id: string;
  name: string;
  nameJa?: string;
  charId?: number;
  iconUrl?: string;
  playableEn?: boolean;
  type?: string;
  rarity?: number;
  kind?: string;
  /** Trainee card base stats [speed…wit]. */
  baseStats?: number[];
  /** Trainee aptitude letters keyed by turf/dirt/… */
  aptitudes?: Record<string, string>;
  /** Pink factor aptitude tag. */
  pinkTag?: string;
  /** Blue factor stat key. */
  statKey?: string;
}

export interface Choice {
  id: string;
  label: string;
}

export interface TraineeStats {
  speed: number;
  stamina: number;
  power: number;
  guts: number;
  wit: number;
}

export interface SimDate {
  year: number;
  month: number;
  half: number;
}

export interface DeckSlot {
  supportId: string;
  bond: number;
  specialty?: string | null;
  assignedFacility?: string | null;
}

export interface DeckState {
  slots: DeckSlot[];
}

export interface ScenarioResources {
  values: Record<string, number>;
}

export interface LegacyState {
  parentNames: string[];
  factorIds: string[];
  inheritedSkillIds: string[];
  sparkCaps: Record<string, number>;
  pinkFactorIds: string[];
  pinkAptitudeTags: string[];
  /** Effective aptitudes after pink inheritance. */
  aptitudes?: Record<string, string>;
  raceFactorIds: string[];
  inheritanceComplete: boolean;
}

export interface RunMeta {
  seed: number;
  scenarioId: string;
  traineeName: string;
  objectiveProfile: string;
  legacyFactors: string[];
  parentNames: string[];
  deckSupports: string[];
}

export interface SimSettings {
  dialogueMode: "OFF" | "CHOICES_ONLY" | "FULL";
  speedMultiplier: number;
  allowDialogueAtHighSpeed: boolean;
  traceTelemetry: boolean;
  traceRng: boolean;
  raceModel: "stub" | "physics";
}

export interface GeneratedSpark {
  color: string;
  factorId: string;
  stars: number;
  label: string;
}

export interface CareerState {
  meta: RunMeta;
  date: SimDate;
  turn: number;
  stats: TraineeStats;
  energy: number;
  maxEnergy: number;
  mood: MoodLevel;
  fans: number;
  skillPoints: number;
  careerComplete: boolean;
  awaitingChoice: boolean;
  pendingEventTitle?: string | null;
  pendingRaceId?: string | null;
  phase: string;
  completedRaces: string[];
  facilityLevels: Record<string, number>;
  facilityTrainCounts: Record<string, number>;
  pendingEventOptions: string[];
  hintLevels: Record<string, number>;
  statuses: string[];
  performanceTokens: Record<string, number>;
  scenarioResources: ScenarioResources;
  legacy: LegacyState;
  learnedSkillIds: string[];
  deck: DeckState;
  log: string[];
  generatedSparks?: GeneratedSpark[];
  /** Trainee base aptitudes before pink inheritance. */
  baseAptitudes?: Record<string, string>;
  /** Race strategy override: front | pace | late | end. */
  preferredRunningStyle?: string | null;
}

export interface RunSnapshot {
  meta: RunMeta;
  settings: SimSettings;
  state: CareerState;
  rngSeed: number;
  rngCalls: number;
  /** Exact internal RNG state words (present on snapshots from current builds). */
  rngState?: {
    x: number;
    y: number;
    z: number;
    w: number;
    v: number;
    addend: number;
  } | null;
}

export interface HealthResponse {
  ok: boolean;
  version: string;
  repoRoot: boolean;
  repoRootPath?: string | null;
}

export interface StepResponse {
  text: string;
  careerEnded: boolean;
  state: RunSnapshot;
  choices: Choice[];
}

export interface StartRequest {
  seed?: number | string;
  scenario?: string;
  trainee?: string;
  speed?: number | string;
  dialogue?: string;
  raceModel?: string;
  policy?: string;
  deckSupports?: string;
  legacyFactors?: string;
  /** Structured 2×2 inheritance tree (preferred over flat legacyFactors when populated). */
  legacyTree?: LegacyTree;
  parentNames?: string;
  /** Lineage compatibility score (0–500+); scales mid-run Inspiration odds. */
  compatibilityScore?: number;
  traceTelemetry?: boolean | string;
}

/** Display grade for overall compatibility (parent_farming_utility.md). */
export function compatibilityGrade(score: number): "◎" | "〇" | "△" {
  if (score > 150) return "◎";
  if (score >= 51) return "〇";
  return "△";
}

/* ------------------------------------------------------------------ */
/* Career lab (T100): named checkpoint library + branch & compare.     */
/* ------------------------------------------------------------------ */

export interface ContentFingerprint {
  coreVersion: string;
  snapshotSchema: number;
  eventCatalogCount: number;
  contentPackEvents: number;
  repoRootDetected: boolean;
}

export interface LibraryEntry {
  name: string;
  label: string;
  note: string;
  savedAt: string;
  savedAtUnix: number;
  seed: number;
  scenarioId: string;
  traineeName: string;
  turn: number;
  dateLabel: string;
  phase: string;
  rngCalls: number;
  fingerprint: ContentFingerprint;
}

export interface SessionInfo {
  id: string;
  label: string;
  turn: number;
  phase: string;
  careerComplete: boolean;
  seed: number;
  scenarioId: string;
  traineeName: string;
}

export interface ActionOverride {
  turn: number;
  actionId: string;
}

export interface BranchConfig {
  policy: string;
  maxActions: number;
  overrides: ActionOverride[];
}

export interface LabBranchSummary {
  id: string;
  name: string;
  checkpointName: string;
  seed: number;
  scenarioId: string;
  traineeName: string;
  policy: string;
  steps: number;
  careerComplete: boolean;
  finalTurn: number;
  fans: number;
  startedAt: string;
}

export interface BranchStep {
  stepIndex: number;
  turn: number;
  dateLabel: string;
  phase: string;
  actionId: string;
  actionLabel: string;
  overrideApplied: boolean;
  overrideRejected: boolean;
  rngCallsBefore: number;
  rngCallsAfter: number;
  energy: number;
  mood: string;
  fans: number;
  skillPoints: number;
  stats: TraineeStats;
  newRaces: string[];
  totalRaces: number;
  totalSkills: number;
}

export interface SparkSummary {
  color: string;
  factorId: string;
  stars: number;
  label: string;
}

export interface BranchOutcome {
  steps: number;
  finalTurn: number;
  careerComplete: boolean;
  completedRaces: string[];
  stats: TraineeStats;
  energy: number;
  mood: string;
  fans: number;
  skillPoints: number;
  learnedSkills: string[];
  sparks: SparkSummary[];
  scenarioResources: Record<string, number>;
  totalRngCalls: number;
  telemetryRecords: number;
}

export interface LabBranchResult {
  id: string;
  name: string;
  checkpointName: string;
  checkpointTurn: number;
  seed: number;
  scenarioId: string;
  traineeName: string;
  config: BranchConfig;
  startedAt: string;
  timeline: BranchStep[];
  outcome: BranchOutcome;
}

export interface Divergence {
  stepIndex: number;
  turn: number;
  dateLabel: string;
  phase: string;
  kind: "decision" | "outcome" | string;
  actionA: string;
  actionB: string;
  labelA: string;
  labelB: string;
  rngCallsAAfter: number;
  rngCallsBAfter: number;
  note: string;
}

export interface TurnCompare {
  stepIndex: number;
  turnA: number | null;
  turnB: number | null;
  actionA: string | null;
  actionB: string | null;
  labelA: string | null;
  labelB: string | null;
  sameAction: boolean;
  energyA: number | null;
  energyB: number | null;
  moodA: string | null;
  moodB: string | null;
  fansA: number | null;
  fansB: number | null;
  skillPointsA: number | null;
  skillPointsB: number | null;
  sameOutcome: boolean;
}

export interface BranchComparison {
  aId: string;
  aName: string;
  bId: string;
  bName: string;
  checkpointName: string;
  checkpointTurn: number;
  seed: number;
  sameCheckpoint: boolean;
  comparedAt: string;
  firstDivergence: Divergence | null;
  aligned: TurnCompare[];
  outcomeA: BranchOutcome;
  outcomeB: BranchOutcome;
  caveats: string[];
}
