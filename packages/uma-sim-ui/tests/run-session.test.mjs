import test, { after } from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { create, act } from "react-test-renderer";
import { loadUiModule, cleanCompiledUi } from "./compile-ui.mjs";

const { useRunStore } = loadUiModule("state/runStore");
const { api } = loadUiModule("api/client");
after(cleanCompiledUi);

function deferred() {
  let resolve, reject;
  const promise = new Promise((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

function fixtureServer() {
  const calls = [];
  const holds = [];
  const turns = { "": 1, a: 2, b: 3 };
  let active = "a";
  let offered = null;
  const meta = (id) => ({
    seed: id === "" ? 10 : id === "a" ? 20 : 30,
    traineeName: id || "main", scenarioId: "ura", objectiveProfile: "default",
    legacyFactors: [], parentNames: [], deckSupports: [],
  });
  const snapshot = (id) => ({
    meta: meta(id),
    settings: {}, rngSeed: 1, rngCalls: turns[id],
    state: {
      turn: turns[id], meta: meta(id), date: { year: 1, month: 1, half: 1 },
      stats: { speed: 100, stamina: 100, power: 100, guts: 100, wit: 100 },
      energy: 100, maxEnergy: 100, mood: "NORMAL", fans: 0, skillPoints: 0,
      careerComplete: false, awaitingChoice: false, phase: "TRAINING",
      completedRaces: [], facilityLevels: {}, facilityTrainCounts: {},
      pendingEventOptions: [], hintLevels: {}, statuses: [], performanceTokens: {},
      scenarioResources: { values: {} }, legacy: { aptitudes: {}, inheritedSkillIds: [] },
      learnedSkillIds: [], deck: { slots: [] }, log: [],
    },
  });
  const choices = (id) => offered || [{ id: "rest", label: `Rest ${id || "main"}` }];
  const fetch = async (path, options = {}) => {
    const url = new URL(path, "http://fixture.invalid");
    const body = options.body ? JSON.parse(options.body) : {};
    const method = options.method || "GET";
    const target = Object.hasOwn(body, "session") ? body.session
      : url.searchParams.has("session") ? url.searchParams.get("session") : active;
    const call = { path: url.pathname, query: url.search, method, body, target };
    calls.push(call);
    let result;
    if (url.pathname === "/v1/health") result = { ok: true, version: "fixture", repoRoot: true };
    else if (url.pathname.startsWith("/v1/catalog/")) result = { items: [] };
    else if (url.pathname === "/v1/library") result = { entries: [] };
    else if (url.pathname === "/v1/lab/branches") result = { branches: [] };
    else if (url.pathname === "/v1/sessions") {
      result = { sessions: Object.keys(turns).map((id) => ({ id, label: id || "main" })), active };
    } else if (url.pathname === "/v1/run/start") {
      active = body.session ?? "";
      turns[active] = 1;
      result = snapshot(active);
    } else if (url.pathname === "/v1/run/state") result = snapshot(target);
    else if (url.pathname === "/v1/run/text") result = { text: `Career ${target || "main"}` };
    else if (url.pathname === "/v1/run/choices") result = { choices: choices(target) };
    else if (url.pathname === "/v1/run/telemetry") result = { session: target };
    else if (["/v1/run/action", "/v1/run/auto"].includes(url.pathname)) {
      turns[target]++;
      result = { state: snapshot(target), choices: choices(target), text: `Step ${target}`, careerEnded: false };
    } else if (url.pathname === "/v1/run/fast") {
      turns[target] = 72;
      result = { careerEnded: true, turn: 72, fans: 0 };
    } else if (["/v1/run/deck/place", "/v1/run/style"].includes(url.pathname)) result = snapshot(target);
    else throw new Error(`Unexpected fixture route ${method} ${url.pathname}`);
    const holdIndex = holds.findIndex((hold) => hold.method === method && hold.path === url.pathname);
    if (holdIndex >= 0) await holds.splice(holdIndex, 1)[0].gate.promise;
    return new Response(JSON.stringify(result), { status: 200, headers: { "Content-Type": "application/json" } });
  };
  return {
    calls, fetch, turns,
    setActive(id) { active = id; },
    setChoices(value) { offered = value; },
    hold(method, path) { const gate = deferred(); holds.push({ method, path, gate }); return gate; },
  };
}

async function mountStore(t, server) {
  const previousFetch = globalThis.fetch;
  globalThis.fetch = server.fetch;
  let current;
  function Probe() { current = useRunStore(); return null; }
  let renderer;
  await act(async () => { renderer = create(React.createElement(Probe)); });
  t.after(async () => {
    await act(async () => renderer.unmount());
    globalThis.fetch = previousFetch;
  });
  return {
    get value() { return current; },
    async call(method, ...args) { await act(async () => { await current[method](...args); }); },
    async begin(method, ...args) {
      let pending;
      await act(async () => { pending = current[method](...args); });
      return { async finish(gate, error) {
        await act(async () => { error ? gate.reject(error) : gate.resolve(); await pending; });
      } };
    },
  };
}

test("a delayed action cannot replace a newly selected session", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  const gate = server.hold("POST", "/v1/run/action");
  const pending = await store.begin("act", "rest");
  server.setActive("b");
  await store.call("refreshActive");
  await pending.finish(gate);
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "b");
  assert.deepEqual(store.value.state.textLines, ["Career b"]);
});

test("a superseded refresh cannot restore the previous session", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  const gate = server.hold("GET", "/v1/run/state");
  const pending = await store.begin("refreshActive");
  server.setActive("b");
  await store.call("refreshActive");
  await pending.finish(gate);
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "b");
  assert.equal(store.value.state.sessionId, "b");
});

