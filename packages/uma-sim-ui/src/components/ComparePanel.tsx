import { useState } from "react";
import { api } from "../api/client";
import type { BranchComparison } from "../api/types";
import { useLabStore } from "../state/labStore";
import { validateBranchDraft } from "./branchInput";

const POLICIES = ["bot", "default"];

function OutcomeCard({
  title,
  outcome,
}: {
  title: string;
  outcome: BranchComparison["outcomeA"];
}) {
  return (
    <div className="card">
      <h3>{title}</h3>
      <div className="meta-line">
        <span>
          Career {outcome.careerComplete ? "complete" : "ended early"} · turn{" "}
          {outcome.finalTurn} · {outcome.steps} steps
        </span>
      </div>
      <div className="stat-row">
        <span>SPD</span> {outcome.stats.speed}
      </div>
      <div className="stat-row">
        <span>STA</span> {outcome.stats.stamina}
      </div>
      <div className="stat-row">
        <span>POW</span> {outcome.stats.power}
      </div>
      <div className="stat-row">
        <span>GUT</span> {outcome.stats.guts}
      </div>
      <div className="stat-row">
        <span>WIT</span> {outcome.stats.wit}
      </div>
      <div className="meta-line">
        <span>
          Energy {outcome.energy} · Mood {outcome.mood} · Fans {outcome.fans}{" "}
          · SP {outcome.skillPoints}
        </span>
      </div>
      <div className="meta-line">
        <span>
          Races {outcome.completedRaces.length} · Skills{" "}
          {outcome.learnedSkills.length} · RNG calls {outcome.totalRngCalls}
        </span>
      </div>
    </div>
  );
}


function OutcomeDeltaSummary({ comparison }: { comparison: BranchComparison }) {
  const a = comparison.outcomeA;
  const b = comparison.outcomeB;
  const metrics: [string, number, number][] = [
    ["Speed", a.stats.speed, b.stats.speed],
    ["Stamina", a.stats.stamina, b.stats.stamina],
    ["Power", a.stats.power, b.stats.power],
    ["Guts", a.stats.guts, b.stats.guts],
    ["Wit", a.stats.wit, b.stats.wit],
    ["Fans", a.fans, b.fans],
    ["Skill points", a.skillPoints, b.skillPoints],
    ["Races", a.completedRaces.length, b.completedRaces.length],
    ["Skills", a.learnedSkills.length, b.learnedSkills.length],
  ];
  return (
    <section className="comparison-delta-summary" aria-label="Outcome differences">
      <h4>What changed in the outcome</h4>
      <p className="hint">
        B minus A: {comparison.bName} compared with {comparison.aName}.
        These are descriptive changes, not a winner or a causal effect.
      </p>
      {!comparison.sameCheckpoint && (
        <p className="banner warn" role="status">
          These branches do not share a checkpoint. Their differences cannot
          isolate the effect of one action or policy.
        </p>
      )}
      <div className="comparison-delta-grid">
        {metrics.map(([label, before, after]) => {
          const diff = after - before;
          const formatted = diff > 0 ? "+" + diff.toLocaleString() : diff.toLocaleString();
          return (
            <div className="comparison-delta" key={label}>
              <span>{label}</span>
              <strong>{formatted}</strong>
              <small>{before.toLocaleString()} → {after.toLocaleString()}</small>
            </div>
          );
        })}
      </div>
    </section>
  );
}

