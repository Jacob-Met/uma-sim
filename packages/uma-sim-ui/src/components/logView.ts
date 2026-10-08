export interface LogPart {
  text: string;
  matched: boolean;
}

export interface LogEntry {
  /** Position in the supplied history, not a turn number. */
  index: number;
  parts: LogPart[];
  matches: number;
}

export interface LogView {
  entries: LogEntry[];
  matchCount: number;
  totalEntries: number;
}

/** Literal, Unicode-aware case-insensitive search; keep original text intact. */
export function searchLog(history: readonly string[], query: string): LogView {
  const pattern = query
    ? new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "giu")
    : null;
  const entries: LogEntry[] = [];
  let matchCount = 0;

  history.forEach((line, index) => {
    if (!pattern) {
      entries.push({ index, parts: [{ text: line, matched: false }], matches: 0 });
      return;
    }
    const parts: LogPart[] = [];
    let cursor = 0;
    let matches = 0;
    for (const match of line.matchAll(pattern)) {
      const start = match.index!;
      if (start > cursor) parts.push({ text: line.slice(cursor, start), matched: false });
      parts.push({ text: match[0], matched: true });
      cursor = start + match[0].length;
      matches += 1;
    }
    if (!matches) return;
    if (cursor < line.length) parts.push({ text: line.slice(cursor), matched: false });
    entries.push({ index, parts, matches });
    matchCount += matches;
  });

  return { entries, matchCount, totalEntries: history.length };
}

/** Describes only the supplied sequences; a prefix is not a session identity. */
export function historyChange(
  previous: readonly string[],
  current: readonly string[],
): "unchanged" | "appended" | "replaced" {
  if (current.length < previous.length) return "replaced";
  for (let i = 0; i < previous.length; i += 1) {
    if (previous[i] !== current[i]) return "replaced";
  }
  return current.length === previous.length ? "unchanged" : "appended";
}
