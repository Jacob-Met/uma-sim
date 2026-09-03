import { useCallback, useReducer } from "react";
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
      snapshot: RunSnapshot;
      choices?: Choice[];
      text?: string;
    }
  | { type: "appendText"; text: string }
  | { type: "reset" };

const initial: RunUiState = {
  health: null,
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

  const withBusy = useCallback(async <T,>(fn: () => Promise<T>): Promise<T | null> => {
    dispatch({ type: "setBusy", busy: true });
    dispatch({ type: "setError", error: null });
    try {
      return await fn();
    } catch (e) {
      dispatch({
        type: "setError",
        error: e instanceof Error ? e.message : String(e),
      });
      return null;
    } finally {
      dispatch({ type: "setBusy", busy: false });
    }
  }, []);

  const bootstrap = useCallback(async () => {
    await withBusy(async () => {
      const [health, scenarios, trainees, supports, factors] = await Promise.all([
        api.health(),
        api.catalogScenarios(),
        api.catalogTrainees(),
        api.catalogSupports(),
        api.catalogFactors(),
      ]);
      dispatch({ type: "setHealth", health });
      dispatch({
        type: "setCatalogs",
        catalogs: { scenarios, trainees, supports, factors },
      });
    });
  }, [withBusy]);

  const startRun = useCallback(
    async (req: StartRequest) => {
      await withBusy(async () => {
        const snapshot = await api.start({
          ...req,
          traceTelemetry: true,
        });
        const [choices, text] = await Promise.all([api.choices(), api.text()]);
        dispatch({
          type: "applySnapshot",
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
      await withBusy(async () => {
        const step = await api.action(actionId);
        const text = await api.text();
        dispatch({
          type: "applySnapshot",
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
      await withBusy(async () => {
        const step = await api.auto(policy);
        const text = await api.text();
        dispatch({
          type: "applySnapshot",
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
      await withBusy(async () => {
        await api.fast(multiplier, policy);
        const [snapshot, choices, text] = await Promise.all([
          api.state(),
          api.choices(),
          api.text(),
        ]);
        dispatch({ type: "applySnapshot", snapshot, choices, text });
      });
    },
    [withBusy],
  );

  const placeDeck = useCallback(
    async (supportId: string, facility: string) => {
      await withBusy(async () => {
        const snapshot = await api.deckPlace(supportId, facility);
        const choices = await api.choices();
        dispatch({ type: "applySnapshot", snapshot, choices });
        dispatch({ type: "setToast", toast: `Placed ${supportId} on ${facility}` });
      });
    },
    [withBusy],
  );

  const setStyle = useCallback(
    async (style: string) => {
      await withBusy(async () => {
        const snapshot = await api.setStyle(style);
        dispatch({ type: "applySnapshot", snapshot });
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
    dispatch({ type: "reset" });
  }, []);

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
    clearError,
    clearToast,
  };
}
