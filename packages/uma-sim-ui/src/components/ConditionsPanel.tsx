import { useId } from "react";
import type { CareerState } from "../api/types";
import { conditionsView } from "./conditionsView";
import "../styles/conditions.css";

export function ConditionsPanel({ state }: { state: CareerState }) {
  const titleId = useId();
  const view = conditionsView(state.statuses);

  return (
    <section className="card conditions-panel" aria-labelledby={titleId}>
      <h2 id={titleId}>
        Conditions &amp; titles
        {view.reported && <span className="conditions-count">{view.rows.length}</span>}
      </h2>
      {!view.reported ? (
        <p className="hint">Conditions and titles were not reported by this career snapshot.</p>
      ) : view.rows.length === 0 ? (
        <p className="hint">No conditions or titles recorded.</p>
      ) : (
        <>
          {view.hasInjury && (
            <div className="conditions-injury-note">
              <strong>{state.careerComplete ? "Injury retained in this completed career" : "Training is blocked by injury"}</strong>
              {!state.careerComplete && (
                <p>Rest clears injury conditions when that choice is available.</p>
              )}
            </div>
          )}
          <ul className="conditions-list" tabIndex={0} aria-label="Recorded conditions and titles">
            {view.rows.map((row, index) => (
              <li key={row.id + ":" + index} className={"conditions-row conditions-row-" + row.kind}>
                <div className="conditions-row-heading">
                  <strong>{row.label ?? (row.id.trim() === "" ? "Unnamed status" : row.id)}</strong>
                  <span className="conditions-kind">
                    {row.kind === "title" ? "Simulator title" : row.kind === "injury" ? "Condition" : "Retained status"}
                  </span>
                </div>
                {(row.label || row.id.trim() === "") && (
                  <code className="conditions-id">{row.id.trim() === "" ? JSON.stringify(row.id) : row.id}</code>
                )}
              </li>
            ))}
          </ul>
          <p className="hint conditions-footnote">Only recorded IDs are shown; other effects and durations are not defined here.</p>
        </>
      )}
    </section>
  );
}
