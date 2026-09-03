import type {
  CatalogItem,
  Choice,
  HealthResponse,
  RunSnapshot,
  StartRequest,
  StepResponse,
} from "./types";

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let data: unknown = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    throw new Error(`${method} ${path}: invalid JSON (${res.status})`);
  }
  if (!res.ok) {
    const err =
      data && typeof data === "object" && "error" in data
        ? String((data as { error: unknown }).error)
        : res.statusText;
    throw new Error(`${method} ${path}: ${err}`);
  }
  return data as T;
}

export const api = {
  health: () => req<HealthResponse>("GET", "/v1/health"),
  catalogScenarios: () =>
    req<{ items: CatalogItem[] }>("GET", "/v1/catalog/scenarios").then((r) => r.items),
  catalogTrainees: () =>
    req<{ items: CatalogItem[] }>("GET", "/v1/catalog/trainees").then((r) => r.items),
  catalogSupports: () =>
    req<{ items: CatalogItem[] }>("GET", "/v1/catalog/supports").then((r) => r.items),
  catalogFactors: () =>
    req<{ items: CatalogItem[] }>("GET", "/v1/catalog/factors").then((r) => r.items),
  start: (body: StartRequest) => req<RunSnapshot>("POST", "/v1/run/start", body),
  state: () => req<RunSnapshot>("GET", "/v1/run/state"),
  text: () => req<{ text: string }>("GET", "/v1/run/text").then((r) => r.text),
  choices: () =>
    req<{ choices: Choice[] }>("GET", "/v1/run/choices").then((r) => r.choices),
  action: (action: string) =>
    req<StepResponse>("POST", "/v1/run/action", { action }),
  auto: (policy = "bot") =>
    req<StepResponse>("POST", "/v1/run/auto", { policy }),
  fast: (multiplier: number, policy = "bot") =>
    req<{ careerEnded: boolean; turn: number; fans: number }>(
      "POST",
      "/v1/run/fast",
      { multiplier, policy },
    ),
  telemetry: () => req<unknown>("GET", "/v1/run/telemetry"),
  deckPlace: (supportId: string, facility: string) =>
    req<RunSnapshot>("POST", "/v1/run/deck/place", { supportId, facility }),
};
