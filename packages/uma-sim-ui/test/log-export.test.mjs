import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import ts from "typescript";

// Existing compiler and React dependencies; no browser or extra test dependency.
async function sourceModule(name, dependencies = {}) {
  const source = await readFile(new URL(`../src/components/${name}`, import.meta.url), "utf8");
  let code = ts.transpileModule(source, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.ESNext,
      jsx: ts.JsxEmit.ReactJSX,
    },
  }).outputText;
  const imports = {
    "react": import.meta.resolve("react"),
    "react/jsx-runtime": import.meta.resolve("react/jsx-runtime"),
    ...dependencies,
  };
  for (const [specifier, url] of Object.entries(imports)) {
    code = code.replaceAll(JSON.stringify(specifier), JSON.stringify(url));
  }
  code = code.replace(/^import ["'][^"']+\.css["'];\s*/gm, "");
  return `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
}

const exportUrl = await sourceModule("logExport.ts");
const { retainedLogFile, downloadLogFile } = await import(exportUrl);
const downloadUrl = await sourceModule("LogDownload.tsx", { "./logExport": exportUrl });
const { LogDownload } = await import(downloadUrl);
const viewUrl = await sourceModule("logView.ts");
const panelUrl = await sourceModule("LogPanel.tsx", { "./logView": viewUrl, "./LogDownload": downloadUrl });
const { LogPanel } = await import(panelUrl);

const entries = [
  "Started — ウマ娘 🏇",
  "", "Rested", "Rested", " leading and trailing ",
  "line 1\r\nline 2\t\u0000tail",
  "<img src=x onerror=alert(1)> </script> & ` | =SUM(A1:A2)",
  "e\u0301 é \ud83d end",
];

test("JSON preserves every original entry and explicit retained-history scope", () => {
  const history = Object.freeze([...entries]);
  const file = retainedLogFile(history, "json");
  const parsed = JSON.parse(file.content);
  assert.deepEqual(parsed, {
    format: "uma-sim-retained-log/1",
    scope: "displayed-retained-history",
    entryCount: entries.length,
    entries,
  });
  assert.equal(file.filename, "uma-sim-retained-log.json");
  assert.equal(file.contentType, "application/json;charset=utf-8");
  assert.deepEqual(history, entries);
  assert.equal(file.content.endsWith("\n"), true);
});

test("a prepared file survives later append or replacement without taking a filtered subset", () => {
  const history = ["Race one", "Rested", "Race two"];
  const file = retainedLogFile(history, "json");
  history.push("Later");
  history.splice(0, history.length, "New career");
  assert.deepEqual(JSON.parse(file.content).entries, ["Race one", "Rested", "Race two"]);
  assert.equal(JSON.parse(file.content).entryCount, 3);
});

test("readable text retains ordered duplicate and empty numbered blocks", () => {
  const file = retainedLogFile(["Rested", "", "Rested", "a\nb", "ウマ娘 🏇"], "text");
  assert.equal(file.filename, "uma-sim-retained-log.txt");
  assert.equal(file.contentType, "text/plain;charset=utf-8");
  assert.ok(file.content.includes("5 retained entries\n"));
  assert.ok(file.content.includes("Earlier events may no longer be retained."));
  assert.equal(file.content.slice(file.content.indexOf("[Entry 1]")),
    "[Entry 1]\nRested\n\n[Entry 2]\n\n\n[Entry 3]\nRested\n\n[Entry 4]\na\nb\n\n[Entry 5]\nウマ娘 🏇\n");
});

test("empty, single-entry and long histories have explicit complete counts", () => {
  assert.deepEqual(JSON.parse(retainedLogFile([], "json").content).entries, []);
  assert.ok(retainedLogFile([], "text").content.includes("0 retained entries\n"));
  assert.ok(retainedLogFile(["only"], "text").content.includes("1 retained entry\n"));
  const long = Array.from({ length: 5000 }, (_, i) => `${i} 🏇 ${"x".repeat(80)}`);
  const parsed = JSON.parse(retainedLogFile(long, "json").content);
  assert.equal(parsed.entryCount, long.length);
  assert.deepEqual(parsed.entries, long);
  assert.throws(() => retainedLogFile(["kept"], "html"), /Unsupported log file format/);
});

