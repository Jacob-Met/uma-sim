import { useId, useLayoutEffect, useRef } from "react";
import type { CareerState } from "../api/types";

interface Props {
  state: CareerState;
  busy: boolean;
  onChoose: (id: string) => void;
}

export function EventDialog({ state, busy, onChoose }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const headingId = useId();
  const open = (
    state.awaitingChoice ||
    state.phase.toUpperCase() === "EVENT" ||
    (state.pendingEventOptions?.length ?? 0) > 0
  ) && (state.pendingEventOptions?.length ?? 0) > 0;

  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (!open || !dialog) return;
    if (!dialog.open) dialog.showModal();
    return () => {
      if (dialog.open) dialog.close();
    };
  }, [open]);

  if (!open) return null;

  return (
    <dialog
      ref={dialogRef}
      className="modal event-modal"
      aria-labelledby={headingId}
      onCancel={(event) => event.preventDefault()}
    >
      <h2 id={headingId} style={{ marginTop: 0, color: "var(--event)" }}>Event</h2>
      <p style={{ marginTop: 0 }}>{state.pendingEventTitle ?? "Choose an option"}</p>
      {busy && <p role="status">Working…</p>}
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
    </dialog>
  );
}
