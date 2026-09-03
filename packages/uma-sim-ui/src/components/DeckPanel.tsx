import type { CatalogItem, CareerState } from "../api/types";

const FACILITIES = ["speed", "stamina", "power", "guts", "wit"] as const;

interface Props {
  state: CareerState;
  supports: CatalogItem[];
  busy: boolean;
  onPlace: (supportId: string, facility: string) => void;
}

export function DeckPanel({ state, supports, busy, onPlace }: Props) {
  const nameOf = (id: string) =>
    supports.find((s) => s.id === id)?.name ?? id;

  if (!state.deck?.slots?.length) {
    return (
      <div className="card">
        <h2>Deck</h2>
        <div className="meta-line">No deck slots</div>
      </div>
    );
  }

  return (
    <div className="card">
      <h2>Deck</h2>
      {state.deck.slots.map((slot) => (
        <div className="deck-slot" key={slot.supportId}>
          <div>
            <div>{nameOf(slot.supportId)}</div>
            <div className="chip">
              bond {slot.bond}
              {slot.specialty ? ` · ${slot.specialty}` : ""}
              {slot.assignedFacility ? ` · ${slot.assignedFacility}` : ""}
            </div>
            <div className="bar" style={{ marginTop: "0.25rem" }}>
              <span style={{ width: `${Math.min(100, (slot.bond / 100) * 100)}%` }} />
            </div>
          </div>
          <select
            disabled={busy}
            value={slot.assignedFacility ?? ""}
            onChange={(e) => {
              const fac = e.target.value;
              if (fac) onPlace(slot.supportId, fac);
            }}
          >
            <option value="">Facility…</option>
            {FACILITIES.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </div>
      ))}
    </div>
  );
}