export function ComparePanel({ lab, onInspectTrace }: {
  lab: ReturnType<typeof useLabStore>;
  onInspectTrace: (id: string) => void;
}) {
  const { state, runBranch, deleteBranch, compare, setCompareSelection } = lab;
  const [checkpoint, setCheckpoint] = useState("");
  const [branchName, setBranchName] = useState("");
  const [policy, setPolicy] = useState("bot");
  const [maxActions, setMaxActions] = useState("120");
  const [overridesText, setOverridesText] = useState("");
  const [onlyChangedSteps, setOnlyChangedSteps] = useState(false);
  const draft = validateBranchDraft(overridesText, maxActions);
  const issues = draft.ok ? [] : draft.issues;
  const overrideIssues = issues.filter((issue) => issue.field === "overrides");
  const limitIssue = issues.find((issue) => issue.field === "maxActions");

  const run = () => {
    if (state.busy || !checkpoint.trim() || !draft.ok) return;
    void runBranch({
      checkpoint: checkpoint.trim(),
      name: branchName.trim(),
      policy,
      maxActions: draft.maxActions,
      overrides: draft.overrides,
    });
  };

  const c = state.comparison;
  const selectionChanged = c && (c.aId !== state.compareA || c.bId !== state.compareB);
  const changedSteps = c?.aligned.filter((step) => !step.sameAction || !step.sameOutcome) ?? [];
  const visibleSteps = c ? (onlyChangedSteps ? changedSteps : c.aligned) : [];

  return (
    <div className="card">
      <h2>Branch &amp; compare</h2>
      <p className="hint">
        A branch replays a checkpoint in a fresh engine with its own isolated
        telemetry. Identical checkpoints and actions reproduce state and RNG
        exactly; a branch can never mutate its checkpoint or a sibling.
      </p>
      <h3>Run a branch</h3>
      <div className="field-row">
        <label>
          Checkpoint
          <select
            value={checkpoint}
            onChange={(e) => setCheckpoint(e.target.value)}
          >
            <option value="">— pick a checkpoint —</option>
            {state.library.map((e) => (
              <option key={e.name} value={e.name}>
                {e.name} (turn {e.turn}, {e.scenarioId})
              </option>
            ))}
          </select>
        </label>
        <label>
          Branch name (optional)
          <input
            value={branchName}
            onChange={(e) => setBranchName(e.target.value)}
            placeholder="e.g. rest-early"
          />
        </label>
        <label>
          Policy
          <select value={policy} onChange={(e) => setPolicy(e.target.value)}>
            {POLICIES.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>
        <label>
          Max actions
          <input
            type="number"
            min={1}
            max={500}
            step={1}
            value={maxActions}
            onChange={(e) => setMaxActions(e.target.value)}
            aria-invalid={Boolean(limitIssue)}
            aria-describedby={limitIssue ? "branch-limit-error" : undefined}
          />
        </label>
      </div>
      {limitIssue && (
        <p id="branch-limit-error" className="banner error" role="alert">
          {limitIssue.message}
        </p>
      )}
      <div className="field-row">
        <label style={{ flexGrow: 1 }}>
          Per-turn overrides (one per line, <code>turn:actionId</code>; only
          legal choices apply — rejected ones are flagged in the run)
          <textarea
            value={overridesText}
            onChange={(e) => setOverridesText(e.target.value)}
            placeholder={"8:rest\n10:train_stamina"}
            rows={3}
            aria-invalid={overrideIssues.length > 0}
            aria-describedby={overrideIssues.length > 0 ? "branch-override-errors" : undefined}
          />
        </label>
      </div>
      {overrideIssues.length > 0 && (
        <div id="branch-override-errors" className="banner error" role="alert">
          <p>Correct these overrides before running the branch:</p>
          <ul>
            {overrideIssues.map((issue) => <li key={issue.line}>{issue.message}</li>)}
          </ul>
        </div>
      )}
      {draft.ok && (
        <div className="hint">
          <p>
            Ready to run up to {draft.maxActions} actions with {draft.overrides.length}{" "}
            {draft.overrides.length === 1 ? "override" : "overrides"}.
          </p>
          {draft.overrides.length > 0 && (
            <details>
              <summary>Review overrides</summary>
              <ol>
                {draft.overrides.map((override) => (
                  <li key={override.turn}>
                    <code>{override.turn}:{override.actionId}</code>
                  </li>
                ))}
              </ol>
            </details>
          )}
        </div>
      )}
      <div className="field-row">
        <button disabled={state.busy || !checkpoint.trim() || !draft.ok} onClick={run}>
          Run branch
        </button>
      </div>

      <h3>Saved branches</h3>
      {state.branches.length === 0 ? (
        <p className="hint">No branches yet — run one above.</p>
      ) : (
        <div className="comparison-table-scroll" role="region" aria-label="Saved branches" tabIndex={0}>
        <table className="data-table">
          <thead>
            <tr>
              <th>Branch</th>
              <th>Checkpoint</th>
              <th>Policy</th>
              <th>Steps</th>
              <th>Final turn</th>
              <th>Fans</th>
              <th>A</th>
              <th>B</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {state.branches.map((b) => (
              <tr key={b.id}>
                <td>{b.name}</td>
                <td>{b.checkpointName}</td>
                <td>{b.policy}</td>
                <td>{b.steps}</td>
                <td>{b.finalTurn}</td>
                <td>{b.fans}</td>
                <td>
                  <input
                    type="radio"
                    name="cmpA"
                    checked={state.compareA === b.id}
                    onChange={() => setCompareSelection(b.id, state.compareB)}
                    aria-label={`Compare A: ${b.name}`}
                  />
                </td>
                <td>
                  <input
                    type="radio"
                    name="cmpB"
                    checked={state.compareB === b.id}
                    onChange={() => setCompareSelection(state.compareA, b.id)}
                    aria-label={`Compare B: ${b.name}`}
                  />
                </td>
                <td>
                  <button disabled={state.busy} onClick={() => onInspectTrace(b.id)}>
                    Inspect trace
                  </button>{" "}
                  <button
                    disabled={state.busy}
                    onClick={() => {
                      if (window.confirm(`Delete branch "${b.name}"?`))
                        void deleteBranch(b.id);
                    }}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
      <div className="field-row">
        <button
          disabled={state.busy || !state.compareA || !state.compareB || state.compareA === state.compareB}
          onClick={() => void compare(state.compareA, state.compareB)}
        >
          Compare selected branches
        </button>
        {c && (
          <>
            <a
              className="link-button"
              href={api.labReportUrl(c.aId, c.bId, "markdown")}
              download
            >
              Download markdown report
            </a>
            <a
              className="link-button"
              href={api.labReportUrl(c.aId, c.bId, "json")}
              download
            >
              Download JSON report
            </a>
          </>
        )}
      </div>
      {selectionChanged && (
        <p className="hint" role="status">
          Showing {c.aName} vs {c.bName}; downloads contain this displayed comparison.
          Compare selected branches to update the result.
        </p>
      )}

      {c && (
        <div>
          <h3>
            {c.aName} vs {c.bName}
          </h3>
          <div className="meta-line">
            <span>
              Checkpoint <strong>{c.checkpointName}</strong> (turn{" "}
              {c.checkpointTurn}, seed {c.seed}) — same checkpoint:{" "}
              {c.sameCheckpoint ? "yes" : "no"}
            </span>
          </div>
          {c.firstDivergence ? (
            <div className="banner warn">
              First divergence: step {c.firstDivergence.stepIndex} (turn{" "}
              {c.firstDivergence.turn}, {c.firstDivergence.dateLabel},{" "}
              {c.firstDivergence.phase}) — {c.firstDivergence.kind}:{" "}
              <strong>{c.firstDivergence.labelA}</strong> vs{" "}
              <strong>{c.firstDivergence.labelB}</strong>
              {c.firstDivergence.note ? ` — ${c.firstDivergence.note}` : ""}
            </div>
          ) : (
            <div className="banner ok">
              No divergence recorded across {c.aligned.length} aligned steps.
            </div>
          )}
          <div className="turn-layout">
            <OutcomeCard title={`A — ${c.aName}`} outcome={c.outcomeA} />
            <OutcomeCard title={`B — ${c.bName}`} outcome={c.outcomeB} />
          </div>
          <OutcomeDeltaSummary comparison={c} />
          <h4>Turn-by-turn timeline</h4>
          <div className="comparison-timeline-tools">
            <label className="checkbox">
              <input type="checkbox" checked={onlyChangedSteps}
                onChange={(event) => setOnlyChangedSteps(event.target.checked)} />
              Show changed steps only
            </label>
            <span role="status" aria-live="polite">
              Showing {visibleSteps.length} of {c.aligned.length} steps · {changedSteps.length} differ
            </span>
          </div>
          <div className="comparison-table-scroll" role="region" aria-label="Branch comparison timeline" tabIndex={0}>
          <table className="data-table">
            <thead>
              <tr>
                <th>#</th>
                <th>Action A</th>
                <th>Action B</th>
                <th>E A</th>
                <th>E B</th>
                <th>Fans A</th>
                <th>Fans B</th>
                <th>SP A</th>
                <th>SP B</th>
                <th>Same?</th>
              </tr>
            </thead>
            <tbody>
              {visibleSteps.length === 0 && (
                <tr><td colSpan={10}>No changed steps in this comparison. Show all steps to inspect the complete timeline.</td></tr>
              )}
              {visibleSteps.map((r) => (
                <tr
                  key={r.stepIndex}
                  className={!r.sameAction || !r.sameOutcome ? "diverged" : ""}
                >
                  <td>{r.stepIndex}</td>
                  <td>{r.labelA ?? "—"}</td>
                  <td>{r.labelB ?? "—"}</td>
                  <td>{r.energyA ?? "—"}</td>
                  <td>{r.energyB ?? "—"}</td>
                  <td>{r.fansA ?? "—"}</td>
                  <td>{r.fansB ?? "—"}</td>
                  <td>{r.skillPointsA ?? "—"}</td>
                  <td>{r.skillPointsB ?? "—"}</td>
                  <td>{r.sameAction && r.sameOutcome ? "yes" : "no"}</td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
          <h4>Reading these results</h4>
          <ul>
            {c.caveats.map((cv, i) => (
              <li key={i} className="hint">
                {cv}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
