export interface SkillCatalogItem {
  id: string;
  name: string | null;
  description: string | null;
}

export interface SkillRow extends SkillCatalogItem {
  kind: "skill" | "other";
  level?: number;
}

export type SkillCatalog = ReadonlyMap<string, SkillCatalogItem>;

const TRAINING_HINT_NAMES = new Map([
  ["speed", "Speed training"],
  ["stamina", "Stamina training"],
  ["power", "Power training"],
  ["guts", "Guts training"],
  ["wit", "Wit training"],
  ["generic", "Generic hint"],
]);

/** Accept catalog text only; an older server's HTML fallback is not a catalog. */
export function parseSkillCatalog(value: unknown): SkillCatalog {
  if (!value || typeof value !== "object" ||
      !("items" in value) || !Array.isArray(value.items)) {
    throw new Error("Skill catalog response is unavailable");
  }
  const catalog = new Map<string, SkillCatalogItem>();
  for (const item of value.items) {
    if (!item || typeof item !== "object" ||
        typeof item.id !== "string" || !item.id.startsWith("skill:")) continue;
    // Keep the first exact ID, matching the catalog's existing lookup precedence.
    if (catalog.has(item.id)) continue;
    catalog.set(item.id, {
      id: item.id,
      name: typeof item.name === "string" && item.name.trim() ? item.name : null,
      description: typeof item.description === "string" && item.description.trim()
        ? item.description : null,
    });
  }
  return catalog;
}

/** Derive a view from the current snapshot; never infer purchases or learned status from hints. */
export function skillRows(
  learnedIds: readonly string[],
  hintLevels: Readonly<Record<string, number>>,
  catalog: SkillCatalog,
) {
  const skill = (id: string): SkillRow => ({
    id,
    name: catalog.get(id)?.name ?? null,
    description: catalog.get(id)?.description ?? null,
    kind: "skill",
  });
  // Preserve all retained learned entries, including order and duplicate IDs.
  const learned = learnedIds.map(skill);
  const skillHints: SkillRow[] = [];
  const otherHints: SkillRow[] = [];
  for (const [id, level] of Object.entries(hintLevels)) {
    // Raw event names and facility keys are not canonical skill identities.
    if (id.startsWith("skill:")) skillHints.push({ ...skill(id), level });
    else otherHints.push({
      id, name: TRAINING_HINT_NAMES.get(id) ?? id, description: null, kind: "other", level,
    });
  }
  const byId = (a: SkillRow, b: SkillRow) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
  return { learned, skillHints: skillHints.sort(byId), otherHints: otherHints.sort(byId) };
}
