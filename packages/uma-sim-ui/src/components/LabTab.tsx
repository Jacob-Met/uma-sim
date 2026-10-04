import { useEffect } from "react";
import { ComparePanel } from "./ComparePanel";
import { LibraryPanel } from "./LibraryPanel";
import { SessionPanel } from "./SessionPanel";
import { useLabStore } from "../state/labStore";

/** Career-lab tab: sessions, the named checkpoint library, branch & compare. */
export function LabTab({ refreshRun }: { refreshRun: () => Promise<void> }) {
  const lab = useLabStore(refreshRun);

  useEffect(() => {
    void lab.reloadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const { state, clearError, clearNotice } = lab;

  return (
    <div>
      {state.error && (
        <div className="banner error">
          {state.error}{" "}
          <button onClick={clearError} style={{ marginLeft: 8 }}>
            Dismiss
          </button>
        </div>
      )}
      {state.notice && (
        <div className="banner ok">
          {state.notice}{" "}
          <button onClick={clearNotice} style={{ marginLeft: 8 }}>
            Dismiss
          </button>
        </div>
      )}
      <SessionPanel lab={lab} />
      <LibraryPanel lab={lab} />
      <ComparePanel lab={lab} />
      {state.busy && (
        <div className="busy-overlay">
          <div className="busy-box">Working…</div>
        </div>
      )}
    </div>
  );
}
