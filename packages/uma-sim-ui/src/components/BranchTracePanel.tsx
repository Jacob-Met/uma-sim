import { useId, useState } from "react";
import type { LabBranchResult } from "../api/types";
import type { BranchTraceController } from "../state/branchTraceStore";
import "./branch-trace.css";

function downloadTrace(branch: LabBranchResult) {
  const blob = new Blob([JSON.stringify(branch, null, 2) + "\n"], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `uma-sim-branch-${branch.id.replace(/[^a-zA-Z0-9._-]+/g, "_")}.json`;
  document.body.append(link);
  try {
    link.click();
  } finally {
    link.remove();
    // Let the browser acquire the Blob before releasing the object URL.
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  }
}

export function BranchTracePanel({ trace }: { trace: BranchTraceController }) {
  const titleId = useId();
  const [onlyOverrides, setOnlyOverrides] = useState(true);
  const { state } = trace;
  const branch = state.status === "ready" ? state.branch : null;
  const steps = branch?.timeline ?? [];
  const applied = steps.filter(step => step.overrideApplied).length;
  const rejected = steps.filter(step => step.overrideRejected).length;
  const visibleSteps = onlyOverrides
    ? steps.filter(step => step.overrideApplied || step.overrideRejected)
    : steps;

  return (
    <section className="card branch-trace" aria-labelledby={titleId}>
      <div className="trace-actions">
        <h2 id={titleId}>{branch ? `Trace: ${branch.name}` : "Branch trace"}</h2>
        {state.status !== "idle" && (
          <button onClick={trace.close}>Close trace</button>
        )}
      </div>

      {state.status === "idle" && (
        <p className="hint">Inspect a saved branch to review the actions and override flags it recorded.</p>
      )}
      {state.status === "loading" && (
        <p role="status">Loading trace for <code>{state.requestedId}</code>…</p>
      )}
      {state.status === "error" && (
        <div>
          <p className="banner error" role="alert">
            Could not load trace <code>{state.requestedId}</code>: {state.error}
          </p>
          <button onClick={() => void trace.retry()}>Retry trace</button>
        </div>
      )}

      {branch && (
        <>
          <p className="hint" role="status">
            Loaded {steps.length} recorded {steps.length === 1 ? "step" : "steps"} for {branch.name}.
          </p>
          <dl className="trace-metadata">
            <dt>Branch ID</dt><dd><code>{branch.id}</code></dd>
            <dt>Checkpoint</dt><dd>{branch.checkpointName} (turn {branch.checkpointTurn})</dd>
            <dt>Seed / scenario</dt><dd>{branch.seed} / {branch.scenarioId}</dd>
            <dt>Trainee</dt><dd>{branch.traineeName}</dd>
            <dt>Policy</dt><dd>{branch.config.policy}</dd>
            <dt>Requested limit</dt><dd>{branch.config.maxActions} actions</dd>
            <dt>Outcome</dt><dd>{branch.outcome.careerComplete ? "Career complete" : "Stopped before career completion"}</dd>
          </dl>
          <div className="trace-actions">
            <button onClick={() => downloadTrace(branch)}>Download full trace JSON</button>
          </div>

          <h3>Authored overrides</h3>
          {branch.config.overrides.length === 0 ? (
            <p className="hint">No overrides were configured.</p>
          ) : (
            <>
              <div className="trace-table-wrap" role="region" aria-label="Authored overrides" tabIndex={0}>
                <table className="data-table">
                  <thead><tr><th>Turn</th><th>Requested action</th><th>Recorded at this turn</th></tr></thead>
                  <tbody>
                    {branch.config.overrides.map((override, index) => {
                      const atTurn = steps.filter(step => step.turn === override.turn);
                      return (
                        <tr key={index}>
                          <td>{override.turn}</td>
                          <td><code>{override.actionId}</code></td>
                          <td>
                            {atTurn.length === 0
                              ? "No step recorded at this turn"
                              : `${atTurn.length} steps; ${atTurn.filter(step => step.overrideApplied).length} applied flags; ${atTurn.filter(step => step.overrideRejected).length} rejected flags`}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              <p className="hint">
                Flags describe recorded steps at each turn. A turn can contain multiple phases,
                so it can include both applied and rejected attempts. A turn with no recorded
                step has no observed override outcome in this trace. When more than one
                override is listed for a turn, the first entry takes precedence.
              </p>
            </>
          )}

          <h3>Recorded actions</h3>
          <p className="trace-counts">
            <span><strong>{applied}</strong> applied override {applied === 1 ? "step" : "steps"}</span>
            <span><strong>{rejected}</strong> rejected override {rejected === 1 ? "step" : "steps"}</span>
          </p>
          <p className="hint">
            The table shows the action selected for each recorded step. Rejected overrides fall back to
            the branch policy; detailed rejection reasons are not recorded.
          </p>

          {steps.length === 0 ? (
            <p className="hint">No simulation steps were recorded for this branch.</p>
          ) : (
            <>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={onlyOverrides}
                  onChange={event => setOnlyOverrides(event.target.checked)}
                />
                Only steps with override flags
              </label>
              <p className="hint">Showing {visibleSteps.length} of {steps.length} recorded steps.</p>
              {visibleSteps.length === 0 ? (
                <div>
                  <p className="hint">No applied or rejected override flags were recorded.</p>
                  <button onClick={() => setOnlyOverrides(false)}>Show all recorded steps</button>
                </div>
              ) : (
                <div className="trace-table-wrap" role="region" aria-label="Recorded branch steps" tabIndex={0}>
                  <table className="data-table">
                    <thead>
                      <tr><th>Step</th><th>Turn / date</th><th>Phase</th><th>Recorded action</th><th>Override flags</th><th>RNG calls</th></tr>
                    </thead>
                    <tbody>
                      {visibleSteps.map(step => (
                        <tr key={step.stepIndex}>
                          <td>{step.stepIndex}</td>
                          <td>{step.turn}<br />{step.dateLabel}</td>
                          <td>{step.phase}</td>
                          <td>{step.actionLabel}<br /><code>{step.actionId}</code></td>
                          <td>
                            {step.overrideApplied && <strong className="trace-applied">Applied</strong>}
                            {step.overrideApplied && step.overrideRejected && " / "}
                            {step.overrideRejected && <strong className="trace-rejected">Rejected</strong>}
                            {!step.overrideApplied && !step.overrideRejected && "No override flag"}
                          </td>
                          <td>{step.rngCallsBefore} → {step.rngCallsAfter}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </>
          )}
        </>
      )}
    </section>
  );
}
