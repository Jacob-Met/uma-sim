// Optional receiving: built baseline/candidate React UI + an isolated real native API.
// Provide UMA_SETUP_CHROMIUM and UMA_SETUP_API_BASE; no existing browser/session is used.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";

const [baselineDistArg, candidateDistArg, outputArg] = process.argv.slice(2);
assert(baselineDistArg && candidateDistArg && outputArg,
  "usage: node tests/run-setup.browser.mjs BASELINE_DIST CANDIDATE_DIST NEW_EVIDENCE_DIR");
const baselineDist = path.resolve(baselineDistArg);
const candidateDist = path.resolve(candidateDistArg);
const output = path.resolve(outputArg);
const apiBase = new URL(process.env.UMA_SETUP_API_BASE);
assert.equal(apiBase.hostname, "127.0.0.1", "Use the receiver's private native API");
assert.equal(apiBase.protocol, "http:");
const chrome = process.env.UMA_SETUP_CHROMIUM;
assert(chrome, "Provide an already installed Chromium executable");
await mkdir(output);
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const receipt = {
  startedAt: new Date().toISOString(),
  nativeApiBase: apiBase.href, browserExecutable: chrome,
  baselineDist, candidateDist, checks: [], requests: [], pageErrors: [],
  blockedExternalRequests: [], browserInstances: [], artifacts: {},
};
let dist = baselineDist;
let phase = "baseline";
let browser;
let page;
let server;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function until(condition, label) {
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    if (await condition()) return;
    await delay(40);
  }
  throw new Error("Timed out: " + label);
}
async function json(route) {
  const response = await fetch(new URL(route, apiBase), { signal: AbortSignal.timeout(10000) });
  const text = await response.text();
  assert.equal(response.status, 200, route + ": " + text.slice(0, 300));
  return JSON.parse(text);
}
async function artifact(name, bytes) {
  await writeFile(path.join(output, name), bytes);
  receipt.artifacts[name] = { bytes: Buffer.byteLength(bytes), sha256: digest(bytes) };
}
async function builtAssets(directory) {
  const result = {};
  for (const name of ["index.html", ...(await readdir(path.join(directory, "assets"))).map((n) => "assets/" + n)]) {
    const bytes = await readFile(path.join(directory, name));
    result[name] = { bytes: bytes.length, sha256: digest(bytes) };
  }
  return result;
}
const mime = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml" };
const postCount = () => receipt.requests.filter((r) => r.method === "POST").length;
const field = (label, selector = "select") =>
  page.locator(".grid-setup > .field").filter({
    has: page.locator("label", { hasText: new RegExp("^" + label + "$") }),
  }).locator(selector);
async function screenshot(name) {
  const bytes = await page.screenshot({ fullPage: true });
  await artifact(name, bytes);
}
async function launch() {
  browser = await chromium.launch({
    executablePath: chrome, headless: true,
    args: ["--disable-dev-shm-usage"], timeout: 30000,
  });
  receipt.browserInstances.push({ version: browser.version(), launchedAt: new Date().toISOString() });
}
async function freshPage(base) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, acceptDownloads: true });
  await context.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    if (url.origin === base || url.protocol === "blob:" || url.protocol === "data:") await route.continue();
    else {
      receipt.blockedExternalRequests.push(route.request().url());
      await route.abort();
    }
  });
  page = await context.newPage();
  page.on("pageerror", (error) => receipt.pageErrors.push({ phase, message: String(error) }));
  await page.goto(base);
  await page.locator(".banner.ok").filter({ hasText: "Connected" }).waitFor();
  await until(() => page.getByRole("button", { name: "Start run", exact: true }).isEnabled(), "form bootstrap");
  return context;
}
async function download(name) {
  const event = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download setup", exact: true }).click();
  const saved = await event;
  assert.equal(saved.suggestedFilename(), "uma-sim-setup-v1.json");
  assert.equal(await saved.failure(), null);
  await saved.saveAs(path.join(output, name));
  const bytes = await readFile(path.join(output, name));
  receipt.artifacts[name] = { bytes: bytes.length, sha256: digest(bytes) };
  return bytes;
}

