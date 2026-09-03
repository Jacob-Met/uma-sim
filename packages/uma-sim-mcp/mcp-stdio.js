#!/usr/bin/env node
/**
 * Minimal MCP stdio server wrapping uma-sim REST API.
 * Requires: cargo run --bin uma-sim-api (or uma-sim serve) on UMA_SIM_API (default :8765)
 */
const API = process.env.UMA_SIM_API ?? "http://127.0.0.1:8765";

const RESOURCES = [
  { uri: "uma-sim://run/state", name: "Current run state", description: "Full career snapshot JSON", mimeType: "application/json" },
  { uri: "uma-sim://run/text", name: "Event log text", description: "Rendered career text", mimeType: "text/plain" },
  { uri: "uma-sim://run/telemetry", name: "Turn telemetry", description: "Telemetry JSON array", mimeType: "application/json" },
];

const TOOLS = [
  { name: "sim_start", description: "Start career run", inputSchema: { type: "object", properties: { seed: { type: "number" }, scenario: { type: "string" }, trainee: { type: "string" }, speed: { type: "number" }, deckSupports: { type: "string" }, legacyFactors: { type: "string" } } } },
  { name: "sim_state", description: "Get run state JSON", inputSchema: { type: "object", properties: {} } },
  { name: "sim_text", description: "Get rendered text", inputSchema: { type: "object", properties: {} } },
  { name: "sim_choices", description: "List available actions", inputSchema: { type: "object", properties: {} } },
  { name: "sim_act", description: "Perform action", inputSchema: { type: "object", properties: { action: { type: "string" } }, required: ["action"] } },
  { name: "sim_auto", description: "One bot-policy step", inputSchema: { type: "object", properties: { policy: { type: "string" } } } },
  { name: "sim_fast_forward", description: "Play to completion", inputSchema: { type: "object", properties: { multiplier: { type: "number" } } } },
  { name: "sim_export_telemetry", description: "Export turn telemetry JSON", inputSchema: { type: "object", properties: {} } },
  { name: "sim_load_content_pack", description: "Load events from content_packs/*.json", inputSchema: { type: "object", properties: { path: { type: "string" } }, required: ["path"] } },
  { name: "sim_deck_place", description: "Reposition support card onto facility", inputSchema: { type: "object", properties: { supportId: { type: "string" }, facility: { type: "string" } }, required: ["supportId", "facility"] } },
];

async function api(method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  return res.json();
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
      throw new Error(`Unknown resource: ${uri}`);
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
      });
    case "sim_state":
      return api("GET", "/v1/run/state");
    case "sim_text":
      return api("GET", "/v1/run/text");
    case "sim_choices":
      return api("GET", "/v1/run/choices");
    case "sim_act":
      return api("POST", "/v1/run/action", { action: args.action });
    case "sim_auto":
      return api("POST", "/v1/run/auto", { policy: args.policy ?? "bot" });
    case "sim_fast_forward":
      return api("POST", "/v1/run/fast", { multiplier: String(args.multiplier ?? 100) });
    case "sim_export_telemetry":
      return api("GET", "/v1/run/telemetry");
    case "sim_load_content_pack":
      return api("POST", "/v1/run/load_content_pack", { path: args.path });
    case "sim_deck_place":
      return api("POST", "/v1/run/deck/place", {
        supportId: args.supportId,
        facility: args.facility,
      });
    default:
      throw new Error(`Unknown tool: ${name}`);
  }
}

function send(msg) {
  const body = JSON.stringify(msg);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
}

let buffer = Buffer.alloc(0);

process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) break;
    const header = buffer.slice(0, headerEnd).toString("utf8");
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) break;
    const len = parseInt(match[1], 10);
    const start = headerEnd + 4;
    if (buffer.length < start + len) break;
    const body = buffer.slice(start, start + len).toString("utf8");
    buffer = buffer.slice(start + len);
    handle(JSON.parse(body)).catch((e) => {
      send({ jsonrpc: "2.0", id: null, error: { code: -32603, message: e.message } });
    });
  }
});

async function handle(req) {
  const { id, method, params } = req;
  if (method === "initialize") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: "2024-11-05",
        capabilities: { tools: {}, resources: {} },
        serverInfo: { name: "uma-sim-mcp", version: "0.3.0" },
      },
    });
    return;
  }
  if (method === "resources/list") {
    send({ jsonrpc: "2.0", id, result: { resources: RESOURCES } });
    return;
  }
  if (method === "resources/read") {
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
    const result = await callTool(params.name, params.arguments ?? {});
    send({
      jsonrpc: "2.0",
      id,
      result: { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] },
    });
    return;
  }
  send({ jsonrpc: "2.0", id, error: { code: -32601, message: `Method not found: ${method}` } });
}
