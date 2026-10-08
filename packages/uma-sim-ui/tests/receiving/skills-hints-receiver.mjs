import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

// Native API + actual built application. All careers/checkpoints belong to this
// disposable receiver; authored checkpoint cases are not simulated game outcomes.
// node skills-hints-receiver.mjs DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]
// UMA_SKILLS_EXPECT=absent runs the historical baseline observation only.
const [distArg, outputArg, binaryArg, chromiumExecutable] = process.argv.slice(2);
assert(distArg && outputArg && binaryArg, "Expected DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]");
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../../../..");
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
let catalogMode = "native";
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
  if (!absent) {
    const before = await native("/v1/run/state");
    assert(before.status >= 400 && before.status < 500, "No career exists before the first read");
    const sessions = await json("/v1/sessions");
    const catalog = await json("/v1/catalog/skills");
    assert(catalog.items.length > 1000);
    const after = await native("/v1/run/state");
    assert.equal(after.status, before.status);
    assert.equal(after.text, before.text);
    assert.deepEqual(await json("/v1/sessions"), sessions);
    pass("Catalog is available before a career without creating a session", { catalogItems: catalog.items.length });
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
  if (absent) {
    assert.equal(await panel.count(), 0);
    const catalog = await native("/v1/catalog/skills");
    assert(!catalog.text.trim().startsWith('{"items":'));
    pass("Baseline retains skills/hints but has no active-career panel or JSON catalog", {
      learnedSkillIds: snapshot.state.learnedSkillIds, hintLevels: snapshot.state.hintLevels,
      unknownEndpointStatus: catalog.status, unknownEndpointBodyKind: "non-catalog HTML",
    });
    await page.screenshot({ path: path.join(output, "baseline-career.png"), fullPage: true });
  } else {
    await panel.waitFor();
    const catalog = await json("/v1/catalog/skills");
    assert(catalog.items.length > 1000);
    assert.equal(new Set(catalog.items.map(item => item.id)).size, catalog.items.length);
    const group = title => panel.getByRole("region", { name: title, exact: true });
    const learned = group("Learned skills");
    const hints = group("Skill hints");
    const other = group("Training & other hints");
    const firstId = snapshot.state.learnedSkillIds[0];
    const firstSkill = catalog.items.find(item => item.id === firstId);
    assert(firstSkill?.name);
    await learned.getByText(firstSkill.name, { exact: true }).waitFor();
    assert.equal(await learned.getByRole("listitem").count(), snapshot.state.learnedSkillIds.length);
    assert.equal(await hints.getByRole("listitem").count(), Object.keys(snapshot.state.hintLevels).length);
    await panel.screenshot({ path: path.join(output, "native-career-skills.png") });
    pass("Actual native career displays catalog names and every retained innate hint", {
      catalogItems: catalog.items.length, firstSkill, learned: snapshot.state.learnedSkillIds.length,
      hints: Object.keys(snapshot.state.hintLevels).length,
    });

    const before = await native("/v1/run/state");
    for (let i = 0; i < 3; i++) await json("/v1/catalog/skills");
    await learned.getByRole("list").focus();
    await page.keyboard.press("End");
    const rejected = await native("/v1/catalog/skills", {});
    assert.equal(rejected.status, 405);
    const after = await native("/v1/run/state");
    assert.equal(after.text, before.text);
    pass("Catalog reads, keyboard inspection and rejected POST preserve the full native snapshot and RNG", {
      stateSha256: sha(before.text), rngCalls: snapshot.rngCalls, postStatus: rejected.status,
    });

    const checkpoint = async (name, changes) => {
      const value = structuredClone(snapshot);
      Object.assign(value.state, changes);
      await json("/v1/library/import", { name, snapshot: value });
      receipt.authoredCheckpoints.push({ name, sha256: sha(JSON.stringify(value)), fields: Object.keys(changes) });
      await page.getByRole("button", { name: "Library & lab", exact: true }).click();
      const row = page.getByRole("row").filter({ has: page.getByText(name, { exact: true }) });
      await row.getByRole("button", { name: "Resume", exact: true }).click();
      await page.waitForFunction(() => !document.querySelector(".busy-overlay"));
      await page.getByRole("button", { name: "Run", exact: true }).click();
      await panel.waitFor();
      return value;
    };
    const unknownIds = Array.from({ length: 32 }, (_, i) => "skill:receiver-" + i);
    const rawHint = '<img src=x onerror="window.receiverExecuted=true">';
    const rich = await checkpoint("skills-rich-receiver", {
      learnedSkillIds: [firstId, ...unknownIds],
      hintLevels: { [firstId]: 2, "skill:receiver-hint": 5, speed: 1, generic: 0, [rawHint]: 3 },
      careerComplete: true, awaitingChoice: false, pendingEventOptions: [],
    });
    await learned.getByText(firstSkill.name, { exact: true }).waitFor();
    assert.equal(await learned.getByRole("listitem").count(), 33);
    await learned.getByText("skill:receiver-31", { exact: true }).waitFor();
    assert.equal(await hints.getByRole("listitem").count(), 2);
    assert.equal(await other.getByRole("listitem").count(), 3);
    await other.getByText("Speed training", { exact: true }).waitFor();
    await other.getByText(rawHint, { exact: true }).waitFor();
    assert.equal(await panel.locator("img, script").count(), 0);
    assert.equal(await page.evaluate(() => window.receiverExecuted), undefined);
    await learned.getByRole("list").focus();
    receipt.keyboardBefore = await learned.getByRole("list").evaluate(element => ({
      focused: document.activeElement === element, scrollTop: element.scrollTop,
      scrollHeight: element.scrollHeight, clientHeight: element.clientHeight,
    }));
    assert(receipt.keyboardBefore.focused);
    assert(receipt.keyboardBefore.scrollHeight > receipt.keyboardBefore.clientHeight);
    await page.keyboard.press("End");
    // Chromium scrolls the focused list on animation frames after the key event.
    await page.waitForFunction(() => {
      const list = document.querySelector('.skills-list[aria-label="Learned skills"]');
      return list && list.scrollTop > 0;
    }, undefined, { timeout: 2000 });
    receipt.keyboardAfter = await learned.getByRole("list").evaluate(element => element.scrollTop);
    const retained = await native("/v1/run/state");
    await panel.screenshot({ path: path.join(output, "retained-checkpoint-skills.png") });
    await page.setViewportSize({ width: 390, height: 844 });
    await panel.scrollIntoViewIfNeeded();
    assert(await panel.evaluate(element => element.scrollWidth <= element.clientWidth + 1));
    await panel.screenshot({ path: path.join(output, "retained-checkpoint-mobile.png") });
    assert.equal((await native("/v1/run/state")).text, retained.text);
    pass("Native imported completed checkpoint displays all 33 skills, literal fallbacks and separate generic/training hints", {
      learned: rich.state.learnedSkillIds.length, hintCount: 5, stateSha256: sha(retained.text),
      keyboardScroll: true, mobileWidth: 390, literalMarkup: true,
    });

    await page.setViewportSize({ width: 1440, height: 1000 });
    await checkpoint("skills-empty-receiver", { learnedSkillIds: [], hintLevels: {} });
    for (const text of ["No learned skills recorded.", "No skill hints recorded.", "No other hints recorded."]) {
      await panel.getByText(text, { exact: true }).waitFor();
    }
    assert.equal(await panel.getByRole("listitem").count(), 0);
    pass("Resuming another native checkpoint clears all prior learned skills and hints");

    catalogMode = "html";
    await checkpoint("skills-fallback-receiver", { learnedSkillIds: [firstId], hintLevels: { [firstId]: 1 } });
    await panel.getByRole("button", { name: "Retry names", exact: true }).waitFor();
    assert.equal(await learned.getByText(firstId, { exact: true }).count(), 1);
    const fallbackBefore = await native("/v1/run/state");
    catalogMode = "native";
    await panel.getByRole("button", { name: "Retry names", exact: true }).click();
    await learned.getByText(firstSkill.name, { exact: true }).waitFor();
    assert.equal((await native("/v1/run/state")).text, fallbackBefore.text);
    pass("Older-server HTML falls back to retained IDs; retry restores names without changing a career");

    catalogMode = "hold";
    await page.getByRole("button", { name: "Library & lab", exact: true }).click();
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await panel.getByText("Loading skill names…", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Library & lab", exact: true }).click();
    catalogMode = "native";
    pendingCatalogs.splice(0).forEach(resolve => resolve());
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await learned.getByText(firstSkill.name, { exact: true }).waitFor();
    assert.equal((await native("/v1/run/state")).text, fallbackBefore.text);
    pass("Leaving the view cancels a pending catalog request; a fresh view resolves normally");

    assert.deepEqual(receipt.pageErrors, []);
  }
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
