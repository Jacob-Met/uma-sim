import type { CareerState } from "../api/types";

const STATS = ["speed", "stamina", "power", "guts", "wit"] as const;

interface Props {
  state: CareerState;
}

export function StatsPanel({ state }: Props) {
  const half = state.date.half === 1 ? "Early" : "Late";
  return (
    <div className="card">
      <h2>Status</h2>
      <div className="meta-line">
        <span>Turn</span>
        <strong>{state.turn}</strong>
      </div>
      <div className="meta-line">
        <span>Date</span>
        <strong>
          Y{state.date.year} M{state.date.month} {half}
        </strong>
      </div>
      <div className="meta-line">
        <span>Phase</span>
        <strong>{state.phase}</strong>
      </div>
      <div className="meta-line">
        <span>Energy</span>
        <strong>
          {state.energy}/{state.maxEnergy}
        </strong>
      </div>
      <div className="meta-line">
        <span>Mood</span>
        <strong>{state.mood}</strong>
      </div>
      <div className="meta-line">
        <span>Fans</span>
        <strong>{state.fans.toLocaleString()}</strong>
      </div>
      <div className="meta-line">
        <span>Skill pts</span>
        <strong>{state.skillPoints}</strong>
      </div>

      <h3 style={{ marginTop: "0.85rem" }}>Stats</h3>
      {STATS.map((key) => {
        const v = state.stats[key];
        const pct = Math.min(100, (v / 1200) * 100);
        return (
          <div className="stat-row" key={key}>
            <span>{key}</span>
            <div className="bar">
              <span style={{ width: `${pct}%` }} />
            </div>
            <strong>{v}</strong>
          </div>
        );
      })}

      <h3 style={{ marginTop: "0.85rem" }}>Facilities</h3>
      {STATS.map((key) => (
        <div className="meta-line" key={key}>
          <span>{key}</span>
          <strong>Lv {state.facilityLevels[key] ?? 1}</strong>
        </div>
      ))}
    </div>
  );
}
