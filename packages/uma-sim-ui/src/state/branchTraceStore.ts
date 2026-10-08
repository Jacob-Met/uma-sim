import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import type { LabBranchResult } from "../api/types";

export type BranchTraceState =
  | { status: "idle"; requestedId: null }
  | { status: "loading"; requestedId: string }
  | { status: "error"; requestedId: string; error: string }
  | { status: "ready"; requestedId: string; branch: LabBranchResult };

export interface BranchTraceController {
  state: BranchTraceState;
  open: (id: string) => Promise<void>;
  retry: () => Promise<void>;
  close: () => void;
}

/** Read one saved branch without changing the active career or A/B selection. */
export function useBranchTrace(): BranchTraceController {
  const [state, setState] = useState<BranchTraceState>({ status: "idle", requestedId: null });
  const request = useRef(0);
  const selectedId = useRef<string | null>(null);

  useEffect(() => () => {
    // Client reads cannot be aborted through the current API wrapper. Ignore
    // every completion after unmount, including rejected fetches.
    request.current += 1;
    selectedId.current = null;
  }, []);

  const close = useCallback(() => {
    request.current += 1;
    selectedId.current = null;
    setState({ status: "idle", requestedId: null });
  }, []);

  const open = useCallback(async (id: string) => {
    if (!id) {
      close();
      return;
    }
    const ticket = ++request.current;
    selectedId.current = id;
    setState({ status: "loading", requestedId: id });
    try {
      const branch = await api.labBranchGet(id);
      if (ticket !== request.current) return;
      if (!branch || branch.id !== id) {
        throw new Error("The returned trace did not match the requested branch.");
      }
      if (!Array.isArray(branch.timeline) || !Array.isArray(branch.config?.overrides)) {
        throw new Error("The returned trace is missing its recorded steps or configuration.");
      }
      setState({ status: "ready", requestedId: id, branch });
    } catch (error) {
      if (ticket !== request.current) return;
      setState({
        status: "error",
        requestedId: id,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }, [close]);

  const retry = useCallback(async () => {
    if (selectedId.current !== null) await open(selectedId.current);
  }, [open]);

  return { state, open, retry, close };
}
