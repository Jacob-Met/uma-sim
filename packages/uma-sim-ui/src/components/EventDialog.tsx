import type { CareerState } from "../api/types";

interface Props {
  state: CareerState;
  busy: boolean;
  onChoose: (id: string) => void;
}

export function EventDialog({ state, busy, onChoose }: Props) {
  const open =
    state.awaitingChoice ||
    state.phase.toUpperCase() === "EVENT" ||
    (state.pendingEventOptions?.length ?? 0) > 0;
  if (!open || !state.pendingEventOptions?.length) return null;

  return (
    <div className="modal-backdrop">
      <div className="modal">
        <h2 style={{ marginTop: 0, color: "var(--event)" }}>Event</h2>
        <p style={{ marginTop: 0 }}>{state.pendingEventTitle ?? "Choose an option"}</p>
        <div className="choice-list">
          {state.pendingEventOptions.map((label, i) => (
            <button
              key={i}
              className="event"
              disabled={busy}
              onClick={() => onChoose(`event_${i}`)}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