test("reset prevents a delayed action from resurrecting a career", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  const gate = server.hold("POST", "/v1/run/action");
  const pending = await store.begin("act", "rest");
  await store.call("newRun");
  await pending.finish(gate);
  assert.equal(store.value.state.snapshot, null);
  assert.equal(store.value.state.busy, false);
});

test("late failures and cleanup cannot alter a newer pending refresh", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  const oldGate = server.hold("POST", "/v1/run/action");
  const old = await store.begin("act", "rest");
  server.setActive("b");
  const newGate = server.hold("GET", "/v1/run/state");
  const current = await store.begin("refreshActive");
  await old.finish(oldGate, new Error("Late old-session failure"));
  assert.equal(store.value.state.busy, true);
  assert.equal(store.value.state.error, null);
  await current.finish(newGate);
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "b");
});

test("failed session selection clears stale playable state", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  server.setActive("b");
  const gate = server.hold("GET", "/v1/run/state");
  const pending = await store.begin("refreshActive");
  await pending.finish(gate, new Error("Selected session unavailable"));
  assert.equal(store.value.state.snapshot, null);
  assert.deepEqual(store.value.state.choices, []);
  assert.match(store.value.state.error, /Selected session unavailable/);
  const before = server.calls.length;
  await store.call("act", "rest");
  assert.equal(server.calls.length, before, "stale choices must not submit an action");
});

test("same-turn duplicate actions submit only once", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  const gate = server.hold("POST", "/v1/run/action");
  const pending = await store.begin("act", "rest");
  await store.call("act", "rest");
  await pending.finish(gate);
  assert.equal(server.calls.filter((call) => call.path === "/v1/run/action").length, 1);
});

test("run operations remain bound to the displayed session after external activation", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  server.setActive("b");
  server.calls.length = 0;
  await store.call("act", "rest");
  await store.call("autoStep", "default");
  await store.call("fastForward", 20, "bot");
  await store.call("placeDeck", "support:1", "speed");
  await store.call("setStyle", "front");
  const runCalls = server.calls.filter((call) => call.path.startsWith("/v1/run/"));
  assert.ok(runCalls.length >= 8);
  assert.ok(runCalls.every((call) => call.target === "a"), JSON.stringify(runCalls, null, 2));
  assert.equal(server.turns.b, 3);
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "a");
});

test("explicit default session is preserved in read requests", async () => {
  const previousFetch = globalThis.fetch;
  const server = fixtureServer();
  globalThis.fetch = server.fetch;
  try {
    await api.state(""); await api.choices(""); await api.text(""); await api.telemetry("");
    assert.ok(server.calls.every((call) => call.query === "?session="));
    assert.ok(server.calls.every((call) => call.target === ""));
    server.calls.length = 0;
    await api.state();
    assert.equal(server.calls[0].query, "", "omission still follows the server's active session");
  } finally { globalThis.fetch = previousFetch; }
});

test("the displayed main session remains the action target after a fork activates", async (t) => {
  const server = fixtureServer();
  server.setActive("");
  const store = await mountStore(t, server);
  await store.call("refreshActive");
  server.setActive("b");
  server.calls.length = 0;
  await store.call("act", "rest");
  assert.ok(server.calls.every((call) => call.target === ""));
  assert.equal(server.turns.b, 3);
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "main");
});

