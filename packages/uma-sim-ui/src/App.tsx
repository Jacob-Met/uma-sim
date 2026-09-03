import { useEffect } from "react";
import { RunSetup } from "./components/RunSetup";
import { StatsPanel } from "./components/StatsPanel";
import { LogPanel } from "./components/LogPanel";
import { ChoicePanel } from "./components/ChoicePanel";
import { useRunStore } from "./state/runStore";

export default function App() {
  const {
    state,
    bootstrap,
    startRun,
    act,
    newRun,
    clearError,
  } = useRunStore();

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (state.busy || !state.snapshot) return;
      const n = Number(e.key);
      if (n >= 1 && n <= 9) {
        const choice = state.choices[n - 1];
        if (choice) void act(choice.id);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [act, state.busy, state.choices, state.snapshot]);

  const cs = state.snapshot?.state;

  return (
    <div className="app">
      <header className="header">
        <div>
          <h1>uma-sim</h1>
          <div className="sub">Interactive career simulator</div>
        </div>
        {cs && (
          <button disabled={state.busy} onClick={newRun}>
            New run
          </button>
        )}
      </header>

      {state.health && !state.health.repoRoot && (
        <div className="banner warn">
          Research / knowledge root not detected — engine is using the builtin
          fallback catalog. Run `uma-sim serve` from the repo (or a release zip
          that includes `research/` and `knowledge/`).
        </div>
      )}

      {state.error && (
        <div className="banner error" onClick={clearError}>
          {state.error} (click to dismiss)
        </div>
      )}

      {!cs && (
        <RunSetup
          scenarios={state.catalogs.scenarios}
          trainees={state.catalogs.trainees}
          supports={state.catalogs.supports}
          factors={state.catalogs.factors}
          busy={state.busy}
          onStart={(req) => void startRun(req)}
        />
      )}

      {cs && (
        <div className="turn-layout">
          <StatsPanel state={cs} />
          <LogPanel lines={state.textLines} />
          <ChoicePanel
            choices={state.choices}
            state={cs}
            busy={state.busy}
            onChoose={(id) => void act(id)}
          />
        </div>
      )}

      {state.busy && (
        <div className="busy-overlay">
          <div className="card">Working…</div>
        </div>
      )}
    </div>
  );
}
