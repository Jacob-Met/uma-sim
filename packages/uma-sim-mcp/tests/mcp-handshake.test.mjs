#!/usr/bin/env node
/**
 * Conformant MCP stdio test client for uma-sim-mcp/mcp-stdio.js.
 *
 * Speaks the spec-required framing: newline-delimited JSON-RPC messages, one
 * object per line (no Content-Length headers). Drives a full handshake:
 *   initialize -> notifications/initialized -> ping -> tools/list -> resources/list
 * then verifies error framing for a malformed line and for a tools/call whose
 * upstream REST API is unreachable (UMA_SIM_API points at a closed port, so the
 * server must answer -32603 rather than hang or crash).
 *
 * Run: node --test tests/mcp-handshake.test.mjs
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { fileURLToPath } from "node:url";
import path from "node:path";

const SERVER = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "mcp-stdio.js");

function startServer(apiUrl = "http://127.0.0.1:1") {
  const child = spawn("node", [SERVER], {
    stdio: ["pipe", "pipe", "pipe"],
    env: { ...process.env, UMA_SIM_API: apiUrl }, // closed port by default: tools fail cleanly
  });
  child.stdout.setEncoding("utf8");
  let text = "";
  let pending = [];
  child.stdout.on("data", (chunk) => {
    text += chunk;
    const parts = text.split("\n");
    text = parts.pop();
    for (const line of parts) {
      if (line.trim() === "") continue;
      const msg = JSON.parse(line);
      const waiter = pending.shift();
      assert(waiter, "unexpected server message");
      waiter(msg);
    }
  });
  return {
    send: (obj) => child.stdin.write(`${JSON.stringify(obj)}\n`),
    sendRaw: (s) => child.stdin.write(`${s}\n`),
    recv: () =>
      new Promise((resolve) => {
        const waiter = (msg) => resolve(msg);
        pending.push(waiter);
      }),
    recvTimeout: (ms) =>
      new Promise((resolve) => {
        const waiter = (msg) => {
          clearTimeout(timer);
          resolve(msg);
        };
        const timer = setTimeout(() => {
          const i = pending.indexOf(waiter);
          if (i !== -1) pending.splice(i, 1); // do not let a dead waiter eat the next reply
          resolve(null);
        }, ms);
        pending.push(waiter);
      }),
    stop: () => child.kill(),
  };
}

test("conformant handshake with newline-delimited framing", async () => {
  const srv = startServer();
  try {
    // 1. initialize — single JSON line request, single JSON line response.
    srv.send({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: { name: "uma-sim-mcp-test", version: "0.1.0" },
      },
    });
    const init = await srv.recv();
    assert.equal(init.jsonrpc, "2.0");
    assert.equal(init.id, 1);
    assert.equal(init.result.protocolVersion, "2024-11-05");
    assert.equal(init.result.serverInfo.name, "uma-sim-mcp");
    assert.deepEqual(Object.keys(init.result.capabilities).sort(), ["resources", "tools"]);

    // 2. notifications/initialized carries no id — server must stay silent.
    srv.send({ jsonrpc: "2.0", method: "notifications/initialized" });
    const surprise = await srv.recvTimeout(300);
    assert.equal(surprise, null, "server responded to a notification");

    // 3. ping.
    srv.send({ jsonrpc: "2.0", id: 2, method: "ping" });
    const pong = await srv.recv();
    assert.equal(pong.id, 2);
    assert.deepEqual(pong.result, {});

    // 4. tools/list and resources/list.
    srv.send({ jsonrpc: "2.0", id: 3, method: "tools/list" });
    const tools = await srv.recv();
    assert.equal(tools.id, 3);
    const names = tools.result.tools.map((t) => t.name).sort();
    assert.deepEqual(names, [
      "sim_act",
      "sim_auto",
      "sim_choices",
      "sim_deck_place",
      "sim_export_telemetry",
      "sim_fast_forward",
      "sim_lab_branch",
      "sim_lab_branch_delete",
      "sim_lab_branch_get",
      "sim_lab_branches",
      "sim_lab_compare",
      "sim_lab_report",
      "sim_library_delete",
      "sim_library_export",
      "sim_library_import",
      "sim_library_list",
      "sim_library_load",
      "sim_library_save",
      "sim_load_content_pack",
      "sim_session_activate",
      "sim_session_close",
      "sim_session_fork",
      "sim_sessions",
      "sim_start",
      "sim_state",
      "sim_text"
    ]);

    srv.send({ jsonrpc: "2.0", id: 4, method: "resources/list" });
    const resources = await srv.recv();
    assert.equal(resources.id, 4);
    assert.equal(resources.result.resources.length, 3);

    // 5. Malformed line -> -32700 parse error, framed as one JSON line.
    srv.sendRaw("{not json");
    const parseErr = await srv.recv();
    assert.equal(parseErr.error.code, -32700);

    // 6. tools/call with unreachable REST API -> -32603 internal error, still framed.
    srv.send({ jsonrpc: "2.0", id: 5, method: "tools/call", params: { name: "sim_state", arguments: {} } });
    const toolErr = await srv.recv();
    assert.equal(toolErr.id, 5);
    assert.equal(toolErr.error.code, -32603);

    // 7. Unknown method -> -32601.
    srv.send({ jsonrpc: "2.0", id: 6, method: "bogus/method" });
    const notFound = await srv.recv();
    assert.equal(notFound.error.code, -32601);
  } finally {
    srv.stop();
  }
});

test("protocol negotiation, -32602 error mapping, and argument validation", async () => {
  const srv = startServer();
  try {
    // 1. Client requests a version we speak -> echo it.
    srv.send({ jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-11-25", capabilities: {}, clientInfo: { name: "t", version: "0" } } });
    const v1 = await srv.recv();
    assert.equal(v1.result.protocolVersion, "2025-11-25");

    // 2. Client requests an ancient version we still speak -> echo it (backward compat).
    srv.send({ jsonrpc: "2.0", id: 2, method: "initialize", params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "t", version: "0" } } });
    const v2 = await srv.recv();
    assert.equal(v2.result.protocolVersion, "2024-11-05");

    // 3. Client requests a version we do not speak -> newest supported, not the stale default.
    srv.send({ jsonrpc: "2.0", id: 3, method: "initialize", params: { protocolVersion: "1999-01-01", capabilities: {}, clientInfo: { name: "t", version: "0" } } });
    const v3 = await srv.recv();
    assert.equal(v3.result.protocolVersion, "2025-11-25");

    // 4. Unknown tool -> -32602 (not -32603).
    srv.send({ jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "nope", arguments: {} } });
    const t = await srv.recv();
    assert.equal(t.error.code, -32602);

    // 5. Unknown resource -> -32602 (not -32603).
    srv.send({ jsonrpc: "2.0", id: 5, method: "resources/read", params: { uri: "uma-sim://bogus" } });
    const r = await srv.recv();
    assert.equal(r.error.code, -32602);

    // 6. Missing required argument -> -32602 before any backend fetch.
    srv.send({ jsonrpc: "2.0", id: 6, method: "tools/call", params: { name: "sim_act", arguments: {} } });
    const a = await srv.recv();
    assert.equal(a.error.code, -32602);
    assert.match(a.error.message, /action/);

    // 7. Missing params object entirely -> -32602, not a crash.
    srv.send({ jsonrpc: "2.0", id: 7, method: "resources/read" });
    const p = await srv.recv();
    assert.equal(p.error.code, -32602);

    // 8. Backend-unreachable tool error still maps to -32603 (genuine internal failure).
    srv.send({ jsonrpc: "2.0", id: 8, method: "tools/call", params: { name: "sim_state", arguments: {} } });
    const b = await srv.recv();
    assert.equal(b.error.code, -32603);
  } finally {
    srv.stop();
  }
});

async function startBackend(t, respond) {
  const requests = [];
  const backend = createServer(async (req, res) => {
    let body = "";
    for await (const chunk of req) body += chunk;
    requests.push({ method: req.method, path: req.url, body: body ? JSON.parse(body) : null });
    respond(req, res);
  });
  await new Promise((resolve, reject) => {
    backend.once("error", reject);
    backend.listen(0, "127.0.0.1", resolve);
  });
  t.after(() => new Promise((resolve) => {
    backend.closeAllConnections();
    backend.close(resolve);
  }));
  return { url: `http://127.0.0.1:${backend.address().port}`, requests };
}

async function request(srv, message) {
  srv.send(message);
  const response = await srv.recvTimeout(1500);
  assert.notEqual(response, null, "MCP process did not respond");
  assert.equal(response.jsonrpc, "2.0");
  return response;
}

test("HTTP failures are actionable tool errors, including non-JSON and empty bodies", { timeout: 10000 }, async (t) => {
  // Authored bridge contract fixtures, not reproductions against the Rust API.
  const cases = [
    { name: "sim_act", args: { action: "bad" }, path: "/v1/run/action", status: 400, body: '{"error":"unknown action"}' },
    { name: "sim_state", args: {}, path: "/v1/run/state", status: 404, body: '{"error":"no active run"}' },
    { name: "sim_fast_forward", args: {}, path: "/v1/run/fast", status: 503, body: '{"error":"external policy unavailable"}' },
    { name: "sim_choices", args: {}, path: "/v1/run/choices", status: 502, body: "upstream is unavailable\nplease retry" },
    { name: "sim_export_telemetry", args: {}, path: "/v1/run/telemetry", status: 500, body: "" },
  ];
  const backend = await startBackend(t, (req, res) => {
    const response = cases.find((item) => item.path === req.url);
    res.writeHead(response.status);
    res.end(response.body);
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  for (const [index, item] of cases.entries()) {
    const response = await request(srv, { jsonrpc: "2.0", id: index, method: "tools/call", params: { name: item.name, arguments: item.args } });
    assert.equal(response.id, index);
    assert.equal(response.error, undefined, item.name);
    assert.equal(response.result.isError, true, `${item.name} must not report HTTP ${item.status} as success`);
    assert.equal(response.result.content[0].type, "text");
    assert.match(response.result.content[0].text, new RegExp(`HTTP ${item.status}`));
    assert.ok(response.result.content[0].text.includes(item.body));
  }
  assert.deepEqual(backend.requests.map((req) => req.path), cases.map((item) => item.path));
  const ping = await request(srv, { jsonrpc: "2.0", id: "after-errors", method: "ping" });
  assert.deepEqual(ping.result, {});
});

test("failed resource reads return RPC errors without advertising resource contents", { timeout: 10000 }, async (t) => {
  const backend = await startBackend(t, (_req, res) => {
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end('{"error":"no active run"}');
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  const response = await request(srv, { jsonrpc: "2.0", id: 1, method: "resources/read", params: { uri: "uma-sim://run/state" } });
  assert.equal(response.id, 1);
  assert.equal(response.result, undefined);
  assert.equal(response.error.code, -32603);
  assert.match(response.error.message, /HTTP 404/);
  assert.match(response.error.message, /no active run/);
});

test("successful HTTP payloads and run setup arguments keep their existing shape", { timeout: 10000 }, async (t) => {
  const payloads = {
    "/v1/run/start": { turn: 0, seed: 7 },
    "/v1/run/state": { turn: 8, error: "ordinary data in a successful response" },
    "/v1/run/telemetry": [{ turn: 1, action: "rest" }],
    "/v1/run/text": "Career log\nSecond line",
  };
  const backend = await startBackend(t, (req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(payloads[req.url]));
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  const setup = { seed: 7, scenario: "unity", trainee: "Special Week", speed: 3, deckSupports: "support:10001", legacyFactors: "factor:blue:1@3" };
  const cases = [
    { name: "sim_start", args: setup, path: "/v1/run/start" },
    { name: "sim_state", args: undefined, path: "/v1/run/state" },
    { name: "sim_export_telemetry", args: {}, path: "/v1/run/telemetry" },
    { name: "sim_text", args: {}, path: "/v1/run/text" },
  ];
  for (const [id, item] of cases.entries()) {
    const response = await request(srv, { jsonrpc: "2.0", id, method: "tools/call", params: { name: item.name, arguments: item.args } });
    assert.equal(response.id, id);
    assert.equal(response.error, undefined);
    assert.notEqual(response.result.isError, true);
    assert.deepEqual(JSON.parse(response.result.content[0].text), payloads[item.path]);
  }
  assert.deepEqual(backend.requests[0], {
    method: "POST", path: "/v1/run/start",
    body: { ...setup, seed: "7", speed: "3", traceTelemetry: "true" },
  });
  for (const [id, name] of [[10, "state"], [11, "text"], [12, "telemetry"]]) {
    const uri = `uma-sim://run/${name}`;
    const response = await request(srv, { jsonrpc: "2.0", id, method: "resources/read", params: { uri } });
    assert.equal(response.error, undefined);
    assert.deepEqual(response.result.contents, [{
      uri, mimeType: name === "text" ? "text/plain" : "application/json",
      text: name === "text" ? payloads[`/v1/run/${name}`] : JSON.stringify(payloads[`/v1/run/${name}`], null, 2),
    }]);
  }
});

test("invalid JSON-RPC frames are rejected and the stdio connection stays usable", { timeout: 10000 }, async () => {
  const srv = startServer();
  try {
    for (const frame of [null, 7, true, "ping", [], {}, { jsonrpc: "2.0", id: 1 }, { jsonrpc: "1.0", id: 1, method: "ping" }, { jsonrpc: "2.0", id: {}, method: "ping" }]) {
      const response = await request(srv, frame);
      assert.equal(response.id, null);
      assert.equal(response.result, undefined);
      assert.equal(response.error.code, -32600, JSON.stringify(frame));
    }
    const ping = await request(srv, { jsonrpc: "2.0", id: 0, method: "ping" });
    assert.equal(ping.id, 0);
    assert.deepEqual(ping.result, {});
    srv.send({ jsonrpc: "2.0", method: "notifications/initialized" });
    assert.equal(await srv.recvTimeout(150), null, "valid notifications must remain silent");
  } finally {
    srv.stop();
  }
});

test("malformed parameters are rejected before any backend mutation", { timeout: 10000 }, async (t) => {
  const backend = await startBackend(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end('{"ok":true}');
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  const invalid = [
    null, [], "bad params",
    { name: "sim_start", arguments: null },
    { name: "sim_start", arguments: [] },
    { name: "sim_start", arguments: "bad args" },
    { name: "sim_start", arguments: { seed: "42" } },
    { name: "sim_act", arguments: { action: {} } },
    { name: "sim_act", arguments: { action: null } },
    { name: "sim_deck_place", arguments: { supportId: "support:10001", facility: 7 } },
  ];
  for (const [id, params] of invalid.entries()) {
    const response = await request(srv, { jsonrpc: "2.0", id, method: "tools/call", params });
    assert.equal(response.id, id);
    assert.equal(response.result, undefined);
    assert.equal(response.error.code, -32602, JSON.stringify(params));
  }
  assert.deepEqual(backend.requests, [], "invalid input reached the REST backend");
  const valid = await request(srv, { jsonrpc: "2.0", id: "valid", method: "tools/call", params: { name: "sim_act", arguments: { action: "rest" } } });
  assert.deepEqual(JSON.parse(valid.result.content[0].text), { ok: true });
  assert.deepEqual(backend.requests, [{ method: "POST", path: "/v1/run/action", body: { action: "rest" } }]);
});

test("MCP invalid or unsafe numeric request IDs cannot execute a tool", { timeout: 10000 }, async (t) => {
  const backend = await startBackend(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end('{"ok":true}');
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  // MCP is stricter than base JSON-RPC: IDs are strings or integers, never null.
  // Use raw JSON for overflow and unsafe integers; serializing them first would
  // hide the wire-ID precision failure this guard must prevent.
  for (const id of ["1.25", "-0.5", "null", "1e400", "-1e400", "true", "false", "[]", "{}", "9007199254740992", "9007199254740993", "-9007199254740993"]) {
    srv.sendRaw(`{"jsonrpc":"2.0","id":${id},"method":"tools/call","params":{"name":"sim_act","arguments":{"action":"rest"}}}`);
    const response = await srv.recvTimeout(1500);
    assert.notEqual(response, null, `invalid ID ${id} was ignored`);
    assert.equal(response.jsonrpc, "2.0");
    assert.equal(response.id, null);
    assert.equal(response.result, undefined);
    assert.equal(response.error.code, -32600, id);
    assert.deepEqual(backend.requests, [], `invalid ID ${id} reached the REST backend`);
  }
  srv.send({ jsonrpc: "2.0", method: "notifications/initialized" });
  assert.equal(await srv.recvTimeout(150), null, "an omitted ID remains a silent notification");
  const valid = await request(srv, { jsonrpc: "2.0", id: 0, method: "tools/call", params: { name: "sim_act", arguments: { action: "rest" } } });
  assert.equal(valid.id, 0);
  assert.deepEqual(JSON.parse(valid.result.content[0].text), { ok: true });
  assert.deepEqual(backend.requests, [{ method: "POST", path: "/v1/run/action", body: { action: "rest" } }]);
});

test("string and safe integer IDs preserve accepted career-lab arguments and zero seeds", { timeout: 10000 }, async (t) => {
  const backend = await startBackend(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end('{"ok":true}');
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  const startDefaults = { seed: "42", scenario: "ura", trainee: "Special Week", speed: "1", deckSupports: "", legacyFactors: "", traceTelemetry: "true" };
  const cases = [
    { id: 0, name: "sim_start", path: "/v1/run/start", body: startDefaults },
    { id: "0", name: "sim_start", args: { seed: 0, speed: 1 }, path: "/v1/run/start", body: { ...startDefaults, seed: "0", speed: "1" } },
    { id: -3, name: "sim_auto", args: {}, path: "/v1/run/auto", body: { policy: "bot" } },
    { id: "", name: "sim_fast_forward", args: {}, path: "/v1/run/fast", body: { multiplier: "100" } },
    { id: "snow-雪", name: "sim_fast_forward", args: { multiplier: 1 }, path: "/v1/run/fast", body: { multiplier: "1" } },
    { id: Number.MIN_SAFE_INTEGER, name: "sim_act", args: { action: "rest" }, path: "/v1/run/action", body: { action: "rest" } },
    { id: Number.MAX_SAFE_INTEGER, name: "sim_act", args: { action: "rest" }, path: "/v1/run/action", body: { action: "rest" } },
  ];
  for (const item of cases) {
    const response = await request(srv, { jsonrpc: "2.0", id: item.id, method: "tools/call", params: { name: item.name, arguments: item.args } });
    assert.equal(response.id, item.id);
    assert.equal(response.error, undefined);
    assert.notEqual(response.result.isError, true);
    assert.deepEqual(JSON.parse(response.result.content[0].text), { ok: true });
  }
  assert.deepEqual(backend.requests, cases.map((item) => ({ method: "POST", path: item.path, body: item.body })));
});

test("numeric overflow in valid JSON arguments is rejected before REST", { timeout: 10000 }, async (t) => {
  const backend = await startBackend(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end('{"ok":true}');
  });
  const srv = startServer(backend.url);
  t.after(() => srv.stop());
  const cases = [
    { name: "sim_start", args: '{"seed":1e400}' },
    { name: "sim_start", args: '{"speed":-1e400}' },
    { name: "sim_fast_forward", args: '{"multiplier":1e400}' },
  ];
  for (const [id, item] of cases.entries()) {
    srv.sendRaw(`{"jsonrpc":"2.0","id":${id},"method":"tools/call","params":{"name":"${item.name}","arguments":${item.args}}}`);
    const response = await srv.recvTimeout(1500);
    assert.notEqual(response, null);
    assert.equal(response.id, id);
    assert.equal(response.result, undefined);
    assert.equal(response.error.code, -32602);
  }
  assert.deepEqual(backend.requests, [], "nonfinite arguments reached REST");
});
