import { useCallback, useState } from "react";
import { api } from "../api/client";
import type {
  ActionOverride,
  BranchComparison,
  LabBranchSummary,
  LibraryEntry,
  SessionInfo,
} from "../api/types";

export interface LabUiState {
  sessions: SessionInfo[];
  activeSession: string;
  library: LibraryEntry[];
  branches: LabBranchSummary[];
  comparison: BranchComparison | null;
  compareA: string;
  compareB: string;
  busy: boolean;
  error: string | null;
  notice: string | null;
}

/**
 * Career-lab state (T100): named checkpoint library, forkable sessions, and
 * branch-and-compare runs. `refreshRun` re-reads the server's active session
 * into the main run view after fork/activate/load operations.
 */
export function useLabStore(refreshRun: () => Promise<void>) {
  const [state, setState] = useState<LabUiState>({
    sessions: [],
    activeSession: "",
    library: [],
    branches: [],
    comparison: null,
    compareA: "",
    compareB: "",
    busy: false,
    error: null,
    notice: null,
  });

  const patch = useCallback(
    (p: Partial<LabUiState>) => setState((s) => ({ ...s, ...p })),
    [],
  );

  const withBusy = useCallback(
    async <T,>(fn: () => Promise<T>, notice?: string): Promise<T | null> => {
      patch({ busy: true, error: null, notice: null });
      try {
        const r = await fn();
        patch({ notice: notice ?? null });
        return r;
      } catch (e) {
        patch({ error: e instanceof Error ? e.message : String(e) });
        return null;
      } finally {
        patch({ busy: false });
      }
    },
    [patch],
  );

  const reloadAll = useCallback(async () => {
    await withBusy(async () => {
      const [sessions, library, branches] = await Promise.all([
        api.sessions(),
        api.library(),
        api.labBranches(),
      ]);
      patch({
        sessions: sessions.sessions,
        activeSession: sessions.active,
        library,
        branches,
      });
    });
  }, [patch, withBusy]);

  const saveCheckpoint = useCallback(
    async (name: string, label: string, note: string, overwrite: boolean) => {
      const entry = await withBusy(() =>
        api.librarySave({
          name: name.trim() || undefined,
          label: label.trim() || undefined,
          note: note.trim() || undefined,
          overwrite,
        }),
      );
      if (entry) {
        patch({ library: await api.library() });
        return entry;
      }
      return null;
    },
    [patch, withBusy],
  );

  const loadCheckpoint = useCallback(
    async (name: string, intoNewSession: boolean) => {
      const res = await withBusy(async () => {
        if (intoNewSession) {
          // Fork keeps the current live run untouched and activates the copy.
          const forked = await api.forkSession({ checkpoint: name });
          return { state: null as null, entry: null as null, compatAdvisories: forked.compatAdvisories };
        }
        const loaded = await api.libraryLoad({ name });
        return { state: loaded.state, entry: loaded.entry, compatAdvisories: loaded.compatAdvisories };
      });
      if (res) {
        await refreshRun();
        await reloadAll();
        if (res.compatAdvisories.length > 0) {
          patch({
            notice: `Loaded with advisories: ${res.compatAdvisories.join("; ")}`,
          });
        }
      }
    },
    [patch, refreshRun, reloadAll, withBusy],
  );

  const deleteCheckpoint = useCallback(
    async (name: string) => {
      const ok = await withBusy(() => api.libraryDelete(name), `Deleted checkpoint ${name}`);
      if (ok) patch({ library: await api.library() });
    },
    [patch, withBusy],
  );

  const importCheckpoint = useCallback(
    async (raw: string, name: string) => {
      let snapshot: unknown;
      try {
        snapshot = JSON.parse(raw);
      } catch {
        patch({ error: "Import: not valid JSON" });
        return;
      }
      const entry = await withBusy(() =>
        api.libraryImport({ snapshot, name: name.trim() || undefined }),
      );
      if (entry) patch({ library: await api.library() });
    },
    [patch, withBusy],
  );

  const forkSession = useCallback(
    async (source: { checkpoint?: string; session?: string }, id: string, label: string) => {
      const res = await withBusy(() =>
        api.forkSession({
          ...source,
          id: id.trim() || undefined,
          label: label.trim() || undefined,
        }),
      );
      if (res) {
        await refreshRun();
        await reloadAll();
        if (res.compatAdvisories.length > 0) {
          patch({ notice: `Forked with advisories: ${res.compatAdvisories.join("; ")}` });
        }
      }
    },
    [patch, refreshRun, reloadAll, withBusy],
  );

  const activateSession = useCallback(
    async (id: string) => {
      const ok = await withBusy(() => api.activateSession(id));
      if (ok) {
        await refreshRun();
        patch({ activeSession: id });
      }
    },
    [patch, refreshRun, withBusy],
  );

  const closeSession = useCallback(
    async (id: string) => {
      const ok = await withBusy(() => api.closeSession(id), `Closed session ${id || "main"}`);
      if (ok) {
        await refreshRun();
        await reloadAll();
      }
    },
    [refreshRun, reloadAll, withBusy],
  );

  const runBranch = useCallback(
    async (args: {
      checkpoint: string;
      name: string;
      policy: string;
      maxActions: number;
      overrides: ActionOverride[];
    }) => {
      const res = await withBusy(
        () => api.labBranch(args),
        `Branch ${args.name || args.checkpoint} finished`,
      );
      if (res) patch({ branches: await api.labBranches() });
      return res;
    },
    [patch, withBusy],
  );

  const deleteBranch = useCallback(
    async (id: string) => {
      const ok = await withBusy(() => api.labBranchDelete(id));
      if (ok) {
        setState((s) => ({
          ...s,
          comparison:
            s.comparison && (s.comparison.aId === id || s.comparison.bId === id)
              ? null
              : s.comparison,
          compareA: s.compareA === id ? "" : s.compareA,
          compareB: s.compareB === id ? "" : s.compareB,
        }));
        patch({ branches: await api.labBranches() });
      }
    },
    [patch, withBusy],
  );

  const compare = useCallback(
    async (a: string, b: string) => {
      if (!a || !b || a === b) {
        patch({ error: "Pick two different branches to compare." });
        return;
      }
      const c = await withBusy(() => api.labCompare(a, b));
      // A completed result must not replace a newer draft selection.
      if (c) patch({ comparison: c });
    },
    [patch, withBusy],
  );

  const clearError = useCallback(() => patch({ error: null }), [patch]);
  const clearNotice = useCallback(() => patch({ notice: null }), [patch]);

  return {
    state,
    reloadAll,
    saveCheckpoint,
    loadCheckpoint,
    deleteCheckpoint,
    importCheckpoint,
    forkSession,
    activateSession,
    closeSession,
    runBranch,
    deleteBranch,
    compare,
    clearError,
    clearNotice,
    setCompareSelection: (a: string, b: string) => patch({ compareA: a, compareB: b }),
  };
}
