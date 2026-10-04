import { useState } from "react";
import { useLabStore } from "../state/labStore";

export function SessionPanel({ lab }: { lab: ReturnType<typeof useLabStore> }) {
  const { state, forkSession, activateSession, closeSession } = lab;
  const [forkSource, setForkSource] = useState("");
  const [forkId, setForkId] = useState("");
  const [forkLabel, setForkLabel] = useState("");

  return (
    <div className="card">
      <h2>Sessions</h2>
      <p className="hint">
        Each session is an independent live career. The run tab shows the
        <strong> active </strong>
        session; forks and checkpoints cannot mutate each other.
      </p>
      <table className="data-table">
        <thead>
          <tr>
            <th>Session</th>
            <th>Turn</th>
            <th>Phase</th>
            <th>Done</th>
            <th>Seed</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {state.sessions.map((s) => (
            <tr key={s.id || "(default)"}>
              <td>
                {s.id === state.activeSession && <strong>▶ </strong>}
                {s.label || "(default)"}
              </td>
              <td>{s.turn}</td>
              <td>{s.phase}</td>
              <td>{s.careerComplete ? "yes" : "no"}</td>
              <td>{s.seed}</td>
              <td>
                {s.id !== state.activeSession && (
                  <button
                    disabled={state.busy}
                    onClick={() => void activateSession(s.id)}
                  >
                    Activate
                  </button>
                )}{" "}
                {s.id !== "" && (
                  <button
                    disabled={state.busy}
                    onClick={() => void closeSession(s.id)}
                  >
                    Close
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <h3>Fork a session</h3>
      <div className="field-row">
        <label>
          Source session (leave empty for the current one)
          <input
            value={forkSource}
            onChange={(e) => setForkSource(e.target.value)}
            placeholder='session id, e.g. "branch-a"'
          />
        </label>
        <label>
          New session id (optional)
          <input
            value={forkId}
            onChange={(e) => setForkId(e.target.value)}
            placeholder="auto-generated"
          />
        </label>
        <label>
          Label (optional)
          <input
            value={forkLabel}
            onChange={(e) => setForkLabel(e.target.value)}
            placeholder='e.g. "spent-fans early"'
          />
        </label>
        <button
          disabled={state.busy}
          onClick={() =>
            void forkSession(
              forkSource.trim() ? { session: forkSource.trim() } : {},
              forkId,
              forkLabel,
            )
          }
        >
          Fork &amp; activate
        </button>
      </div>
    </div>
  );
}
