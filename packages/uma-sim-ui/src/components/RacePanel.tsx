interface RaceResult {
  summary: string;
  place?: string;
  finish?: string;
  epithet?: string;
}

/** Parse recent log lines for race outcome cards (engine emits text only). */
export function parseRaceResult(lines: string[]): RaceResult | null {
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    const physics = line.match(
      /Race\s+(\S+).*place\s*[:=]?\s*(\d+).*finish(?:_time)?\s*[:=]?\s*([\d.]+)/i,
    );
    if (physics) {
      return {
        summary: line,
        place: physics[2],
        finish: physics[3],
      };
    }
    const stub = line.match(/Race\s+(\S+)\s+\+(\d+)\s+fans/i);
    if (stub) {
      return { summary: line, place: "1" };
    }
  }
  return null;
}

interface Props {
  lines: string[];
  mandatory: boolean;
  pendingRaceId?: string | null;
  busy: boolean;
  onRace: () => void;
}

export function RacePanel({
  lines,
  mandatory,
  pendingRaceId,
  busy,
  onRace,
}: Props) {
  const result = parseRaceResult(lines);
  if (!mandatory && !result) return null;

  return (
    <div className="card">
      <h2>Race</h2>
      {mandatory && (
        <>
          <p style={{ marginTop: 0 }}>
            Mandatory race{pendingRaceId ? `: ${pendingRaceId}` : ""}
          </p>
          <button className="primary" disabled={busy} onClick={onRace}>
            Enter race
          </button>
        </>
      )}
      {result && (
        <div style={{ marginTop: mandatory ? "0.75rem" : 0 }}>
          <div className="meta-line">
            <span>Latest result</span>
            <strong>
              {result.place ? `${result.place} place` : "finished"}
              {result.finish ? ` · ${result.finish}s` : ""}
            </strong>
          </div>
          <div className="chip" style={{ marginTop: "0.35rem" }}>
            {result.summary}
          </div>
        </div>
      )}
    </div>
  );
}
