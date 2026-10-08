#!/usr/bin/env node
/**
 * Native-backed browser smoke for the accepted PR65 UI and report renderer.
 * node scripts/verify-lab-accepted-ui.mjs API_BINARY REPO_ROOT UI_ROOT OUTPUT_DIR
 *
 * Serves the actual built UI with a transparent loopback /v1 proxy, matching the
 * project's existing Vite development topology. API responses are never mocked
 * or edited. The native server uses its own disposable state directory.
 * Uses existing PLAYWRIGHT_MODULE, MARKED_MODULE, CHROMIUM_EXECUTABLE runtimes.
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs";
import http from "node:http";
import { createRequire } from "node:module";
import net from "node:net";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright-core");
const markedRef = process.env.MARKED_MODULE || "marked";
const { marked } = await import(path.isAbsolute(markedRef) ? pathToFileURL(markedRef).href : markedRef);
const [binaryArg, repoArg, uiArg, outputArg] = process.argv.slice(2);
assert.ok(binaryArg && repoArg && uiArg && outputArg && process.env.CHROMIUM_EXECUTABLE,
  "Supply API_BINARY REPO_ROOT UI_ROOT OUTPUT_DIR and CHROMIUM_EXECUTABLE.");
const [binary, repo, ui] = [binaryArg, repoArg, uiArg].map(filename => fs.realpathSync(filename));
const output = path.resolve(outputArg);
fs.mkdirSync(output, { recursive: true });
assert.equal(fs.readdirSync(output).length, 0, "Use a fresh output directory.");
const stateDir = path.join(output, "state");
fs.mkdirSync(stateDir);
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const fileHash = filename => hash(fs.readFileSync(filename));
const save = (name, data) => fs.writeFileSync(path.join(output, name), JSON.stringify(data, null, 2) + "\n");
const walk = directory => fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
  const filename = path.join(directory, entry.name);
  return entry.isDirectory() ? walk(filename) : [filename];
});
const sourcePins = () => Object.fromEntries([
  path.join(repo, "uma-sim-core/src/career_lab.rs"), path.join(repo, "uma-sim-core/src/api.rs"),
  path.join(ui, "src/state/labStore.ts"), path.join(ui, "src/components/ComparePanel.tsx"),
  ...walk(path.join(ui, "dist")),
].map(filename => [filename, fileHash(filename)]));
const receipt = {
  schema: "uma-sim-accepted-native-ui-smoke-v1", startedAt: new Date().toISOString(),
  boundary: "Built React UI and actual native API through an unmodified loopback proxy. Real branch creation, simulation, comparison and browser downloads. Public catalog portrait requests are blocked and recorded. No HTTP fixtures, store edits, external policy, deployed service, or embedded release binary.",
  binary: { path: binary, sha256: fileHash(binary) },
  source: { repo, ui, before: sourcePins() },
  harnessSha256: fileHash(fileURLToPath(import.meta.url)),
  browser: { executable: process.env.CHROMIUM_EXECUTABLE, sha256: fileHash(process.env.CHROMIUM_EXECUTABLE) },
  apiRequests: [], proxyResponses: [], browserRequests: [], outsideRequests: [], blockedCatalogImages: [], unexpectedOutsideRequests: [], pageErrors: [], downloads: [], checks: [],
};
const check = (name, fn) => { fn(); receipt.checks.push({ name, passed: true }); };
let native;
let nativeDone;
let nativeExited = false;
let log;
let front;
let browser;
let activePage;
let failure;
try {
  const reserver = net.createServer();
  reserver.listen(0, "127.0.0.1");
  await once(reserver, "listening");
  const nativePort = reserver.address().port;
  await new Promise(resolve => reserver.close(resolve));
  const nativeOrigin = `http://127.0.0.1:${nativePort}`;
  receipt.nativeOrigin = nativeOrigin;
  const env = { ...process.env, UMA_REPO_ROOT: repo };
  delete env.UMA_POLICY_CMD;
  native = spawn(binary, [String(nativePort)], { cwd: stateDir, env, stdio: ["ignore", "pipe", "pipe"] });
  nativeDone = once(native, "exit").then(([code, signal]) => {
    nativeExited = true; receipt.nativeExit = { code, signal };
  });
  log = fs.createWriteStream(path.join(output, "server.log"));
  native.stdout.pipe(log, { end: false });
  native.stderr.pipe(log, { end: false });
  for (let attempt = 0; ; attempt++) {
    assert.ok(!nativeExited, "Native server exited before health was available.");
    try {
      const response = await fetch(nativeOrigin + "/v1/health", { signal: AbortSignal.timeout(1000) });
      if (response.ok) { receipt.health = await response.json(); break; }
    } catch (error) { if (attempt >= 99) throw error; }
    assert.ok(attempt < 99, "Native health timeout.");
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  const post = async (route, body) => {
    const response = await fetch(nativeOrigin + route, { method: "POST", body: JSON.stringify(body),
      headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(30000) });
    const bytes = Buffer.from(await response.arrayBuffer());
    receipt.apiRequests.push({ method: "POST", route, input: body, status: response.status, bytes: bytes.length, sha256: hash(bytes) });
    assert.equal(response.status, 200, bytes.toString("utf8"));
    return JSON.parse(bytes.toString("utf8"));
  };
  await post("/v1/run/start", { seed: 4242, scenario: "ura", trainee: "Special Week", raceModel: "stub", policy: "bot" });
  await post("/v1/library/save", { name: "native-ui-receiving" });
  const dist = path.join(ui, "dist");
  front = http.createServer((request, response) => {
    if (request.url.startsWith("/v1/")) {
      const upstream = http.request(new URL(request.url, nativeOrigin), {
        method: request.method, headers: { ...request.headers, host: `127.0.0.1:${nativePort}` },
      }, nativeResponse => {
        const chunks = [];
        nativeResponse.on("data", chunk => chunks.push(chunk));
        nativeResponse.on("end", () => {
          const bytes = Buffer.concat(chunks);
          receipt.proxyResponses.push({ method: request.method, route: request.url,
            status: nativeResponse.statusCode, headers: nativeResponse.headers, bytes: bytes.length, sha256: hash(bytes) });
          response.writeHead(nativeResponse.statusCode, nativeResponse.headers);
          response.end(bytes);
        });
      });
      upstream.on("error", error => { response.writeHead(502); response.end(String(error)); });
      request.pipe(upstream);
      return;
    }
    const pathname = new URL(request.url, "http://localhost").pathname;
    if (pathname === "/favicon.ico") { response.writeHead(204); response.end(); return; }
    const filename = path.resolve(dist, pathname === "/" ? "index.html" : pathname.slice(1));
    if (!filename.startsWith(dist + path.sep) || !fs.existsSync(filename) || !fs.statSync(filename).isFile()) {
      response.writeHead(404); response.end("Not found"); return;
    }
    const contentType = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8" }[path.extname(filename)] || "application/octet-stream";
    response.writeHead(200, { "Content-Type": contentType });
    response.end(fs.readFileSync(filename));
  });
  front.listen(0, "127.0.0.1");
  await once(front, "listening");
  const origin = `http://127.0.0.1:${front.address().port}`;
  receipt.uiOrigin = origin;
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_EXECUTABLE, headless: true,
    args: ["--disable-dev-shm-usage", "--disable-gpu"] });
  receipt.browser.version = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  await context.route("**/*", route => {
    const url = route.request().url();
    receipt.browserRequests.push({ method: route.request().method(), url });
    if (new URL(url).origin !== origin) {
      receipt.outsideRequests.push(url);
      if (new URL(url).origin === "https://gametora.com" && route.request().resourceType() === "image") {
        receipt.blockedCatalogImages.push(url);
      } else { receipt.unexpectedOutsideRequests.push(url); }
      return route.abort();
    }
    return route.continue();
  });
  const page = await context.newPage();
  activePage = page;
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  await page.goto(origin);
  await page.getByRole("button", { name: "Library & lab", exact: true }).click();
  const panel = page.locator(".card").filter({ has: page.getByRole("heading", { name: "Branch & compare", exact: true }) });
  await panel.getByLabel("Checkpoint").selectOption("native-ui-receiving");
  await panel.getByLabel("Max actions", { exact: true }).fill("2");
  const names = ["Rest | **early**", "Control <em>steady</em> &amp;"];
  for (const name of names) {
    await panel.getByLabel("Branch name (optional)", { exact: true }).fill(name);
    await panel.getByRole("button", { name: "Run branch", exact: true }).click();
    await panel.getByRole("radio", { name: `Compare A: ${name}`, exact: true }).waitFor();
    await page.locator(".busy-overlay").waitFor({ state: "hidden" });
  }
  check("Actual UI creates both adversarially named native branches", () => {
    assert.equal(receipt.proxyResponses.filter(response => response.method === "POST" && response.route === "/v1/lab/branch" && response.status === 200).length, 2);
  });
  await panel.getByRole("radio", { name: `Compare A: ${names[0]}`, exact: true }).check();
  await panel.getByRole("radio", { name: `Compare B: ${names[1]}`, exact: true }).check();
  await panel.getByRole("button", { name: "Compare selected branches", exact: true }).click();
  await panel.getByRole("heading", { name: `${names[0]} vs ${names[1]}`, exact: true }).waitFor();
  await page.locator(".busy-overlay").waitFor({ state: "hidden" });
  receipt.visibleComparison = await panel.innerText();
  await panel.screenshot({ path: path.join(output, "ui-comparison.png") });
  const attachments = {};
  for (const [format, linkName, extension] of [["markdown", "Download markdown report", "md"], ["json", "Download JSON report", "json"]]) {
    const downloadPromise = page.waitForEvent("download");
    await panel.getByRole("link", { name: linkName, exact: true }).click();
    const download = await downloadPromise;
    const filename = path.join(output, `report.${extension}`);
    await download.saveAs(filename);
    const bytes = fs.readFileSync(filename);
    attachments[format] = bytes;
    receipt.downloads.push({ format, url: download.url(), suggestedFilename: download.suggestedFilename(),
      failure: await download.failure(), bytes: bytes.length, sha256: hash(bytes) });
  }
  const report = JSON.parse(attachments.json.toString("utf8"));
  check("Downloaded JSON agrees with the visible ordered branch names", () => {
    assert.deepEqual([report.aName, report.bName], names);
    assert.notEqual(report.aId, report.bId);
    assert.equal(report.sameCheckpoint, true);
    assert.equal(report.outcomeA.steps, 2);
    assert.equal(report.outcomeB.steps, 2);
  });
  check("Each saved browser attachment exactly matches the native proxy bytes", () => {
    for (const attachment of receipt.downloads) {
      const url = new URL(attachment.url);
      const upstream = receipt.proxyResponses.find(response => response.route === url.pathname + url.search);
      assert.ok(upstream);
      assert.equal(upstream.status, 200);
      assert.equal(upstream.sha256, attachment.sha256);
      assert.equal(upstream.bytes, attachment.bytes);
      assert.equal(url.searchParams.get("a"), report.aId);
      assert.equal(url.searchParams.get("b"), report.bId);
      assert.equal(attachment.failure, null);
      assert.ok(attachment.suggestedFilename.includes(`${report.aId}-vs-${report.bId}`));
    }
  });
  const reportPage = await context.newPage();
  const rendered = marked.parse(attachments.markdown.toString("utf8"), { gfm: true });
  const html = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'"><style>body{font:15px/1.5 system-ui;margin:32px;color:#17202a}table{border-collapse:collapse;width:100%}td,th{border:1px solid #ccd4dc;padding:6px;text-align:left}h1{font-size:25px}</style><main>${rendered}</main>`;
  fs.writeFileSync(path.join(output, "report.html"), html);
  await reportPage.setContent(html);
  const dom = await reportPage.evaluate(() => ({
    headings: [...document.querySelectorAll("h1")].map(node => node.innerText),
    branchRows: [...document.querySelectorAll("tbody tr")].filter(row => row.firstElementChild?.innerText === "Branch").map(row => [...row.children].map(cell => cell.innerText)),
  }));
  receipt.renderedReport = dom;
  save("rendered-dom.json", dom);
  await reportPage.screenshot({ path: path.join(output, "rendered-report.png"), fullPage: true });
  check("Downloaded Markdown renders the same pair as literal heading and cells", () => {
    assert.deepEqual(dom.headings, [`Branch comparison: ${names[0]} vs ${names[1]}`]);
    assert.deepEqual(dom.branchRows, [["Branch", ...names]]);
  });
  await panel.getByRole("radio", { name: `Compare B: ${names[0]}`, exact: true }).check();
  const staleHeadingCount = await panel.getByRole("heading", { name: `${names[0]} vs ${names[1]}`, exact: true }).count();
  const reportLinkCount = await panel.getByRole("link", { name: /Download .* report/ }).count();
  const compareDisabled = await panel.getByRole("button", { name: "Compare selected branches", exact: true }).isDisabled();
  const retainedDownloadPromise = page.waitForEvent("download");
  await panel.getByRole("link", { name: "Download JSON report", exact: true }).click();
  const retainedDownload = await retainedDownloadPromise;
  const retainedFile = path.join(output, "after-draft-report.json");
  await retainedDownload.saveAs(retainedFile);
  const retainedBytes = fs.readFileSync(retainedFile);
  const retainedReport = JSON.parse(retainedBytes.toString("utf8"));
  const retainedUrl = new URL(retainedDownload.url());
  const retainedProxy = receipt.proxyResponses.findLast(response => response.route === retainedUrl.pathname + retainedUrl.search);
  receipt.afterDraftDownload = { url: retainedDownload.url(), suggestedFilename: retainedDownload.suggestedFilename(),
    failure: await retainedDownload.failure(), bytes: retainedBytes.length, sha256: hash(retainedBytes),
    ids: [retainedReport.aId, retainedReport.bId], names: [retainedReport.aName, retainedReport.bName] };
  check("Changing the draft pair preserves the displayed report and its actual download", () => {
    assert.equal(staleHeadingCount, 1);
    assert.equal(reportLinkCount, 2);
    assert.equal(compareDisabled, true);
    assert.deepEqual([retainedReport.aId, retainedReport.bId], [report.aId, report.bId]);
    assert.deepEqual([retainedReport.aName, retainedReport.bName], names);
    assert.equal(retainedUrl.searchParams.get("a"), report.aId);
    assert.equal(retainedUrl.searchParams.get("b"), report.bId);
    assert.equal(receipt.afterDraftDownload.failure, null);
    assert.ok(retainedProxy);
    assert.equal(retainedProxy.sha256, receipt.afterDraftDownload.sha256);
    assert.equal(retainedProxy.bytes, retainedBytes.length);
  });
  await panel.screenshot({ path: path.join(output, "ui-draft-preserves-report.png") });
  check("Native-backed browser session has no endpoint errors or unexpected external requests", () => {
    assert.deepEqual(receipt.unexpectedOutsideRequests, []);
    assert.equal(receipt.outsideRequests.length, receipt.blockedCatalogImages.length);
    assert.deepEqual(receipt.pageErrors, []);
    assert.ok(receipt.proxyResponses.length > 0);
    assert.ok(receipt.proxyResponses.every(response => response.status === 200));
  });
  await context.close();
} catch (error) {
  failure = error;
  receipt.error = error.stack;
  if (activePage && !activePage.isClosed()) {
    receipt.failureVisibleText = await activePage.locator("body").innerText().catch(() => "unavailable");
    await activePage.screenshot({ path: path.join(output, "failure-ui.png"), fullPage: true }).catch(() => {});
  }
} finally {
  if (browser) await browser.close();
  if (front) {
    front.closeAllConnections();
    await new Promise(resolve => front.close(resolve));
  }
  if (native && !nativeExited) { native.kill("SIGTERM"); await nativeDone; }
  if (log) await new Promise(resolve => log.end(resolve));
  receipt.source.after = sourcePins();
  receipt.source.unchanged = JSON.stringify(receipt.source.before) === JSON.stringify(receipt.source.after);
  receipt.binary.afterSha256 = fileHash(binary);
  receipt.binary.unchanged = receipt.binary.sha256 === receipt.binary.afterSha256;
  receipt.accepted = !failure && receipt.checks.length === 6 && receipt.source.unchanged && receipt.binary.unchanged;
  receipt.finishedAt = new Date().toISOString();
  save("receipt.json", receipt);
  console.log(JSON.stringify({ accepted: receipt.accepted, checksPassed: receipt.checks.length,
    proxyResponses: receipt.proxyResponses.length, outsideRequests: receipt.outsideRequests.length,
    blockedCatalogImages: receipt.blockedCatalogImages.length, unexpectedOutsideRequests: receipt.unexpectedOutsideRequests.length,
    pageErrors: receipt.pageErrors.length, sourceUnchanged: receipt.source.unchanged, binaryUnchanged: receipt.binary.unchanged }));
  if (failure) console.error(failure.message);
  process.exitCode = receipt.accepted ? 0 : 1;
}
