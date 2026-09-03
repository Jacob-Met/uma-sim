import type { CareerState } from "../api/types";

interface Props {
  state: CareerState;
  onNew: () => void;
}

export function CareerSummary({ state, onNew }: Props) {
  if (!state.careerComplete) return null;
  return (
    <div className="card">
      <h2>Career complete</h2>
      <div className="meta-line">
        <span>Trainee</span>
        <strong>{state.meta.traineeName}</strong>
      </div>
      <div className="meta-line">
        <span>Fans</span>
        <strong>{state.fans.toLocaleString()}</strong>
      </div>
      <div className="meta-line">
        <span>Final stats</span>
        <strong>
          Sp {state.stats.speed} / St {state.stats.stamina} / Po {state.stats.power}{" "}
          / Gu {state.stats.guts} / Wi {state.stats.wit}
        </strong>
      </div>
      <div className="meta-line">
        <span>Statuses</span>
        <strong>{state.statuses?.join(", ") || "—"}</strong>
      </div>
      <div className="meta-line">
        <span>Skills</span>
        <strong>{state.learnedSkillIds?.length ?? 0}</strong>
      </div>
      <div style={{ marginTop: "0.45rem" }}>
        {(state.learnedSkillIds ?? []).slice(0, 24).map((id) => (
          <span className="chip" key={id} style={{ margin: "0.15rem" }}>
            {id}
          </span>
        ))}
      </div>
      <div className="meta-line" style={{ marginTop: "0.55rem" }}>
        <span>Races</span>
        <strong>{state.completedRaces?.length ?? 0}</strong>
      </div>
      <div style={{ marginTop: "0.35rem" }}>
        {(state.completedRaces ?? []).map((r) => (
          <span className="chip" key={r} style={{ margin: "0.15rem" }}>
            {r}
          </span>
        ))}
      </div>
      <div className="meta-line" style={{ marginTop: "0.55rem" }}>
        <span>Generated sparks</span>
        <strong>{state.generatedSparks?.length ?? 0}</strong>
      </div>
      <div style={{ marginTop: "0.35rem" }}>
        {(state.generatedSparks ?? []).map((s) => (
          <span
            className="chip"
            key={`${s.color}-${s.factorId}-${s.stars}`}
            style={{ margin: "0.15rem" }}
            title={s.factorId}
          >
            {s.color} {s.stars}★ · {s.label}
          </span>
        ))}
      </div>
      <div className="controls" style={{ marginTop: "0.85rem" }}>
        <button className="primary" onClick={onNew}>
          New run
        </button>
      </div>
    </div>
  );
}
