#!/usr/bin/env node
/**
 * Minimal MCP stdio server wrapping uma-sim REST API.
 * Requires: cargo run --bin uma-sim-api (or uma-sim serve) on UMA_SIM_API (default :8765)
 */
const API = process.env.UMA_SIM_API ?? "http://127.0.0.1:8765";

// Single source of truth for the version: package.json. serverInfo must not
// hardcode a version that drifts from the package (it claimed 0.3.0 while
// package.json said 0.1.0).
import { readFileSync } from "node:fs";
let PKG_VERSION = "0.0.0";
try {
  const rawPkg = readFileSync(new URL("./package.json", import.meta.url), "utf8").replace(/^\uFEFF/, "");
  const pkg = JSON.parse(rawPkg);
  if (pkg && typeof pkg.version === "string" && pkg.version) PKG_VERSION = pkg.version;
} catch {
  /* keep default; server still starts without package.json */
}

// MCP protocol versions this server speaks over the legacy `initialize`
// handshake. (2026-07-28 moved to a stateless server/discover handshake and
// is intentionally out of scope for this stdio server.)
const SUPPORTED_PROTOCOL_VERSIONS = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];
const LATEST_PROTOCOL_VERSION = SUPPORTED_PROTOCOL_VERSIONS[SUPPORTED_PROTOCOL_VERSIONS.length - 1];

function negotiateProtocolVersion(requested) {
  // Echo the client's version when we speak it; otherwise answer the newest
  // version we support and let the client decide whether to proceed.
  if (SUPPORTED_PROTOCOL_VERSIONS.includes(requested)) return requested;
  return LATEST_PROTOCOL_VERSION;
}

// JSON-RPC -32602 is for client mistakes (unknown tool/resource, bad args);
// -32603 stays reserved for genuine internal/backend failures.
class InvalidParamsError extends Error {}

