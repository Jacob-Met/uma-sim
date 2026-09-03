import type { CatalogItem, StartRequest } from "../api/types";
import { useMemo, useState } from "react";

interface Props {
  scenarios: CatalogItem[];
  trainees: CatalogItem[];
  supports: CatalogItem[];
  factors: CatalogItem[];
  busy: boolean;
  onStart: (req: StartRequest) => void;
}

export function RunSetup({
  scenarios,
  trainees,
  supports,
  factors,
  busy,
  onStart,
}: Props) {
  const [seed, setSeed] = useState(42);
  const [scenario, setScenario] = useState(scenarios[0]?.id ?? "ura");
  const [trainee, setTrainee] = useState(trainees[0]?.name ?? "Special Week");
  const [speed, setSpeed] = useState(1);
  const [dialogue, setDialogue] = useState("choices");
  const [raceModel, setRaceModel] = useState("physics");
  const [policy, setPolicy] = useState("bot");
  const [deck, setDeck] = useState<string[]>([]);
  const [legacy, setLegacy] = useState<string[]>([]);
  const [supportFilter, setSupportFilter] = useState("");
  const [factorFilter, setFactorFilter] = useState("");

  const filteredSupports = useMemo(() => {
    const q = supportFilter.toLowerCase();
    return supports
      .filter((s) => !q || s.name.toLowerCase().includes(q) || s.id.includes(q))
      .slice(0, 200);
  }, [supports, supportFilter]);

  const filteredFactors = useMemo(() => {
    const q = factorFilter.toLowerCase();
    return factors
      .filter((f) => !q || f.name.toLowerCase().includes(q) || f.id.includes(q))
      .slice(0, 200);
  }, [factors, factorFilter]);

  function toggle(list: string[], id: string, max: number): string[] {
    if (list.includes(id)) return list.filter((x) => x !== id);
    if (list.length >= max) return list;
    return [...list, id];
  }

  return (
    <div className="card">
      <h2>New career</h2>
      <div className="grid-setup">
        <div className="field">
          <label>Scenario</label>
          <select value={scenario} onChange={(e) => setScenario(e.target.value)}>
            {scenarios.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>Trainee</label>
          <select value={trainee} onChange={(e) => setTrainee(e.target.value)}>
            {trainees.map((t) => (
              <option key={t.id} value={t.name}>
                {t.name}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>Seed</label>
          <input
            type="number"
            value={seed}
            onChange={(e) => setSeed(Number(e.target.value))}
          />
        </div>
        <div className="field">
          <label>Speed</label>
          <input
            type="number"
            min={1}
            max={100}
            value={speed}
            onChange={(e) => setSpeed(Number(e.target.value))}
          />
        </div>
        <div className="field">
          <label>Dialogue</label>
          <select value={dialogue} onChange={(e) => setDialogue(e.target.value)}>
            <option value="off">Off</option>
            <option value="choices">Choices</option>
            <option value="full">Full</option>
          </select>
        </div>
        <div className="field">
          <label>Race model</label>
          <select value={raceModel} onChange={(e) => setRaceModel(e.target.value)}>
            <option value="physics">Physics</option>
            <option value="stub">Stub</option>
          </select>
        </div>
        <div className="field">
          <label>Policy</label>
          <select value={policy} onChange={(e) => setPolicy(e.target.value)}>
            <option value="bot">Bot (scoring)</option>
            <option value="default">Default</option>
          </select>
        </div>
      </div>

      <div className="grid-setup" style={{ marginTop: "0.85rem" }}>
        <div className="field">
          <label>Deck supports ({deck.length}/6)</label>
          <input
            placeholder="Filter…"
            value={supportFilter}
            onChange={(e) => setSupportFilter(e.target.value)}
          />
          <div className="multi">
            {filteredSupports.map((s) => (
              <label key={s.id}>
                <input
                  type="checkbox"
                  checked={deck.includes(s.id)}
                  onChange={() => setDeck(toggle(deck, s.id, 6))}
                />
                <span>
                  {s.name}{" "}
                  <span className="chip">
                    {s.type} R{s.rarity}
                  </span>
                </span>
              </label>
            ))}
          </div>
        </div>
        <div className="field">
          <label>Legacy factors ({legacy.length}/6)</label>
          <input
            placeholder="Filter…"
            value={factorFilter}
            onChange={(e) => setFactorFilter(e.target.value)}
          />
          <div className="multi">
            {filteredFactors.map((f) => (
              <label key={f.id}>
                <input
                  type="checkbox"
                  checked={legacy.includes(f.id)}
                  onChange={() => setLegacy(toggle(legacy, f.id, 6))}
                />
                <span>
                  {f.name} <span className="chip">{f.kind}</span>
                </span>
              </label>
            ))}
          </div>
        </div>
      </div>

      <div className="controls" style={{ marginTop: "0.9rem" }}>
        <button
          className="primary"
          disabled={busy}
          onClick={() =>
            onStart({
              seed,
              scenario,
              trainee,
              speed,
              dialogue,
              raceModel,
              policy,
              deckSupports: deck.join(","),
              legacyFactors: legacy.join(","),
            })
          }
        >
          Start run
        </button>
      </div>
    </div>
  );
}
