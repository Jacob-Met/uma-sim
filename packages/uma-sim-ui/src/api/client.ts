import type {
  ActionOverride,
  BranchComparison,
  CatalogItem,
  Choice,
  HealthResponse,
  LabBranchResult,
  LabBranchSummary,
  LibraryEntry,
  RunSnapshot,
  SessionInfo,
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

/** Append ?session= for run endpoints when a non-default session is targeted. */
function withSession(path: string, session?: string): string {
  if (!session) return path;
  const sep = path.includes("?") ? "&" : "?";
  return `${path}${sep}session=${encodeURIComponent(session)}`;
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
  state: (session?: string) =>
    req<RunSnapshot>("GET", withSession("/v1/run/state", session)),
  text: (session?: string) =>
    req<{ text: string }>("GET", withSession("/v1/run/text", session)).then((r) => r.text),
  choices: (session?: string) =>
    req<{ choices: Choice[] }>("GET", withSession("/v1/run/choices", session)).then(
      (r) => r.choices,
    ),
  action: (action: string, session?: string) =>
    req<StepResponse>("POST", "/v1/run/action", { action, session }),
  auto: (policy = "bot", session?: string) =>
    req<StepResponse>("POST", "/v1/run/auto", { policy, session }),
  fast: (multiplier: number, policy = "bot", session?: string) =>
    req<{ careerEnded: boolean; turn: number; fans: number }>(
      "POST",
      "/v1/run/fast",
      { multiplier, policy, session },
    ),
  telemetry: (session?: string) =>
    req<unknown>("GET", withSession("/v1/run/telemetry", session)),
  deckPlace: (supportId: string, facility: string, session?: string) =>
    req<RunSnapshot>("POST", "/v1/run/deck/place", { supportId, facility, session }),
  setStyle: (style: string, session?: string) =>
    req<RunSnapshot>("POST", "/v1/run/style", { style, session }),

  /* Career lab (T100). */
  sessions: () =>
    req<{ sessions: SessionInfo[]; active: string }>("GET", "/v1/sessions"),
  forkSession: (body: {
    checkpoint?: string;
    session?: string;
    id?: string;
    label?: string;
  }) =>
    req<{ session: SessionInfo; compatAdvisories: string[] }>(
      "POST",
      "/v1/session/fork",
      body,
    ),
  closeSession: (session: string) =>
    req<{ closed: string; active: string }>("POST", "/v1/session/close", { session }),
  activateSession: (session: string) =>
    req<{ session: SessionInfo }>("POST", "/v1/session/activate", { session }),
  library: () =>
    req<{ entries: LibraryEntry[] }>("GET", "/v1/library").then((r) => r.entries),
  librarySave: (body: {
    name?: string;
    label?: string;
    note?: string;
    overwrite?: boolean;
    session?: string;
  }) =>
    req<{ entry: LibraryEntry }>("POST", "/v1/library/save", body).then((r) => r.entry),
  libraryLoad: (body: { name: string; session?: string; label?: string }) =>
    req<{ state: RunSnapshot; entry: LibraryEntry; compatAdvisories: string[] }>(
      "POST",
      "/v1/library/load",
      body,
    ),
  libraryDelete: (name: string) =>
    req<{ deleted: string }>("POST", "/v1/library/delete", { name }),
  libraryImport: (body: { snapshot: unknown; name?: string; overwrite?: boolean }) =>
    req<{ entry: LibraryEntry }>("POST", "/v1/library/import", body).then((r) => r.entry),
  labBranch: (body: {
    checkpoint: string;
    name?: string;
    policy?: string;
    maxActions?: number;
    overrides?: ActionOverride[];
  }) =>
    req<{ branch: LabBranchSummary; outcome: LabBranchResult["outcome"] }>(
      "POST",
      "/v1/lab/branch",
      body,
    ),
  labBranches: () =>
    req<{ branches: LabBranchSummary[] }>("GET", "/v1/lab/branches").then((r) => r.branches),
  labBranchGet: (id: string) =>
    req<{ branch: LabBranchResult }>(
      "GET",
      `/v1/lab/branch?id=${encodeURIComponent(id)}`,
    ).then((r) => r.branch),
  labBranchDelete: (id: string) =>
    req<{ deleted: string }>("POST", "/v1/lab/branch/delete", { id }),
  labCompare: (a: string, b: string) =>
    req<BranchComparison>("POST", "/v1/lab/compare", { a, b }),
  /** Download URL for the comparison report (markdown or json). */
  labReportUrl: (a: string, b: string, format: "markdown" | "json") =>
    `/v1/lab/report?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}&format=${format}`,
};
