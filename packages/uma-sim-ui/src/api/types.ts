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
}

export interface RunSnapshot {
  meta: RunMeta;
  settings: SimSettings;
  state: CareerState;
  rngSeed: number;
  rngCalls: number;
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
