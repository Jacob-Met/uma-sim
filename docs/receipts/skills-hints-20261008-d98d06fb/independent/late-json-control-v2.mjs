import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
const root = "/tmp/hamon-product-d98d06fb-20261008/uma-sim";
const review = "/tmp/hamon-uma-skills-independent-d98d06fb-20261008";
const out = path.join(review, "run-v2");
const ui = path.join(root, "packages/uma-sim-ui");
const require = createRequire(import.meta.url);
const esbuild = require(path.join(ui, "node_modules/esbuild"));
const { chromium } = require(path.join(ui, "node_modules/playwright"));
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const manifestPath = path.join(root, "../evidence/candidate-v1-source.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath));
const before = Object.fromEntries(Object.keys(manifest.files).map(p => [p, sha(fs.readFileSync(path.join(root, p)))]));
if (JSON.stringify(before) !== JSON.stringify(manifest.files)) throw new Error("Frozen source pin mismatch");
fs.mkdirSync(out, { recursive: false });
const result = {
  schema: "uma.skills-independent-late-json.v1",
  startedAt: new Date().toISOString(),
  sourceHead: execFileSync("git", ["-C", root, "rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
  sourceBase: manifest.base,
  sourceManifestSha256: sha(fs.readFileSync(manifestPath)),
  sourceHashesBefore: before,
  scriptSha256: sha(fs.readFileSync(new URL(import.meta.url))),
  scope: "One controlled late-JSON fixture against the exact actual SkillsPanel in real React StrictMode and Chromium; no native API, career session, or RNG process.",
  nativeApiCalls: 0, installedMutations: false, sourceEdits: false,
  pass: false
};
const entry = [
  'import React, { StrictMode } from "react";',
  'import { createRoot } from "react-dom/client";',
  'import { SkillsPanel } from ' + JSON.stringify(path.join(ui, "src/components/SkillsPanel.tsx")) + ';',
  'const review = window.__review = { requests: [], events: [], inputs: [] };',
  'window.fetch = (url, options = {}) => {',
  '  const index = review.requests.length;',
  '  let resolveJson;',
  '  const body = new Promise(resolve => { resolveJson = resolve; });',
  '  const item = { index, url: String(url), method: options.method ?? "GET", hasBody: options.body !== undefined, signal: options.signal, resolveJson, jsonCalled: false };',
  '  review.requests.push(item); review.events.push({event:"response-fulfilled", index});',
  '  options.signal?.addEventListener("abort", () => review.events.push({event:"abort", index}));',
  '  return Promise.resolve({ok:true, json: () => {item.jsonCalled = true; review.events.push({event:"json-called", index}); return body;}});',
  '};',
  'const root = createRoot(document.getElementById("root"));',
  'review.render = (id) => {',
  ' const state = Object.freeze({learnedSkillIds:Object.freeze([id]), hintLevels:Object.freeze({[id]:2})});',
  ' review.inputs.push({state, before:JSON.stringify(state)});',
  ' root.render(<StrictMode><SkillsPanel state={state as any} /></StrictMode>);',
  '};',
  'review.unmount = () => root.unmount();',
  'review.render("skill:old");'
].join("\n");
let browser;
const blocked = [];
try {
  const build = await esbuild.build({
    stdin: { contents: entry, resolveDir: ui, loader: "tsx", sourcefile: "independent-late-json-entry.tsx" },
    bundle: true, format: "iife", platform: "browser", jsx: "automatic",
    outfile: path.join(out, "fixture.js"), write: false,
    define: { "process.env.NODE_ENV": '"development"' }
  });
  const js = build.outputFiles.find(f => f.path.endsWith(".js")).contents;
  const css = build.outputFiles.find(f => f.path.endsWith(".css"))?.contents ?? Buffer.from("");
  fs.writeFileSync(path.join(out, "entry.tsx"), entry, { flag: "wx" });
  result.bundleSha256 = sha(js);
  result.entrySha256 = sha(entry);
  result.chromiumExecutable = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  result.dependencies = Object.fromEntries(["react", "react-dom", "playwright", "esbuild"].map(name => [name, require(path.join(ui, "node_modules", name, "package.json")).version]));
  browser = await chromium.launch({ executablePath: result.chromiumExecutable, headless: true });
  result.chromiumVersion = browser.version();
  const context = await browser.newContext();
  const errors = [];
  const page = await context.newPage();
  page.setDefaultTimeout(5000);
  page.on("pageerror", error => errors.push(String(error)));
  await context.route("**/*", route => { blocked.push(route.request().url()); return route.abort("blockedbyclient"); });
  await page.setContent('<!doctype html><html><head></head><body><div id="root"></div></body></html>', {waitUntil:"domcontentloaded"});
  await page.addStyleTag({content:Buffer.from(css).toString("utf8")});
  await page.addScriptTag({content:Buffer.from(js).toString("utf8")});
  await page.waitForFunction(() => window.__review?.requests.length === 2 && window.__review.requests.every(r => r.jsonCalled));
  const initial = await page.evaluate(() => window.__review.requests.map(r => ({index:r.index, url:r.url, method:r.method, hasBody:r.hasBody, aborted:r.signal.aborted, jsonCalled:r.jsonCalled})));
  if (!initial[0].aborted || initial[1].aborted) throw new Error("StrictMode did not exercise stale/fresh effect lifetimes");
  const fresh = { items: [
    { id:"skill:old", name:"FRESH old", description:"Fresh catalog" },
    { id:"skill:new", name:"FRESH new", description:"Fresh catalog" }
  ] };
  await page.evaluate(value => window.__review.requests[1].resolveJson(value), fresh);
  await page.getByRole("region", {name:"Learned skills", exact:true}).getByText("FRESH old", {exact:true}).waitFor();
  await page.evaluate(() => window.__review.render("skill:new"));
  await page.getByRole("region", {name:"Learned skills", exact:true}).getByText("FRESH new", {exact:true}).waitFor();
  const beforeLate = await page.locator(".skills-panel").innerText();
  await page.evaluate(() => {
    window.__review.events.push({event:"late-json-resolved", index:0});
    window.__review.requests[0].resolveJson({items:[
      {id:"skill:old", name:"STALE old", description:"Stale catalog"},
      {id:"skill:new", name:"STALE new", description:"Stale catalog"}
    ]});
  });
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const afterLate = await page.locator(".skills-panel").innerText();
  if (beforeLate !== afterLate || afterLate.includes("STALE")) throw new Error("Stale JSON changed the current panel");
  const inspected = await page.evaluate(() => ({
    events: window.__review.events,
    inputsPreserved: window.__review.inputs.every(({state,before}) => Object.isFrozen(state) && Object.isFrozen(state.learnedSkillIds) && Object.isFrozen(state.hintLevels) && JSON.stringify(state) === before),
    metadataOnly: window.__review.requests.every(r => r.url === "/v1/catalog/skills" && r.method === "GET" && !r.hasBody),
    requestCount: window.__review.requests.length
  }));
  if (!inspected.inputsPreserved || !inspected.metadataOnly || inspected.requestCount !== 2 || errors.length) throw new Error("Input or request side-effect boundary changed");
  await page.evaluate(() => window.__review.unmount());
  const finalSignals = await page.evaluate(() => window.__review.requests.map(r => r.signal.aborted));
  if (!finalSignals.every(Boolean)) throw new Error("Effect cleanup did not abort all owned signals");
  result.initialRequests = initial;
  result.lateCompletion = inspected;
  result.finalAbortSignals = finalSignals;
  result.panelTextBeforeLate = beforeLate;
  result.panelTextAfterLate = afterLate;
  result.pageErrors = errors;
  result.blockedExternalRequests = blocked;
  result.pass = true;
} catch (error) {
  result.error = String(error?.stack ?? error);
} finally {
  if (browser) await browser.close();
  result.sourceHashesAfter = Object.fromEntries(Object.keys(manifest.files).map(p => [p, sha(fs.readFileSync(path.join(root, p)))]));
  result.sourceUnchanged = JSON.stringify(result.sourceHashesAfter) === JSON.stringify(before);
  result.pass = result.pass && result.sourceUnchanged;
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  console.log(JSON.stringify({pass:result.pass, resultPath:path.join(out,"result.json"), resultSha256:sha(fs.readFileSync(path.join(out,"result.json"))), sourceUnchanged:result.sourceUnchanged, error:result.error}));
  if (!result.pass) process.exitCode = 1;
}