test("starting main hydrates main even when another client activates a fork", async (t) => {
  const server = fixtureServer();
  const store = await mountStore(t, server);
  const gate = server.hold("POST", "/v1/run/start");
  const pending = await store.begin("startRun", { seed: 10 });
  server.setActive("b");
  await pending.finish(gate);
  assert.equal(store.value.state.sessionId, "");
  assert.equal(store.value.state.snapshot.state.meta.traineeName, "main");
  assert.deepEqual(store.value.state.textLines, ["Career main"]);
  assert.equal(store.value.state.choices[0].label, "Rest main");
});

async function mountApp(t, server) {
  const previousFetch = globalThis.fetch;
  const previousWindow = globalThis.window;
  globalThis.fetch = server.fetch;
  const listeners = new Map();
  globalThis.window = {
    addEventListener(type, fn) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type).add(fn);
    },
    removeEventListener(type, fn) { listeners.get(type)?.delete(fn); },
    setTimeout, clearTimeout,
  };
  const App = loadUiModule("App").default;
  const { RunSetup } = loadUiModule("components/RunSetup");
  let renderer;
  await act(async () => { renderer = create(React.createElement(App)); });
  t.after(async () => {
    await act(async () => renderer.unmount());
    globalThis.fetch = previousFetch;
    globalThis.window = previousWindow;
  });
  await act(async () => renderer.root.findByType(RunSetup).props.onStart({ seed: 10 }));
  return {
    async click(text) {
      const button = renderer.root.findAllByType("button").find((node) => node.children.join("") === text);
      assert.ok(button, `button ${text} must exist`);
      await act(async () => button.props.onClick());
    },
    async key(options = {}) {
      const event = {
        key: "1", target: { tagName: "DIV", isContentEditable: false },
        defaultPrevented: false, preventDefault() { this.defaultPrevented = true; },
        ...options,
      };
      await act(async () => { for (const listener of listeners.get("keydown") || []) listener(event); });
      return event;
    },
  };
}

test("keyboard shortcuts operate only the visible run and its displayed choice numbers", async (t) => {
  const server = fixtureServer();
  server.setChoices([
    { id: "gl_tech_0", label: "Scenario technique" },
    { id: "rest", label: "Rest" },
  ]);
  const app = await mountApp(t, server);
  await app.click("Library & lab");
  await app.key();
  assert.equal(server.calls.filter((call) => call.path === "/v1/run/action").length, 0);
  await app.click("Run");
  for (const options of [
    { ctrlKey: true }, { metaKey: true }, { altKey: true }, { repeat: true },
    { isComposing: true }, { defaultPrevented: true },
    { target: { tagName: "SPAN", isContentEditable: true } },
    { target: { tagName: "INPUT" } }, { target: { tagName: "SELECT" } },
    { target: { tagName: "TEXTAREA" } },
  ]) await app.key(options);
  assert.equal(server.calls.filter((call) => call.path === "/v1/run/action").length, 0);
  const event = await app.key();
  const actions = server.calls.filter((call) => call.path === "/v1/run/action");
  assert.equal(actions.length, 1);
  assert.equal(actions[0].body.action, "rest", "shortcut 1 must match the first displayed numbered choice");
  assert.equal(event.defaultPrevented, true);
});

test("telemetry export uses the displayed career even after external activation", async (t) => {
  const server = fixtureServer();
  const app = await mountApp(t, server);
  server.setActive("b");
  const previousDocument = globalThis.document;
  const previousCreate = URL.createObjectURL;
  const previousRevoke = URL.revokeObjectURL;
  const downloads = [];
  let blob;
  globalThis.document = { createElement() { return { click() { downloads.push({ filename: this.download, href: this.href }); } }; } };
  URL.createObjectURL = (value) => { blob = value; return "blob:fixture"; };
  URL.revokeObjectURL = () => {};
  try {
    await app.click("Export telemetry");
    assert.deepEqual(JSON.parse(await blob.text()), { session: "" });
    assert.deepEqual(downloads, [{ filename: "uma-sim-telemetry-10.json", href: "blob:fixture" }]);
  } finally {
    globalThis.document = previousDocument;
    URL.createObjectURL = previousCreate;
    URL.revokeObjectURL = previousRevoke;
  }
});
