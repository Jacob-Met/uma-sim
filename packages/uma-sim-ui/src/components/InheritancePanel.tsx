import { useId } from "react";
import type { CatalogItem, CareerState } from "../api/types";
import {
  inheritanceView,
  type RecordedBonus,
  type RecordedFactor,
} from "./inheritanceView";
import "./inheritance-record.css";

interface Props {
  state: CareerState;
  factors: CatalogItem[];
}

function FactorName({ factor }: { factor: RecordedFactor }) {
  return (
    <span className="inheritance-factor">
      {factor.name !== null && <span>{factor.name}</span>}
      <code>{factor.id === "" ? "(empty ID)" : factor.id}</code>
    </span>
  );
}

function RecordedStrings({
  values,
  empty,
}: {
  values: string[] | null;
  empty: string;
}) {
  if (values === null) return <p className="hint">Not recorded.</p>;
  if (values.length === 0) return <p className="hint">{empty}</p>;
  return (
    <ul className="inheritance-list">
      {values.map((value, index) => (
        <li key={index}><code>{value === "" ? "(empty entry)" : value}</code></li>
      ))}
    </ul>
  );
}

function BonusList({
  title,
  values,
}: {
  title: string;
  values: RecordedBonus[] | null;
}) {
  return (
    <div className="inheritance-bonuses">
      <h3>{title}</h3>
      {values === null ? (
        <p className="hint">Not recorded.</p>
      ) : values.length === 0 ? (
        <p className="hint">No additions recorded.</p>
      ) : (
        <dl>
          {values.map((row) => (
            <div key={row.key}>
              <dt>{row.label}</dt>
              <dd>{row.value === null ? "Not recorded" : String(row.value)}</dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

export function InheritancePanel({ state, factors }: Props) {
  const heading = useId();
  const view = inheritanceView(state.meta, state.legacy, factors);
  const parents = view.parentNames === null
    ? "Not recorded"
    : view.parentNames.length === 0
      ? "No parents recorded"
      : view.parentNames.map((name) => name === "" ? "(empty name)" : name).join(" · ");

  return (
    <section className="card inheritance-record" aria-labelledby={heading}>
      <h2 id={heading}>Inheritance record</h2>
      <p className="hint">
        Inspect the lineage and inheritance values saved with this career.
      </p>
      <dl className="inheritance-overview">
        <div><dt>Parent names</dt><dd>{parents}</dd></div>
        <div>
          <dt>Initial inheritance choice</dt>
          <dd>{view.choiceComplete === null ? "Not recorded" : view.choiceComplete ? "Completed" : "Not completed"}</dd>
        </div>
        <div>
          <dt>Mid-run inspirations completed</dt>
          <dd>{view.inspirations === null ? "Not recorded" : String(view.inspirations)}</dd>
        </div>
      </dl>

      <details className="inheritance-disclosure">
        <summary>Lineage and recorded sparks</summary>
        <dl className="inheritance-overview">
          <div>
            <dt>Recorded compatibility</dt>
            <dd>{view.compatibility === null ? "Not recorded" : String(view.compatibility)}</dd>
          </div>
        </dl>
        {view.ancestors === null ? (
          <p className="hint">
            No structured lineage is recorded. Flat entries below do not identify ancestor positions.
          </p>
        ) : (
          <div className="inheritance-ancestors">
            {view.ancestors.map((ancestor) => (
              <article className="inheritance-ancestor" key={ancestor.key} data-ancestor={ancestor.key}>
                <h3>{ancestor.label}</h3>
                {!ancestor.recorded ? (
                  <p className="hint">This ancestor position is not recorded.</p>
                ) : (
                  <>
                    <p className="inheritance-name">
                      {ancestor.name === null ? "Name not recorded" : ancestor.name === "" ? "No name recorded" : ancestor.name}
                    </p>
                    {ancestor.sparks.length === 0 && ancestor.unavailableSlots.length === 0 && (
                      <p className="hint">No sparks recorded.</p>
                    )}
                    <ul className="inheritance-list">
                      {ancestor.sparks.map((spark) => (
                        <li key={spark.kind} data-factor-id={spark.id}>
                          <div className="inheritance-spark-label">
                            <strong>{spark.label}</strong>
                            <span>{spark.stars === null ? "Stars not recorded" : String(spark.stars) + "★ recorded"}</span>
                          </div>
                          <FactorName factor={spark} />
                        </li>
                      ))}
                    </ul>
                    {ancestor.unavailableSlots.length > 0 && (
                      <p className="hint">
                        Spark slots not recorded: {ancestor.unavailableSlots.join(", ")}.
                      </p>
                    )}
                  </>
                )}
              </article>
            ))}
          </div>
        )}
        <h3>Recorded flat entries</h3>
        <RecordedStrings values={view.flatEntries} empty="No flat entries recorded." />
      </details>

      <details className="inheritance-disclosure">
        <summary>Retained factors and bonuses</summary>
        <p className="hint">Saved values are shown directly; totals and future inheritance outcomes are not estimated.</p>
        <div className="inheritance-bonus-grid">
          <BonusList title="Stat-cap additions" values={view.capAdditions} />
          <BonusList title="Starting-stat bonuses" values={view.startingBonuses} />
        </div>
        <h3>Retained factor IDs</h3>
        {view.retainedFactors === null ? (
          <p className="hint">Not recorded.</p>
        ) : view.retainedFactors.length === 0 ? (
          <p className="hint">No factors recorded.</p>
        ) : (
          <ul className="inheritance-list">
            {view.retainedFactors.map((factor, index) => (
              <li key={index}><FactorName factor={factor} /></li>
            ))}
          </ul>
        )}
        <h3>Inherited skill candidates</h3>
        <p className="hint">
          These are the skill IDs retained for the inheritance choice. The Skills panel shows skills actually learned.
        </p>
        <RecordedStrings values={view.inheritedSkillIds} empty="No inherited skill candidates recorded." />
      </details>
    </section>
  );
}
