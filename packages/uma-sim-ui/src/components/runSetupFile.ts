import type { CatalogItem } from "../api/types";
import type { AncestorSparks, LegacyTree, SparkSlot } from "./LegacyPanel";

export const RUN_SETUP_SCHEMA = "uma-sim.run-setup";
export const RUN_SETUP_VERSION = 1;
export const RUN_SETUP_MAX_BYTES = 64 * 1024;
export const RUN_SETUP_FILENAME = "uma-sim-setup-v1.json";

export const ANCESTOR_KEYS = [
  "parentA", "gpA1", "gpA2", "parentB", "gpB1", "gpB2",
] as const;
export const SPARK_KEYS = ["blue", "pink", "white", "green", "race"] as const;

const SPARK_KINDS = {
  blue: "blue", pink: "pink", white: "skill", green: "scenario", race: "race",
} as const;

export interface RunSetupData {
  seed: string;
  scenario: string;
  trainee: string;
  speed: number;
  dialogue: string;
  raceModel: string;
  policy: string;
  deck: string[];
  legacyEnabled: boolean;
  legacyTree: LegacyTree;
  compatibilityScore: number;
}

export interface SetupCatalogs {
  scenarios: readonly CatalogItem[];
  trainees: readonly CatalogItem[];
  supports: readonly CatalogItem[];
  factors: readonly CatalogItem[];
}

const SETUP_KEYS = [
  "seed", "scenario", "trainee", "speed", "dialogue", "raceModel", "policy",
  "deck", "legacyEnabled", "legacyTree", "compatibilityScore",
] as const;

