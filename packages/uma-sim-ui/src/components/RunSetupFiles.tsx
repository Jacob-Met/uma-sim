import { useEffect, useId, useRef, useState } from "react";
import {
  ANCESTOR_KEYS,
  SPARK_KEYS,
  RUN_SETUP_FILENAME,
  RUN_SETUP_MAX_BYTES,
  decodeRunSetup,
  encodeRunSetup,
  validateRunSetup,
  type RunSetupData,
  type SetupCatalogs,
} from "./runSetupFile";
import "./runSetupFiles.css";

interface Props {
  setup: RunSetupData;
  catalogs: SetupCatalogs;
  revision: number;
  busy: boolean;
  onApply: (setup: RunSetupData) => void;
}

interface OpenContext {
  setup: RunSetupData;
  catalogs: SetupCatalogs;
  revision: number;
  busy: boolean;
}

interface Preview {
  setup: RunSetupData;
  name: string;
  context: OpenContext;
  ticket: number;
}

const ANCESTOR_LABELS = {
  parentA: "Parent A", gpA1: "Grandparent A1", gpA2: "Grandparent A2",
  parentB: "Parent B", gpB1: "Grandparent B1", gpB2: "Grandparent B2",
};

const SPARK_LABELS = {
  blue: "Blue", pink: "Pink/red", white: "White", green: "Green", race: "Race",
};

function sameContext(left: OpenContext, right: OpenContext): boolean {
  return left.setup === right.setup && left.catalogs === right.catalogs &&
    left.revision === right.revision && left.busy === right.busy;
}

