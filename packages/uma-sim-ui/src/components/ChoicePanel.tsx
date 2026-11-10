import type { CareerState, Choice, TrainingInspection, TrainingInspectionRow } from "../api/types";

interface Props {
  choices: Choice[];
  state: CareerState;
  inspection?: TrainingInspection;
  busy: boolean;
  onChoose: (id: string) => void;
}

const STATS = [
  ["speed", "Speed"], ["stamina", "Stamina"], ["power", "Power"],
  ["guts", "Guts"], ["wit", "Wit"],
] as const;

function kind(id: string): string {
  if (id.startsWith("train_")) return "train";
  if (id === "rest" || id === "recreation") return "rest";
  if (id === "race") return "race";
  if (id.startsWith("event_")) return "event";
  if (id.startsWith("gl_")) return "scenario";
  return "";
}

function signed(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

function TrainingDetail({ row, energy }: { row: TrainingInspectionRow; energy: number }) {
  const blocked = row.blockedReason === "injured"
    ? "Injured · rest to recover"
    : row.blockedReason === "insufficient_energy"
      ? `Needs ${row.energyCost} energy · ${energy} available`
      : null;
  return (
    <>
      <span className="training-gains" aria-label="Sample stat gains">
        {STATS.filter(([key]) => row.statGains[key] !== 0).map(([key, label]) => (
          <span key={key}><span>{label}</span> <strong>{signed(row.statGains[key])}</strong></span>
        ))}
        {STATS.every(([key]) => row.statGains[key] === 0) && <span>No sampled stat gain</span>}
      </span>
      {blocked ? <span className="training-blocked">{blocked}</span> : (
        <span className="training-cost">
          <span>Energy <strong>{row.energyDelta === null ? "—" : signed(row.energyDelta)}</strong></span>
          <span className={row.failureChancePct !== null && row.failureChancePct >= 20 ? "training-risk" : ""}>
            Fail <strong>{row.failureChancePct === null ? "—" : `${row.failureChancePct}%`}</strong>
          </span>
        </span>
      )}
    </>
  );
}

export function ChoicePanel({ choices, state, inspection, busy, onChoose }: Props) {
  // Only display samples belonging to this exact visible turn and phase.
  const current = inspection?.turn === state.turn && inspection.phase === state.phase
    && inspection.scenarioId === state.meta.scenarioId
    && inspection.traineeName === state.meta.traineeName ? inspection : undefined;
  const rows = new Map(current?.rows.map((row) => [row.actionId, row]) ?? []);
  const hasTraining = choices.some((choice) => choice.id.startsWith("train_"));
  const phase = state.careerComplete ? "Career complete"
    : state.phase === "MANDATORY_RACE" ? "Race day"
    : state.phase === "EVENT" ? "Event choice" : "Choose your next move";
  return (
    <section className="turn-decisions" aria-labelledby="turn-heading">
      <div className="turn-heading-row">
        <div>
          <h2 id="turn-heading" tabIndex={-1}>Turn {state.turn} <span>/ 72</span> · {phase}</h2>
          <p>{state.meta.traineeName} · Year {state.date.year}, month {state.date.month}, {state.date.half === 1 ? "early" : "late"}</p>
        </div>
        <div className="turn-energy">
          <span>Energy</span><strong>{state.energy}<small> / {state.maxEnergy}</small></strong>
          <meter min={0} max={state.maxEnergy} value={state.energy} aria-label="Current energy" />
        </div>
      </div>
      <dl className="turn-stats">
        {STATS.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{state.stats[key]}</dd></div>)}
      </dl>
      <div className="turn-context">
        <span>Mood <strong>{state.mood.toLowerCase()}</strong></span>
        <span>Fans <strong>{state.fans.toLocaleString()}</strong></span>
        <span>Skill points <strong>{state.skillPoints}</strong></span>
      </div>
      {hasTraining && <p className="training-caption">
        {current ? "Sample gains · energy on success · current failure chance" : "Training details are unavailable from this server."}
      </p>}
      <div className="choice-list decision-list">
        {choices.length === 0 && <p className="hint">{state.careerComplete ? "This career has finished. Review its results or start another run." : "No actions are available in this phase."}</p>}
        {choices.map((c, i) => {
          const row = rows.get(c.id);
          const level = row?.level ?? state.facilityLevels[c.id.replace("train_", "")];
          const label = row ? row.label : c.label;
          return (
            <button
              key={c.id}
              className={`decision-row ${kind(c.id)}${row ? " has-training" : ""}`}
              disabled={busy || row?.available === false}
              onClick={() => onChoose(c.id)}
              title={`Shortcut ${i + 1}`}
            >
              <span className="decision-label">
                <kbd>{i + 1}</kbd><strong>{label}</strong>
                {c.id.startsWith("train_") && level != null && <small>Lv {level}</small>}
              </span>
              {row && <TrainingDetail row={row} energy={state.energy} />}
            </button>
          );
        })}
      </div>
      {hasTraining && current && <details className="training-explainer">
        <summary>What the training numbers mean</summary>
        <p>Stat gains are a fixed sample before stat caps and later scenario or event effects. They are neither guaranteed next-turn results nor expected values.</p>
        <p>Energy is the successful training change before later effects. Failure is the current chance of an available attempt. Checking these numbers does not advance the career or its random sequence.</p>
      </details>}
    </section>
  );
}
