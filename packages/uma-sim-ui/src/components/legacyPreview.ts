/** Client-side legacy preview (mirrors uma-sim-core `legacy.rs` tables). */

import type { CatalogItem, TraineeStats } from "../api/types";
import type { LegacyTree } from "./LegacyPanel";
import { flattenLegacyFactors } from "./LegacyPanel";

const BLUE_STAT: Record<string, keyof TraineeStats> = {
  "factor:blue:1": "speed",
  "factor:blue:2": "stamina",
  "factor:blue:3": "power",
  "factor:blue:4": "guts",
  "factor:blue:5": "wit",
};

const APT_ORDER = ["G", "F", "E", "D", "C", "B", "A", "S"] as const;

export function blueStartingStatBonus(stars: number): number {
  switch (Math.max(1, Math.min(3, stars || 1))) {
    case 1:
      return 5;
    case 2:
      return 12;
    default:
      return 21;
  }
}

export function pinkAptitudeRankUps(starTotal: number): number {
  if (starTotal <= 0) return 0;
  if (starTotal <= 3) return 1;
  if (starTotal <= 6) return 2;
  if (starTotal <= 9) return 3;
  return 4;
}

export function raiseAptitudeLetter(letter: string, ups: number): string {
  if (ups <= 0) return letter.toUpperCase();
  const cur = letter.toUpperCase();
  const idx = Math.max(0, APT_ORDER.indexOf(cur as (typeof APT_ORDER)[number]));
  const capA = APT_ORDER.indexOf("A");
  return APT_ORDER[Math.min(idx + ups, capA)];
}

export interface LegacyPreview {
  startingStats: TraineeStats;
  aptitudes: Record<string, string>;
}

function parseEntry(entry: string): { id: string; stars: number } {
  const at = entry.lastIndexOf("@");
  if (at <= 0) return { id: entry, stars: 3 };
  const stars = Math.max(1, Math.min(3, Number(entry.slice(at + 1)) || 3));
  return { id: entry.slice(0, at), stars };
}

export function previewLegacy(
  tree: LegacyTree,
  baseStats: TraineeStats,
  baseAptitudes: Record<string, string>,
  factors: CatalogItem[],
): LegacyPreview {
  const stats = { ...baseStats };
  const pinkStars: Record<string, number> = {};
  const byId = new Map(factors.map((f) => [f.id, f]));

  for (const entry of flattenLegacyFactors(tree)) {
    const { id, stars } = parseEntry(entry);
    const meta = byId.get(id);
    const kind = (meta?.kind ?? "").toLowerCase();
    if (kind === "blue" || BLUE_STAT[id]) {
      const key =
        (meta?.statKey as keyof TraineeStats | undefined) ??
        BLUE_STAT[id] ??
        "speed";
      stats[key] = (stats[key] ?? 0) + blueStartingStatBonus(stars);
      continue;
    }
    if (kind === "pink") {
      const tag = meta?.pinkTag ?? meta?.name?.toLowerCase() ?? "";
      if (!tag) continue;
      pinkStars[tag] = (pinkStars[tag] ?? 0) + stars;
    }
  }

  const aptitudes = { ...baseAptitudes };
  for (const [tag, total] of Object.entries(pinkStars)) {
    const base = aptitudes[tag] ?? "G";
    aptitudes[tag] = raiseAptitudeLetter(base, pinkAptitudeRankUps(total));
  }

  return { startingStats: stats, aptitudes };
}
