import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import { createServer } from "node:net";
import { mkdir, mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
const base = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(process.env.UMA_LAB_REPO || path.join(base, "../.."));
const binary = path.resolve(process.env.UMA_LAB_BINARY || path.join(repo, "target/debug/uma-sim-api"));
assert(process.env.UMA_LAB_FIXTURE, "Set UMA_LAB_FIXTURE to a valid saved RunSnapshot JSON");
const fixture = path.resolve(process.env.UMA_LAB_FIXTURE);
const sourceRef = process.env.UMA_LAB_SOURCE_REF || "WORKTREE";
const expectMode = process.env.UMA_LAB_EXPECT || "preserved";
assert(["preserved", "loss"].includes(expectMode), "UMA_LAB_EXPECT must be preserved or loss");
const outputRoot = path.resolve(process.env.UMA_LAB_OUTPUT_ROOT || path.join(repo, ".branch-publication-receiving"));
await mkdir(outputRoot, { recursive: true });
const output = await mkdtemp(path.join(outputRoot, expectMode + "-"));
const cwd = path.join(output, "storage"); await mkdir(cwd);
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const hash = async file => ({ sha256: digest(await readFile(file)) });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const receipt = {
  source: { main: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repo, encoding: "utf8" }).trim(),
    binaryBuild: process.env.UMA_LAB_BINARY_BUILD || null, sourceRef,
    attribution: "Generated branch results in landed main." },
  startedAt: new Date().toISOString(), expectMode, output, cwd, sourceFiles: {}, nativeBinary: await hash(binary),
  fixture: await hash(fixture), boots: [], requests: [], cases: [],
};
for (const file of ["uma-sim-core/src/career_lab.rs", "uma-sim-core/src/api.rs", "Cargo.lock"]) {
  const bytes = sourceRef === "WORKTREE" ? await readFile(path.join(repo, file))
    : execFileSync("git", ["show", sourceRef + ":" + file], { cwd: repo });
  receipt.sourceFiles[file] = { sha256: digest(bytes) };
}
const live = new Set();
const unusedPort = async () => {
  const s = createServer(); s.listen(0, "127.0.0.1"); await once(s, "listening");
  const port = s.address().port; await new Promise(resolve => s.close(resolve)); return port;
};
const request = async (server, pathname, body) => {
  const start = Date.now();
  const response = await fetch(server.url + pathname, {
    method: body === undefined ? "GET" : "POST", headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(10000),
  });
  const data = await response.json();
  receipt.requests.push({ process: server.record.name, pid: server.child.pid, pathname,
    startMs: start, endMs: Date.now(), status: response.status });
  assert.equal(response.status, 200, JSON.stringify(data)); return data;
};
const boot = async name => {
  const port = await unusedPort();
  const child = spawn(binary, ["serve", "--port=" + port], { cwd, env: { ...process.env, UMA_REPO_ROOT: repo }, stdio: ["ignore", "pipe", "pipe"] });
  const record = { name, pid: child.pid, port, startedAt: new Date().toISOString() };
  const server = { child, record, url: "http://127.0.0.1:" + port, output: "" };
  live.add(server); receipt.boots.push(record);
  child.stdout.on("data", data => { server.output += data; }); child.stderr.on("data", data => { server.output += data; });
  let error; child.on("error", value => { error = value; });
  for (let attempt = 0; attempt < 150; attempt++) {
    if (error) throw error;
    if (child.exitCode !== null || child.signalCode !== null) throw new Error(server.output);
    try {
      const health = await fetch(server.url + "/v1/health", { signal: AbortSignal.timeout(100) });
      if (health.ok) { record.health = await health.json(); assert.equal(record.health.repoRootPath, repo); return server; }
    } catch {}
    await delay(20);
  }
  throw new Error("API failed to become healthy");
};
const stop = async server => {
  if (server.child.exitCode === null && server.child.signalCode === null) {
    const exited = once(server.child, "exit"); server.child.kill("SIGTERM");
    await Promise.race([exited, delay(5000).then(() => { throw new Error("API did not exit"); })]);
  }
  server.record.exitedAt = new Date().toISOString(); server.record.exitCode = server.child.exitCode;
  server.record.exitSignal = server.child.signalCode; live.delete(server);
  await writeFile(path.join(output, server.record.name + ".log"), server.output);
};
const hashes = async dir => {
  const result = {};
  const walk = async relative => {
    for (const ent of await readdir(path.join(dir, relative), { withFileTypes: true })) {
      const name = path.join(relative, ent.name);
      if (ent.isDirectory()) await walk(name); else result[name] = await hash(path.join(dir, name));
    }
  };
  await walk(""); return result;
};
try {
  const setup = await boot("setup");
  const snapshot = JSON.parse(await readFile(fixture, "utf8"));
  await request(setup, "/v1/library/import", { name: "process-fixture", snapshot });
  await writeFile(path.join(cwd, ".uma-sim/session.json"), JSON.stringify(snapshot, null, 2));
  const sentinel = await request(setup, "/v1/lab/branch", { checkpoint: "process-fixture", name: "earlier-sibling", policy: "default", maxActions: 1 });
  receipt.sentinel = sentinel.branch; await stop(setup);
  const before = await hashes(path.join(cwd, ".uma-sim"));
  const first = await boot("first-fresh-process"); const second = await boot("second-fresh-process");
  await delay(1000 - Date.now() % 1000 + 25);
  const a = await request(first, "/v1/lab/branch", { checkpoint: "process-fixture", name: "first-acknowledged-result", policy: "default", maxActions: 1 });
  const aPath = path.join(cwd, ".uma-sim/lab", a.branch.id + ".json");
  const aRaw = await readFile(aPath); await writeFile(path.join(output, "first-result-before-second.json"), aRaw);
  const b = await request(second, "/v1/lab/branch", { checkpoint: "process-fixture", name: "second-acknowledged-result", policy: "bot", maxActions: 2 });
  const afterRaw = await readFile(aPath); await writeFile(path.join(output, "first-id-after-second.json"), afterRaw);
  const fetchedA = await request(first, "/v1/lab/branch?id=" + a.branch.id);
  const listed = await request(second, "/v1/lab/branches");
  receipt.first = a; receipt.second = b; receipt.readFirstAfterSecond = fetchedA; receipt.listAfterSecond = listed;
  receipt.firstBefore = { sha256: digest(aRaw) }; receipt.firstAfter = { sha256: digest(afterRaw) };
  const after = await hashes(path.join(cwd, ".uma-sim"));
  receipt.protectedBefore = before; receipt.protectedAfter = Object.fromEntries(Object.keys(before).map(key => [key, after[key]]));
  const branchRequests = receipt.requests.filter(item => item.pathname === "/v1/lab/branch" && ["first-fresh-process", "second-fresh-process"].includes(item.process));
  const requestSeconds = new Set(branchRequests.flatMap(item => [item.startMs, item.endMs]).map(ms => Math.floor(ms / 1000)));
  const sameSecond = a.branch.startedAt === b.branch.startedAt && requestSeconds.size === 1;
  receipt.cases.push({ name: "two fresh native processes completed first branches in the same Unix second", pass: sameSecond });
  assert(sameSecond, "Timing unqualified; rerun in new storage");
  receipt.cases.push({ name: "independent process results have distinct persistent IDs", pass: a.branch.id !== b.branch.id });
  receipt.cases.push({ name: "first acknowledged result stays byte-identical after second write", pass: aRaw.equals(afterRaw) });
  receipt.cases.push({ name: "reading first ID returns first acknowledged result", pass: fetchedA.branch.name === a.branch.name });
  receipt.cases.push({ name: "prior sibling, checkpoint files, and CLI session stay byte-identical",
    pass: JSON.stringify(receipt.protectedBefore) === JSON.stringify(receipt.protectedAfter) });
  await stop(first); await stop(second);
  const restart = await boot("restart");
  const restarted = await request(restart, "/v1/lab/branches"); const resumedA = await request(restart, "/v1/lab/branch?id=" + a.branch.id);
  receipt.restartList = restarted; receipt.restartFirst = resumedA;
  receipt.cases.push({ name: "restart retains all three acknowledged results", pass: restarted.branches.length === 3 });
  receipt.cases.push({ name: "restart retains original payload at first result ID", pass: resumedA.branch.name === a.branch.name });
  await stop(restart);
  receipt.observedLoss = a.branch.id === b.branch.id && !aRaw.equals(afterRaw) && resumedA.branch.name === b.branch.name;
  if (expectMode === "loss") assert(receipt.observedLoss, "Expected baseline overwrite did not reproduce");
  else assert(receipt.cases.every(item => item.pass), "Branch result preservation regression");
} catch (error) { receipt.error = String(error.stack || error); process.exitCode = 1; }
finally {
  for (const server of [...live]) { try { await stop(server); } catch (error) { receipt.cleanupError = String(error); process.exitCode = 1; } }
  receipt.finishedAt = new Date().toISOString(); receipt.pass = receipt.cases.filter(item => item.pass).length; receipt.fail = receipt.cases.filter(item => !item.pass).length;
  await writeFile(path.join(output, "receipt.json"), JSON.stringify(receipt, null, 2) + "\n");
  console.log(JSON.stringify({ output, pass: receipt.pass, fail: receipt.fail, observedLoss: receipt.observedLoss,
    first: receipt.first?.branch?.id, second: receipt.second?.branch?.id, error: receipt.error }));
}
