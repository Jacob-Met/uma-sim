import { useEffect, useMemo, useState } from "react";
import { RunSetup } from "./components/RunSetup";
import { StatsPanel } from "./components/StatsPanel";
import { ConditionsPanel } from "./components/ConditionsPanel";
import { LogPanel } from "./components/LogPanel";
import { ChoicePanel } from "./components/ChoicePanel";
import { EventDialog } from "./components/EventDialog";
import { RacePanel } from "./components/RacePanel";
import { GrandLivePanel } from "./components/GrandLivePanel";
import { DeckPanel } from "./components/DeckPanel";
import { AptitudePanel } from "./components/AptitudePanel";
import { InheritancePanel } from "./components/InheritancePanel";
import { SkillsPanel } from "./components/SkillsPanel";
import { CareerSummary } from "./components/CareerSummary";
import { ControlsBar } from "./components/ControlsBar";
import { api } from "./api/client";
import { useRunStore } from "./state/runStore";
import { LabTab } from "./components/LabTab";

export default function App() {
  const {
    state,
    bootstrap,
    startRun,
    act,
    autoStep,
    fastForward,
    placeDeck,
    setStyle,
    newRun,
    refreshActive,
    clearError,
    clearToast,
  } = useRunStore();
  const [policy, setPolicy] = useState("bot");
  const [tab, setTab] = useState<"run" | "lab">("run");
  const visibleChoices = useMemo(() => state.choices.filter(
    (choice) => !choice.id.startsWith("gl_song_") && !choice.id.startsWith("gl_tech_"),
  ), [state.choices]);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    if (!state.toast) return;
    const t = window.setTimeout(() => clearToast(), 2200);
    return () => window.clearTimeout(t);
  }, [state.toast, clearToast]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (tab !== "run" || state.busy || !state.snapshot || e.defaultPrevented ||
          e.repeat || e.isComposing || e.ctrlKey || e.metaKey || e.altKey) return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName;
      if (target?.isContentEditable || tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      const n = Number(e.key);
      if (n >= 1 && n <= 9) {
        const choice = visibleChoices[n - 1];
        if (choice) {
          e.preventDefault();
          void act(choice.id);
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [act, state.busy, state.snapshot, tab, visibleChoices]);

  const cs = state.snapshot?.state;
  const phase = (cs?.phase ?? "").toUpperCase();
  const mandatory = phase === "MANDATORY_RACE";

  async function exportTelemetry() {
    if (state.sessionId === null) return;
    try {
      const data = await api.telemetry(state.sessionId);
      const blob = new Blob([JSON.stringify(data, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `uma-sim-telemetry-${cs?.meta.seed ?? "run"}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      clearError();
      alert(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="app">
      <header className="header">
        <div>
          <h1>uma-sim</h1>
          <div className="sub">
            Interactive career simulator
            {state.health?.version ? ` · v${state.health.version}` : ""}
          </div>
          {tab === "run" && state.sessionId !== null && (
            <div className="sub">Session: {state.sessionId || "main"}</div>
          )}
        </div>
        <nav className="tabs">
          <button
            className={tab === "run" ? "active" : ""}
            onClick={() => setTab("run")}
          >
            Run
          </button>
          <button
            className={tab === "lab" ? "active" : ""}
            onClick={() => setTab("lab")}
          >
            Library &amp; lab
          </button>
        </nav>
      </header>

      {state.health && !state.health.repoRoot && (
        <div className="banner warn">
          Research / knowledge root not detected — engine is using the builtin
          fallback catalog. Run `uma-sim serve` from the repo (or a release zip
          that includes `research/` and `knowledge/`).
        </div>
      )}

      {state.health?.repoRoot && !cs && (
        <div className="banner ok">
          Connected · catalogs loaded
          {state.health.repoRootPath
            ? ` · ${state.health.repoRootPath}`
            : ""}
        </div>
      )}

      {state.error && (
        <div className="banner error" onClick={clearError} role="button">
          {state.error} (click to dismiss)
        </div>
      )}

      {tab === "lab" && <LabTab refreshRun={refreshActive} />}

      {tab === "run" && (
        <>
          {!cs && (
        <RunSetup
          scenarios={state.catalogs.scenarios}
          trainees={state.catalogs.trainees}
          supports={state.catalogs.supports}
          factors={state.catalogs.factors}
          busy={state.busy}
          onStart={(req) => void startRun({ ...req, policy })}
        />
      )}

      {cs && (
        <>
          <ControlsBar
            busy={state.busy}
            policy={policy}
            onPolicy={setPolicy}
            onAuto={() => void autoStep(policy)}
            onFast={(m) => void fastForward(m, policy)}
            onNew={newRun}
            onExport={() => void exportTelemetry()}
          />
          <CareerSummary state={cs} onNew={newRun} />
          <div className="turn-layout">
            <div style={{ display: "flex", flexDirection: "column", gap: "0.85rem" }}>
              <StatsPanel state={cs} />
              <ConditionsPanel state={cs} />
              <DeckPanel
                state={cs}
                supports={state.catalogs.supports}
                busy={state.busy}
                onPlace={(id, fac) => void placeDeck(id, fac)}
              />
              <AptitudePanel
                state={cs}
                busy={state.busy}
                onStyle={(style) => void setStyle(style)}
              />
              <InheritancePanel state={cs} factors={state.catalogs.factors} />
              <SkillsPanel state={cs} />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.85rem" }}>
              <LogPanel lines={state.textLines} history={cs.log} />
              <RacePanel
                lines={cs.log}
                mandatory={mandatory}
                pendingRaceId={cs.pendingRaceId}
                busy={state.busy}
                onRace={() => void act("race")}
              />
              <GrandLivePanel
                state={cs}
                choices={state.choices}
                busy={state.busy}
                onChoose={(id) => void act(id)}
              />
            </div>
            <ChoicePanel
              choices={visibleChoices}
              state={cs}
              busy={state.busy}
              onChoose={(id) => void act(id)}
            />
          </div>
          <EventDialog
            state={cs}
            busy={state.busy}
            onChoose={(id) => void act(id)}
          />
        </>
      )}
        </>
      )}

      {state.busy && tab === "run" && (
        <div className="busy-overlay">
          <div className="card">Working…</div>
        </div>
      )}
      {state.toast && <div className="toast">{state.toast}</div>}
    </div>
  );
}