export function RunSetupFiles({ setup, catalogs, revision, busy, onApply }: Props) {
  const [preview, setPreview] = useState<Preview | null>(null);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const mounted = useRef(false);
  const epoch = useRef(0);
  const opening = useRef<OpenContext | null>(null);
  const current = useRef<OpenContext>({ setup, catalogs, revision, busy });
  const previous = useRef(current.current);
  const downloadTimers = useRef(new Map<string, number>());
  const reviewId = useId();
  current.current = { setup, catalogs, revision, busy };

  function cancel(message: string | null) {
    epoch.current += 1;
    opening.current = null;
    setReading(false);
    setPreview(null);
    setError(null);
    setStatus(message);
  }

  useEffect(() => {
    mounted.current = true;
    const element = input.current;
    const cancelled = () => {
      if (opening.current) cancel("Opening cancelled. Your setup is unchanged.");
    };
    element?.addEventListener("cancel", cancelled);
    const timers = downloadTimers.current;
    return () => {
      mounted.current = false;
      epoch.current += 1;
      opening.current = null;
      element?.removeEventListener("cancel", cancelled);
      for (const [url, timer] of timers) {
        window.clearTimeout(timer);
        URL.revokeObjectURL(url);
      }
      timers.clear();
    };
  }, []);

  useEffect(() => {
    if (!sameContext(previous.current, current.current)) {
      const hadOpen = reading || preview !== null || opening.current !== null;
      epoch.current += 1;
      opening.current = null;
      setReading(false);
      setPreview(null);
      if (hadOpen) {
        setError(null);
        setStatus("The form or available choices changed. Open the file again to review it against this setup.");
      }
      previous.current = current.current;
    }
  }, [setup, catalogs, revision, busy, reading, preview]);

  function isCurrent(ticket: number, context: OpenContext): boolean {
    return mounted.current && epoch.current === ticket && !current.current.busy &&
      sameContext(context, current.current);
  }

  function chooseFile() {
    if (busy) return;
    cancel(null);
    opening.current = current.current;
    if (input.current) {
      input.current.value = "";
      input.current.click();
    }
  }

  async function openFile(file: File | undefined) {
    const context = opening.current;
    opening.current = null;
    const ticket = ++epoch.current;
    setPreview(null);
    setError(null);
    setStatus(null);
    if (!file) {
      setReading(false);
      setStatus("Opening cancelled. Your setup is unchanged.");
      return;
    }
    if (!context || !isCurrent(ticket, context)) {
      setReading(false);
      setStatus("The form or available choices changed. Open the file again to review it against this setup.");
      return;
    }
    setReading(true);
    try {
      if (file.size > RUN_SETUP_MAX_BYTES) throw new Error("Setup file exceeds 64 KiB.");
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (!isCurrent(ticket, context)) return;
      const next = decodeRunSetup(bytes, current.current.catalogs);
      setReading(false);
      setPreview({ setup: next, name: file.name, context, ticket });
    } catch (cause) {
      if (!isCurrent(ticket, context)) return;
      setReading(false);
      setError(cause instanceof Error ? cause.message : "Could not read the setup file.");
    }
  }

  function replaceSetup() {
    if (!preview) return;
    if (!isCurrent(preview.ticket, preview.context)) {
      cancel("The form or available choices changed. Open the file again to review it against this setup.");
      return;
    }
    let next: RunSetupData;
    try {
      // Recheck the current catalog at the actual replacement boundary.
      next = validateRunSetup(preview.setup, current.current.catalogs);
    } catch (cause) {
      cancel(null);
      setError(cause instanceof Error ? cause.message : "This setup is no longer available.");
      return;
    }
    cancel(null);
    onApply(next);
    setStatus("Setup replaced. Review or edit the form, then choose Start run when ready.");
  }

  function downloadSetup() {
    setError(null);
    try {
      const text = encodeRunSetup(setup, catalogs);
      const blob = new Blob([text], { type: "application/json;charset=utf-8" });
      const anchor = document.createElement("a");
      const url = URL.createObjectURL(blob);
      try {
        anchor.href = url;
        anchor.download = RUN_SETUP_FILENAME;
        document.body.appendChild(anchor);
        anchor.click();
        setStatus("Setup download requested.");
      } finally {
        anchor.remove();
        const timer = window.setTimeout(() => {
          URL.revokeObjectURL(url);
          downloadTimers.current.delete(url);
        }, 1000);
        downloadTimers.current.set(url, timer);
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not start the setup download.");
    }
  }

  return (
    <section className="setup-files" data-setup-files aria-label="Career setup files">
      <p className="hint">
        Keep these choices in a setup file to edit or reuse in a new career.
        Opening a file lets you review it before replacing this form. Start the run separately.
      </p>
      <div className="controls">
        <button type="button" disabled={busy} onClick={downloadSetup}>Download setup</button>
        <button type="button" disabled={busy} onClick={chooseFile}>Open setup</button>
        <input
          ref={input}
          type="file"
          accept=".json,application/json"
          aria-label="Choose a career setup file"
          hidden
          onChange={(event) => {
            const file = event.currentTarget.files?.[0];
            event.currentTarget.value = "";
            void openFile(file);
          }}
        />
        {reading && <button type="button" onClick={() => cancel("Opening cancelled. Your setup is unchanged.")}>Cancel opening</button>}
      </div>
      {reading && <p role="status">Reading setup file…</p>}
      {status && <p role="status">{status}</p>}
      {error && <p role="alert">{error} Your setup is unchanged.</p>}
      {preview && (
        <section className="setup-file-review" aria-labelledby={reviewId}>
          <h3 id={reviewId}>Review setup</h3>
          <p className="setup-file-name">{preview.name}</p>
          <dl className="setup-file-values">
            <div><dt>Seed</dt><dd><code>{preview.setup.seed}</code></dd></div>
            <div><dt>Scenario</dt><dd>{catalogs.scenarios.find((item) => item.id === preview.setup.scenario)?.name} <code>({preview.setup.scenario})</code></dd></div>
            <div><dt>Trainee</dt><dd>{preview.setup.trainee}</dd></div>
            <div><dt>Speed</dt><dd>{preview.setup.speed}</dd></div>
            <div><dt>Dialogue</dt><dd>{preview.setup.dialogue}</dd></div>
            <div><dt>Race model</dt><dd>{preview.setup.raceModel}</dd></div>
            <div><dt>Policy</dt><dd>{preview.setup.policy}</dd></div>
            <div><dt>Compatibility score</dt><dd>{preview.setup.compatibilityScore}{!preview.setup.legacyEnabled && " (retained while inheritance is disabled)"}</dd></div>
          </dl>
          <h4>Deck supports ({preview.setup.deck.length}/6)</h4>
          {preview.setup.deck.length === 0 ? <p>No supports selected.</p> : (
            <ol>
              {preview.setup.deck.map((id) => (
                <li key={id}>{catalogs.supports.find((item) => item.id === id)?.name} <code>({id})</code></li>
              ))}
            </ol>
          )}
          <details>
            <summary>Inheritance: {preview.setup.legacyEnabled ? "enabled" : "disabled — saved choices retained"}</summary>
            <div className="setup-file-ancestors">
              {ANCESTOR_KEYS.map((key) => (
                <div key={key}>
                  <h4>{ANCESTOR_LABELS[key]}</h4>
                  <p>{preview.setup.legacyTree[key].uma || "No character selected"}</p>
                  <dl>
                    {SPARK_KEYS.map((kind) => {
                      const slot = preview.setup.legacyTree[key][kind];
                      const factor = catalogs.factors.find((item) => item.id === slot.factorId);
                      return (
                        <div key={kind}>
                          <dt>{SPARK_LABELS[kind]}</dt>
                          <dd>{slot.factorId ? <>{factor?.name} <code>({slot.factorId})</code></> : "No factor"} · {slot.stars}★</dd>
                        </div>
                      );
                    })}
                  </dl>
                </div>
              ))}
            </div>
          </details>
          <p>Replace all choices in the current form with this setup?</p>
          <div className="controls">
            <button type="button" className="primary" disabled={busy} onClick={replaceSetup}>Replace setup</button>
            <button type="button" onClick={() => cancel("Opening cancelled. Your setup is unchanged.")}>Cancel</button>
          </div>
        </section>
      )}
    </section>
  );
}