// HTTP failures are tool execution failures. Keep them separate from invalid
// MCP parameters and transport/decoding failures in the bridge itself.
class BackendHttpError extends Error {
  constructor(status, body) {
    super(`uma-sim API returned HTTP ${status}${body ? `: ${body}` : ""}`);
  }
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

const RESOURCES = [
  { uri: "uma-sim://run/state", name: "Current run state", description: "Full career snapshot JSON", mimeType: "application/json" },
  { uri: "uma-sim://run/text", name: "Event log text", description: "Rendered career text", mimeType: "text/plain" },
  { uri: "uma-sim://run/telemetry", name: "Turn telemetry", description: "Telemetry JSON array", mimeType: "application/json" },
];

const STRING = { type: "string" };
const NONEMPTY = { type: "string", minLength: 1 };
const POLICY = { type: "string", enum: ["bot", "default", "external"] };
const NAMED_SESSION = {
  ...NONEMPTY,
  description: "Nonempty named career id. Omit to use the server's active session.",
};

function tool(name, description, properties = {}, required = []) {
  return {
    name,
    description,
    inputSchema: { type: "object", properties, required, additionalProperties: false },
  };
}

const TOOLS = [
  tool("sim_start", "Start or replace a career and make it active. Omitted session starts the default main career.", {
    seed: { type: "integer", minimum: Number.MIN_SAFE_INTEGER, maximum: Number.MAX_SAFE_INTEGER },
    scenario: STRING, trainee: STRING,
    speed: { type: "integer", minimum: 1, maximum: 100 },
    deckSupports: STRING, legacyFactors: STRING, session: STRING, label: STRING,
    policy: POLICY, raceModel: { type: "string", enum: ["physics", "stub"] },
  }),
  tool("sim_state", "Get run state JSON", { session: NAMED_SESSION }),
  tool("sim_text", "Get rendered text", { session: NAMED_SESSION }),
  tool("sim_choices", "List available actions", { session: NAMED_SESSION }),
  tool("sim_act", "Perform an action in the target career", { action: NONEMPTY, session: NAMED_SESSION }, ["action"]),
  tool("sim_auto", "One policy step in the target career", { policy: POLICY, session: NAMED_SESSION }),
  tool("sim_fast_forward", "Play the target career to completion", {
    multiplier: { type: "integer", minimum: 1, maximum: 100 }, policy: POLICY, session: NAMED_SESSION,
  }),
  tool("sim_export_telemetry", "Export target career turn telemetry JSON", { session: NAMED_SESSION }),
  tool("sim_load_content_pack", "Load events from content_packs/*.json into the shared catalog for every session", { path: NONEMPTY }, ["path"]),
  tool("sim_deck_place", "Reposition a support card onto a facility in the target career", {
    supportId: NONEMPTY, facility: NONEMPTY, session: NAMED_SESSION,
  }, ["supportId", "facility"]),
  tool("sim_sessions", "List live careers and the server's active session id"),
  tool("sim_session_fork", "Fork an independent live career and make it active. Use a checkpoint or a source session, never both; omit both for the active career. Source session may be empty for main.", {
    checkpoint: NONEMPTY, session: STRING, id: NONEMPTY, label: STRING,
  }),
  tool("sim_session_activate", "Make an existing live career active; an empty session selects main", { session: STRING }, ["session"]),
  tool("sim_session_close", "Discard a named live career. Save a checkpoint first to retain its progress.", { session: NONEMPTY }, ["session"]),
  tool("sim_library_list", "List durable named career checkpoints and their compatibility fingerprints"),
  tool("sim_library_save", "Save a durable checkpoint of the target career. Existing names are preserved unless overwrite is explicitly true.", {
    name: NONEMPTY, label: STRING, note: STRING, overwrite: { type: "boolean" }, session: NAMED_SESSION,
  }),
  tool("sim_library_load", "Restore a checkpoint into a target career and make it active, replacing that career's current progress", {
    name: NONEMPTY, session: NAMED_SESSION, label: STRING,
  }, ["name"]),
  tool("sim_library_delete", "Delete a durable checkpoint by name", { name: NONEMPTY }, ["name"]),
  tool("sim_library_import", "Validate and import an exported snapshot object as a durable checkpoint. Existing names are preserved unless overwrite is explicitly true.", {
    snapshot: { type: "object" }, name: NONEMPTY, overwrite: { type: "boolean" },
  }, ["snapshot"]),
  tool("sim_library_export", "Read a checkpoint's raw snapshot JSON for a portable backup or later import", { name: NONEMPTY }, ["name"]),
  tool("sim_lab_branch", "Run an independent experiment from a checkpoint or session:<id> and retain its decisions and outcome. Source careers are unchanged. Different actions can diverge the RNG stream; one comparison does not establish policy superiority.", {
    checkpoint: NONEMPTY, name: NONEMPTY,
    policy: { type: "string", enum: ["bot", "default"] },
    maxActions: { type: "integer", minimum: 1, maximum: 500 },
    overrides: {
      type: "array",
      items: {
        type: "object",
        properties: {
          turn: { type: "integer", minimum: -2147483648, maximum: 2147483647 },
          actionId: NONEMPTY,
        },
        required: ["turn", "actionId"],
        additionalProperties: false,
      },
    },
  }, ["checkpoint"]),
  tool("sim_lab_branches", "List retained branch experiments and outcome summaries"),
  tool("sim_lab_branch_get", "Read a retained branch's complete decision and outcome trace", { id: NONEMPTY }, ["id"]),
  tool("sim_lab_branch_delete", "Delete one retained branch experiment by id", { id: NONEMPTY }, ["id"]),
  tool("sim_lab_compare", "Compare two retained branch ids, including first divergence and outcome differences", { a: NONEMPTY, b: NONEMPTY }, ["a", "b"]),
  tool("sim_lab_report", "Read a complete branch comparison report as Markdown (default) or JSON", {
    a: NONEMPTY, b: NONEMPTY, format: { type: "string", enum: ["markdown", "json"] },
  }, ["a", "b"]),
];

// Validate the schema features used above before dispatch. In particular,
// REST intentionally coerces some fields and filters malformed overrides;
// an MCP caller must not silently run a different experiment after a typo.
function validate(value, schema, path = "arguments") {
  const type = schema.type;
  const validType = type === "object" ? value !== null && typeof value === "object" && !Array.isArray(value)
    : type === "array" ? Array.isArray(value)
    : type === "integer" ? Number.isSafeInteger(value)
    : typeof value === type;
  if (!validType) throw new InvalidParamsError(`${path} must be ${type}`);
  if (schema.enum && !schema.enum.includes(value)) {
    throw new InvalidParamsError(`${path} must be one of: ${schema.enum.join(", ")}`);
  }
  if (schema.minLength !== undefined && value.length < schema.minLength) {
    throw new InvalidParamsError(`${path} must not be empty`);
  }
  if ((schema.minimum !== undefined && value < schema.minimum) || (schema.maximum !== undefined && value > schema.maximum)) {
    throw new InvalidParamsError(`${path} must be between ${schema.minimum} and ${schema.maximum}`);
  }
  if (type === "object") {
    for (const key of schema.required ?? []) {
      if (!Object.hasOwn(value, key)) throw new InvalidParamsError(`Missing required argument: ${path}.${key}`);
    }
    for (const [key, child] of Object.entries(value)) {
      if (Object.hasOwn(schema.properties ?? {}, key)) {
        validate(child, schema.properties[key], `${path}.${key}`);
      } else if (schema.additionalProperties === false) {
        throw new InvalidParamsError(`Unknown argument: ${path}.${key}`);
      }
    }
  } else if (type === "array") {
    value.forEach((item, index) => validate(item, schema.items, `${path}[${index}]`));
  }
}

function queryPath(path, params) {
  const query = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) query.set(key, value);
  }
  return query.size ? `${path}?${query}` : path;
}

