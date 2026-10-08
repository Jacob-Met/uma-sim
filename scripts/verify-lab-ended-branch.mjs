#!/usr/bin/env node
/**
 * Receive actual comparison/report notes for unequal native branch lengths.
 * node scripts/verify-lab-ended-branch.mjs API_BINARY REPO_ROOT OUTPUT_DIR
 * Uses existing PLAYWRIGHT_MODULE, MARKED_MODULE, CHROMIUM_EXECUTABLE runtimes.
 * Three branches are created only through the real API (2, 3, and 2 actions).
 * No authored simulator responses or persisted-file edits are used.
 */
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs";
import { createRequire } from "node:module";
import net from "node:net";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright-core");
const markedRef = process.env.MARKED_MODULE || "marked";
const { marked } = await import(path.isAbsolute(markedRef) ? pathToFileURL(markedRef).href : markedRef);
const [binaryArg, repoArg, outputArg] = process.argv.slice(2);
assert.ok(binaryArg && repoArg && outputArg && process.env.CHROMIUM_EXECUTABLE,
  "Supply API_BINARY REPO_ROOT OUTPUT_DIR and CHROMIUM_EXECUTABLE.");
const binary = fs.realpathSync(binaryArg), repo = fs.realpathSync(repoArg), output = path.resolve(outputArg);
fs.mkdirSync(output, { recursive: true });
assert.equal(fs.readdirSync(output).length, 0, "Use a new output directory.");
const stateDir = path.join(output, "state");
fs.mkdirSync(stateDir);
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const fileHash = filename => hash(fs.readFileSync(filename));
const save = (filename, data) => fs.writeFileSync(filename, JSON.stringify(data, null, 2) + "\n");
const sourcePins = () => Object.fromEntries(["Cargo.lock", "uma-sim-core/src/career_lab.rs", "uma-sim-core/src/api.rs"].map(filename => [filename, fileHash(path.join(repo, filename))]));
const gitHead = spawnSync("git", ["-C", repo, "rev-parse", "HEAD"], { encoding: "utf8" });
assert.equal(gitHead.status, 0, gitHead.stderr);
const receipt = {
  schema: "uma-sim-ended-branch-receiving-v1", startedAt: new Date().toISOString(),
  boundary: "Actual native branch API with built-in bot/stub races; direct HTTP comparison and attachment receiving; saved Markdown parsed with marked GFM and rendered in Chromium. No HTTP fixtures or persisted edits.",
  source: { root: repo, commit: gitHead.stdout.trim(), before: sourcePins() },
  binary: { path: binary, sha256: fileHash(binary) },
  harnessSha256: fileHash(fileURLToPath(import.meta.url)),
  parser: markedRef, browser: { executable: process.env.CHROMIUM_EXECUTABLE, sha256: fileHash(process.env.CHROMIUM_EXECUTABLE) },
  requests: [], cases: [], pageErrors: [], outsideRequests: [],
};
let native, nativeDone, browser, log, fatal;
let nativeExited = false;
try {
  const reserver = net.createServer();
  reserver.listen(0, "127.0.0.1");
  await once(reserver, "listening");
  const port = reserver.address().port;
  await new Promise(resolve => reserver.close(resolve));
  const origin = `http://127.0.0.1:${port}`;
  receipt.nativeOrigin = origin;
  const env = { ...process.env, UMA_REPO_ROOT: repo };
  delete env.UMA_POLICY_CMD;
  native = spawn(binary, [String(port)], { cwd: stateDir, env, stdio: ["ignore", "pipe", "pipe"] });
  nativeDone = once(native, "exit").then(([code, signal]) => { nativeExited = true; receipt.nativeExit = { code, signal }; });
  log = fs.createWriteStream(path.join(output, "server.log"));
  native.stdout.pipe(log, { end: false }); native.stderr.pipe(log, { end: false });
  for (let attempt = 0; ; attempt++) {
    assert.ok(!nativeExited, "Native server exited before health was available.");
    try {
      const response = await fetch(origin + "/v1/health", { signal: AbortSignal.timeout(1000) });
      if (response.ok) { receipt.health = await response.json(); break; }
    } catch (error) { if (attempt >= 99) throw error; }
    assert.ok(attempt < 99, "Native server health timeout.");
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  const request = async (method, route, body, filename) => {
    const response = await fetch(origin + route, { method,
      headers: body ? { "Content-Type": "application/json" } : undefined,
      body: body ? JSON.stringify(body) : undefined, signal: AbortSignal.timeout(30000) });
    const bytes = Buffer.from(await response.arrayBuffer());
    const headers = Object.fromEntries(response.headers.entries());
    receipt.requests.push({ method, route, input: body, status: response.status, headers, bytes: bytes.length, sha256: hash(bytes) });
    if (filename) fs.writeFileSync(filename, bytes);
    assert.equal(response.status, 200, bytes.toString("utf8"));
    return { bytes, headers };
  };
  const post = async (route, body) => JSON.parse((await request("POST", route, body)).bytes.toString("utf8"));
  await post("/v1/run/start", { seed: 4242, scenario: "ura", trainee: "Special Week", raceModel: "stub", policy: "bot" });
  await post("/v1/library/save", { name: "ended-branch-receiving" });
  const branches = [];
  for (const [name, maxActions] of [["Short two", 2], ["Long three", 3], ["Equal two", 2]]) {
    const result = await post("/v1/lab/branch", { checkpoint: "ended-branch-receiving", name, maxActions, policy: "bot" });
    assert.equal(result.branch.steps, maxActions);
    assert.equal(result.outcome.steps, maxActions);
    branches.push(result);
  }
  save(path.join(output, "created-branches.json"), branches);
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_EXECUTABLE,
    headless: true, args: ["--disable-dev-shm-usage", "--disable-gpu"] });
  receipt.browser.version = browser.version();
  const context = await browser.newContext({ viewport: { width: 1180, height: 1000 } });
  await context.route("**/*", route => { receipt.outsideRequests.push(route.request().url()); return route.abort(); });
  const page = await context.newPage();
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  const definitions = [
    { id: "a_ends_first", a: 0, b: 1, ended: "A" },
    { id: "b_ends_first", a: 1, b: 0, ended: "B" },
    { id: "equal_length_control", a: 0, b: 2, ended: null },
  ];
  for (const definition of definitions) {
    const caseDir = path.join(output, definition.id);
    fs.mkdirSync(caseDir);
    const [a, b] = [branches[definition.a], branches[definition.b]];
    const ids = [a.branch.id, b.branch.id], steps = [a.branch.steps, b.branch.steps];
    const expectedNote = definition.ended
      ? `Branch ${definition.ended} ended after ${Math.min(...steps)} steps while the other continued.` : null;
    const result = { id: definition.id, ids, steps, expectedNote, checks: [] };
    receipt.cases.push(result);
    const check = (name, fn) => {
      try { fn(); result.checks.push({ name, passed: true }); }
      catch (error) { result.checks.push({ name, passed: false, error: error.message }); }
    };
    const comparison = JSON.parse((await request("POST", "/v1/lab/compare", { a: ids[0], b: ids[1] }, path.join(caseDir, "comparison.json"))).bytes.toString("utf8"));
    const query = `a=${encodeURIComponent(ids[0])}&b=${encodeURIComponent(ids[1])}`;
    const jsonDownload = await request("GET", `/v1/lab/report?${query}&format=json`, undefined, path.join(caseDir, "report.json"));
    const markdownDownload = await request("GET", `/v1/lab/report?${query}&format=markdown`, undefined, path.join(caseDir, "report.md"));
    const report = JSON.parse(jsonDownload.bytes.toString("utf8"));
    result.observedNotes = { comparison: comparison.firstDivergence?.note ?? null, jsonReport: report.firstDivergence?.note ?? null };
    check("Comparison identities and actual outcome lengths match API-created branches", () => {
      assert.deepEqual([comparison.aId, comparison.bId], ids);
      assert.deepEqual([comparison.outcomeA.steps, comparison.outcomeB.steps], steps);
      assert.equal(comparison.aligned.length, Math.max(...steps));
      assert.ok(comparison.aligned.slice(0, Math.min(...steps)).every(row => row.sameAction && row.sameOutcome));
    });
    check("Comparison note counts the branch that actually ended", () => {
      assert.equal(comparison.firstDivergence?.note ?? null, expectedNote);
      if (definition.ended) {
        assert.equal(comparison.firstDivergence.stepIndex, Math.min(...steps));
        assert.equal(comparison.firstDivergence.kind, "outcome");
      }
    });
    check("Downloaded JSON preserves the comparison and correct shorter-branch note", () => {
      const comparable = value => { const copy = { ...value }; delete copy.comparedAt; return copy; };
      assert.deepEqual(comparable(report), comparable(comparison));
      assert.equal(report.firstDivergence?.note ?? null, expectedNote);
    });
    check("Actual report attachments retain correct headers and received byte lengths", () => {
      for (const [download, type, extension] of [[jsonDownload, "application/json", "json"], [markdownDownload, "text/markdown", "md"]]) {
        assert.ok(download.headers["content-type"].startsWith(type));
        assert.ok(download.headers["content-disposition"].includes(`branch-compare-${ids[0]}-vs-${ids[1]}.${extension}`));
        assert.equal(Number(download.headers["content-length"]), download.bytes.length);
      }
    });
    const rendered = marked.parse(markdownDownload.bytes.toString("utf8"), { gfm: true });
    const html = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'"><style>body{font:15px/1.5 system-ui;margin:32px;color:#17202a}table{border-collapse:collapse;width:100%}td,th{border:1px solid #ccd4dc;padding:6px;text-align:left}h1{font-size:25px}</style><main>${rendered}</main>`;
    fs.writeFileSync(path.join(caseDir, "report.html"), html);
    await page.setContent(html);
    const dom = await page.evaluate(() => {
      const heading = [...document.querySelectorAll("h2")].find(node => node.innerText === "First divergence");
      const following = heading.nextElementSibling;
      return {
        divergenceTag: following.tagName,
        divergenceText: following.innerText,
        divergenceItems: following.tagName === "UL" ? [...following.children].map(node => node.innerText) : [],
        outcomeSteps: [...document.querySelectorAll("tbody tr")].filter(row => row.firstElementChild?.innerText === "Steps").map(row => [...row.children].map(cell => cell.innerText)),
      };
    });
    save(path.join(caseDir, "rendered-dom.json"), dom);
    result.observedNotes.markdown = definition.ended ? dom.divergenceItems.at(-1) ?? null : null;
    check("Received Markdown states the correct shorter count or equal-length control", () => {
      if (expectedNote) { assert.equal(dom.divergenceTag, "UL"); assert.equal(dom.divergenceItems.at(-1), expectedNote); }
      else { assert.equal(dom.divergenceTag, "P"); assert.equal(dom.divergenceText, "No divergence: the recorded timelines are identical."); }
    });
    check("Rendered outcome counts independently expose each branch length", () => {
      assert.deepEqual(dom.outcomeSteps, [["Steps", ...steps.map(String)]]);
    });
    await page.screenshot({ path: path.join(caseDir, "rendered.png"), fullPage: true });
    result.passed = result.checks.every(check => check.passed);
    save(path.join(caseDir, "case.json"), result);
    console.log(`${result.passed ? "PASS" : "FAIL"} ${result.id}: ${result.checks.filter(check => check.passed).length}/${result.checks.length}`);
  }
  await context.close();
} catch (error) { fatal = error; receipt.error = error.stack; }
finally {
  if (browser) await browser.close();
  if (native && !nativeExited) { native.kill("SIGTERM"); await nativeDone; }
  if (log) await new Promise(resolve => log.end(resolve));
  receipt.source.after = sourcePins();
  receipt.source.unchanged = JSON.stringify(receipt.source.before) === JSON.stringify(receipt.source.after);
  receipt.binary.afterSha256 = fileHash(binary);
  receipt.binary.unchanged = receipt.binary.sha256 === receipt.binary.afterSha256;
  receipt.summary = { passed: receipt.cases.filter(test => test.passed).length, failed: receipt.cases.filter(test => !test.passed).length,
    checksPassed: receipt.cases.flatMap(test => test.checks).filter(check => check.passed).length,
    checksFailed: receipt.cases.flatMap(test => test.checks).filter(check => !check.passed).length };
  receipt.accepted = !fatal && receipt.cases.length === 3 && receipt.summary.failed === 0 && receipt.source.unchanged && receipt.binary.unchanged
    && receipt.pageErrors.length === 0 && receipt.outsideRequests.length === 0;
  receipt.finishedAt = new Date().toISOString();
  save(path.join(output, "receipt.json"), receipt);
  console.log(JSON.stringify({ accepted: receipt.accepted, ...receipt.summary, sourceUnchanged: receipt.source.unchanged, binaryUnchanged: receipt.binary.unchanged }));
  process.exitCode = receipt.accepted ? 0 : 1;
}
