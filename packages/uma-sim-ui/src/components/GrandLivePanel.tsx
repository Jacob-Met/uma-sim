import type { CareerState, Choice } from "../api/types";

const TOKENS = ["Da", "Pa", "Vo", "Vi", "Me"] as const;

interface Props {
  state: CareerState;
  choices: Choice[];
  busy: boolean;
  onChoose: (id: string) => void;
}

export function GrandLivePanel({ state, choices, busy, onChoose }: Props) {
  if (state.meta.scenarioId !== "grand_concert") return null;
  const vals = state.scenarioResources?.values ?? {};
  const lessons = choices.filter(
    (c) => c.id.startsWith("gl_song_") || c.id.startsWith("gl_tech_"),
  );

  return (
    <div className="card">
      <h2>Grand Live</h2>
      <div className="token-grid">
        {TOKENS.map((t) => (
          <div className="token" key={t}>
            <div className="chip">{t}</div>
            <div className="n">{state.performanceTokens?.[t] ?? 0}</div>
          </div>
        ))}
      </div>
      <div style={{ marginTop: "0.65rem" }}>
        <div className="meta-line">
          <span>Cycle songs</span>
          <strong>{vals.cycle_songs ?? vals.cycleSongs ?? "—"}</strong>
        </div>
        <div className="meta-line">
          <span>Concert index</span>
          <strong>{vals.concert_index ?? vals.concertIndex ?? "—"}</strong>
        </div>
        <div className="meta-line">
          <span>Last live</span>
          <strong>
            {vals.last_live_result ?? vals.lastLiveResult ?? "—"}
            {vals.last_live_type != null || vals.lastLiveType != null
              ? ` (${vals.last_live_type ?? vals.lastLiveType})`
              : ""}
          </strong>
        </div>
        <div className="meta-line">
          <span>Members ready</span>
          <strong>
            {vals.member_ready_count ?? vals.memberReadyCount ?? "—"}
          </strong>
        </div>
      </div>
      {lessons.length > 0 && (
        <>
          <h3 style={{ marginTop: "0.75rem" }}>Lesson board</h3>
          <div className="choice-list">
            {lessons.map((c) => (
              <button
                key={c.id}
                className="scenario"
                disabled={busy}
                onClick={() => onChoose(c.id)}
              >
                {c.label}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