try {
  receipt.assets = {
    baseline: await builtAssets(baselineDist),
    candidate: await builtAssets(candidateDist),
  };
  const health = await json("/v1/health");
  assert.equal(health.repoRoot, true, "Native data package must be available");
  receipt.nativeHealth = health;
  const catalogs = Object.fromEntries(await Promise.all(
    ["scenarios", "trainees", "supports", "factors"].map(async (kind) => [kind, (await json("/v1/catalog/" + kind)).items]),
  ));
  await artifact("native-catalogs.json", JSON.stringify(catalogs, null, 2) + "\n");
  assert(catalogs.trainees.length >= 2);
  const selectedTraineeIndex = 1;
  const trainee = catalogs.trainees[selectedTraineeIndex].name;
  const scenario = (catalogs.scenarios[1] ?? catalogs.scenarios[0]).id;
  const allowedSupports = catalogs.supports.filter((s) => s.name.toLowerCase() !== trainee.toLowerCase()).slice(0, 200);
  assert(allowedSupports.length >= 6);
  const deckOrder = [5, 1, 4, 0, 3, 2];
  const kinds = ["blue", "pink", "white", "green", "race"];
  const kindMap = { blue: "blue", pink: "pink", white: "skill", green: "scenario", race: "race" };
  const factors = Object.fromEntries(kinds.map((k) => [
    k, catalogs.factors.find((f) => (f.kind ?? "").toLowerCase() === kindMap[k]),
  ]));
  receipt.catalogCounts = Object.fromEntries(Object.entries(catalogs).map(([k, items]) => [k, items.length]));
  receipt.availableSparkKinds = kinds.filter((k) => factors[k]);
  const ancestorKeys = ["parentA", "gpA1", "gpA2", "parentB", "gpB1", "gpB2"];
  const otherTrainee = catalogs.trainees.find((t) => t.name.toLowerCase() !== trainee.toLowerCase()).name;
  const expected = {
    seed: "9223372036854775807", scenario, trainee, speed: 7,
    dialogue: "full", raceModel: "stub", policy: "default",
    deck: deckOrder.map((i) => allowedSupports[i].id),
    legacyEnabled: false, legacyTree: {}, compatibilityScore: 413,
  };
  for (let i = 0; i < ancestorKeys.length; i++) {
    expected.legacyTree[ancestorKeys[i]] = {
      uma: ancestorKeys[i].startsWith("parent") ? otherTrainee : trainee,
      ...Object.fromEntries(kinds.map((k, j) => [k, {
        factorId: factors[k]?.id ?? "",
        stars: factors[k] ? (i + j) % 3 + 1 : (k === "blue" || k === "pink" ? 3 : 1),
      }])),
    };
  }
  if (factors.race) expected.legacyTree.gpB2.race = { factorId: "", stars: 3 };

  server = createServer(async (req, res) => {
    try {
      if (req.url.startsWith("/v1/")) {
        const chunks = [];
        for await (const chunk of req) chunks.push(chunk);
        const body = Buffer.concat(chunks).toString("utf8");
        const record = { phase, method: req.method, path: req.url, body, at: new Date().toISOString() };
        receipt.requests.push(record);
        const upstream = await fetch(new URL(req.url, apiBase), {
          method: req.method,
          headers: body ? { "content-type": "application/json" } : undefined,
          body: body || undefined, signal: AbortSignal.timeout(15000),
        });
        const bytes = Buffer.from(await upstream.arrayBuffer());
        record.responseStatus = upstream.status;
        record.responseBody = bytes.toString("utf8");
        res.writeHead(upstream.status, { "content-type": upstream.headers.get("content-type") ?? "application/json" });
        res.end(bytes);
        return;
      }
      const pathname = decodeURIComponent(new URL(req.url, "http://localhost").pathname);
      const target = path.resolve(dist, "." + (pathname === "/" ? "/index.html" : pathname));
      if (!target.startsWith(dist + path.sep)) { res.writeHead(403); res.end(); return; }
      const bytes = await readFile(target);
      res.writeHead(200, { "content-type": mime[path.extname(target)] ?? "application/octet-stream" });
      res.end(bytes);
    } catch (error) {
      res.writeHead(error.code === "ENOENT" ? 404 : 502);
      res.end(String(error));
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = "http://127.0.0.1:" + server.address().port;
  receipt.uiBase = base;
  await launch();

  let context = await freshPage(base);
  await field("Seed", "input").fill("9007199254740993");
  await field("Policy").selectOption("default");
  const baselineVisibleSeed = await field("Seed", "input").inputValue();
  await screenshot("baseline-form.png");
  assert.equal(await page.getByRole("button", { name: "Download setup", exact: true }).count(), 0);
  await page.getByRole("button", { name: "Start run", exact: true }).click();
  await until(() => receipt.requests.some((r) => r.phase === "baseline" && r.path === "/v1/run/start" && r.responseStatus), "baseline native Start");
  const baselineStart = receipt.requests.find((r) => r.phase === "baseline" && r.path === "/v1/run/start");
  const baselineRequest = JSON.parse(baselineStart.body);
  assert.equal(baselineRequest.seed, 9007199254740992);
  assert.equal(baselineRequest.policy, "bot");
  assert.equal(baselineStart.responseStatus, 200);
  receipt.checks.push({
    name: "actual baseline counterexamples", passed: true, expectedProductFailures: [
      "No setup file controls", "Integer text rounded before native request", "Selected form policy overwritten",
    ],
    requestedSeed: "9007199254740993", displayedSeed: baselineVisibleSeed,
    receivedSeed: baselineRequest.seed, selectedPolicy: "default", receivedPolicy: baselineRequest.policy,
  });
  await context.close();

  dist = candidateDist;
  phase = "candidate-author";
  context = await freshPage(base);
  const beforePosts = postCount();
  const beforeState = await fetch(new URL("/v1/run/state", apiBase)).then((r) => r.text());
  await artifact("native-state-before-files.json", beforeState);
  await page.locator(".trainee-card").nth(selectedTraineeIndex).click();
  await field("Scenario").selectOption(scenario);
  await page.getByLabel("Seed", { exact: true }).fill(expected.seed);
  await field("Speed", "input").fill(String(expected.speed));
  await field("Dialogue").selectOption(expected.dialogue);
  await field("Race model").selectOption(expected.raceModel);
  await field("Policy").selectOption(expected.policy);
  for (const i of deckOrder) await page.locator(".multi input[type=checkbox]").nth(i).check();
  const legacy = () => page.locator(".legacy-toggle input[type=checkbox]");
  await legacy().check();
  for (let i = 0; i < ancestorKeys.length; i++) {
    const node = page.locator(".legacy-node").nth(i);
    const data = expected.legacyTree[ancestorKeys[i]];
    await node.locator(".field select").selectOption(data.uma);
    for (let j = 0; j < kinds.length; j++) {
      const k = kinds[j];
      if (!factors[k]) continue;
      const row = node.locator(".spark-row").nth(j);
      await row.locator("select").nth(0).selectOption(factors[k].id);
      await row.locator("select").nth(1).selectOption(String(data[k].stars));
      if (!data[k].factorId) await row.locator("select").nth(0).selectOption("");
    }
  }
  await page.locator(".field").filter({
    has: page.locator("label", { hasText: /^Compatibility score/ }),
  }).locator("input").fill(String(expected.compatibilityScore));
  await legacy().uncheck();
  const original = await download("setup-original.json");
  assert.deepEqual(JSON.parse(original).setup, expected);
  assert.equal(postCount(), beforePosts);
  await screenshot("candidate-configured.png");
  receipt.checks.push({ name: "real form download contains every chosen and hidden value", passed: true, postsDuringDownload: 0 });
  await context.close();
  await browser.close();
  browser = null;

  await launch();
  context = await freshPage(base);
  assert.equal(await page.getByLabel("Seed", { exact: true }).inputValue(), "42");
  const chooserEvent = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Open setup", exact: true }).click();
  const chooser = await chooserEvent;
  await chooser.setFiles(path.join(output, "setup-original.json"));
  const review = page.getByRole("region", { name: "Review setup", exact: true });
  await review.waitFor();
  assert.equal(await page.getByLabel("Seed", { exact: true }).inputValue(), "42");
  assert.equal(await legacy().isChecked(), false);
  assert.equal(await page.locator(".multi input:checked").count(), 0);
  assert((await review.innerText()).includes(expected.seed));
  assert.equal(postCount(), beforePosts);
  await screenshot("candidate-review-desktop.png");
  await page.setViewportSize({ width: 390, height: 844 });
  const geometry = await review.evaluate((node) => {
    const rect = node.getBoundingClientRect();
    const children = [...node.querySelectorAll("button, code, dd, summary")].map((child) => {
      const r = child.getBoundingClientRect();
      return { tag: child.tagName, left: r.left, right: r.right, width: r.width };
    });
    return { viewport: innerWidth, left: rect.left, right: rect.right, scrollWidth: node.scrollWidth, clientWidth: node.clientWidth, children };
  });
  assert(geometry.left >= 0 && geometry.right <= geometry.viewport);
  assert(geometry.scrollWidth <= geometry.clientWidth + 1, "New file review must not overflow on a phone");
  await artifact("phone-review-geometry.json", JSON.stringify(geometry, null, 2) + "\n");
  await screenshot("candidate-review-phone.png");
  receipt.checks.push({ name: "real downloaded file previews after a fresh browser process without changing the form or native run", passed: true });
  await page.getByRole("button", { name: "Replace setup", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "Setup replaced" }).waitFor();
  assert.equal(await page.getByLabel("Seed", { exact: true }).inputValue(), expected.seed);
  assert.equal(await field("Policy").inputValue(), expected.policy);
  assert.equal(await legacy().isChecked(), false);
  const reopened = await download("setup-reopened.json");
  assert.equal(Buffer.compare(original, reopened), 0, "Reopened form re-download must be byte-identical");
  const afterState = await fetch(new URL("/v1/run/state", apiBase)).then((r) => r.text());
  await artifact("native-state-after-files.json", afterState);
  assert.equal(afterState, beforeState);
  assert.equal(postCount(), beforePosts);
  receipt.checks.push({ name: "explicit replacement and re-download preserve exact bytes and leave the pre-existing private native run unchanged", passed: true });

  await page.setViewportSize({ width: 1280, height: 900 });
  await legacy().check();
  for (let i = 0; i < ancestorKeys.length; i++) {
    const node = page.locator(".legacy-node").nth(i);
    const data = expected.legacyTree[ancestorKeys[i]];
    assert.equal(await node.locator(".field select").inputValue(), data.uma);
    for (let j = 0; j < kinds.length; j++) {
      const row = node.locator(".spark-row").nth(j);
      assert.equal(await row.locator("select").nth(0).inputValue(), data[kinds[j]].factorId);
      assert.equal(await row.locator("select").nth(1).inputValue(), String(data[kinds[j]].stars));
    }
  }
  assert.equal(await page.locator(".field").filter({
    has: page.locator("label", { hasText: /^Compatibility score/ }),
  }).locator("input").inputValue(), String(expected.compatibilityScore));
  receipt.checks.push({ name: "re-enabled inheritance exposes the actual six restored ancestor editors and all 30 saved star controls", passed: true });

  await page.getByLabel("Seed", { exact: true }).fill("1e3");
  await page.getByRole("button", { name: "Start run", exact: true }).click();
  await page.locator(".banner.error").filter({ hasText: "Seed must be" }).waitFor();
  assert.equal(postCount(), beforePosts);
  assert.equal(await page.getByLabel("Seed", { exact: true }).inputValue(), "1e3");
  receipt.checks.push({ name: "invalid manual seed refuses Start locally without replacing the form", passed: true });
  await page.getByLabel("Seed", { exact: true }).fill(expected.seed);
  await page.getByRole("button", { name: "Start run", exact: true }).click();
  await until(() => receipt.requests.some((r) => r.phase === phase && r.path === "/v1/run/start" && r.responseStatus), "candidate native Start");
  const started = receipt.requests.find((r) => r.phase === phase && r.path === "/v1/run/start");
  const payload = JSON.parse(started.body);
  assert.equal(postCount(), beforePosts + 1);
  assert.deepEqual(payload, {
    seed: expected.seed, scenario: expected.scenario, trainee: expected.trainee,
    speed: expected.speed, dialogue: expected.dialogue, raceModel: expected.raceModel, policy: expected.policy,
    deckSupports: expected.deck.join(","),
    legacyFactors: ancestorKeys.flatMap((key) => kinds.map((k) => expected.legacyTree[key][k])).filter((slot) => slot.factorId).map((slot) => slot.factorId + "@" + slot.stars).join(","),
    legacyTree: expected.legacyTree,
    parentNames: [expected.legacyTree.parentA.uma, expected.legacyTree.parentB.uma].filter(Boolean).join(","),
    compatibilityScore: expected.compatibilityScore,
    traceTelemetry: true,
  });
  assert.equal(started.responseStatus, 200, started.responseBody.slice(0, 700));
  assert(/"seed"\s*:\s*9223372036854775807(?=[,}\s])/.test(started.responseBody), "Native JSON must retain the full i64 seed digits");
  await artifact("native-start-request.json", started.body);
  await artifact("native-start-response.json", started.responseBody);
  await until(() => page.locator(".turn-layout").isVisible(), "actual native run rendered");
  const toolbarPolicy = page.locator(".controls").filter({
    has: page.getByRole("button", { name: "New run", exact: true }),
  }).locator("select");
  assert.equal(await toolbarPolicy.inputValue(), expected.policy);
  receipt.checks.push({ name: "only explicit Start creates the real native career with the exact seed, policy and inherited request values", passed: true });
  assert.deepEqual(receipt.pageErrors, []);
  assert.deepEqual(await builtAssets(baselineDist), receipt.assets.baseline);
  assert.deepEqual(await builtAssets(candidateDist), receipt.assets.candidate);
  receipt.accepted = true;
} catch (error) {
  receipt.accepted = false;
  receipt.failure = { name: error.name, message: error.message, stack: error.stack };
  if (page && !page.isClosed()) {
    try {
      await artifact("failure.html", await page.content());
      await screenshot("failure.png");
    } catch (captureError) {
      receipt.captureFailure = String(captureError);
    }
  }
  process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (server) {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
  receipt.completedAt = new Date().toISOString();
  await writeFile(path.join(output, "RESULT.json"), JSON.stringify(receipt, null, 2) + "\n");
  console.log(JSON.stringify({
    accepted: receipt.accepted, checks: receipt.checks,
    pageErrors: receipt.pageErrors, failure: receipt.failure,
    browserInstances: receipt.browserInstances, output,
  }, null, 2));
}
