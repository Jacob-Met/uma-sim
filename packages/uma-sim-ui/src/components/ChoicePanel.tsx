import type { CareerState, Choice } from "../api/types";

interface Props {
  choices: Choice[];
  state: CareerState;
  busy: boolean;
  onChoose: (id: string) => void;
}

function kind(id: string): string {
  if (id.startsWith("train_")) return "train";
  if (id === "rest" || id === "recreation") return "rest";
  if (id === "race") return "race";
  if (id.startsWith("event_")) return "event";
  if (id.startsWith("gl_")) return "scenario";
  return "";
}

export function ChoicePanel({ choices, state, busy, onChoose }: Props) {
  return (
    <div className="card">
      <h2>Choices</h2>
      <div className="choice-list">
        {choices.length === 0 && <div className="meta-line">No actions</div>}
        {choices.map((c, i) => {
          const k = kind(c.id);
          let label = c.label;
          if (c.id.startsWith("train_")) {
            const fac = c.id.replace("train_", "");
            const lv = state.facilityLevels[fac];
            if (lv != null) label = `${c.label} (Lv ${lv})`;
          }
          return (
            <button
              key={c.id}
              className={k}
              disabled={busy}
              onClick={() => onChoose(c.id)}
              title={`Shortcut ${i + 1}`}
            >
              <span className="chip">{i + 1}</span> {label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
