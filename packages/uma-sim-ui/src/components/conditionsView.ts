export interface ConditionRow {
  id: string;
  label: string | null;
  kind: "injury" | "title" | "other";
}

export interface ConditionsView {
  reported: boolean;
  hasInjury: boolean;
  rows: ConditionRow[];
}

// These are the current simulator's stub epithet IDs, not an official title catalog.
const TITLES = new Map([
  ["epithet:sim_g1_win", "G1 win"],
  ["epithet:sim_climax_win", "Climax win"],
  ["epithet:sim_ura_finale_win", "URA finale win"],
]);

export function conditionsView(value: unknown): ConditionsView {
  if (!Array.isArray(value) || !value.every((id) => typeof id === "string")) {
    return { reported: false, hasInjury: false, rows: [] };
  }

  const rows: ConditionRow[] = value.map((id: string) => {
    // Match CareerState::is_injured / without_injury in the native engine.
    // Injury takes precedence even if an imported ID also resembles a title.
    const lower = id.toLowerCase();
    if (lower === "injured" || lower.includes("injury")) {
      return { id, label: "Injury", kind: "injury" };
    }
    const label = TITLES.get(id);
    return label === undefined
      ? { id, label: null, kind: "other" }
      : { id, label, kind: "title" };
  });

  return {
    reported: true,
    hasInjury: rows.some((row) => row.kind === "injury"),
    rows,
  };
}
