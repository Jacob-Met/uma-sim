import type { CatalogItem } from "../api/types";

export const ANCESTOR_POSITIONS = [
  ["parentA", "Parent A"],
  ["gpA1", "Grandparent A1"],
  ["gpA2", "Grandparent A2"],
  ["parentB", "Parent B"],
  ["gpB1", "Grandparent B1"],
  ["gpB2", "Grandparent B2"],
] as const;

const SPARK_KINDS = [
  ["blue", "Blue"],
  ["pink", "Pink"],
  ["white", "White"],
  ["green", "Green"],
  ["race", "Race"],
] as const;

const STAT_NAMES: Record<string, string> = {
  speed: "Speed",
  stamina: "Stamina",
  power: "Power",
  guts: "Guts",
  wit: "Wit",
};

export interface RecordedFactor {
  id: string;
  name: string | null;
}

export interface RecordedSpark extends RecordedFactor {
  kind: string;
  label: string;
  stars: number | null;
}

export interface RecordedAncestor {
  key: string;
  label: string;
  recorded: boolean;
  name: string | null;
  sparks: RecordedSpark[];
  unavailableSlots: string[];
}

export interface RecordedBonus {
  key: string;
  label: string;
  value: number | null;
}

export interface InheritanceRecord {
  parentNames: string[] | null;
  ancestors: RecordedAncestor[] | null;
  compatibility: number | null;
  flatEntries: string[] | null;
  retainedFactors: RecordedFactor[] | null;
  inheritedSkillIds: string[] | null;
  capAdditions: RecordedBonus[] | null;
  startingBonuses: RecordedBonus[] | null;
  choiceComplete: boolean | null;
  inspirations: number | null;
}

function object(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function strings(value: unknown): string[] | null {
  return Array.isArray(value) && value.every((item) => typeof item === "string")
    ? [...value]
    : null;
}

function number(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function bonuses(value: unknown): RecordedBonus[] | null {
  const values = object(value);
  if (values === null) return null;
  const order = Object.keys(STAT_NAMES);
  return Object.entries(values)
    .map(([key, raw]) => ({
      key,
      label: Object.prototype.hasOwnProperty.call(STAT_NAMES, key) ? STAT_NAMES[key] : key,
      value: number(raw),
    }))
    .sort((a, b) => {
      const ai = order.indexOf(a.key);
      const bi = order.indexOf(b.key);
      const rank = (ai < 0 ? order.length : ai) - (bi < 0 ? order.length : bi);
      return rank || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0);
    });
}

/** Present retained fields only. Do not reuse the setup editor's normalization. */
export function inheritanceView(
  metaValue: unknown,
  legacyValue: unknown,
  catalog: readonly CatalogItem[],
): InheritanceRecord {
  const meta = object(metaValue);
  const legacy = object(legacyValue);
  const tree = object(meta?.legacyTree);
  const names = new Map<string, string | null>();
  for (const item of catalog) {
    if (typeof item?.id !== "string" || names.has(item.id)) continue;
    names.set(item.id, typeof item.name === "string" && item.name.trim() ? item.name : null);
  }
  const factor = (id: string): RecordedFactor => ({
    id,
    name: names.get(id) ?? null,
  });
  const ancestors = tree === null ? null : ANCESTOR_POSITIONS.map(([key, label]) => {
    const node = object(tree[key]);
    const sparks: RecordedSpark[] = [];
    const unavailableSlots: string[] = [];
    if (node !== null) {
      for (const [kind, sparkLabel] of SPARK_KINDS) {
        const slot = object(node[kind]);
        if (slot === null || typeof slot.factorId !== "string") {
          unavailableSlots.push(sparkLabel);
          continue;
        }
        if (slot.factorId !== "") {
          sparks.push({
            ...factor(slot.factorId),
            kind,
            label: sparkLabel,
            stars: number(slot.stars),
          });
        }
      }
    }
    return {
      key,
      label,
      recorded: node !== null,
      name: typeof node?.uma === "string" ? node.uma : null,
      sparks,
      unavailableSlots,
    };
  });
  const ids = strings(legacy?.factorIds);
  return {
    parentNames: strings(legacy?.parentNames),
    ancestors,
    compatibility: number(meta?.compatibilityScore),
    flatEntries: strings(meta?.legacyFactors),
    retainedFactors: ids === null ? null : ids.map(factor),
    inheritedSkillIds: strings(legacy?.inheritedSkillIds),
    capAdditions: bonuses(legacy?.sparkCaps),
    startingBonuses: bonuses(legacy?.blueStartBonuses),
    choiceComplete: typeof legacy?.inheritanceComplete === "boolean"
      ? legacy.inheritanceComplete
      : null,
    inspirations: number(legacy?.inspirationEventsDone),
  };
}
