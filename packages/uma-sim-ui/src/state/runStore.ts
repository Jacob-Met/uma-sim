import { useCallback, useEffect, useReducer, useRef } from "react";
import { api } from "../api/client";
import type {
  CatalogItem,
  Choice,
  HealthResponse,
  RunSnapshot,
  StartRequest,
} from "../api/types";

export interface RunUiState {
  health: HealthResponse | null;
  /** null means no displayed run; the empty string is the main session. */
  sessionId: string | null;
  snapshot: RunSnapshot | null;
  choices: Choice[];
  textLines: string[];
  busy: boolean;
  error: string | null;
  toast: string | null;
  catalogs: {
    scenarios: CatalogItem[];
    trainees: CatalogItem[];
    supports: CatalogItem[];
    factors: CatalogItem[];
  };
}

type Action =
  | { type: "setHealth"; health: HealthResponse }
  | { type: "setCatalogs"; catalogs: RunUiState["catalogs"] }
  | { type: "setBusy"; busy: boolean }
  | { type: "setError"; error: string | null }
  | { type: "setToast"; toast: string | null }
  | {
      type: "applySnapshot";
      sessionId: string;
      snapshot: RunSnapshot;
      choices?: Choice[];
      text?: string;
    }
  | { type: "appendText"; text: string }
  | { type: "clearRun" }
  | { type: "reset" };

const initial: RunUiState = {
  health: null,
  sessionId: null,
  snapshot: null,
  choices: [],
  textLines: [],
  busy: false,
  error: null,
  toast: null,
  catalogs: { scenarios: [], trainees: [], supports: [], factors: [] },
};

function reducer(state: RunUiState, action: Action): RunUiState {
  switch (action.type) {
    case "setHealth":
      return { ...state, health: action.health };
    case "setCatalogs":
      return { ...state, catalogs: action.catalogs };
    case "setBusy":
      return { ...state, busy: action.busy };
    case "setError":
      return { ...state, error: action.error };
    case "setToast":
      return { ...state, toast: action.toast };
    case "applySnapshot": {
      const lines = action.text
        ? action.text.split("\n").filter(Boolean)
        : state.textLines;
      return {
        ...state,
        sessionId: action.sessionId,
        snapshot: action.snapshot,
        choices: action.choices ?? state.choices,
        textLines: lines,
        error: null,
      };
    }
    case "appendText": {
      const extra = action.text.split("\n").filter(Boolean);
      return {
        ...state,
        textLines: [...state.textLines, ...extra].slice(-400),
      };
    }
    case "clearRun":
      return { ...state, sessionId: null, snapshot: null, choices: [], textLines: [], toast: null };
    case "reset":
      return {
        ...initial,
        health: state.health,
        catalogs: state.catalogs,
      };
    default:
      return state;
  }
}

