import { useRef, useState } from "react";
import { useLabStore } from "../state/labStore";

export function LibraryPanel({ lab }: { lab: ReturnType<typeof useLabStore> }) {
  const { state, saveCheckpoint, loadCheckpoint, deleteCheckpoint, importCheckpoint } = lab;
  const [name, setName] = useState("");
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [importName, setImportName] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  return (
    <div className="card">
      <h2>Career library</h2>
      <p className="hint">
        Durable named checkpoints under <code>.uma-sim/library</code>. Saving
        never touches the live run or the original entry — the checkpoint file
        is written atomically and re-saving an existing name without
        &ldquo;overwrite&rdquo; is rejected.
      </p>
      <h3>Save the active session</h3>
      <div className="field-row">
        <label>
          Name
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. debut-fork (letters, digits, -_.  up to 64)"
          />
        </label>
        <label>
          Label (optional)
          <input
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="short display name"
          />
        </label>
        <label>
          Note (optional)
          <input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="why this matters"
          />
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={overwrite}
            onChange={(e) => setOverwrite(e.target.checked)}
          />
          Overwrite existing
        </label>
        <button
          disabled={state.busy}
          onClick={() => void saveCheckpoint(name, label, note, overwrite)}
        >
          Save checkpoint
        </button>
      </div>
      <h3>Saved checkpoints</h3>
      {state.library.length === 0 ? (
        <p className="hint">No checkpoints saved yet.</p>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Seed</th>
              <th>Scenario</th>
              <th>Trainee</th>
              <th>Turn</th>
              <th>Date</th>
              <th>Saved</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {state.library.map((e) => (
              <tr key={e.name}>
                <td title={e.note || undefined}>
                  {e.label || e.name}
                  {e.label && e.label !== e.name ? ` (${e.name})` : ""}
                </td>
                <td>{e.seed}</td>
                <td>{e.scenarioId}</td>
                <td>{e.traineeName}</td>
                <td>{e.turn}</td>
                <td>{e.dateLabel}</td>
                <td title={e.savedAt}>{e.savedAtUnix > 0 ? new Date(e.savedAtUnix * 1000).toLocaleString() : "—"}</td>
                <td>
                  <button
                    disabled={state.busy}
                    onClick={() => void loadCheckpoint(e.name, false)}
                    title="Resume this checkpoint in the active session"
                  >
                    Resume
                  </button>{" "}
                  <button
                    disabled={state.busy}
                    onClick={() => void loadCheckpoint(e.name, true)}
                    title="Open this checkpoint as a new forked session; current run untouched"
                  >
                    Fork open
                  </button>{" "}
                  <button
                    disabled={state.busy}
                    onClick={() => {
                      if (window.confirm(`Delete checkpoint "${e.name}"?`))
                        void deleteCheckpoint(e.name);
                    }}
                  >
                    Delete
                  </button>{" "}
                  <a
                    href={`/v1/library/export?name=${encodeURIComponent(e.name)}`}
                    download
                    className="link-button"
                  >
                    Export
                  </a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <h3>Import a checkpoint file</h3>
      <p className="hint">
        The file is fully validated before anything is written — a bad import
        leaves the library untouched.
      </p>
      <div className="field-row">
        <label>
          Name (optional — generated from seed, turn, and scenario)
          <input
            value={importName}
            onChange={(e) => setImportName(e.target.value)}
          />
        </label>
        <input
          ref={fileRef}
          type="file"
          accept="application/json,.json"
          style={{ alignSelf: "flex-end" }}
        />
        <button
          disabled={state.busy}
          style={{ alignSelf: "flex-end" }}
          onClick={() => {
            const f = fileRef.current?.files?.[0];
            if (!f) return;
            void f.text().then((text) => importCheckpoint(text, importName));
          }}
        >
          Import
        </button>
      </div>
    </div>
  );
}
