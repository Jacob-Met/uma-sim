import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { entry, launch, until } from "./tui-checkpoint-harness.mjs";

const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
const binary = process.env.UMA_SIM_NATIVE_API && path.resolve(process.env.UMA_SIM_NATIVE_API);
if (!binary) throw new Error("Set UMA_SIM_NATIVE_API to a built uma-sim-api binary; native checkpoint tests never skip silently.");
await fs.access(binary);
const artifacts = process.env.UMA_TUI_RECEIPTS;
async function record(name, value) {
  if (!artifacts) return;
  await fs.mkdir(artifacts, { recursive: true });
  await fs.writeFile(path.join(artifacts, name + ".json"), JSON.stringify(value, null, 2) + "\n");
}
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}
async function nativeServer(t) {
  const cwd = await fs.mkdtemp(path.join(os.tmpdir(), "uma-tui-checkpoints-"));
  let child, completion, url, output = "";
  const server = {
    cwd,
    get url() { return url; },
    get pid() { return child.pid; },
    async request(route, body) {
      const response = await fetch(url + route, {
        method: body === undefined ? "GET" : "POST",
        headers: body === undefined ? undefined : { "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(10000),
      });
      const raw = await response.text();
      return { status: response.status, raw, json: JSON.parse(raw) };
    },
    async ok(route, body) {
      const response = await this.request(route, body);
      assert.equal(response.status, 200, route + ": " + response.raw);
      return response.json;
    },
    async snapshot(id) { return this.ok("/v1/run/state?session=" + encodeURIComponent(id)); },
    async mainSnapshot() {
      const previous = (await this.ok("/v1/sessions")).active;
      await this.ok("/v1/session/activate", { session: "" });
      const value = await this.snapshot("");
      if (previous !== "") await this.ok("/v1/session/activate", { session: previous });
      return value;
    },
    async files() {
      const dir = path.join(cwd, ".uma-sim/library");
      const names = await fs.readdir(dir).catch((error) => { if (error.code === "ENOENT") return []; throw error; });
      return Object.fromEntries(await Promise.all(names.filter((name) => /\.(snapshot|meta)\.json$/.test(name))
        .sort().map(async (name) => [name, await fs.readFile(path.join(dir, name), "utf8")])));
    },
    async start() {
      const port = await freePort();
      url = "http://127.0.0.1:" + port;
      child = spawn(binary, [String(port)], {
        cwd, env: { ...process.env, UMA_REPO_ROOT: repoRoot, UMA_EXTERNAL_POLICY_CMD: "" },
        stdio: ["ignore", "pipe", "pipe"],
      });
      child.stdout.on("data", (chunk) => { output += chunk; });
      child.stderr.on("data", (chunk) => { output += chunk; });
      completion = once(child, "close");
      for (let attempt = 0; attempt < 200; attempt++) {
        try {
          if ((await this.ok("/v1/health")).ok) return;
        } catch {}
        if (child.exitCode !== null || child.signalCode !== null) throw new Error("Native API exited: " + output);
        await delay(25);
      }
      throw new Error("Native API did not become healthy: " + output);
    },
    async stop() {
      if (child && child.exitCode === null && child.signalCode === null) child.kill("SIGTERM");
      if (completion) await completion;
    },
    async restart() { await this.stop(); await this.start(); },
  };
  t.after(async () => { await server.stop(); await fs.rm(cwd, { recursive: true, force: true }); });
  await server.start();
  return server;
}
async function seed(server) {
  await server.ok("/v1/run/start", { session: "", seed: "901", scenario: "ura", speed: "3", traceTelemetry: "true" });
  await server.ok("/v1/run/action", { session: "", action: "rest" });
  await server.ok("/v1/run/start", { session: "career", seed: "42", scenario: "ura", traceTelemetry: "true" });
  for (let i = 0; i < 6; i++) await server.ok("/v1/run/action", { session: "career", action: "rest" });
  await server.ok("/v1/run/start", { session: "other", seed: "903", scenario: "ura", traceTelemetry: "true" });
}
async function forwarder(t, server, route, fault) {
  const records = [];
  const proxy = http.createServer(async (req, res) => {
    let body = "";
    for await (const part of req) body += part;
    const r = { method: req.method, route: req.url, body: body ? JSON.parse(body) : undefined };
    records.push(r);
    try {
      const received = await fetch(server.url + req.url, {
        method: req.method, headers: body ? { "Content-Type": "application/json" } : undefined,
        body: body || undefined,
      });
      const raw = await received.text();
      r.nativeStatus = received.status;
      r.nativeResponse = JSON.parse(raw);
      if (r.route === route && req.method === "POST") {
        r.committed = true;
        if (fault === "hold") return;
        if (fault === "disconnect") { res.destroy(); return; }
        res.writeHead(200, { Connection: "close" }); res.end("invalid JSON"); return;
      }
      res.writeHead(received.status, { "Content-Type": "application/json", Connection: "close" });
      res.end(raw);
    } catch (error) { res.destroy(error); }
  });
  await new Promise((resolve) => proxy.listen(0, "127.0.0.1", resolve));
  t.after(async () => { proxy.closeAllConnections(); await new Promise((resolve) => proxy.close(resolve)); });
  return { url: "http://127.0.0.1:" + proxy.address().port, records };
}

test("native checkpoint survives an actual process restart and resumes the exact next RNG step", { timeout: 30000 }, async (t) => {
  const server = await nativeServer(t);
  await seed(server);
  const evidence = { before: await server.snapshot("career"), mainBefore: await server.mainSnapshot() };
  try {
    const cli = launch(t, server.url, ["--session=career"]);
    await cli.prompt();
    cli.send("save before-race");
    await cli.prompt(2);
    evidence.afterSave = await server.snapshot("career");
    evidence.libraryBeforeRestart = await server.ok("/v1/library");
    evidence.savedFiles = await server.files();
    cli.send("rest");
    await cli.prompt(3);
    evidence.afterNextAction = await server.snapshot("career");
    evidence.mainAfterCommands = await server.mainSnapshot();
    evidence.firstTerminal = await cli.quit();
    await server.restart();
    evidence.oldSessionAfterRestart = await server.request("/v1/run/state?session=career");
    evidence.libraryAfterRestart = await server.ok("/v1/library");
    evidence.savedFilesAfterRestart = await server.files();
    evidence.listTerminal = await launch(t, server.url, ["--list-checkpoints"]).finish();
    // Gather the actual loss/restart baseline before requiring the new interface.
    assert.equal(evidence.oldSessionAfterRestart.status, 404);
    assert.equal(evidence.libraryAfterRestart.entries.length, 1);
    assert.deepEqual(evidence.afterSave, evidence.before);
    assert.deepEqual(evidence.mainAfterCommands, evidence.mainBefore);
    assert.deepEqual(evidence.savedFilesAfterRestart, evidence.savedFiles);
    assert.equal(evidence.listTerminal.code, 0);
    assert.match(evidence.listTerminal.stdout, /before-race/);
    await seed(server);
    const mainBeforeFork = await server.mainSnapshot();
    const otherBeforeFork = await server.snapshot("other");
    const resumed = launch(t, server.url, ["--checkpoint=before-race"]);
    await resumed.prompt();
    const id = resumed.stdout.match(/Resume this career with --session=(\S+)/)?.[1];
    assert.match(id, /^tui-[0-9a-f-]{36}$/);
    evidence.resumed = await server.snapshot(id);
    assert.deepEqual(evidence.resumed, evidence.before);
    await server.ok("/v1/session/fork", { checkpoint: "before-race", id: "oracle" });
    // The oracle becomes active; the terminal still acts on its printed id.
    resumed.send("rest");
    await resumed.prompt(2);
    await server.ok("/v1/run/action", { session: "oracle", action: "rest" });
    evidence.nextFromResumed = await server.snapshot(id);
    evidence.nextFromOracle = await server.snapshot("oracle");
    assert.deepEqual(evidence.nextFromResumed, evidence.nextFromOracle);
    assert.deepEqual(evidence.nextFromResumed, evidence.afterNextAction);
    assert.deepEqual(await server.mainSnapshot(), mainBeforeFork);
    assert.deepEqual(await server.snapshot("other"), otherBeforeFork);
    assert.deepEqual(await server.files(), evidence.savedFiles);
    evidence.resumedTerminal = await resumed.quit();
    assert.equal(evidence.resumedTerminal.code, 0);
    evidence.result = "pass";
  } finally { await record("restart", evidence); }
});

test("native create-only save preserves an earlier checkpoint and reports actual storage refusal", async (t) => {
  const server = await nativeServer(t);
  await seed(server);
  await server.ok("/v1/library/save", { name: "keep", session: "career", overwrite: false });
  const files = await server.files(), main = await server.mainSnapshot();
  const cli = launch(t, server.url, ["--session=career"]);
  await cli.prompt();
  cli.send("rest");
  await cli.prompt(2);
  const current = await server.snapshot("career");
  cli.send("save keep");
  await cli.prompt(3);
  assert.match(cli.stderr, /HTTP 409/);
  assert.deepEqual(await server.files(), files);
  // Force an actual create-only snapshot writer error in this test's private library.
  await fs.mkdir(path.join(server.cwd, ".uma-sim/library", ".tmp-" + server.pid + "-blocked.snapshot.json"));
  cli.send("save blocked");
  await cli.prompt(4);
  assert.match(cli.stderr, /HTTP 500.*storage error/);
  assert.doesNotMatch(cli.stdout, /Saved checkpoint/);
  assert.deepEqual(await server.snapshot("career"), current);
  assert.deepEqual(await server.mainSnapshot(), main);
  assert.deepEqual(await server.files(), files);
  const terminal = await cli.quit();
  assert.equal(terminal.code, 0);
  await record("native-save-refusals", { current, preservedFiles: files, terminal });
});

test("native missing and incompatible checkpoints refuse without replacement; compatibility advisories remain visible", async (t) => {
  const server = await nativeServer(t);
  await seed(server);
  await server.ok("/v1/library/save", { name: "keep", session: "career", overwrite: false });
  const metaPath = path.join(server.cwd, ".uma-sim/library/keep.meta.json");
  const original = JSON.parse(await fs.readFile(metaPath, "utf8"));
  const beforeSessions = await server.ok("/v1/sessions");
  const main = await server.mainSnapshot(), career = await server.snapshot("career");
  const missing = await launch(t, server.url, ["--checkpoint=missing"]).finish();
  assert.equal(missing.code, 1);
  assert.match(missing.stderr, /HTTP 404/);
  original.fingerprint.snapshotSchema = 999;
  await fs.writeFile(metaPath, JSON.stringify(original));
  const incompatibleFiles = await server.files();
  const incompatible = await launch(t, server.url, ["--checkpoint=keep"]).finish();
  assert.equal(incompatible.code, 1);
  assert.match(incompatible.stderr, /HTTP 422.*snapshot schema/);
  assert.deepEqual(await server.files(), incompatibleFiles);
  assert.deepEqual(await server.ok("/v1/sessions"), beforeSessions);
  assert.deepEqual(await server.mainSnapshot(), main);
  assert.deepEqual(await server.snapshot("career"), career);
  original.fingerprint.snapshotSchema = 1;
  original.fingerprint.coreVersion = "fixture-earlier-build";
  await fs.writeFile(metaPath, JSON.stringify(original));
  const advisoryFiles = await server.files();
  const compatible = launch(t, server.url, ["--checkpoint=keep"]);
  await compatible.prompt();
  assert.match(compatible.stderr, /Checkpoint advisory: checkpoint was saved by uma-sim-core fixture-earlier-build/);
  assert.deepEqual(await server.files(), advisoryFiles);
  const terminal = await compatible.quit();
  assert.equal(terminal.code, 0);
  await record("native-fork-refusals-advisory", { missing, incompatible, terminal });
});

for (const mode of ["save", "fork"]) {
  for (const fault of ["disconnect", "invalid JSON"]) {
    test("native committed " + mode + " with " + fault + " reports uncertainty and is not replayed", async (t) => {
      const server = await nativeServer(t);
      await seed(server);
      await server.ok("/v1/library/save", { name: "keep", session: "career", overwrite: false });
      const saved = await server.snapshot("career"), main = await server.mainSnapshot();
      const files = await server.files();
      const target = mode === "save" ? "/v1/library/save" : "/v1/session/fork";
      const proxy = await forwarder(t, server, target, fault);
      const cli = launch(t, proxy.url, [mode === "save" ? "--session=career" : "--checkpoint=keep"]);
      let terminal;
      if (mode === "save") {
        await cli.prompt(); cli.send("save uncertain"); await cli.prompt(2);
        assert.match(cli.stderr, /Checkpoint 'uncertain' may have been saved/);
        terminal = await cli.quit();
        assert.equal(terminal.code, 0);
        assert.deepEqual(JSON.parse((await server.files())["uncertain.snapshot.json"]), saved);
      } else {
        terminal = await cli.finish();
        assert.equal(terminal.code, 1);
        const id = proxy.records.find((r) => r.route === target).body.id;
        assert.match(terminal.stderr, new RegExp("--session=" + id));
        assert.deepEqual(await server.snapshot(id), saved);
        assert.deepEqual(await server.files(), files);
      }
      assert.match(terminal.stderr, /may have reached the server/);
      assert.equal(proxy.records.filter((r) => r.method === "POST").length, 1);
      assert.equal(proxy.records.find((r) => r.route === target).nativeStatus, 200);
      assert.deepEqual(await server.mainSnapshot(), main);
      assert.deepEqual(await server.snapshot("career"), saved);
      await record("native-" + mode + "-" + fault.replaceAll(" ", "-"), { terminal, traffic: proxy.records });
    });
  }
}
for (const mode of ["save", "fork"]) {
  for (const stop of ["EOF", "SIGINT"]) {
    test("native " + stop + " after committed " + mode + " preserves resumable state without replay", async (t) => {
      const server = await nativeServer(t);
      await seed(server);
      await server.ok("/v1/library/save", { name: "keep", session: "career", overwrite: false });
      const saved = await server.snapshot("career"), main = await server.mainSnapshot();
      const target = mode === "save" ? "/v1/library/save" : "/v1/session/fork";
      const proxy = await forwarder(t, server, target, "hold");
      const cli = launch(t, proxy.url, [mode === "save" ? "--session=career" : "--checkpoint=keep"]);
      if (mode === "save") { await cli.prompt(); cli.send("save interrupted"); }
      await until(() => proxy.records.some((r) => r.committed), () => "native write did not finish");
      if (stop === "EOF") cli.child.stdin.end(); else cli.child.kill("SIGINT");
      const terminal = await cli.finish();
      assert.equal(terminal.code, stop === "EOF" ? 0 : 130);
      assert.match(terminal.stderr, /may have reached the server/);
      const mutation = proxy.records.find((r) => r.route === target);
      assert.equal(mutation.nativeStatus, 200);
      assert.equal(proxy.records.filter((r) => r.method === "POST").length, 1);
      if (mode === "save") {
        assert.match(terminal.stderr, /Checkpoint 'interrupted'/);
        assert.deepEqual(JSON.parse((await server.files())["interrupted.snapshot.json"]), saved);
      } else {
        assert.match(terminal.stderr, new RegExp("--session=" + mutation.body.id));
        assert.deepEqual(await server.snapshot(mutation.body.id), saved);
      }
      assert.deepEqual(await server.mainSnapshot(), main);
      assert.deepEqual(await server.snapshot("career"), saved);
      await record("native-" + mode + "-" + stop, { terminal, traffic: proxy.records });
    });
  }
}

test.after(async () => {
  await record("provenance", {
    node: process.version, platform: process.platform, arch: process.arch,
    files: [
      { path: entry, sha256: digest(await fs.readFile(entry)) },
      { path: binary, sha256: digest(await fs.readFile(binary)) },
      { path: fileURLToPath(import.meta.url), sha256: digest(await fs.readFile(fileURLToPath(import.meta.url))) },
    ],
    backend: "real disposable Rust uma-sim-api processes, private filesystem library, pinned repository catalogs",
    faultInjection: "forwarded native HTTP responses are deliberately dropped, corrupted or held only in named fault cases",
  });
});