function record(
  value: unknown,
  keys: readonly string[],
  label: string,
): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object.`);
  }
  const names = Reflect.ownKeys(value);
  if (names.length !== keys.length || names.some((key) => typeof key !== "string" || !keys.includes(key))) {
    throw new Error(`${label} has missing or unsupported fields.`);
  }
  for (const key of keys) {
    const field = Object.getOwnPropertyDescriptor(value, key);
    if (!field || !("value" in field)) {
      throw new Error(`${label} must contain ordinary saved values.`);
    }
  }
  return value as Record<string, unknown>;
}

function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new Error(`${label} must be text.`);
  return value;
}

function integer(value: unknown, low: number, high: number, label: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < low || value > high) {
    throw new Error(`${label} must be a whole number from ${low} through ${high}.`);
  }
  return value;
}

function option(value: unknown, options: readonly string[], label: string): string {
  const text = string(value, label);
  if (!options.includes(text)) throw new Error(`${label} is not supported: ${JSON.stringify(text)}.`);
  return text;
}

/** Keep the exact decimal text; JavaScript Number cannot represent every i64 seed. */
export function validateSeedText(value: unknown): string {
  if (typeof value !== "string" || !/^[+-]?[0-9]+$/.test(value)) {
    throw new Error("Seed must be a signed 64-bit decimal integer.");
  }
  const seed = BigInt(value);
  if (seed < -9223372036854775808n || seed > 9223372036854775807n) {
    throw new Error("Seed must be between -9223372036854775808 and 9223372036854775807.");
  }
  return value;
}

function catalogItem(
  value: unknown,
  catalog: readonly CatalogItem[],
  key: "id" | "name",
  label: string,
): CatalogItem {
  const text = string(value, label);
  const item = catalog.find((candidate) => candidate[key] === text);
  if (!item) throw new Error(`${label} is unavailable in the current catalog: ${JSON.stringify(text)}.`);
  return item;
}

function spark(value: unknown, kind: typeof SPARK_KEYS[number], catalogs: SetupCatalogs, label: string): SparkSlot {
  const fields = record(value, ["factorId", "stars"], label);
  const factorId = string(fields.factorId, `${label} factor`);
  const stars = integer(fields.stars, 1, 3, `${label} stars`);
  if (factorId) {
    const item = catalogItem(factorId, catalogs.factors, "id", `${label} factor`);
    if ((item.kind ?? "").toLowerCase() !== SPARK_KINDS[kind]) {
      throw new Error(`${label} factor is not a ${SPARK_KINDS[kind]} factor in the current catalog.`);
    }
  }
  return { factorId, stars };
}

function ancestor(value: unknown, catalogs: SetupCatalogs, label: string): AncestorSparks {
  const fields = record(value, ["uma", ...SPARK_KEYS], label);
  const uma = string(fields.uma, `${label} character`);
  if (uma) catalogItem(uma, catalogs.trainees, "name", `${label} character`);
  return {
    uma,
    blue: spark(fields.blue, "blue", catalogs, `${label} blue`),
    pink: spark(fields.pink, "pink", catalogs, `${label} pink/red`),
    white: spark(fields.white, "white", catalogs, `${label} white`),
    green: spark(fields.green, "green", catalogs, `${label} green`),
    race: spark(fields.race, "race", catalogs, `${label} race`),
  };
}

/** Admit a complete setup using the selections the current form can represent. */
export function validateRunSetup(value: unknown, catalogs: SetupCatalogs): RunSetupData {
  const fields = record(value, SETUP_KEYS, "Setup");
  const seed = validateSeedText(fields.seed);
  const scenario = catalogItem(fields.scenario, catalogs.scenarios, "id", "Scenario").id;
  // The existing form and native request identify a trainee by its English name.
  const trainee = catalogItem(fields.trainee, catalogs.trainees, "name", "Trainee").name;
  const speed = integer(fields.speed, 1, 100, "Speed");
  const dialogue = option(fields.dialogue, ["off", "choices", "full"], "Dialogue");
  const raceModel = option(fields.raceModel, ["physics", "stub"], "Race model");
  const policy = option(fields.policy, ["bot", "default"], "Policy");
  if (!Array.isArray(fields.deck) || fields.deck.length > 6) {
    throw new Error("Deck must contain zero through six support IDs.");
  }
  const deck: string[] = [];
  for (const value of fields.deck) {
    const item = catalogItem(value, catalogs.supports, "id", "Support");
    if (deck.includes(item.id)) throw new Error("Each selected support ID must occur only once.");
    if (item.name.toLowerCase() === trainee.toLowerCase()) {
      throw new Error("Trainee cannot also be a support card in the deck.");
    }
    deck.push(item.id);
  }
  if (typeof fields.legacyEnabled !== "boolean") throw new Error("Legacy enabled must be true or false.");
  const legacyEnabled = fields.legacyEnabled;
  const tree = record(fields.legacyTree, ANCESTOR_KEYS, "Inheritance tree");
  const legacyTree: LegacyTree = {
    parentA: ancestor(tree.parentA, catalogs, "Parent A"),
    gpA1: ancestor(tree.gpA1, catalogs, "Grandparent A1"),
    gpA2: ancestor(tree.gpA2, catalogs, "Grandparent A2"),
    parentB: ancestor(tree.parentB, catalogs, "Parent B"),
    gpB1: ancestor(tree.gpB1, catalogs, "Grandparent B1"),
    gpB2: ancestor(tree.gpB2, catalogs, "Grandparent B2"),
  };
  if (legacyEnabled && [legacyTree.parentA.uma, legacyTree.parentB.uma].some(
    (name) => name.toLowerCase() === trainee.toLowerCase(),
  )) {
    throw new Error("Trainee cannot be a direct parent when inheritance is enabled.");
  }
  const compatibilityScore = integer(fields.compatibilityScore, 0, 500, "Compatibility score");
  return {
    seed, scenario, trainee, speed, dialogue, raceModel, policy, deck,
    legacyEnabled, legacyTree, compatibilityScore,
  };
}

export function encodeRunSetup(value: unknown, catalogs: SetupCatalogs): string {
  const setup = validateRunSetup(value, catalogs);
  const text = JSON.stringify({ schema: RUN_SETUP_SCHEMA, version: RUN_SETUP_VERSION, setup }, null, 2) + "\n";
  if (new TextEncoder().encode(text).byteLength > RUN_SETUP_MAX_BYTES) {
    throw new Error("Setup file exceeds 64 KiB.");
  }
  return text;
}

export function decodeRunSetup(bytes: Uint8Array, catalogs: SetupCatalogs): RunSetupData {
  if (bytes.byteLength > RUN_SETUP_MAX_BYTES) throw new Error("Setup file exceeds 64 KiB.");
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new Error("Setup file must contain valid UTF-8 text.");
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("Setup file is not valid JSON.");
  }
  const fields = record(value, ["schema", "version", "setup"], "Setup file");
  if (fields.schema !== RUN_SETUP_SCHEMA || fields.version !== RUN_SETUP_VERSION) {
    throw new Error("This is not a supported version 1 uma-sim setup file.");
  }
  return validateRunSetup(fields.setup, catalogs);
}