async function downloadBoundary(failAt, exercise) {
  const originalDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
  const create = URL.createObjectURL;
  const revoke = URL.revokeObjectURL;
  const events = [];
  let blob;
  const link = {
    href: "", download: "",
    click() { events.push("click"); if (failAt === "click") throw new Error("refused click"); },
    remove() { events.push("remove"); },
  };
  Object.defineProperty(globalThis, "document", { configurable: true, value: {
    createElement(tag) {
      events.push(`create:${tag}`);
      if (failAt === "create") throw new Error("refused element");
      return link;
    },
    body: { appendChild(value) {
      assert.equal(value, link); events.push("append");
      if (failAt === "append") throw new Error("refused append");
    } },
  } });
  URL.createObjectURL = (value) => {
    events.push("url");
    if (failAt === "url") throw new Error("refused URL");
    blob = value;
    return "blob:retained-log-test";
  };
  URL.revokeObjectURL = (value) => { assert.equal(value, "blob:retained-log-test"); events.push("revoke"); };
  try {
    await exercise({ events, link, getBlob: () => blob });
  } finally {
    await new Promise((resolve) => setTimeout(resolve, 0));
    URL.createObjectURL = create;
    URL.revokeObjectURL = revoke;
    if (originalDocument) Object.defineProperty(globalThis, "document", originalDocument);
    else delete globalThis.document;
  }
}

test("one local request delivers the actual JSON Blob and releases its URL after the click", async () => {
  await downloadBoundary(null, async ({ events, link, getBlob }) => {
    const file = retainedLogFile(entries, "json");
    downloadLogFile(file);
    assert.deepEqual(events, ["url", "create:a", "append", "click", "remove"]);
    assert.equal(link.href, "blob:retained-log-test");
    assert.equal(link.download, file.filename);
    assert.equal(getBlob() instanceof Blob, true);
    assert.equal(getBlob().type, "application/json;charset=utf-8");
    assert.deepEqual(JSON.parse(await getBlob().text()).entries, entries);
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.deepEqual(events, ["url", "create:a", "append", "click", "remove", "revoke"]);
  });
});

test("preparation and dispatch failures propagate without another click or a leaked URL", async (t) => {
  for (const failAt of ["url", "create", "append", "click"]) {
    await t.test(failAt, async () => {
      await downloadBoundary(failAt, async ({ events }) => {
        assert.throws(() => downloadLogFile(retainedLogFile(["kept"], "text")), /refused/);
        await new Promise((resolve) => setTimeout(resolve, 0));
        assert.equal(events.filter((event) => event === "url").length, 1);
        assert.equal(events.filter((event) => event === "click").length, failAt === "click" ? 1 : 0);
        assert.equal(events.filter((event) => event === "revoke").length, failAt === "url" ? 0 : 1);
      });
    });
  }
});

test("native React rendering names the full count and disables only an empty log", () => {
  const full = renderToStaticMarkup(createElement(LogDownload, { history: entries }));
  assert.match(full, /Download all 8 entries/);
  assert.match(full, /<option value="text" selected="">Plain text<\/option>/);
  assert.match(full, /<option value="json">JSON<\/option>/);
  assert.doesNotMatch(full, /disabled/);
  const empty = renderToStaticMarkup(createElement(LogDownload, { history: [] }));
  assert.match(empty, /<button disabled="">Download all 0 entries<\/button>/);
});

test("the actual LogPanel exposes the full-history download beside preserved reader and summary", () => {
  const errors = [];
  const originalError = console.error;
  console.error = (...args) => errors.push(args.map(String).join(" "));
  let markup;
  try {
    markup = renderToStaticMarkup(createElement(LogPanel, { lines: ["Current summary"], history: entries }));
  } finally { console.error = originalError; }
  assert.ok(errors.every((message) => message.includes("useLayoutEffect does nothing on the server")));
  assert.match(markup, /Search retained events/);
  assert.match(markup, /Following latest/);
  assert.equal((markup.match(/Download all 8 entries/g) ?? []).length, 1);
  assert.match(markup, /Current state summary/);
  assert.match(markup, /Current summary/);
  assert.doesNotMatch(markup, /<img src=x/);
});