async function api(method, path, body, format = "json") {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new BackendHttpError(res.status, await res.text());
  return format === "text" ? res.text() : res.json();
}

async function readResource(uri) {
  switch (uri) {
    case "uma-sim://run/state":
      return api("GET", "/v1/run/state");
    case "uma-sim://run/text":
      return api("GET", "/v1/run/text");
    case "uma-sim://run/telemetry":
      return api("GET", "/v1/run/telemetry");
    default:
      throw new InvalidParamsError(`Unknown resource: ${uri}`);
  }
}

async function callTool(name, args) {
  switch (name) {
    case "sim_start":
      return api("POST", "/v1/run/start", {
        seed: String(args.seed ?? 42),
        scenario: args.scenario ?? "ura",
        trainee: args.trainee ?? "Special Week",
        speed: String(args.speed ?? 1),
        deckSupports: args.deckSupports ?? "",
        legacyFactors: args.legacyFactors ?? "",
        traceTelemetry: "true",
        session: args.session,
        label: args.label,
        policy: args.policy,
        raceModel: args.raceModel,
      });
    case "sim_state":
      return api("GET", queryPath("/v1/run/state", { session: args.session }));
    case "sim_text":
      return api("GET", queryPath("/v1/run/text", { session: args.session }));
    case "sim_choices":
      return api("GET", queryPath("/v1/run/choices", { session: args.session }));
    case "sim_act":
      return api("POST", "/v1/run/action", { action: args.action, session: args.session });
    case "sim_auto":
      return api("POST", "/v1/run/auto", { policy: args.policy ?? "bot", session: args.session });
    case "sim_fast_forward":
      return api("POST", "/v1/run/fast", { multiplier: String(args.multiplier ?? 100), policy: args.policy, session: args.session });
    case "sim_export_telemetry":
      return api("GET", queryPath("/v1/run/telemetry", { session: args.session }));
    case "sim_load_content_pack":
      return api("POST", "/v1/run/load_content_pack", { path: args.path });
    case "sim_deck_place":
      return api("POST", "/v1/run/deck/place", {
        supportId: args.supportId,
        facility: args.facility,
        session: args.session,
      });
    case "sim_sessions":
      return api("GET", "/v1/sessions");
    case "sim_session_fork":
      if (args.checkpoint !== undefined && args.session !== undefined) {
        throw new InvalidParamsError("Choose a checkpoint or source session, not both");
      }
      return api("POST", "/v1/session/fork", args);
    case "sim_session_activate":
      return api("POST", "/v1/session/activate", args);
    case "sim_session_close":
      return api("POST", "/v1/session/close", args);
    case "sim_library_list":
      return api("GET", "/v1/library");
    case "sim_library_save":
      return api("POST", "/v1/library/save", args);
    case "sim_library_load":
      return api("POST", "/v1/library/load", args);
    case "sim_library_delete":
      return api("POST", "/v1/library/delete", args);
    case "sim_library_import":
      return api("POST", "/v1/library/import", args);
    case "sim_library_export":
      return api("GET", queryPath("/v1/library/export", args));
    case "sim_lab_branch":
      return api("POST", "/v1/lab/branch", args);
    case "sim_lab_branches":
      return api("GET", "/v1/lab/branches");
    case "sim_lab_branch_get":
      return api("GET", queryPath("/v1/lab/branch", args));
    case "sim_lab_branch_delete":
      return api("POST", "/v1/lab/branch/delete", args);
    case "sim_lab_compare":
      return api("POST", "/v1/lab/compare", args);
    case "sim_lab_report": {
      const format = args.format ?? "markdown";
      return api("GET", queryPath("/v1/lab/report", { ...args, format }), undefined, format === "json" ? "json" : "text");
    }
    default:
      throw new InvalidParamsError(`Unknown tool: ${name}`);
  }
}

