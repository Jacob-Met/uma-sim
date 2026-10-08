import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/components/conditionsView.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
}).outputText;
const { conditionsView } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`
);

test("an empty collection is reported separately from absent or malformed data", () => {
  assert.deepEqual(conditionsView([]), { reported: true, hasInjury: false, rows: [] });
  for (const value of [undefined, null, "", {}, { statuses: [] }, ["injured", null], [1]]) {
    assert.deepEqual(conditionsView(value), { reported: false, hasInjury: false, rows: [] });
  }
});

test("injury matching follows the native exact injured or injury-substring predicate", () => {
  const ids = ["injured", "INJURED", "InJuReD", "left-leg-injury", "INJURY:training", "injury-free"];
  const view = conditionsView(ids);
  assert.equal(view.hasInjury, true);
  assert.deepEqual(view.rows.map(row => [row.id, row.kind, row.label]),
    ids.map(id => [id, "injury", "Injury"]));
});

test("similar words and whitespace do not widen the native injury predicate", () => {
  const ids = ["uninjured", "injure", "injuries", " injured ", "İNJURED", "inju ry"];
  const view = conditionsView(ids);
  assert.equal(view.hasInjury, false);
  assert(view.rows.every(row => row.kind === "other" && row.label === null));
  assert.deepEqual(view.rows.map(row => row.id), ids);
});

test("only the three exact simulator epithet IDs receive title labels", () => {
  const view = conditionsView(["epithet:sim_g1_win", "epithet:sim_climax_win", "epithet:sim_ura_finale_win"]);
  assert.deepEqual(view.rows.map(row => [row.label, row.kind]), [
    ["G1 win", "title"], ["Climax win", "title"], ["URA finale win", "title"],
  ]);
  assert.equal(view.hasInjury, false);
});

test("unknown title-like IDs remain literal without invented names or effects", () => {
  const ids = ["epithet:future", "EPITHET:sim_g1_win", "epithet:sim_g1_win-extra", "friendly-status"];
  const view = conditionsView(ids);
  assert.deepEqual(view.rows, ids.map(id => ({ id, label: null, kind: "other" })));
});

test("native injury admission takes precedence over a title-like namespace", () => {
  const view = conditionsView(["epithet:future_injury_win"]);
  assert.equal(view.hasInjury, true);
  assert.deepEqual(view.rows, [{ id: "epithet:future_injury_win", label: "Injury", kind: "injury" }]);
});

test("every retained entry survives in order including duplicates and long collections", () => {
  const ids = Object.freeze(["injured", ...Array.from({ length: 40 }, (_, i) => "status:" + i), "injured"]);
  const view = conditionsView(ids);
  assert.equal(view.rows.length, 42);
  assert.deepEqual(view.rows.map(row => row.id), ids);
  assert.equal(view.rows.filter(row => row.kind === "injury").length, 2);
});

test("markup-like strings, whitespace and object-property names remain exact data", () => {
  const ids = Object.freeze(['<img src=x onerror="window.statusExecuted=true">', "__proto__", "constructor", "", " \t", "雪"]);
  const view = conditionsView(ids);
  assert.deepEqual(view.rows.map(row => row.id), ids);
  assert(view.rows.every(row => row.label === null && row.kind === "other"));
});

test("a later recovered snapshot removes the warning and retains unrelated titles and statuses", () => {
  const before = Object.freeze(["injured", "epithet:sim_g1_win", "steady"]);
  const after = Object.freeze(["epithet:sim_g1_win", "steady"]);
  const first = conditionsView(before);
  const second = conditionsView(after);
  assert.equal(first.hasInjury, true);
  assert.equal(second.hasInjury, false);
  assert.deepEqual(second.rows.map(row => row.id), after);
  assert.deepEqual(first.rows.map(row => row.id), before);
});
