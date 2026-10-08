import { useState } from "react";
import { api } from "../api/client";
import type { ActionOverride, BranchComparison } from "../api/types";
import { useLabStore } from "../state/labStore";

const POLICIES = ["bot", "default"];

/** Parse "turn:actionId" lines into overrides. */
export function parseOverrides(text: string): ActionOverride[] {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .map((line) => {
      const [turn, ...rest] = line.split(":");
      return { turn: Number(turn.trim()), actionId: rest.join(":").trim() };
    })
    .filter((o) => Number.isFinite(o.turn) && o.turn >= 0 && o.actionId.length > 0);
}

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

export function ComparePanel({ lab }: { lab: ReturnType<typeof useLabStore> }) {
  const { state, runBranch, deleteBranch, compare, setCompareSelection } = lab;
  const [checkpoint, setCheckpoint] = useState("");
  const [branchName, setBranchName] = useState("");
  const [policy, setPolicy] = useState("bot");
  const [maxActions, setMaxActions] = useState(120);
  const [overridesText, setOverridesText] = useState("");

  const run = () => {
    if (!checkpoint.trim()) return;
    void runBranch({
      checkpoint: checkpoint.trim(),
      name: branchName.trim(),
      policy,
      maxActions,
      overrides: parseOverrides(overridesText),
    });
  };

  const c = state.comparison;

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
            value={maxActions}
            onChange={(e) => setMaxActions(Number(e.target.value))}
          />
        </label>
        <button disabled={state.busy || !checkpoint.trim()} onClick={run}>
          Run branch
        </button>
      </div>
      <div className="field-row">
        <label style={{ flexGrow: 1 }}>
          Per-turn overrides (one per line, <code>turn:actionId</code>; only
          legal choices apply — rejected ones are flagged in the run)
          <textarea
            value={overridesText}
            onChange={(e) => setOverridesText(e.target.value)}
            placeholder={"8:rest\n10:train_stamina"}
            rows={3}
          />
        </label>
      </div>

      <h3>Saved branches</h3>
      {state.branches.length === 0 ? (
        <p className="hint">No branches yet — run one above.</p>
      ) : (
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
              <th></th>
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
      )}
      <div className="field-row">
        <button
          disabled={state.busy || !state.compareA || !state.compareB}
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
          <h4>Turn-by-turn timeline</h4>
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
              {c.aligned.map((r) => (
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
