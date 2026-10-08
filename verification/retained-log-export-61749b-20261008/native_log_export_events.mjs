import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { resolveObjectURL } from "node:buffer";
import { createHash } from "node:crypto";

// Native Node/React event receiving only. document/anchor dispatch is simulated;
// Blob, object-URL registration/revocation, timers and React hooks are real.
// No browser process, browser surface, HTTP server or provider is used.
const [uiArg, outputArg, mode, rendererArg] = process.argv.slice(2);
assert(uiArg && outputArg && ["baseline", "candidate"].includes(mode) && rendererArg,
  "Usage: node native_log_export_events.mjs UI_ROOT OUTPUT baseline|candidate MATCHING_REACT_TEST_RENDERER_ROOT");
const ui = path.resolve(uiArg);
const output = path.resolve(outputArg);
await fs.mkdir(output, { recursive: true });
const require = createRequire(path.join(ui, "package.json"));
const rendererRequire = createRequire(path.join(path.resolve(rendererArg), "package.json"));
assert.equal(require.resolve("react"), rendererRequire.resolve("react"), "use the same React instance");
const React = require("react");
const renderer = rendererRequire("./index.js");
const ts = require("typescript");
assert.equal(rendererRequire("./package.json").version, require("react/package.json").version,
  "renderer must match React's exact version");
