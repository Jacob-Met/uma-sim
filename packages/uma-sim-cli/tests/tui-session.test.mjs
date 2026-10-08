import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import http from "node:http";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";

const entry = process.env.UMA_TUI_UNDER_TEST ?? fileURLToPath(new URL("../tui.js", import.meta.url));

async function until(check, describe) {
  const deadline = Date.now() + 5000;
  while (!check()) {
    if (Date.now() >= deadline) throw new Error(`Timed out: ${describe()}`);
    await delay(5);
  }
}

function run(t, url, args) {
  const child = spawn(process.execPath, [entry, ...args], {
    env: { ...process.env, UMA_SIM_API: url }, stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  let ended = false;
  child.stdout.on("data", (data) => { stdout += data; });
  child.stderr.on("data", (data) => { stderr += data; });
  const completion = once(child, "close").then(([code, signal]) => {
    ended = true;
    return { code, signal, stdout, stderr };
  });
  t.after(async () => {
    if (!ended) child.kill("SIGKILL");
    await completion;
  });
  const describe = () => JSON.stringify({ stdout, stderr, ended });
  return {
    child,
    get stdout() { return stdout; },
    get stderr() { return stderr; },
    async prompt(count = 1) {
      await until(() => ended || (stdout.match(/\n> /g) ?? []).length >= count, describe);
      assert.equal(ended, false, describe());
    },
    send(command) { child.stdin.write(`${command}\n`); },
    async finish() {
      await until(() => ended, describe);
      return completion;
    },
    async quit() {
      child.stdin.end("quit\n");
      return this.finish();
    },
  };
}

function snapshot(turn, seed = "7", scenario = "ura") {
  return { state: { turn, stats: { speed: 80, stamina: 70, power: 60, guts: 50, wit: 40 },
    date: { year: 1, month: 6 }, energy: 70, mood: 2, fans: 120, skillPoints: 90,
    phase: "training", meta: { seed, scenarioId: scenario } } };
}

async function receiver(t, intercept = () => false) {
  const state = {
    requests: [], active: "other",
    sessions: new Map([["", snapshot(9, "99")], ["saved-career", snapshot(12)], ["other", snapshot(48, "88")]]),
  };
  const server = http.createServer(async (req, res) => {
    let raw = "";
    for await (const chunk of req) raw += chunk;
    const body = raw ? JSON.parse(raw) : undefined;
    const url = new URL(req.url, "http://localhost");
    const record = { method: req.method, route: url.pathname, query: Object.fromEntries(url.searchParams), body };
    state.requests.push(record);
    const send = (status, value) => {
      res.writeHead(status, { "Content-Type": "application/json", Connection: "close" });
      res.end(JSON.stringify(value));
    };
    if (await intercept(record, { req, res, state, send })) return;
    if (record.route === "/v1/health") return send(200, { ok: true, version: "fixture" });
    if (record.route === "/v1/sessions") return send(200, {
      active: state.active,
      sessions: [...state.sessions].map(([id, snap]) => ({ id, label: id || "main", turn: snap.state.turn,
        scenarioId: snap.state.meta.scenarioId, traineeName: "Special Week" })),
    });
    if (record.route === "/v1/run/start") {
      if (!["ura", "unity", "grand_concert", "trackblazer"].includes(body.scenario)) return send(400, { error: "unknown scenario" });
      const id = body.session ?? "";
      const snap = snapshot(1, body.seed, body.scenario);
      state.sessions.set(id, snap);
      state.active = id;
      return send(200, snap);
    }
    // Reflect the current Rust API's explicit-empty ambiguity. The terminal
    // must use a nonempty selector rather than depending on a backend repair.
    const id = body?.session || url.searchParams.get("session") || state.active;
    const snap = state.sessions.get(id);
    if (!snap) return send(404, { error: `no such session '${id}'` });
    if (record.route === "/v1/run/state") return send(200, snap);
    if (record.route === "/v1/run/text") return send(200, { text: `Career ${id} turn ${snap.state.turn}` });
    if (record.route === "/v1/run/choices") return send(200, { choices: [{ id: "rest", label: "Rest" }] });
    if (record.route === "/v1/run/action" && body.action !== "rest") return send(400, { error: "choice rejected" });
    if (["/v1/run/action", "/v1/run/auto", "/v1/run/fast"].includes(record.route)) {
      snap.state.turn = record.route === "/v1/run/fast" ? 72 : snap.state.turn + 1;
      return send(200, snap);
    }
    return send(404, { error: "unknown route" });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(async () => { server.closeAllConnections(); await new Promise((resolve) => server.close(resolve)); });
  return { ...state, state, url: `http://127.0.0.1:${server.address().port}` };
}

const writes = (fixture) => fixture.requests.filter((request) => request.method === "POST");

test("ordinary seed/scenario creates a fresh named career and every operation stays on it", { timeout: 10000 }, async (t) => {
  const f = await receiver(t);
  const before = JSON.stringify([...f.sessions]);
  const cli = run(t, f.url, ["123", "unity"]);
  await cli.prompt();
  const start = writes(f)[0];
  assert.match(start.body.session, /^tui-[0-9a-f-]{36}$/);
  assert.equal(start.body.seed, "123");
  assert.equal(start.body.scenario, "unity");
  assert.match(cli.stdout, new RegExp(`--session=${start.body.session}`));
  assert.equal(JSON.stringify([...f.sessions].filter(([id]) => id !== start.body.session)), before);
  f.state.active = "other";
  for (const [index, command] of ["rest", "auto", "fast"].entries()) {
    cli.send(command);
    await cli.prompt(index + 2);
  }
  assert.equal(f.sessions.get(start.body.session).state.turn, 72);
  assert.equal(f.sessions.get("other").state.turn, 48);
  assert.ok(f.requests.filter((r) => r.route.startsWith("/v1/run/")).every((r) =>
    (r.method === "GET" ? r.query.session : r.body.session) === start.body.session));
  assert.equal((await cli.quit()).code, 0);
});

test("--new always creates another unique session while existing careers survive", async (t) => {
  const f = await receiver(t);
  const ids = [];
  for (let i = 0; i < 2; i++) {
    const cli = run(t, f.url, ["--new"]);
    await cli.prompt();
    ids.push(writes(f).at(-1).body.session);
    assert.equal((await cli.quit()).code, 0);
  }
  assert.notEqual(ids[0], ids[1]);
  assert.equal(f.sessions.size, 5);
  assert.equal(f.sessions.get("").state.turn, 9);
});

test("resume never starts, activates or closes a career and remains explicit when active changes", async (t) => {
  const f = await receiver(t);
  const cli = run(t, f.url, ["--session", "saved-career"]);
  await cli.prompt();
  assert.match(cli.stdout, /Turn 12/);
  assert.equal(writes(f).length, 0);
  assert.equal(f.state.active, "other");
  f.state.active = "";
  cli.send("rest");
  await cli.prompt(2);
  assert.equal(f.sessions.get("saved-career").state.turn, 13);
  assert.equal(f.sessions.get("").state.turn, 9);
  assert.deepEqual(writes(f).map((r) => r.body), [{ action: "rest", session: "saved-career" }]);
  assert.equal((await cli.quit()).code, 0);
});

test("listing sessions is read-only and includes the legacy main label", async (t) => {
  const f = await receiver(t);
  const cli = run(t, f.url, ["--list-sessions"]);
  const result = await cli.finish();
  assert.equal(result.code, 0);
  assert.match(result.stdout, /\(main\).*turn 9/);
  assert.match(result.stdout, /saved-career.*turn 12/);
  assert.match(result.stdout, /other \*/);
  assert.equal(writes(f).length, 0);
  assert.equal(f.state.active, "other");
});

test("a missing resume target fails without starting or falling back to active", async (t) => {
  const f = await receiver(t);
  const before = JSON.stringify([...f.sessions]);
  const result = await run(t, f.url, ["--session=missing"]).finish();
  assert.equal(result.code, 1);
  assert.match(result.stderr, /HTTP 404: no such session 'missing'/);
  assert.equal(writes(f).length, 0);
  assert.equal(JSON.stringify([...f.sessions]), before);
});

test("rejected new career is reported before entering the prompt", async (t) => {
  const f = await receiver(t);
  const before = JSON.stringify([...f.sessions]);
  const result = await run(t, f.url, ["9", "unknown"]).finish();
  assert.equal(result.code, 1);
  assert.match(result.stderr, /HTTP 400: unknown scenario/);
  assert.doesNotMatch(result.stdout, /\n> /);
  assert.equal(writes(f).length, 1);
  assert.equal(JSON.stringify([...f.sessions]), before);
});

test("rejected actions and unavailable external fast policy leave an interactive prompt", async (t) => {
  const f = await receiver(t, (r, { send }) => {
    if (r.route !== "/v1/run/fast") return false;
    send(503, { error: "external policy unavailable" }); return true;
  });
  const cli = run(t, f.url, ["--session=saved-career"]);
  await cli.prompt();
  for (const [index, command] of ["bad_choice", "fast", "rest"].entries()) {
    cli.send(command);
    await cli.prompt(index + 2);
  }
  assert.match(cli.stderr, /HTTP 400: choice rejected/);
  assert.match(cli.stderr, /HTTP 503: external policy unavailable/);
  assert.equal(f.sessions.get("saved-career").state.turn, 13);
  assert.equal(writes(f).length, 3);
  assert.equal((await cli.quit()).code, 0);
});

for (const failure of ["disconnect", "invalid JSON"]) {
  test(`${failure} after an action reports uncertainty and never retries the action`, async (t) => {
    const f = await receiver(t, (r, { req, res, state }) => {
      if (r.route !== "/v1/run/action") return false;
      state.sessions.get(r.body.session).state.turn++;
      if (failure === "disconnect") req.socket.destroy();
      else { res.writeHead(200, { Connection: "close" }); res.end("not JSON"); }
      return true;
    });
    const cli = run(t, f.url, ["--session=saved-career"]);
    await cli.prompt();
    cli.send("rest");
    await cli.prompt(2);
    assert.match(cli.stderr, /may have reached the server/);
    assert.match(cli.stderr, /--session=saved-career/);
    assert.match(cli.stdout, /Turn 13/);
    assert.equal(writes(f).length, 1);
    assert.equal((await cli.quit()).code, 0);
  });
}

test("a read failure discards the incomplete display and can recover without a mutation", async (t) => {
  let fail = true;
  const f = await receiver(t, (r, { send }) => {
    if (r.route !== "/v1/run/text" || !fail) return false;
    fail = false; send(503, { error: "temporary read failure" }); return true;
  });
  const cli = run(t, f.url, ["--session=saved-career"]);
  await cli.prompt();
  assert.match(cli.stderr, /temporary read failure/);
  assert.doesNotMatch(cli.stdout, /Turn 12/);
  cli.send("state");
  await cli.prompt(2);
  assert.match(cli.stdout, /Turn 12/);
  assert.equal(writes(f).length, 0);
  assert.equal((await cli.quit()).code, 0);
});

for (const stop of ["EOF", "SIGINT"]) {
  test(`${stop} at the prompt exits without changing the career`, async (t) => {
    const f = await receiver(t);
    const cli = run(t, f.url, ["--session=saved-career"]);
    await cli.prompt();
    if (stop === "EOF") cli.child.stdin.end(); else cli.child.kill("SIGINT");
    const result = await cli.finish();
    assert.equal(result.code, stop === "EOF" ? 0 : 130);
    assert.equal(writes(f).length, 0);
  });
  test(`${stop} while an action is pending cancels waiting, keeps its identity and does not replay`, async (t) => {
    let pending = false;
    const f = await receiver(t, (r) => {
      if (r.route !== "/v1/run/action") return false;
      pending = true; return true; // Deliberately hold the response open.
    });
    const cli = run(t, f.url, ["--session=saved-career"]);
    await cli.prompt();
    cli.send("rest");
    await until(() => pending, () => "action never reached receiver");
    if (stop === "EOF") cli.child.stdin.end(); else cli.child.kill("SIGINT");
    const result = await cli.finish();
    assert.equal(result.code, stop === "EOF" ? 0 : 130);
    assert.match(result.stderr, /may have reached the server/);
    assert.equal(writes(f).length, 1);
    assert.equal(writes(f)[0].body.session, "saved-career");
  });
}

test("empty, whitespace, unsafe and conflicting session arguments fail before HTTP", async (t) => {
  const f = await receiver(t);
  for (const args of [["--session="], ["--session", ""], ["--session=   "], ["--session= saved-career"],
    ["--session=.."], ["--session=x/y"], ["--session"], ["--session", "--new"], ["--session=x", "--session=y"],
    ["--session=saved-career", "--new"], ["--session=saved-career", "42"],
    ["--session=saved-career", "--list-sessions"], ["--session=" + "x".repeat(65)]]) {
    const result = await run(t, f.url, args).finish();
    assert.equal(result.code, 1, JSON.stringify({ args, result }));
  }
  assert.equal(f.requests.length, 0);
});

test("a failing health response is fatal and never treated as permission to create a career", async (t) => {
  const f = await receiver(t, (r, { send }) => {
    if (r.route !== "/v1/health") return false;
    send(403, { error: "access denied" }); return true;
  });
  const result = await run(t, f.url, ["--new"]).finish();
  assert.equal(result.code, 1);
  assert.match(result.stderr, /HTTP 403: access denied/);
  assert.equal(writes(f).length, 0);
});

test("help and invalid seeds finish without contacting the server", async (t) => {
  const f = await receiver(t);
  const help = await run(t, f.url, ["--help"]).finish();
  assert.equal(help.code, 0);
  assert.match(help.stdout, /--session=<id>/);
  for (const seed of ["bad", "9223372036854775808", "-9223372036854775809"]) {
    assert.equal((await run(t, f.url, [seed]).finish()).code, 1);
  }
  assert.equal(f.requests.length, 0);
});
