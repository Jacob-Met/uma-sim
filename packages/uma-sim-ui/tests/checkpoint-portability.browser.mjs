import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import { createServer, request as httpRequest } from "node:http";
import { readFile, readdir, mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repo = process.env.UMA_LAB_REPO || path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const stage = process.env.LAB_STAGE || "candidate";
assert(/^[a-z0-9-]+$/.test(stage), "Use a plain stage name");
const outputRoot = process.env.UMA_LAB_OUTPUT_ROOT || path.join(repo, ".checkpoint-portability");
const evidence = path.join(outputRoot, stage);
await mkdir(evidence, { recursive: true });
const storageA = await mkdtemp(path.join(evidence, "storage-a-"));
const storageB = await mkdtemp(path.join(evidence, "storage-b-"));
const { chromium } = await (process.env.UMA_LAB_PLAYWRIGHT_MODULE
  ? import(pathToFileURL(process.env.UMA_LAB_PLAYWRIGHT_MODULE))
  : import("playwright"));
const binary = process.env.UMA_LAB_API_BINARY || path.join(repo, "target/debug/uma-sim-api");
const dist = path.join(repo, "packages/uma-sim-ui/dist");
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const hash = async file => ({ sha256: digest(await readFile(file)) });
const sourceFiles = {};
for (const file of [
  "packages/uma-sim-ui/src/components/LibraryPanel.tsx",
  "packages/uma-sim-ui/src/state/labStore.ts",
  "uma-sim-core/src/api.rs",
  "uma-sim-core/src/career_lab.rs",
  "uma-sim-core/src/snapshot.rs",
  "uma-sim-core/src/engine.rs",
  "uma-sim-core/src/rng.rs",
]) sourceFiles[file] = await hash(path.join(repo, file));
const builtAssets = {};
for (const file of await readdir(path.join(dist, "assets"))) builtAssets[file] = await hash(path.join(dist, "assets", file));
const receipt = {
  stage, startedAt: new Date().toISOString(),
  source: execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
  sourceFiles, builtAssets, nativeBinary: await hash(binary),
  storageRoots: [storageA, storageB], boots: [], requests: [], cases: [], pageErrors: [],
};
let api, apiBase, apiLog, bootRecord, browser, context, page;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const unusedPort = async () => {
  const server = createServer();
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
};
const getJson = async pathname => {
  const response = await fetch(apiBase + pathname, { signal: AbortSignal.timeout(5000) });
  assert.equal(response.status, 200, pathname);
  return response.json();
};
const stopApi = async () => {
  if (!api) return;
  const child = api;
  api = null;
  if (child.exitCode === null && child.signalCode === null) {
    const exit = once(child, "exit");
    child.kill("SIGTERM");
    await Promise.race([exit, delay(5000).then(() => { throw new Error("Native API did not exit"); })]);
  }
  bootRecord.exitedAt = new Date().toISOString();
  bootRecord.exitCode = child.exitCode;
  bootRecord.exitSignal = child.signalCode;
  apiLog.end();
};
const boot = async (name, cwd) => {
  assert(!api, "Previous native API must exit before booting the next one");
  const port = await unusedPort();
  apiBase = "http://127.0.0.1:" + port;
  apiLog = createWriteStream(path.join(evidence, name + ".log"));
  api = spawn(binary, [String(port)], {
    cwd, env: { ...process.env, UMA_REPO_ROOT: repo }, stdio: ["ignore", "pipe", "pipe"],
  });
  let spawnError;
  api.on("error", error => { spawnError = error; });
  api.stdout.pipe(apiLog); api.stderr.pipe(apiLog);
  bootRecord = { name, pid: api.pid, cwd, endpoint: apiBase, startedAt: new Date().toISOString() };
  receipt.boots.push(bootRecord);
  for (let i = 0; i < 100; i++) {
    if (spawnError) throw spawnError;
    try {
      const response = await fetch(apiBase + "/v1/health", { signal: AbortSignal.timeout(500) });
      if (response.ok) {
        const health = await response.json();
        assert.equal(health.repoRoot, true);
        assert.equal(health.repoRootPath, repo);
        bootRecord.health = health;
        return;
      }
    } catch (error) {
      if (api.exitCode !== null || api.signalCode !== null) throw error;
    }
    await delay(50);
  }
  throw new Error("Native API did not become ready: " + name);
};
const mime = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".json": "application/json" };
const proxy = createServer(async (req, res) => {
  if (req.url.startsWith("/v1/")) {
    const upstream = httpRequest(apiBase + req.url, { method: req.method, headers: req.headers }, incoming => {
      res.writeHead(incoming.statusCode, incoming.headers); incoming.pipe(res);
    });
    upstream.on("error", error => { res.writeHead(502); res.end(String(error)); });
    req.pipe(upstream);
    return;
  }
  try {
    const pathname = decodeURIComponent(new URL(req.url, "http://localhost").pathname);
    const file = path.resolve(dist, "." + (pathname === "/" ? "/index.html" : pathname));
    if (!file.startsWith(dist + path.sep)) throw new Error("outside static root");
    res.writeHead(200, { "Content-Type": mime[path.extname(file)] || "application/octet-stream" });
    res.end(await readFile(file));
  } catch { res.writeHead(404); res.end("Not found"); }
});
await new Promise(resolve => proxy.listen(0, "127.0.0.1", resolve));
const uiBase = "http://127.0.0.1:" + proxy.address().port;
receipt.uiBase = uiBase;
const openBrowser = async () => {
  context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  context.setDefaultTimeout(15000);
  await context.route("**/*", route => {
    const hostname = new URL(route.request().url()).hostname;
    return ["127.0.0.1", "localhost"].includes(hostname) ? route.continue() : route.abort();
  });
  page = await context.newPage();
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  page.on("request", request => {
    const pathname = new URL(request.url()).pathname;
    if (!pathname.startsWith("/v1/")) return;
    let body = request.postData();
    if (pathname === "/v1/library/import" && body) {
      const imported = JSON.parse(body);
      body = { name: imported.name ?? null, snapshot: { sha256: digest(JSON.stringify(imported.snapshot)) } };
    }
    receipt.requests.push({ boot: bootRecord.name, method: request.method(), path: pathname, body });
  });
  await page.goto(uiBase);
  await page.getByRole("button", { name: "Start run", exact: true }).waitFor();
};
const libraryTab = async () => {
  await page.getByRole("button", { name: "Library & lab", exact: true }).click();
  await page.getByRole("heading", { name: "Career library", exact: true }).waitFor();
  await page.locator(".busy-overlay").waitFor({ state: "hidden" });
};
const rowFor = name => page.locator("tr").filter({ hasText: name }).filter({ has: page.getByRole("button", { name: "Resume", exact: true }) });
const clickPost = async (button, pathname) => {
  const [response] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === pathname && response.request().method() === "POST"),
    button.click(),
  ]);
  assert.equal(response.status(), 200, pathname + ": " + await response.text());
  return response;
};
const autoStep = async () => {
  const response = await clickPost(page.getByRole("button", { name: "Auto step", exact: true }), "/v1/run/auto");
  assert.equal(response.request().postDataJSON().policy, "bot");
  await page.locator(".busy-overlay").waitFor({ state: "hidden" });
};
const exportThroughBrowser = async (name, file) => {
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    rowFor(name).getByRole("link", { name: "Export", exact: true }).click(),
  ]);
  const filename = path.join(evidence, file);
  await download.saveAs(filename);
  return { filename, suggestedFilename: download.suggestedFilename(), snapshot: JSON.parse(await readFile(filename, "utf8")) };
};
const saveJson = async (name, value) => writeFile(path.join(evidence, name), JSON.stringify(value, null, 2) + "\n");
const persistedFiles = async () => {
  const directory = path.join(storageB, ".uma-sim/library");
  const files = {};
  for (const file of (await readdir(directory)).sort()) files[file] = await hash(path.join(directory, file));
  return files;
};
const expectedLabel = "Name (optional — generated from seed, turn, and scenario)";
try {
  browser = await chromium.launch({ headless: true, ...(process.env.UMA_LAB_CHROMIUM_EXECUTABLE ? { executablePath: process.env.UMA_LAB_CHROMIUM_EXECUTABLE } : {}) });
  receipt.browser = browser.version();
  await boot("origin", storageA);
  assert.equal((await getJson("/v1/library")).entries.length, 0);
  await openBrowser();
  await page.locator(".field").filter({ has: page.locator("label", { hasText: /^Race model$/ }) }).locator("select").selectOption("stub");
  await clickPost(page.getByRole("button", { name: "Start run", exact: true }), "/v1/run/start");
  for (let i = 0; i < 10; i++) await autoStep();
  const origin = await getJson("/v1/run/state");
  assert.equal(origin.meta.seed, 42);
  assert.equal(origin.state.turn, 11);
  assert.deepEqual(Object.keys(origin.rngState).sort(), ["addend", "v", "w", "x", "y", "z"]);
  const originalName = "portable-origin-" + Date.now().toString(36);
  await libraryTab();
  await page.getByPlaceholder("e.g. debut-fork (letters, digits, -_.  up to 64)").fill(originalName);
  await page.getByPlaceholder("short display name").fill("Portable reference");
  await page.getByPlaceholder("why this matters").fill("Browser portability receiving fixture");
  const saveResponse = await clickPost(page.getByRole("button", { name: "Save checkpoint", exact: true }), "/v1/library/save");
  receipt.originalEntry = (await saveResponse.json()).entry;
  await rowFor(originalName).waitFor();
  const exported = await exportThroughBrowser(originalName, "portable-checkpoint.json");
  assert.deepEqual(exported.snapshot, origin);
  assert(!JSON.stringify(exported.snapshot).includes(originalName));
  await saveJson("origin.json", origin);
  receipt.download = { suggestedFilename: exported.suggestedFilename, digest: await hash(exported.filename) };
  receipt.cases.push({ name: "actual browser export preserves complete snapshot and exact RNG words", pass: true });

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await autoStep();
  const expectedNext = await getJson("/v1/run/state");
  assert.notDeepEqual(expectedNext.rngState, origin.rngState);
  assert(expectedNext.rngCalls > origin.rngCalls);
  await saveJson("expected-next.json", expectedNext);
  await context.close(); context = null;
  await stopApi();

  await boot("import", storageB);
  assert.equal((await getJson("/v1/library")).entries.length, 0);
  assert.equal((await getJson("/v1/sessions")).sessions.length, 0);
  await openBrowser();
  await libraryTab();
  const nameInput = page.getByLabel(/^Name \(optional/);
  assert.equal(await nameInput.inputValue(), "");
  const label = (await nameInput.locator("..").innerText()).trim();
  await page.locator('input[type="file"]').setInputFiles(exported.filename);
  const importResponse = await clickPost(page.getByRole("button", { name: "Import", exact: true }), "/v1/library/import");
  const entry = (await importResponse.json()).entry;
  assert(entry);
  assert.equal(entry.name, "seed42-t11-ura");
  receipt.importedEntry = entry;
  await rowFor(entry.name).waitFor();
  const imported = await exportThroughBrowser(entry.name, "imported-roundtrip.json");
  assert.deepEqual(imported.snapshot, origin);
  receipt.cases.push({ name: "fresh-storage browser import and export preserve complete snapshot", pass: true });
  const labelAgrees = label === expectedLabel && entry.name !== originalName;
  receipt.naming = { renderedLabel: label, originalName, importedName: entry.name, expectedGeneratedName: "seed42-t11-ura", labelAgrees };
  receipt.cases.push({ name: "import label accurately describes the generated default name", pass: labelAgrees });
  await page.screenshot({ path: path.join(evidence, "import-name.png"), fullPage: true });
  const beforeRestart = await persistedFiles();
  assert.deepEqual(Object.keys(beforeRestart).sort(), ["seed42-t11-ura.meta.json", "seed42-t11-ura.snapshot.json"]);
  receipt.persistedBeforeRestart = beforeRestart;
  await context.close(); context = null;
  await stopApi();

  await boot("restart", storageB);
  assert.equal((await getJson("/v1/sessions")).sessions.length, 0);
  const afterRestart = await persistedFiles();
  assert.deepEqual(afterRestart, beforeRestart);
  receipt.persistedAfterRestart = afterRestart;
  receipt.cases.push({ name: "checkpoint files persist unchanged across a native API process restart", pass: true });
  await openBrowser();
  await libraryTab();
  await rowFor(entry.name).waitFor();
  await clickPost(rowFor(entry.name).getByRole("button", { name: "Resume", exact: true }), "/v1/library/load");
  const restored = await getJson("/v1/run/state");
  await saveJson("restored.json", restored);
  assert.deepEqual(restored, origin);
  receipt.cases.push({ name: "browser Resume restores the full native engine snapshot after restart", pass: true });
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("button", { name: "Auto step", exact: true }).waitFor();
  await autoStep();
  const actualNext = await getJson("/v1/run/state");
  await saveJson("actual-next.json", actualNext);
  assert.deepEqual(actualNext, expectedNext);
  receipt.cases.push({ name: "next actual browser Auto step reproduces complete state and RNG position", pass: true });
  receipt.rng = { origin: origin.rngState, expectedNext: expectedNext.rngState, actualNext: actualNext.rngState, callsBefore: origin.rngCalls, callsAfter: actualNext.rngCalls };
  assert.equal(new Set(receipt.boots.map(boot => boot.pid)).size, 3);
  assert.equal(receipt.pageErrors.length, 0);
  if (stage === "baseline") {
    assert(!labelAgrees && label.includes("file’s stored name"), "Expected original naming promise was not reproduced");
  } else assert(labelAgrees, "Import label does not describe the actual generated name");
  receipt.completed = true;
} catch (error) {
  receipt.failure = String(error);
  receipt.stack = error.stack;
  if (page && !page.isClosed()) await page.screenshot({ path: path.join(evidence, "failure.png"), fullPage: true }).catch(() => {});
  process.exitCode = 1;
} finally {
  if (context) await context.close();
  if (browser) await browser.close();
  await stopApi();
  await new Promise(resolve => proxy.close(resolve));
  receipt.completedAt = new Date().toISOString();
  await saveJson("receipt.json", receipt);
  console.log(JSON.stringify({ stage, completed: receipt.completed ?? false, cases: receipt.cases, naming: receipt.naming, pageErrors: receipt.pageErrors, failure: receipt.failure, evidence }, null, 2));
}
