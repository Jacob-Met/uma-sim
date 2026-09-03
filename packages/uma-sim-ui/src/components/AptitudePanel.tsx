import type { CareerState } from "../api/types";

const SURFACE = ["turf", "dirt"] as const;
const DISTANCE = ["sprint", "mile", "medium", "long"] as const;
const STYLES = [
  { key: "front", label: "Front" },
  { key: "pace", label: "Pace" },
  { key: "late", label: "Late" },
  { key: "end", label: "End" },
] as const;

interface Props {
  state: CareerState;
  busy: boolean;
  onStyle: (style: string) => void;
}

function letter(
  map: Record<string, string> | undefined,
  key: string,
): string {
  return (map?.[key] ?? "—").toUpperCase();
}

function AptRow({
  label,
  base,
  effective,
}: {
  label: string;
  base: string;
  effective: string;
}) {
  const raised = base !== effective && effective !== "—";
  return (
    <div className="meta-line apt-row">
      <span>{label}</span>
      <strong>
        {raised ? (
          <>
            <span className="apt-base">{base}</span>
            <span className="apt-arrow">→</span>
            <span className="apt-eff">{effective}</span>
          </>
        ) : (
          effective
        )}
      </strong>
    </div>
  );
}

export function AptitudePanel({ state, busy, onStyle }: Props) {
  const base = state.baseAptitudes ?? {};
  const eff = state.legacy?.aptitudes ?? {};
  const preferred = (state.preferredRunningStyle ?? "").toLowerCase();

  return (
    <div className="card">
      <h2>Aptitudes</h2>
      <h3>Surface</h3>
      {SURFACE.map((k) => (
        <AptRow key={k} label={k} base={letter(base, k)} effective={letter(eff, k)} />
      ))}
      <h3 style={{ marginTop: "0.65rem" }}>Distance</h3>
      {DISTANCE.map((k) => (
        <AptRow key={k} label={k} base={letter(base, k)} effective={letter(eff, k)} />
      ))}
      <h3 style={{ marginTop: "0.65rem" }}>Style</h3>
      {STYLES.map(({ key, label }) => (
        <AptRow
          key={key}
          label={label}
          base={letter(base, key)}
          effective={letter(eff, key)}
        />
      ))}
      <h3 style={{ marginTop: "0.85rem" }}>Preferred style</h3>
      <p className="hint">
        Feeds race strategy (default: best style aptitude).
      </p>
      <select
        disabled={busy}
        value={preferred}
        onChange={(e) => onStyle(e.target.value)}
      >
        <option value="">Auto (best aptitude)</option>
        {STYLES.map(({ key, label }) => (
          <option key={key} value={key}>
            {label} ({letter(eff, key)})
          </option>
        ))}
      </select>
    </div>
  );
}
