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

// Actual built UI and native API. Every career and imported checkpoint belongs
// to this isolated receiver. Authored statuses are not claimed as earned outcomes.
// node conditions-titles-receiver.mjs DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]
// UMA_CONDITIONS_EXPECT=absent records the same current-main UI's missing panel.
const [distArg, outputArg, binaryArg, chromiumExecutable] = process.argv.slice(2);
assert(distArg && outputArg && binaryArg, "Expected DIST OUTPUT API_BINARY [CHROMIUM_EXECUTABLE]");
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../../../..");
const dist = path.resolve(distArg);
const output = path.resolve(outputArg);
const binary = path.resolve(binaryArg);
const absent = process.env.UMA_CONDITIONS_EXPECT === "absent";
await fs.mkdir(output);
const storage = await fs.mkdtemp(path.join(output, "native-storage-"));
const sha = value => createHash("sha256").update(value).digest("hex");
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const sourcePaths = [
  "packages/uma-sim-ui/src/App.tsx",
  "packages/uma-sim-ui/src/components/ConditionsPanel.tsx",
  "packages/uma-sim-ui/src/components/conditionsView.ts",
  "packages/uma-sim-ui/src/styles/conditions.css",
  "uma-sim-core/src/state.rs",
  "uma-sim-core/src/engine.rs",
  "uma-sim-core/src/config.rs",
  "uma-sim-core/src/api.rs",
];
const sourcePins = async () => Object.fromEntries(await Promise.all(
  sourcePaths.map(async name => [name, sha(await fs.readFile(path.join(repo, name)))]),
));
const distFiles = async (directory, prefix = "") => {
  const rows = [];
  for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
    const name = prefix + entry.name;
    if (entry.isDirectory()) rows.push(...await distFiles(path.join(directory, entry.name), name + "/"));
    else if (entry.isFile()) {
      const bytes = await fs.readFile(path.join(directory, entry.name));
      rows.push({ path: name, size: bytes.length, sha256: sha(bytes) });
    } else throw new Error("Built UI contains an unexpected non-file entry: " + name);
  }
  return rows.sort((a, b) => a.path.localeCompare(b.path));
};
const receipt = {
  startedAt: new Date().toISOString(),
  expectation: absent ? "baseline absence" : "received conditions and titles",
  sourceWorktreeStatus: execFileSync("git", ["status", "--porcelain"], { cwd: repo, encoding: "utf8" }),
  sourceHead: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repo, encoding: "utf8" }).trim(),
  workingSourceCustodyBefore: await sourcePins(),
  binary: { path: binary, sha256: sha(await fs.readFile(binary)) },
  distIndexSha256: sha(await fs.readFile(path.join(dist, "index.html"))),
  builtUiFiles: await distFiles(dist),
  custodyNote: "Working source hashes record custody; exact executed UI assets are pinned separately. Build provenance is supplied by the qualification record.",
  receiverSha256: sha(await fs.readFile(fileURLToPath(import.meta.url))),
  cases: [], authoredCheckpoints: [], requests: [], blockedExternalRequests: [],
  pageErrors: [], storage, providerCalls: 0,
};
const choosePort = async () => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1"); await once(server, "listening");
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve)); return port;
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
const native = async (pathname, body) => {
  const response = await fetch(nativeUrl + pathname, {
    method: body === undefined ? "GET" : "POST",
    headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(10000),
  });
  const text = await response.text();
  assert.equal(response.status, 200, pathname + ": " + text);
  return JSON.parse(text);
};
const checkpoint = async phase => {
  await fs.writeFile(path.join(output, "progress-" + String(receipt.cases.length).padStart(2, "0") + ".json"), JSON.stringify({
    phase, sourceHead: receipt.sourceHead, receiverSha256: receipt.receiverSha256,
    nativePid: receipt.nativePid, nativePort: receipt.nativePort, cases: receipt.cases,
  }, null, 2) + "\n", { flag: "wx" });
};
const pass = async (name, details = {}) => {
  receipt.cases.push({ name, pass: true, ...details });
  await checkpoint(name);
};
try {
  await checkpoint("starting native API");
  for (let i = 0; ; i++) {
    assert(i < 200 && child.exitCode === null && child.signalCode === null, "Own native API becomes ready");
    try {
      receipt.health = await native("/v1/health");
      assert.equal(receipt.health.repoRootPath, repo); break;
    } catch { await delay(25); }
  }
  const trainees = await native("/v1/catalog/trainees");
  assert(Array.isArray(trainees.items), "Native trainee catalog reports its items");
  const inheritedPortraitUrls = new Set(trainees.items.map(item => item.iconUrl).filter(url => typeof url === "string" && url));
  receipt.inheritedPortraitUrls = [...inheritedPortraitUrls].sort();
  server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, "http://127.0.0.1");
      if (url.pathname.startsWith("/v1/")) {
        let body = ""; for await (const bytes of req) body += bytes;
        const request = { method: req.method, path: url.pathname, query: url.search };
        receipt.requests.push(request);
        const response = await fetch(nativeUrl + url.pathname + url.search, {
          method: req.method, headers: body ? { "Content-Type": "application/json" } : {},
          body: body || undefined, signal: AbortSignal.timeout(10000),
        });
        request.status = response.status;
        const bytes = Buffer.from(await response.arrayBuffer());
        res.writeHead(response.status, { "Content-Type": response.headers.get("content-type") || "application/json" });
        res.end(bytes); return;
      }
      if (url.pathname === "/favicon.ico") { res.writeHead(204); res.end(); return; }
      const name = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname).slice(1);
      const file = path.resolve(dist, name);
      assert(file.startsWith(dist + path.sep), "Fixture stays within its own dist");
      const bytes = await fs.readFile(file);
      const type = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml" }[path.extname(file)] || "application/octet-stream";
      res.writeHead(200, { "Content-Type": type }); res.end(bytes);
    } catch (error) { if (!res.destroyed) { res.writeHead(500); res.end(String(error)); } }
  });
  server.listen(0, "127.0.0.1"); await once(server, "listening");
  const appUrl = "http://127.0.0.1:" + server.address().port;
  browser = await chromium.launch({ headless: true, ...(chromiumExecutable ? { executablePath: chromiumExecutable } : {}) });
  receipt.browserVersion = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, locale: "en-US" });
  await context.route("**/*", route => {
    if (new URL(route.request().url()).origin === appUrl) return route.continue();
    receipt.blockedExternalRequests.push(route.request().url()); return route.abort();
  });
  const page = await context.newPage(); page.setDefaultTimeout(10000);
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  await page.goto(appUrl);
  await page.getByRole("button", { name: "Start run", exact: true }).waitFor();
  await page.locator(".field").filter({ has: page.getByText("Seed", { exact: true }) }).locator("input").fill("421963");
  await page.locator(".field").filter({ has: page.getByText("Dialogue", { exact: true }) }).locator("select").selectOption("off");
  await page.getByRole("button", { name: "Start run", exact: true }).click();
  await page.getByRole("button", { name: "New run", exact: true }).waitFor();
  const initial = await native("/v1/run/state");
  await fs.writeFile(path.join(output, "native-initial-snapshot.json"), JSON.stringify(initial, null, 2) + "\n");
  const panel = page.getByRole("region", { name: /^Conditions & titles/ });
  if (!absent) {
    await panel.waitFor();
    assert.deepEqual(initial.state.statuses, []);
    await panel.getByText("No conditions or titles recorded.", { exact: true }).waitFor();
    await pass("Actual new career reports an empty collection without a false injury warning");
  }

  let fixtureBase = initial;
  if (!absent) {
    let raced = false;
    if (initial.state.phase === "MANDATORY_RACE") {
      await Promise.all([
        page.waitForResponse(response => new URL(response.url()).pathname === "/v1/run/action" && response.request().method() === "POST" && response.status() === 200),
        page.locator(".choice-list button.race").click(),
      ]);
      raced = true;
    }
    fixtureBase = await native("/v1/run/state");
    const available = await native("/v1/run/choices");
    assert(available.choices.some(choice => choice.id === "rest"), "The actual career now offers Rest before the injury fixture is authored");
    await fs.writeFile(path.join(output, "native-before-injury-fixture.json"), JSON.stringify(fixtureBase, null, 2) + "\n");
    await pass("Existing career choices reach an actual state offering Rest before the injury fixture is authored", {
      mandatoryRacePerformed: raced, phaseBefore: initial.state.phase, phaseAfter: fixtureBase.state.phase,
      turnBefore: initial.state.turn, turnAfter: fixtureBase.state.turn,
    });
  }
  let marker = 100123;
  const resumeFixture = async (name, changes) => {
    const snapshot = structuredClone(fixtureBase);
    Object.assign(snapshot.state, changes, { fans: marker++ });
    await native("/v1/library/import", { name, snapshot });
    receipt.authoredCheckpoints.push({
      name, sha256: sha(JSON.stringify(snapshot)), fields: Object.keys(changes),
      fansMarker: snapshot.state.fans, statuses: snapshot.state.statuses,
    });
    await fs.writeFile(path.join(output, name + ".json"), JSON.stringify(snapshot, null, 2) + "\n");
    await page.getByRole("button", { name: "Library & lab", exact: true }).click();
    const row = page.getByRole("row").filter({ has: page.getByText(name, { exact: true }) });
    await Promise.all([
      page.waitForResponse(response => new URL(response.url()).pathname === "/v1/library/load" && response.request().method() === "POST" && response.status() === 200),
      row.getByRole("button", { name: "Resume", exact: true }).click(),
    ]);
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await page.waitForFunction(expected => [...document.querySelectorAll(".meta-line")].some(element =>
      element.querySelector("span")?.textContent === "Fans" &&
      element.querySelector("strong")?.textContent === expected), snapshot.state.fans.toLocaleString("en-US"));
    const active = await native("/v1/run/state");
    assert.deepEqual(active.state.statuses, snapshot.state.statuses);
    return active;
  };
  const literal = '<img src=x onerror="window.statusExecuted=true">';
  const ids = [
    "injured", "LEFT-INJURY", "epithet:future_injury_win",
    "epithet:sim_g1_win", "epithet:sim_climax_win", "epithet:sim_ura_finale_win",
    "steady", literal, "", " \t", "status:" + "long".repeat(45),
    ...Array.from({ length: 34 }, (_, i) => "status:" + i), "injured",
  ];
  const rich = await resumeFixture("conditions-rich-receiver", {
    statuses: ids, careerComplete: false, awaitingChoice: false, pendingEventOptions: [],
  });
  if (absent) {
    assert.equal(await panel.count(), 0);
    await pass("Current-main UI omits all retained conditions and titles in an actually resumed native career", {
      retainedCount: rich.state.statuses.length, nativeInjury: rich.state.statuses.some(id => id.toLowerCase() === "injured" || id.toLowerCase().includes("injury")),
      statuses: rich.state.statuses,
    });
    await page.screenshot({ path: path.join(output, "baseline-career.png"), fullPage: true });
  } else {
    const assertRows = async expected => {
      assert.equal(await panel.getByRole("listitem").count(), expected.length);
      const counts = new Map(); for (const id of expected) counts.set(id, (counts.get(id) || 0) + 1);
      for (const [id, count] of counts) {
        const text = id.trim() === "" ? JSON.stringify(id) : id;
        assert.equal(await panel.getByText(text, { exact: true }).count(), count, "Exact retained ID: " + JSON.stringify(id));
      }
    };
    await panel.getByText("Training is blocked by injury", { exact: true }).waitFor();
    await assertRows(ids);
    for (const label of ["G1 win", "Climax win", "URA finale win"]) await panel.getByText(label, { exact: true }).waitFor();
    assert.equal(await panel.getByText("Simulator title", { exact: true }).count(), 3);
    await pass("Actual imported career shows all 46 entries, duplicate injury, exact title IDs and native injury guidance", {
      retainedCount: ids.length, titleCount: 3,
    });
    await panel.getByText(literal, { exact: true }).waitFor();
    assert.equal(await panel.locator("img, script").count(), 0);
    assert.equal(await page.evaluate(() => window.statusExecuted), undefined);
    await pass("Markup-like IDs, blank IDs and unknown labels remain inert visible data");

    const beforeInspection = await native("/v1/run/state");
    const requestCount = receipt.requests.length;
    await panel.getByRole("list").focus(); await page.keyboard.press("End");
    await panel.getByText("status:33", { exact: true }).scrollIntoViewIfNeeded();
    assert.deepEqual(await native("/v1/run/state"), beforeInspection);
    assert.equal(receipt.requests.slice(requestCount).filter(request => request.method !== "GET").length, 0);
    await pass("Keyboard reading and long-list scrolling preserve the complete career snapshot and RNG", {
      snapshotSha256: sha(JSON.stringify(beforeInspection)), rngCalls: beforeInspection.rngCalls,
    });
    await panel.getByRole("list").evaluate(element => { element.scrollTop = 0; });
    await panel.screenshot({ path: path.join(output, "conditions-desktop.png") });

    const recoveryChoices = await native("/v1/run/choices");
    assert(recoveryChoices.choices.some(choice => choice.id === "rest"), "Rest remains available in the authored native fixture");
    await Promise.all([
      page.waitForResponse(response => new URL(response.url()).pathname === "/v1/run/action" && response.request().method() === "POST" && response.status() === 200),
      page.locator(".choice-list button.rest").filter({ hasText: /\bRest\b/ }).click(),
    ]);
    await panel.getByText("Training is blocked by injury", { exact: true }).waitFor({ state: "hidden" });
    const recovered = await native("/v1/run/state");
    const remaining = ids.filter(id => id.toLowerCase() !== "injured" && !id.toLowerCase().includes("injury"));
    assert.deepEqual(recovered.state.statuses, remaining);
    await assertRows(remaining);
    assert(recovered.state.turn > rich.state.turn);
    await fs.writeFile(path.join(output, "native-after-rest.json"), JSON.stringify(recovered, null, 2) + "\n");
    await pass("Existing Rest action removes every native injury match and refreshes the panel while retaining all other entries", {
      turnBefore: rich.state.turn, turnAfter: recovered.state.turn,
      remainingCount: recovered.state.statuses.length, statuses: recovered.state.statuses,
    });

    await resumeFixture("conditions-completed-receiver", {
      statuses: ["injured", "epithet:sim_g1_win"], careerComplete: true,
      awaitingChoice: false, pendingEventOptions: [],
    });
    await panel.getByText("Injury retained in this completed career", { exact: true }).waitFor();
    assert.equal(await panel.getByText("Rest clears injury conditions when that choice is available.", { exact: true }).count(), 0);
    await pass("Completed career keeps its injury record without offering recovery guidance for an ended run");

    await resumeFixture("conditions-empty-receiver", { statuses: [], careerComplete: true });
    await panel.getByText("No conditions or titles recorded.", { exact: true }).waitFor();
    assert.equal(await panel.getByRole("listitem").count(), 0);
    assert.equal(await panel.locator(".conditions-injury-note").count(), 0);
    await pass("Switching to an empty retained career removes prior conditions and titles");

    await resumeFixture("conditions-mobile-receiver", {
      statuses: ids, careerComplete: false, awaitingChoice: false, pendingEventOptions: [],
    });
    await context.setDefaultTimeout(10000);
    await page.setViewportSize({ width: 390, height: 844 });
    await assertRows(ids);
    const geometry = await panel.evaluate(element => ({
      clientWidth: element.clientWidth, scrollWidth: element.scrollWidth,
      left: element.getBoundingClientRect().left, right: element.getBoundingClientRect().right,
      viewport: window.innerWidth,
    }));
    assert(geometry.left >= 0 && geometry.right <= geometry.viewport, "Panel fits the phone viewport");
    assert(geometry.scrollWidth <= geometry.clientWidth + 1, "Long IDs do not widen the panel");
    await panel.getByText("status:33", { exact: true }).scrollIntoViewIfNeeded();
    await panel.getByRole("list").evaluate(element => { element.scrollTop = 0; });
    await panel.screenshot({ path: path.join(output, "conditions-phone.png") });
    await pass("Phone layout wraps long literal IDs and keeps every retained row reachable", geometry);
  }
  assert.deepEqual(receipt.pageErrors, []);
  const unexpectedOffOrigin = receipt.blockedExternalRequests.filter(url => !inheritedPortraitUrls.has(url));
  assert.deepEqual(unexpectedOffOrigin, [], "No off-origin attempt outside the existing native catalog portraits");
  await pass("Actual browser has no page errors; only inherited catalog portrait requests are attempted and all remain blocked", {
    blockedInheritedPortraitRequests: receipt.blockedExternalRequests.length,
    unexpectedOffOrigin: unexpectedOffOrigin.length,
  });
} catch (error) {
  receipt.error = { name: error.name, message: error.message, stack: error.stack };
  process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (server) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
  if (child.exitCode === null && child.signalCode === null) {
    child.kill("SIGTERM");
    await Promise.race([once(child, "exit"), delay(1500)]);
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  }
  receipt.workingSourceCustodyAfter = await sourcePins();
  receipt.sourceUnchanged = JSON.stringify(receipt.workingSourceCustodyBefore) === JSON.stringify(receipt.workingSourceCustodyAfter);
  receipt.binaryUnchanged = receipt.binary.sha256 === sha(await fs.readFile(binary));
  if (!receipt.sourceUnchanged || !receipt.binaryUnchanged) process.exitCode = 1;
  receipt.finishedAt = new Date().toISOString();
  receipt.exitCode = process.exitCode || 0;
  await fs.writeFile(path.join(output, "native-stdout.log"), nativeStdout);
  await fs.writeFile(path.join(output, "native-stderr.log"), nativeStderr);
  await fs.writeFile(path.join(output, "receipt.json"), JSON.stringify(receipt, null, 2) + "\n");
  console.log(JSON.stringify({
    receipt: path.join(output, "receipt.json"), expectation: receipt.expectation,
    cases: receipt.cases.length, exitCode: receipt.exitCode,
    sourceUnchanged: receipt.sourceUnchanged, binaryUnchanged: receipt.binaryUnchanged,
    error: receipt.error || null,
  }, null, 2));
}