assert.notEqual(process.env.NODE_ENV, "production", "React event QA uses its development act support");
const { act } = renderer;
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const moduleCache = new Map();
const sourcePins = [];
async function moduleUrl(name) {
  if (moduleCache.has(name)) return moduleCache.get(name);
  const filename = path.join(ui, "src/components", name);
  const source = await fs.readFile(filename, "utf8");
  sourcePins.push({ path: `src/components/${name}`, sha256: digest(source) });
  let code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX,
  } }).outputText;
  const locals = [...code.matchAll(/from ["'](\.\/[^"']+)["']/g)].map((m) => m[1]);
  for (const local of new Set(locals)) {
    const stem = local.slice(2);
    let target;
    for (const extension of [".tsx", ".ts"]) {
      try { await fs.access(path.join(ui, "src/components", stem + extension)); target = stem + extension; break; }
      catch (error) { if (error.code !== "ENOENT") throw error; }
    }
    assert(target, `local module exists: ${local}`);
    code = code.replaceAll(JSON.stringify(local), JSON.stringify(await moduleUrl(target)));
  }
  for (const bare of ["react", "react/jsx-runtime"]) {
    code = code.replaceAll(JSON.stringify(bare), JSON.stringify(pathToFileURL(require.resolve(bare)).href));
  }
  code = code.replace(/^import ["'][^"']+\.css["'];\s*/gm, "");
  const url = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
  moduleCache.set(name, url);
  return url;
}
const { LogPanel } = await import(await moduleUrl("LogPanel.tsx"));
const fixture = Object.freeze([
  "Started — ウマ娘 🏇", "Rested", "", "Race authored result +10 fans",
  "Rested", "line 1\r\nline 2", "<script>literal</script> \ud83d",
]);
let currentHistory = fixture;
let component;
const cases = [];
const originalCreate = URL.createObjectURL;
const originalRevoke = URL.revokeObjectURL;
const originalDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
const originalFetch = globalThis.fetch;
let requestCount = 0;
let refusePreparation = false;
const liveLinks = new Set();
const created = [];
const revoked = [];
const downloads = [];
URL.createObjectURL = (blob) => {
  if (refusePreparation) throw new Error("authored local preparation refusal");
  const url = originalCreate(blob); created.push(url); return url;
};
URL.revokeObjectURL = (url) => { revoked.push(url); originalRevoke(url); };
globalThis.fetch = () => { requestCount += 1; throw new Error("receiving forbids network requests"); };
Object.defineProperty(globalThis, "document", { configurable: true, value: {
  createElement(tag) {
    assert.equal(tag, "a");
    return {
      href: "", download: "",
      click() {
        assert(liveLinks.has(this), "the request anchor was attached");
        const blob = resolveObjectURL(this.href);
        assert(blob instanceof Blob, "the actual Node object URL resolves to a Blob at the click");
        downloads.push({ filename: this.download, blob, url: this.href });
      },
      remove() { liveLinks.delete(this); },
    };
  },
  body: { appendChild(link) { liveLinks.add(link); } },
} });
function text(node) {
  if (node == null) return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  return (node.children ?? []).map(text).join("");
}
function controls() {
  return component.root.findAllByType("button").filter((button) => text(button).startsWith("Download all "));
}
function reader() {
  return {
    query: component.root.findByProps({ type: "search" }).props.value,
    status: text(component.root.findByProps({ role: "status" })),
    entries: component.root.findAllByType("li").map(text),
  };
}
function alerts() { return component.root.findAllByProps({ role: "alert" }).map(text); }
async function clickDownload() {
  const before = reader();
  act(() => controls()[0].props.onClick());
  assert.deepEqual(reader(), before, "export preserves search, following and the visible sequence");
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(liveLinks.size, 0);
}
function chooseFormat(value) {
  const select = component.root.findByType("select");
  act(() => select.props.onChange({ target: { value } }));
}
function search(value) {
  act(() => component.root.findByProps({ type: "search" }).props.onChange({ target: { value } }));
}
function replaceHistory(history) {
  currentHistory = Object.freeze([...history]);
  act(() => component.update(React.createElement(LogPanel, { history: currentHistory, lines: ["Authored current summary"] })));
}
let receipt;
try {
  act(() => { component = renderer.create(React.createElement(LogPanel, {
    history: currentHistory, lines: ["Authored current summary"],
  })); });
  assert.equal(reader().entries.length, fixture.length);
  search("race");
  assert.equal(reader().entries.length, 1);
  assert.equal(reader().query, "race");
  if (mode === "baseline") {
    assert.equal(controls().length, 0, "the pinned original has no retained-log download control");
    receipt = { mode, result: "original capability absence reproduced", controls: reader(), download_controls: 0,
      fixture: [...fixture], cases: [{ name: "retained reader works but cannot download its history", passed: true }] };
  } else {
    assert.equal(controls().length, 1);
    assert.equal(text(controls()[0]), "Download all 7 entries");
    chooseFormat("json");
    await clickDownload();
    assert.deepEqual(JSON.parse(await downloads.at(-1).blob.text()).entries, fixture);
    cases.push({ name: "one-match search exports all seven retained entries", passed: true });

    search("no such entry");
    assert.equal(reader().entries.length, 0);
    assert.equal(controls()[0].props.disabled, false);
    await clickDownload();
    assert.deepEqual(JSON.parse(await downloads.at(-1).blob.text()).entries, fixture);
    cases.push({ name: "zero-match search keeps the full-history download enabled", passed: true });

    const prepared = await downloads[0].blob.text();
    replaceHistory([...fixture, "Later retained event"]);
    assert.equal(text(controls()[0]), "Download all 8 entries");
    await clickDownload();
    assert.deepEqual(JSON.parse(await downloads.at(-1).blob.text()).entries, currentHistory);
    assert.equal(await downloads[0].blob.text(), prepared);
    cases.push({ name: "append changes only the next click's snapshot", passed: true });

    replaceHistory(["Replacement career", "", "Replacement career"]);
    assert.equal(reader().entries.length, 0, "the old search text still has no match");
    await clickDownload();
    assert.deepEqual(JSON.parse(await downloads.at(-1).blob.text()).entries, currentHistory);
    cases.push({ name: "replacement history exports only the currently displayed career", passed: true });

    chooseFormat("text");
    await clickDownload();
    const readable = await downloads.at(-1).blob.text();
    assert.equal(downloads.at(-1).filename, "uma-sim-retained-log.txt");
    assert.equal(downloads.at(-1).blob.type, "text/plain;charset=utf-8");
    assert(readable.endsWith("[Entry 1]\nReplacement career\n\n[Entry 2]\n\n\n[Entry 3]\nReplacement career\n"));
    assert(readable.includes("Earlier events may no longer be retained."));
    cases.push({ name: "format selection requests a readable ordered text Blob", passed: true });

    refusePreparation = true;
    const countBefore = downloads.length;
    await clickDownload();
    assert.equal(downloads.length, countBefore);
    assert.deepEqual(alerts(), ["Could not start the log download. Please try again."]);
    refusePreparation = false;
    chooseFormat("json");
    await clickDownload();
    assert.equal(downloads.length, countBefore + 1);
    assert.deepEqual(alerts(), []);
    assert.deepEqual(JSON.parse(await downloads.at(-1).blob.text()).entries, currentHistory);
    cases.push({ name: "preparation refusal stays visible until an explicit successful retry", passed: true });

    replaceHistory([]);
    assert.equal(text(controls()[0]), "Download all 0 entries");
    assert.equal(controls()[0].props.disabled, true);
    assert.equal(requestCount, 0);
    assert.deepEqual(fixture, ["Started — ウマ娘 🏇", "Rested", "", "Race authored result +10 fans", "Rested", "line 1\r\nline 2", "<script>literal</script> \ud83d"]);
    assert.deepEqual(revoked, created);
    assert(created.every((url) => resolveObjectURL(url) === undefined));
    cases.push({ name: "empty-history control, unchanged inputs, no fetch, all URLs released", passed: true });
    receipt = { mode, result: "pass", fixture: [...fixture], cases, local_download_requests: downloads.length };
  }
  const outputFiles = [];
  for (const [index, item] of downloads.entries()) {
    const bytes = Buffer.from(await item.blob.arrayBuffer());
    const filename = `native-blob-${index + 1}.${item.filename.endsWith(".json") ? "json" : "txt"}`;
    await fs.writeFile(path.join(output, filename), bytes, { flag: "wx" });
    outputFiles.push({ file: filename, requested_filename: item.filename, content_type: item.blob.type,
      bytes: bytes.length, sha256: digest(bytes) });
  }
  for (const pin of sourcePins) assert.equal(digest(await fs.readFile(path.join(ui, pin.path))), pin.sha256);
  Object.assign(receipt, {
    boundary: "actual native React hooks/callbacks and Node Blob/URL lifecycle; simulated document/anchor dispatch; no browser or real save dialog",
    ui_root: ui, source_files: sourcePins,
    harness_sha256: digest(await fs.readFile(new URL(import.meta.url))),
    runtime: { node: process.version, typescript: ts.version, react: React.version,
      renderer: rendererRequire("./package.json").version, renderer_root: path.resolve(rendererArg),
      react_entry_sha256: digest(await fs.readFile(require.resolve("react"))),
      renderer_entry_sha256: digest(await fs.readFile(rendererRequire.resolve("./index.js"))) },
    fetch_requests: requestCount, remaining_anchors: liveLinks.size,
    object_urls_created: created.length, object_urls_revoked: revoked.length,
    files: outputFiles, source_unchanged: true, browser_acceptance: "unestablished",
  });
  await fs.writeFile(path.join(output, "receipt.json"), JSON.stringify(receipt, null, 2) + "\n", { flag: "wx" });
  console.log(JSON.stringify({ mode, result: receipt.result, cases: receipt.cases.length,
    native_blobs: outputFiles.length, fetch_requests: requestCount, source_unchanged: true }));
} finally {
  if (component) act(() => component.unmount());
  await new Promise((resolve) => setTimeout(resolve, 0));
  for (const url of created) originalRevoke(url);
  URL.createObjectURL = originalCreate;
  URL.revokeObjectURL = originalRevoke;
  globalThis.fetch = originalFetch;
  if (originalDocument) Object.defineProperty(globalThis, "document", originalDocument);
  else delete globalThis.document;
}
