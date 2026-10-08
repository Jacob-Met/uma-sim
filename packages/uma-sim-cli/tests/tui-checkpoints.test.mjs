import assert from "node:assert/strict";
import http from "node:http";
import test from "node:test";
import { launch, until } from "./tui-checkpoint-harness.mjs";

const clone = (value) => JSON.parse(JSON.stringify(value));
const snapshot = (turn, seed) => ({ meta: { seed, scenarioId: "ura" }, rngCalls: turn * 3,
  state: { turn, stats: { speed: 90 }, date: { year: 1, month: 6 }, phase: "training" } });
const metadata = (name, snap) => ({ name, label: name, turn: snap.state.turn,
  scenarioId: "ura", traineeName: "Special Week", savedAt: "2026-10-08T10:00:00Z" });

async function receiver(t, intercept = () => false) {
  const state = {
    requests: [], active: "other",
    sessions: new Map([["", snapshot(9, 91)], ["career", snapshot(12, 42)], ["other", snapshot(40, 92)]]),
    checkpoints: new Map([["before-race", snapshot(7, 42)]]),
  };
  const server = http.createServer(async (req, res) => {
    let raw = "";
    for await (const part of req) raw += part;
    const url = new URL(req.url, "http://localhost");
    const body = raw ? JSON.parse(raw) : undefined;
    const record = { method: req.method, route: url.pathname, query: Object.fromEntries(url.searchParams), body };
    state.requests.push(record);
    const send = (status, value) => { res.writeHead(status, { "Content-Type": "application/json", Connection: "close" }); res.end(JSON.stringify(value)); };
    if (await intercept(record, { req, res, state, send })) return;
    if (record.route === "/v1/health") return send(200, { ok: true });
    if (record.route === "/v1/library") return send(200, {
      entries: [...state.checkpoints].map(([name, snap]) => metadata(name, snap)),
    });
    if (record.route === "/v1/session/fork") {
      const saved = state.checkpoints.get(body.checkpoint);
      if (!saved) return send(404, { error: "checkpoint not found" });
      if (state.sessions.has(body.id)) return send(409, { error: "session already exists" });
      state.sessions.set(body.id, clone(saved));
      state.active = body.id;
      return send(200, { session: { id: body.id }, compatAdvisories: ["catalog revision changed"] });
    }
    const id = body?.session || url.searchParams.get("session") || state.active;
    const snap = state.sessions.get(id);
    if (!snap) return send(404, { error: "session not found" });
    if (record.route === "/v1/run/state") return send(200, clone(snap));
    if (record.route === "/v1/run/text") return send(200, { text: "Career " + id });
    if (record.route === "/v1/run/choices") return send(200, { choices: [{ id: "rest", label: "Rest" }] });
    if (record.route === "/v1/library/save") {
      if (state.checkpoints.has(body.name)) return send(409, { error: "checkpoint already exists" });
      state.checkpoints.set(body.name, clone(snap));
      return send(200, { entry: metadata(body.name, snap) });
    }
    if (record.route === "/v1/run/action" && body.action === "rest") {
      snap.state.turn++; snap.rngCalls += 3;
      return send(200, clone(snap));
    }
    return send(400, { error: "deliberate fixture refusal" });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(async () => { server.closeAllConnections(); await new Promise((resolve) => server.close(resolve)); });
  return { ...state, state, url: "http://127.0.0.1:" + server.address().port };
}
const writes = (f) => f.requests.filter((r) => r.method === "POST");

test("checkpoint opens an independent named career, preserves existing state, and shows advisories", async (t) => {
  const f = await receiver(t);
  const before = clone([...f.sessions]);
  const saved = clone([...f.checkpoints]);
  const cli = launch(t, f.url, ["--checkpoint", "before-race"]);
  await cli.prompt();
  const fork = writes(f)[0];
  assert.equal(fork.route, "/v1/session/fork");
  assert.deepEqual(Object.keys(fork.body).sort(), ["checkpoint", "id"]);
  assert.equal(fork.body.checkpoint, "before-race");
  assert.match(fork.body.id, /^tui-[0-9a-f-]{36}$/);
  assert.match(cli.stdout, new RegExp("--session=" + fork.body.id));
  assert.match(cli.stderr, /Checkpoint advisory: catalog revision changed/);
  assert.equal(writes(f).length, 1);
  f.state.active = "other";
  cli.send("rest");
  await cli.prompt(2);
  assert.equal(f.sessions.get(fork.body.id).state.turn, 8);
  assert.deepEqual([...f.sessions].filter(([id]) => id !== fork.body.id), before);
  assert.deepEqual([...f.checkpoints], saved);
  assert.ok(f.requests.filter((r) => r.route.startsWith("/v1/run/")).every((r) =>
    (r.method === "GET" ? r.query.session : r.body.session) === fork.body.id));
  assert.equal((await cli.quit()).code, 0);
});

test("save is explicitly create-only and selected-session bound; checkpoint listing never advances play", async (t) => {
  const f = await receiver(t);
  const before = clone([...f.sessions]);
  const cli = launch(t, f.url, ["--session=career"]);
  await cli.prompt();
  cli.send("save half-time");
  await cli.prompt(2);
  assert.deepEqual(writes(f)[0], { method: "POST", route: "/v1/library/save", query: {},
    body: { name: "half-time", overwrite: false, session: "career" } });
  assert.deepEqual(f.checkpoints.get("half-time"), f.sessions.get("career"));
  assert.match(cli.stdout, /Saved checkpoint 'half-time'.*--checkpoint=half-time/);
  f.state.active = "";
  cli.send("checkpoints");
  await cli.prompt(3);
  assert.match(cli.stdout, /half-time.*turn 12.*saved 2026-10-08/);
  assert.equal(writes(f).length, 1);
  assert.deepEqual([...f.sessions], before);
  cli.send("rest");
  await cli.prompt(4);
  cli.send("save half-time");
  await cli.prompt(5);
  assert.match(cli.stderr, /HTTP 409: checkpoint already exists/);
  assert.equal(f.checkpoints.get("half-time").state.turn, 12);
  assert.equal(f.sessions.get("career").state.turn, 13);
  assert.equal(writes(f).length, 3);
  assert.equal((await cli.quit()).code, 0);
});

test("standalone list is read-only, exits with closed stdin, and supports an empty library", async (t) => {
  const f = await receiver(t);
  const before = clone([...f.sessions]);
  const cli = launch(t, f.url, ["--list-checkpoints"]);
  cli.child.stdin.end();
  const result = await cli.finish();
  assert.equal(result.code, 0);
  assert.match(result.stdout, /before-race.*turn 7/);
  assert.deepEqual(f.requests.map((r) => [r.method, r.route]), [["GET", "/v1/health"], ["GET", "/v1/library"]]);
  assert.deepEqual([...f.sessions], before);
  f.checkpoints.clear();
  const empty = await launch(t, f.url, ["--list-checkpoints"]).finish();
  assert.equal(empty.code, 0);
  assert.match(empty.stdout, /No saved checkpoints/);
});

test("checkpoint argument conflicts and malformed names fail before HTTP", async (t) => {
  const f = await receiver(t);
  const invalid = [["--checkpoint="], ["--checkpoint", ""], ["--checkpoint", "   "], ["--checkpoint"],
    ["--checkpoint", "--new"], ["--checkpoint=.."], ["--checkpoint= a"], ["--checkpoint=x/y"],
    ["--checkpoint=" + "x".repeat(65)], ["--checkpoint=a", "--checkpoint=b"],
    ["--checkpoint=a", "--session=career"], ["--checkpoint=a", "--new"],
    ["--checkpoint=a", "42"], ["--checkpoint=a", "ura"], ["--checkpoint=a", "--list-sessions"],
    ["--checkpoint=a", "--list-checkpoints"], ["--list-checkpoints", "--list-sessions"],
    ["--list-checkpoints", "--session=career"], ["--list-checkpoints", "--new"], ["--list-checkpoints", "42"]];
  for (const args of invalid) {
    const result = await launch(t, f.url, args).finish();
    assert.equal(result.code, 1, JSON.stringify({ args, result }));
  }
  assert.equal(f.requests.length, 0);
});

test("malformed save commands are refused locally rather than sent as game choices", async (t) => {
  const f = await receiver(t);
  const cli = launch(t, f.url, ["--session=career"]);
  await cli.prompt();
  for (const [index, command] of ["save", "save   ", "save .", "save ..", "save /tmp/a",
    "save two names", "save\t--overwrite existing", "save " + "x".repeat(65)].entries()) {
    cli.send(command);
    await cli.prompt(index + 2);
  }
  assert.equal(writes(f).length, 0);
  assert.match(cli.stderr, /Existing checkpoints are never overwritten/);
  assert.equal((await cli.quit()).code, 0);
});

test("fork refusal never falls back to start, load, or another career", async (t) => {
  for (const status of [404, 409, 422, 500]) {
    const f = await receiver(t, (r, { send }) => {
      if (r.route !== "/v1/session/fork") return false;
      send(status, { error: "checkpoint refused " + status }); return true;
    });
    const before = clone([...f.sessions]);
    const result = await launch(t, f.url, ["--checkpoint=before-race"]).finish();
    assert.equal(result.code, 1);
    assert.match(result.stderr, new RegExp("HTTP " + status));
    assert.doesNotMatch(result.stdout, /\n> /);
    assert.deepEqual(writes(f).map((r) => r.route), ["/v1/session/fork"]);
    assert.deepEqual([...f.sessions], before);
  }
});

test("save storage or session refusal stays visible and permits later play", async (t) => {
  for (const status of [404, 500]) {
    const f = await receiver(t, (r, { send }) => {
      if (r.route !== "/v1/library/save") return false;
      send(status, { error: "save refused " + status }); return true;
    });
    const cli = launch(t, f.url, ["--session=career"]);
    await cli.prompt();
    cli.send("save next-race");
    await cli.prompt(2);
    assert.match(cli.stderr, new RegExp("HTTP " + status));
    assert.doesNotMatch(cli.stdout, /Saved checkpoint 'next-race'/);
    cli.send("rest");
    await cli.prompt(3);
    assert.equal(f.sessions.get("career").state.turn, 13);
    assert.equal(writes(f).length, 2);
    assert.equal((await cli.quit()).code, 0);
  }
});

for (const mode of ["save", "fork"]) {
  for (const fault of ["disconnect", "invalid JSON", "wrong identity"]) {
    test(mode + " with " + fault + " after commit retains recovery identity and never retries", async (t) => {
      const f = await receiver(t, (r, { req, res, state, send }) => {
        if (r.route !== (mode === "save" ? "/v1/library/save" : "/v1/session/fork")) return false;
        if (mode === "save") state.checkpoints.set(r.body.name, clone(state.sessions.get(r.body.session)));
        else state.sessions.set(r.body.id, clone(state.checkpoints.get(r.body.checkpoint)));
        if (fault === "disconnect") req.socket.destroy();
        else if (fault === "invalid JSON") { res.writeHead(200); res.end("no JSON"); }
        else send(200, mode === "save" ? { entry: { name: "wrong" } } : { session: { id: "wrong" }, compatAdvisories: [] });
        return true;
      });
      const cli = launch(t, f.url, [mode === "save" ? "--session=career" : "--checkpoint=before-race"]);
      let result;
      if (mode === "save") {
        await cli.prompt();
        cli.send("save next-race");
        await cli.prompt(2);
        assert.match(cli.stderr, /Checkpoint 'next-race' may have been saved/);
        assert.doesNotMatch(cli.stdout, /Saved checkpoint 'next-race'/);
        result = await cli.quit();
        assert.equal(result.code, 0);
      } else {
        result = await cli.finish();
        assert.equal(result.code, 1);
        assert.doesNotMatch(result.stdout, /Opened checkpoint/);
      }
      assert.match(result.stderr, /may have reached the server/);
      assert.match(result.stderr, new RegExp("--session=" + (mode === "save" ? "career" : writes(f)[0].body.id)));
      assert.equal(writes(f).length, 1);
    });
  }
}

for (const mode of ["save", "fork"]) {
  for (const stop of ["EOF", "SIGINT"]) {
    test(stop + " during pending " + mode + " preserves identity without replay", async (t) => {
      let pending = false;
      const f = await receiver(t, (r) => {
        if (r.route !== (mode === "save" ? "/v1/library/save" : "/v1/session/fork")) return false;
        pending = true; return true;
      });
      const cli = launch(t, f.url, [mode === "save" ? "--session=career" : "--checkpoint=before-race"]);
      if (mode === "save") { await cli.prompt(); cli.send("save interrupted"); }
      await until(() => pending, () => "mutation was not received");
      if (stop === "EOF") cli.child.stdin.end(); else cli.child.kill("SIGINT");
      const result = await cli.finish();
      assert.equal(result.code, stop === "EOF" ? 0 : 130);
      assert.match(result.stderr, /may have reached the server/);
      if (mode === "save") assert.match(result.stderr, /Checkpoint 'interrupted'/);
      assert.equal(writes(f).length, 1);
    });
  }
}

test("invalid library response reports failure without mutating a session", async (t) => {
  const f = await receiver(t, (r, { send }) => {
    if (r.route !== "/v1/library") return false;
    send(200, { entries: [{}] }); return true;
  });
  const result = await launch(t, f.url, ["--list-checkpoints"]).finish();
  assert.equal(result.code, 1);
  assert.match(result.stderr, /Invalid checkpoint list/);
  const cli = launch(t, f.url, ["--session=career"]);
  await cli.prompt();
  cli.send("checkpoints");
  await cli.prompt(2);
  assert.match(cli.stderr, /Invalid checkpoint list/);
  assert.equal(writes(f).length, 0);
  assert.equal((await cli.quit()).code, 0);
});
