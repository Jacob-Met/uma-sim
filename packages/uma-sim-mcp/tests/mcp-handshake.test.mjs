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
import { fileURLToPath } from "node:url";
import path from "node:path";

const SERVER = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "mcp-stdio.js");

function startServer() {
  const child = spawn("node", [SERVER], {
    stdio: ["pipe", "pipe", "pipe"],
    env: { ...process.env, UMA_SIM_API: "http://127.0.0.1:1" }, // closed port: tools fail cleanly
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
      "sim_act", "sim_auto", "sim_choices", "sim_deck_place", "sim_export_telemetry",
      "sim_fast_forward", "sim_load_content_pack", "sim_start", "sim_state", "sim_text",
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