export function useRunStore() {
  const [state, dispatch] = useReducer(reducer, initial);
  const session = useRef<string | null>(null);
  const requestId = useRef(0);
  const pending = useRef(false);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      requestId.current++;
      pending.current = false;
      session.current = null;
    };
  }, []);

  // An operation owns its success, failure and busy cleanup. Selection/reset
  // invalidates that ownership; it does not cancel an already submitted action.
  // The ref also closes the interval before React paints disabled controls.
  const withBusy = useCallback(async <T,>(
    fn: (isCurrent: () => boolean) => Promise<T>,
    replace = false,
  ): Promise<T | null> => {
    if (!mounted.current || (pending.current && !replace)) return null;
    const id = ++requestId.current;
    pending.current = true;
    const isCurrent = () => mounted.current && id === requestId.current;
    dispatch({ type: "setBusy", busy: true });
    dispatch({ type: "setError", error: null });
    try {
      return await fn(isCurrent);
    } catch (e) {
      if (isCurrent()) {
        dispatch({
          type: "setError",
          error: e instanceof Error ? e.message : String(e),
        });
      }
      return null;
    } finally {
      if (isCurrent()) {
        pending.current = false;
        dispatch({ type: "setBusy", busy: false });
      }
    }
  }, []);

  const bootstrap = useCallback(async () => {
    await withBusy(async (isCurrent) => {
      const [health, scenarios, trainees, supports, factors] = await Promise.all([
        api.health(),
        api.catalogScenarios(),
        api.catalogTrainees(),
        api.catalogSupports(),
        api.catalogFactors(),
      ]);
      if (!isCurrent()) return;
      dispatch({ type: "setHealth", health });
      dispatch({
        type: "setCatalogs",
        catalogs: { scenarios, trainees, supports, factors },
      });
    });
  }, [withBusy]);

  const startRun = useCallback(
    async (req: StartRequest) => {
      await withBusy(async (isCurrent) => {
        // The setup form starts the default session, as it always has. Read
        // that same session even if another client activates a fork meanwhile.
        const sessionId = "";
        session.current = null;
        dispatch({ type: "clearRun" });
        const snapshot = await api.start({
          ...req,
          traceTelemetry: true,
        });
        if (!isCurrent()) return;
        const [choices, text] = await Promise.all([api.choices(sessionId), api.text(sessionId)]);
        if (!isCurrent()) return;
        session.current = sessionId;
        dispatch({
          type: "applySnapshot",
          sessionId,
          snapshot,
          choices,
          text,
        });
      });
    },
    [withBusy],
  );

  const act = useCallback(
    async (actionId: string) => {
      const sessionId = session.current;
      if (sessionId === null) return;
      await withBusy(async (isCurrent) => {
        const step = await api.action(actionId, sessionId);
        if (!isCurrent()) return;
        const text = await api.text(sessionId);
        if (!isCurrent()) return;
        dispatch({
          type: "applySnapshot",
          sessionId,
          snapshot: step.state,
          choices: step.choices,
          text,
        });
      });
    },
    [withBusy],
  );

  const autoStep = useCallback(
    async (policy = "bot") => {
      const sessionId = session.current;
      if (sessionId === null) return;
      await withBusy(async (isCurrent) => {
        const step = await api.auto(policy, sessionId);
        if (!isCurrent()) return;
        const text = await api.text(sessionId);
        if (!isCurrent()) return;
        dispatch({
          type: "applySnapshot",
          sessionId,
          snapshot: step.state,
          choices: step.choices,
          text,
        });
      });
    },
    [withBusy],
  );

  const fastForward = useCallback(
    async (multiplier: number, policy = "bot") => {
      const sessionId = session.current;
      if (sessionId === null) return;
      await withBusy(async (isCurrent) => {
        await api.fast(multiplier, policy, sessionId);
        if (!isCurrent()) return;
        const [snapshot, choices, text] = await Promise.all([
          api.state(sessionId),
          api.choices(sessionId),
          api.text(sessionId),
        ]);
        if (!isCurrent()) return;
        dispatch({ type: "applySnapshot", sessionId, snapshot, choices, text });
      });
    },
    [withBusy],
  );

  const placeDeck = useCallback(
    async (supportId: string, facility: string) => {
      const sessionId = session.current;
      if (sessionId === null) return;
      await withBusy(async (isCurrent) => {
        const snapshot = await api.deckPlace(supportId, facility, sessionId);
        if (!isCurrent()) return;
        const choices = await api.choices(sessionId);
        if (!isCurrent()) return;
        dispatch({ type: "applySnapshot", sessionId, snapshot, choices });
        dispatch({ type: "setToast", toast: `Placed ${supportId} on ${facility}` });
      });
    },
    [withBusy],
  );

  const setStyle = useCallback(
    async (style: string) => {
      const sessionId = session.current;
      if (sessionId === null) return;
      await withBusy(async (isCurrent) => {
        const snapshot = await api.setStyle(style, sessionId);
        if (!isCurrent()) return;
        dispatch({ type: "applySnapshot", sessionId, snapshot });
        dispatch({
          type: "setToast",
          toast: style
            ? `Preferred style: ${style}`
            : "Preferred style: auto",
        });
      });
    },
    [withBusy],
  );

  const newRun = useCallback(() => {
    requestId.current++;
    pending.current = false;
    session.current = null;
    dispatch({ type: "reset" });
  }, []);

  /** Re-read the server's active session (after fork/activate/load). */
  const refreshActive = useCallback(async () => {
    await withBusy(async (isCurrent) => {
      session.current = null;
      dispatch({ type: "clearRun" });
      const { active: sessionId } = await api.sessions();
      if (!isCurrent()) return;
      const [snapshot, choices, text] = await Promise.all([
        api.state(sessionId),
        api.choices(sessionId),
        api.text(sessionId),
      ]);
      if (!isCurrent()) return;
      session.current = sessionId;
      dispatch({ type: "applySnapshot", sessionId, snapshot, choices, text });
    }, true);
  }, [withBusy]);

  const clearError = useCallback(() => {
    dispatch({ type: "setError", error: null });
  }, []);

  const clearToast = useCallback(() => {
    dispatch({ type: "setToast", toast: null });
  }, []);

  return {
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
  };
}
