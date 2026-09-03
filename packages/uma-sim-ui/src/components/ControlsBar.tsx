interface Props {
  busy: boolean;
  policy: string;
  onPolicy: (p: string) => void;
  onAuto: () => void;
  onFast: (mult: number) => void;
  onNew: () => void;
  onExport: () => void;
}

export function ControlsBar({
  busy,
  policy,
  onPolicy,
  onAuto,
  onFast,
  onNew,
  onExport,
}: Props) {
  return (
    <div className="card controls">
      <label className="field" style={{ minWidth: 140 }}>
        <span style={{ fontSize: "0.75rem", color: "var(--muted)" }}>Policy</span>
        <select
          value={policy}
          disabled={busy}
          onChange={(e) => onPolicy(e.target.value)}
        >
          <option value="bot">Bot</option>
          <option value="default">Default</option>
        </select>
      </label>
      <button disabled={busy} onClick={onAuto}>
        Auto step
      </button>
      <button disabled={busy} onClick={() => onFast(20)}>
        Fast ×20
      </button>
      <button disabled={busy} onClick={() => onFast(100)}>
        Fast ×100
      </button>
      <button disabled={busy} onClick={onExport}>
        Export telemetry
      </button>
      <button disabled={busy} onClick={onNew}>
        New run
      </button>
    </div>
  );
}
