// SPDX-License-Identifier: GPL-3.0-only
import { useId } from "react";
import { parseRaceResults, type RaceResult } from "./raceResults";
import "./race-results.css";

export { parseRaceResult } from "./raceResults";

interface Props {
  /** Retained log entries from the currently supplied career snapshot. */
  lines: string[];
  mandatory: boolean;
  pendingRaceId?: string | null;
  busy: boolean;
  onRace: () => void;
}

function RecordedResult({ result }: { result: RaceResult }) {
  return (
    <div className="race-result" data-race-kind={result.kind}>
      <h3>{result.kind === "unknown" ? "Unrecognized race record" : result.raceId}</h3>
      {result.kind === "physics" ? (
        <dl className="race-facts">
          <div><dt>Place</dt><dd>{result.placeLabel}</dd></div>
          <div><dt>Field size</dt><dd>{result.fieldSize}</dd></div>
          <div><dt>Finish time</dt><dd>{result.finishTime} s</dd></div>
          <div><dt>Fan gain</dt><dd>+{result.fanGain}</dd></div>
        </dl>
      ) : result.kind === "fan-only" ? (
        <>
          <dl className="race-facts">
            <div><dt>Fan gain</dt><dd>+{result.fanGain}</dd></div>
          </dl>
          <p className="race-note">Place, finish time and field size were not recorded.</p>
        </>
      ) : (
        <p className="race-note">This record uses an unrecognized format. Its original text is preserved below.</p>
      )}
      <details className="race-record">
        <summary>Original race record · log entry {result.logIndex + 1}</summary>
        <pre>{result.summary}</pre>
      </details>
    </div>
  );
}

export function RacePanel({ lines, mandatory, pendingRaceId, busy, onRace }: Props) {
  const headingId = useId();
  const results = parseRaceResults(lines);
  const latest = results[results.length - 1];
  if (!mandatory && !latest) return null;

  return (
    <section className="card race-panel" aria-labelledby={headingId}>
      <h2 id={headingId}>Race</h2>
      {mandatory && (
        <div className="race-pending">
          <p>Mandatory race{pendingRaceId ? `: ${pendingRaceId}` : ""}</p>
          <button className="primary" disabled={busy} onClick={onRace}>
            Enter race
          </button>
        </div>
      )}
      {latest ? (
        <>
          <div className="race-latest">
            <p className="race-label">Latest retained race</p>
            <RecordedResult result={latest} />
          </div>
          <p className="race-note">
            {results.length} retained race {results.length === 1 ? "record" : "records"} in the loaded career log.
          </p>
          {results.length > 1 && (
            <details className="race-history">
              <summary>Earlier retained races ({results.length - 1})</summary>
              <ol aria-label="Earlier retained race records">
                {results.slice(0, -1).reverse().map(result => (
                  <li key={result.logIndex}><RecordedResult result={result} /></li>
                ))}
              </ol>
            </details>
          )}
        </>
      ) : (
        <p className="race-note">No race result recorded yet.</p>
      )}
    </section>
  );
}
