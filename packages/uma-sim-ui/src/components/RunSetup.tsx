import {
  compatibilityGrade,
  type CatalogItem,
  type StartRequest,
} from "../api/types";
import { useMemo, useState } from "react";
import {
  emptyLegacyTree,
  flattenLegacyFactors,
  legacyParentNames,
  LegacyPanel,
  type LegacyTree,
} from "./LegacyPanel";

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
  const [legacyEnabled, setLegacyEnabled] = useState(false);
  const [legacyTree, setLegacyTree] = useState<LegacyTree>(() => emptyLegacyTree());
  const [compatibilityScore, setCompatibilityScore] = useState(0);
  const [supportFilter, setSupportFilter] = useState("");
  const [traineeFilter, setTraineeFilter] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const compatGrade = useMemo(
    () => compatibilityGrade(compatibilityScore),
    [compatibilityScore],
  );

  const filteredTrainees = useMemo(() => {
    const q = traineeFilter.toLowerCase().trim();
    return trainees.filter((t) => {
      if (!q) return true;
      return (
        t.name.toLowerCase().includes(q) ||
        (t.nameJa ?? "").toLowerCase().includes(q) ||
        t.id.toLowerCase().includes(q) ||
        String(t.charId ?? "").includes(q)
      );
    });
  }, [trainees, traineeFilter]);

  /** Career trainee cannot also be a support card (by character name). */
  const filteredSupports = useMemo(() => {
    const q = supportFilter.toLowerCase();
    const traineeKey = trainee.toLowerCase();
    return supports
      .filter((s) => s.name.toLowerCase() !== traineeKey)
      .filter((s) => !q || s.name.toLowerCase().includes(q) || s.id.includes(q))
      .slice(0, 200);
  }, [supports, supportFilter, trainee]);

  function toggle(list: string[], id: string, max: number): string[] {
    if (list.includes(id)) return list.filter((x) => x !== id);
    if (list.length >= max) return list;
    return [...list, id];
  }

  function selectTrainee(name: string) {
    setTrainee(name);
    setFormError(null);
    // Drop illegal supports matching the new trainee.
    const key = name.toLowerCase();
    setDeck((prev) =>
      prev.filter((id) => {
        const s = supports.find((x) => x.id === id);
        return !s || s.name.toLowerCase() !== key;
      }),
    );
    // Clear direct parents if they collide with the trainee.
    setLegacyTree((tree) => {
      const next = { ...tree };
      if (next.parentA.uma.toLowerCase() === key) {
        next.parentA = { ...next.parentA, uma: "" };
      }
      if (next.parentB.uma.toLowerCase() === key) {
        next.parentB = { ...next.parentB, uma: "" };
      }
      return next;
    });
  }

  function start() {
    const traineeKey = trainee.toLowerCase();
    const supportChars = deck
      .map((id) => supports.find((s) => s.id === id)?.name ?? "")
      .filter(Boolean);
    if (supportChars.some((n) => n.toLowerCase() === traineeKey)) {
      setFormError("Trainee cannot also be a support card in the deck.");
      return;
    }
    if (legacyEnabled) {
      const parents = [legacyTree.parentA.uma, legacyTree.parentB.uma].filter(Boolean);
      if (parents.some((n) => n.toLowerCase() === traineeKey)) {
        setFormError("Trainee cannot be a direct parent.");
        return;
      }
    }
    setFormError(null);
    const legacyFactors =
      legacyEnabled ? flattenLegacyFactors(legacyTree).join(",") : "";
    const parentNames =
      legacyEnabled ? legacyParentNames(legacyTree).join(",") : "";
    onStart({
      seed,
      scenario,
      trainee,
      speed,
      dialogue,
      raceModel,
      policy,
      deckSupports: deck.join(","),
      legacyFactors: legacyFactors || undefined,
      legacyTree: legacyEnabled ? legacyTree : undefined,
      parentNames: parentNames || undefined,
      compatibilityScore: legacyEnabled ? compatibilityScore : undefined,
    });
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
        <div className="field" style={{ gridColumn: "1 / -1" }}>
          <label>Trainee</label>
          <input
            placeholder="Search name or 日本語…"
            value={traineeFilter}
            onChange={(e) => setTraineeFilter(e.target.value)}
          />
          <div className="trainee-grid">
            {filteredTrainees.map((t) => {
              const selected = trainee === t.name;
              return (
                <button
                  type="button"
                  key={t.id}
                  className={`trainee-card${selected ? " selected" : ""}`}
                  disabled={busy}
                  onClick={() => selectTrainee(t.name)}
                  title={`${t.name}${t.nameJa ? ` / ${t.nameJa}` : ""}`}
                >
                  {t.iconUrl ? (
                    <img src={t.iconUrl} alt="" loading="lazy" width={56} height={56} />
                  ) : (
                    <div className="trainee-fallback">{t.name.slice(0, 2)}</div>
                  )}
                  <div className="trainee-meta">
                    <div className="trainee-en">{t.name}</div>
                    {t.nameJa ? <div className="trainee-ja">{t.nameJa}</div> : null}
                  </div>
                </button>
              );
            })}
          </div>
          <div className="chip" style={{ marginTop: "0.35rem" }}>
            Selected: {trainee}
            {trainees.find((t) => t.name === trainee)?.nameJa
              ? ` · ${trainees.find((t) => t.name === trainee)?.nameJa}`
              : ""}
          </div>
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

      <div className="field" style={{ marginTop: "0.85rem" }}>
        <label>
          Deck supports ({deck.length}/6){" "}
          <span className="chip">trainee excluded</span>
        </label>
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

      <LegacyPanel
        enabled={legacyEnabled}
        tree={legacyTree}
        trainees={trainees}
        factors={factors}
        traineeName={trainee}
        busy={busy}
        onEnabled={setLegacyEnabled}
        onChange={setLegacyTree}
      />

      {legacyEnabled && (
        <div className="field" style={{ marginTop: "0.85rem" }}>
          <label>
            Compatibility score{" "}
            <span className="chip" title="◎ &gt;150 · 〇 51–150 · △ ≤50">
              {compatGrade} {compatibilityScore}
            </span>
          </label>
          <input
            type="number"
            min={0}
            max={500}
            step={1}
            value={compatibilityScore}
            disabled={busy}
            onChange={(e) =>
              setCompatibilityScore(
                Math.max(0, Math.min(500, Number(e.target.value) || 0)),
              )
            }
          />
          <div className="chip" style={{ marginTop: "0.35rem" }}>
            Mid-run Inspiration odds × (1 + score/100). Target &gt;300, optimal ~500.
          </div>
        </div>
      )}

      {formError && (
        <div className="banner error" style={{ marginTop: "0.75rem" }}>
          {formError}
        </div>
      )}

      <div className="controls" style={{ marginTop: "0.9rem" }}>
        <button className="primary" disabled={busy} onClick={start}>
          Start run
        </button>
      </div>
    </div>
  );
}
