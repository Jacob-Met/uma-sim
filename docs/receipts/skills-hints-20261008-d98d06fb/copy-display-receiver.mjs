import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "/tmp/hamon-product-d98d06fb-20261008/uma-sim/packages/uma-sim-ui/node_modules/playwright/index.mjs";

// Native API + actual built application. All careers/checkpoints belong to this
// disposable receiver; authored checkpoint cases are not simulated game outcomes.
// node skills-hints-receiver.mjs DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]
// UMA_SKILLS_EXPECT=absent runs the historical baseline observation only.
const [distArg, outputArg, binaryArg, chromiumExecutable] = process.argv.slice(2);
assert(distArg && outputArg && binaryArg, "Expected DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]");
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = "/tmp/hamon-product-d98d06fb-20261008/receiving-copy-v2";
const dist = path.resolve(distArg);
const output = path.resolve(outputArg);
const binary = path.resolve(binaryArg);
const absent = process.env.UMA_SKILLS_EXPECT === "absent";
await fs.mkdir(output, { recursive: true });
const storage = await fs.mkdtemp(path.join(output, "native-storage-"));
const sha = value => createHash("sha256").update(value).digest("hex");
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const receipt = {
  startedAt: new Date().toISOString(), expectation: absent ? "baseline absence" : "received",
  sourceHead: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repo, encoding: "utf8" }).trim(),
  binary: { path: binary, sha256: sha(await fs.readFile(binary)) },
  sourceFiles: {}, cases: [], requests: [], blockedExternalRequests: [], pageErrors: [],
  authoredCheckpoints: [], storage, providerCalls: 0,
};
for (const name of [
  "uma-sim-core/src/catalog/skill.rs", "uma-sim-core/src/api.rs",
  "packages/uma-sim-ui/src/components/SkillsPanel.tsx",
  "packages/uma-sim-ui/src/components/skillsView.ts",
]) {
  if (absent && name.startsWith("packages/")) receipt.sourceFiles[name] = null;
  else receipt.sourceFiles[name] = sha(absent
    ? execFileSync("git", ["show", "HEAD:" + name], { cwd: repo })
    : await fs.readFile(path.join(repo, name)));
}
receipt.builtUiIndexSha256 = sha(await fs.readFile(path.join(dist, "index.html")));
receipt.receiverSha256 = sha(await fs.readFile(fileURLToPath(import.meta.url)));
const choosePort = async () => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1"); await once(server, "listening");
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
};
const port = await choosePort();
const nativeUrl = "http://127.0.0.1:" + port;
const env = { ...process.env, UMA_REPO_ROOT: repo };
delete env.UMA_POLICY_CMD;
const child = spawn(binary, [String(port)], { cwd: storage, env, stdio: ["ignore", "pipe", "pipe"] });
receipt.nativePid = child.pid; receipt.nativePort = port;
let nativeStdout = "", nativeStderr = "", browser, server;
child.stdout.on("data", bytes => { nativeStdout += bytes; });
child.stderr.on("data", bytes => { nativeStderr += bytes; });
const native = async (pathname, body, method = body === undefined ? "GET" : "POST") => {
  const response = await fetch(nativeUrl + pathname, {
    method, headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(10000),
  });
  const text = await response.text();
  return { status: response.status, text, json: () => JSON.parse(text) };
};
const json = async (pathname, body) => {
  const reply = await native(pathname, body);
  assert.equal(reply.status, 200, pathname + ": " + reply.text);
  return reply.json();
};
let catalogMode = "html";
const pendingCatalogs = [];
const serve = async (req, res) => {
  try {
    const url = new URL(req.url, "http://127.0.0.1");
    if (url.pathname.startsWith("/v1/")) {
      let body = "";
      for await (const bytes of req) body += bytes;
      const request = { method: req.method, path: url.pathname, query: url.search, at: new Date().toISOString() };
      receipt.requests.push(request);
      if (url.pathname === "/v1/catalog/skills" && catalogMode !== "native") {
        request.fixture = catalogMode;
        if (catalogMode === "hold") {
          await new Promise(resolve => pendingCatalogs.push(resolve));
          if (res.destroyed) return;
        } else if (catalogMode === "html") {
          res.writeHead(200, { "Content-Type": "text/html" }); res.end("<html>Old server fallback</html>"); return;
        } else {
          res.writeHead(503, { "Content-Type": "application/json" }); res.end('{"error":"Receiver catalog unavailable"}'); return;
        }
      }
      const response = await fetch(nativeUrl + url.pathname + url.search, {
        method: req.method, headers: body ? { "Content-Type": "application/json" } : {},
        body: body || undefined, signal: AbortSignal.timeout(10000),
      });
      const bytes = Buffer.from(await response.arrayBuffer());
      request.status = response.status;
      if (!res.destroyed) {
        res.writeHead(response.status, { "Content-Type": response.headers.get("content-type") || "application/json" });
        res.end(bytes);
      }
      return;
    }
    const name = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname).slice(1);
    const file = path.resolve(dist, name);
    assert(file.startsWith(dist + path.sep), "Static fixture stays within its own dist");
    const bytes = await fs.readFile(file);
    const mime = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml" }[path.extname(file)] || "application/octet-stream";
    res.writeHead(200, { "Content-Type": mime }); res.end(bytes);
  } catch (error) {
    if (!res.destroyed) { res.writeHead(500); res.end(String(error)); }
  }
};
const pass = (name, details = {}) => receipt.cases.push({ name, pass: true, ...details });
try {
  for (let i = 0; ; i++) {
    assert(i < 200 && child.exitCode === null && child.signalCode === null, "Own native API becomes ready");
    try {
      const health = await json("/v1/health");
      assert.equal(health.repoRootPath, repo);
      receipt.health = health; break;
    } catch { await delay(25); }
  }
  server = http.createServer((req, res) => { void serve(req, res); });
  server.listen(0, "127.0.0.1"); await once(server, "listening");
  const appUrl = "http://127.0.0.1:" + server.address().port;
  browser = await chromium.launch({ headless: true, ...(chromiumExecutable ? { executablePath: chromiumExecutable } : {}) });
  receipt.browserVersion = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  await context.route("**/*", route => {
    if (new URL(route.request().url()).origin === appUrl) return route.continue();
    receipt.blockedExternalRequests.push(route.request().url());
    return route.abort();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(10000);
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  await page.goto(appUrl);
  await page.getByRole("button", { name: "Start run", exact: true }).waitFor();
  const seedField = page.locator(".field").filter({ has: page.getByText("Seed", { exact: true }) }).locator("input");
  await seedField.fill("421963");
  await page.locator(".field").filter({ has: page.getByText("Dialogue", { exact: true }) }).locator("select").selectOption("off");
  await page.getByRole("button", { name: "Start run", exact: true }).click();
  await page.getByRole("button", { name: "New run", exact: true }).waitFor();
  const snapshot = await json("/v1/run/state");
  await fs.writeFile(path.join(output, "native-initial-snapshot.json"), JSON.stringify(snapshot, null, 2) + "\n");
  assert(snapshot.state.learnedSkillIds.length > 0);
  assert(Object.keys(snapshot.state.hintLevels).length > 0);
  const panel = page.getByRole("region", { name: "Skills & hints", exact: true });
  await panel.waitFor();
  await panel.getByRole("button", { name: "Retry names", exact: true }).waitFor();
  const expectedRows = snapshot.state.learnedSkillIds.length + Object.keys(snapshot.state.hintLevels).length;
  assert.equal(await panel.getByText("Name unavailable", { exact: true }).count(), expectedRows);
  assert.equal(await panel.getByText("Name not in catalog", { exact: true }).count(), 0);
  await panel.getByText(snapshot.state.learnedSkillIds[0], { exact: true }).waitFor();
  const before = await native("/v1/run/state");
  await panel.screenshot({ path: path.join(output, "name-unavailable-built-display.png") });
  const after = await native("/v1/run/state");
  assert.equal(after.text, before.text);
  assert.deepEqual(receipt.pageErrors, []);
  pass("Built application shows accurate name-unavailable row text during catalog failure", {
    expectedRows, actualText: "Name unavailable", oldTextAbsent: true,
    stateSha256: sha(before.text), globalStatusProvidesFailureDetail: true,
    scope: "One copy-refinement display control; no broader behavioral suite rerun"
  });
  receipt.passed = true;
} catch (error) {
  receipt.passed = false; receipt.failure = String(error?.stack || error);
  process.exitCode = 1;
} finally {
  pendingCatalogs.splice(0).forEach(resolve => resolve());
  if (browser) await browser.close();
  if (server) {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
  if (child.exitCode === null && child.signalCode === null) {
    const closed = once(child, "exit"); child.kill("SIGTERM"); await closed;
  }
  receipt.finishedAt = new Date().toISOString();
  receipt.nativeExit = { code: child.exitCode, signal: child.signalCode };
  await fs.writeFile(path.join(output, "native.stdout.txt"), nativeStdout);
  await fs.writeFile(path.join(output, "native.stderr.txt"), nativeStderr);
  await fs.writeFile(path.join(output, "receiving.json"), JSON.stringify(receipt, null, 2) + "\n");
  console.log(JSON.stringify({
    passed: receipt.passed, cases: receipt.cases.length, output, failure: receipt.failure,
    nativeBinarySha256: receipt.binary.sha256, providerCalls: receipt.providerCalls,
  }));
}