function send(msg) {
  // MCP stdio transport: newline-delimited JSON-RPC messages (no Content-Length headers)
  process.stdout.write(JSON.stringify(msg) + "\n");
}

let buffer = "";

process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buffer += chunk;
  let idx;
  while ((idx = buffer.indexOf("\n")) !== -1) {
    const line = buffer.slice(0, idx).replace(/\r$/, "");
    buffer = buffer.slice(idx + 1);
    if (!line) continue;
    let req;
    try {
      req = JSON.parse(line);
    } catch (e) {
      send({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "Parse error" } });
      continue;
    }
    handle(req).catch((e) => {
      // -32602 for client mistakes, -32603 for genuine internal/backend failures.
      const code = e instanceof InvalidParamsError ? -32602 : -32603;
      send({ jsonrpc: "2.0", id: req && req.id !== undefined ? req.id : null, error: { code, message: e.message } });
    });
  }
});

async function handle(req) {
  const hasId = isObject(req) && Object.hasOwn(req, "id");
  if (!isObject(req) || req.jsonrpc !== "2.0" || typeof req.method !== "string" ||
      (hasId && typeof req.id !== "string" && !Number.isSafeInteger(req.id))) {
    send({ jsonrpc: "2.0", id: null, error: { code: -32600, message: "Invalid Request" } });
    return;
  }
  const { id, method, params } = req;
  // Strings and safe integers preserve request identity after JSON parsing.
  // Use a string for larger identifiers; only an omitted ID is a notification.
  if (!hasId) return;
  if (params !== undefined && !isObject(params)) {
    throw new InvalidParamsError("params must be an object");
  }
  if (method === "initialize") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: negotiateProtocolVersion(params?.protocolVersion),
        capabilities: { tools: {}, resources: {} },
        serverInfo: { name: "uma-sim-mcp", version: PKG_VERSION },
      },
    });
    return;
  }
  if (method === "resources/list") {
    send({ jsonrpc: "2.0", id, result: { resources: RESOURCES } });
    return;
  }
  if (method === "resources/read") {
    if (typeof params?.uri !== "string") {
      throw new InvalidParamsError("resources/read requires params.uri");
    }
    const data = await readResource(params.uri);
    send({
      jsonrpc: "2.0",
      id,
      result: {
        contents: [{
          uri: params.uri,
          mimeType: params.uri.endsWith("/text") ? "text/plain" : "application/json",
          text: typeof data === "string" ? data : JSON.stringify(data, null, 2),
        }],
      },
    });
    return;
  }
  if (method === "tools/list") {
    send({ jsonrpc: "2.0", id, result: { tools: TOOLS } });
    return;
  }
  if (method === "tools/call") {
    const tool = TOOLS.find((t) => t.name === params?.name);
    if (!tool) throw new InvalidParamsError(`Unknown tool: ${params?.name}`);
    const args = params.arguments === undefined ? {} : params.arguments;
    validate(args, tool.inputSchema);
    let result;
    try {
      result = await callTool(params.name, args);
    } catch (error) {
      if (!(error instanceof BackendHttpError)) throw error;
      send({
        jsonrpc: "2.0",
        id,
        result: { content: [{ type: "text", text: error.message }], isError: true },
      });
      return;
    }
    send({
      jsonrpc: "2.0",
      id,
      result: { content: [{ type: "text", text: params.name === "sim_lab_report" && args.format !== "json" ? result : JSON.stringify(result, null, 2) }] },
    });
    return;
  }
  if (method === "ping") {
    send({ jsonrpc: "2.0", id, result: {} });
    return;
  }
  send({ jsonrpc: "2.0", id, error: { code: -32601, message: `Method not found: ${method}` } });
}
