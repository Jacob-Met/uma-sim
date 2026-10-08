import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/components/inheritanceView.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
}).outputText;
const { inheritanceView } = await import(
  "data:text/javascript;base64," + Buffer.from(compiled).toString("base64")
);

function ancestor(name = "") {
  return {
    uma: name,
    blue: { factorId: "", stars: 3 },
    pink: { factorId: "", stars: 3 },
    white: { factorId: "", stars: 1 },
    green: { factorId: "", stars: 1 },
    race: { factorId: "", stars: 1 },
  };
}

function legacy(patch = {}) {
  return {
    parentNames: [],
    factorIds: [],
    inheritedSkillIds: [],
    sparkCaps: {},
    blueStartBonuses: {},
    inheritanceComplete: false,
    inspirationEventsDone: 0,
    ...patch,
  };
}

function freeze(value) {
  if (value && typeof value === "object") {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}

test("all six recorded positions and repeated sparks survive without star normalization", () => {
  const keys = ["parentA", "gpA1", "gpA2", "parentB", "gpB1", "gpB2"];
  const tree = Object.fromEntries(keys.map((key, i) => {
    const node = ancestor("Ancestor " + i);
    node.blue = { factorId: "factor:blue:1", stars: [3, 1, 0, 4, -1, 2][i] };
    return [key, node];
  }));
  tree.parentA.pink = { factorId: "factor:blue:1", stars: 2 };
  delete tree.gpB2.blue.stars;
  const view = inheritanceView({ legacyTree: tree, legacyFactors: [], compatibilityScore: 0 }, legacy(), []);
  assert.deepEqual(view.ancestors.map((row) => row.key), keys);
  assert.deepEqual(view.ancestors.map((row) => row.name), keys.map((_, i) => "Ancestor " + i));
  assert.deepEqual(view.ancestors.map((row) => row.sparks[0].stars), [3, 1, 0, 4, -1, null]);
  assert.equal(view.ancestors.flatMap((row) => row.sparks).length, 7);
  assert.deepEqual(view.ancestors[0].sparks.map((row) => row.kind), ["blue", "pink"]);
  assert.equal(view.compatibility, 0);
});

test("flat-only input remains literal and has no invented ancestor or star rows", () => {
  const entries = ["factor:blue:1@3", "future@not-a-star", "factor:blue:1@1", "", "<literal>"];
  const view = inheritanceView({ legacyFactors: entries, parentNames: ["input only"] }, legacy(), []);
  assert.equal(view.ancestors, null);
  assert.deepEqual(view.flatEntries, entries);
  assert.deepEqual(view.parentNames, []);
  assert.equal(view.compatibility, null);
});

test("retained bonuses and progress are displayed directly even when factor inputs differ", () => {
  const view = inheritanceView({ legacyFactors: ["factor:blue:1@3"] }, legacy({
    parentNames: ["Native A", "Native B"],
    factorIds: ["factor:blue:1", "factor:blue:1", "unknown"],
    sparkCaps: { speed: 7, stamina: 0, future: -2 },
    blueStartBonuses: { speed: 101, wit: 12 },
    inheritanceComplete: true,
    inspirationEventsDone: 1,
    inheritedSkillIds: ["skill:1", "skill:1"],
  }), []);
  assert.deepEqual(view.parentNames, ["Native A", "Native B"]);
  assert.deepEqual(view.capAdditions.map((row) => [row.key, row.value]), [["speed", 7], ["stamina", 0], ["future", -2]]);
  assert.deepEqual(view.startingBonuses.map((row) => [row.key, row.value]), [["speed", 101], ["wit", 12]]);
  assert.deepEqual(view.retainedFactors.map((row) => row.id), ["factor:blue:1", "factor:blue:1", "unknown"]);
  assert.deepEqual(view.inheritedSkillIds, ["skill:1", "skill:1"]);
  assert.equal(view.choiceComplete, true);
  assert.equal(view.inspirations, 1);
  assert.equal(Object.prototype.hasOwnProperty.call(view, "selectedChoice"), false);
});

test("missing data stays distinct from a recorded empty inheritance", () => {
  const missing = inheritanceView({}, undefined, []);
  for (const key of ["parentNames", "ancestors", "flatEntries", "retainedFactors", "inheritedSkillIds", "capAdditions", "startingBonuses", "choiceComplete", "inspirations"]) {
    assert.equal(missing[key], null, key);
  }
  const empty = inheritanceView({ legacyFactors: [], legacyTree: {} }, legacy(), []);
  assert.deepEqual(empty.parentNames, []);
  assert.deepEqual(empty.retainedFactors, []);
  assert.deepEqual(empty.startingBonuses, []);
  assert.equal(empty.choiceComplete, false);
  assert.equal(empty.inspirations, 0);
  assert.equal(empty.ancestors.length, 6);
  assert.ok(empty.ancestors.every((row) => !row.recorded));
});

test("recorded empty nodes and unavailable slots are not treated as a missing tree", () => {
  const full = ancestor();
  const partial = { uma: "Partial", blue: { factorId: "factor:future" } };
  const view = inheritanceView({ legacyTree: { parentA: full, parentB: partial } }, legacy(), []);
  assert.equal(view.ancestors[0].recorded, true);
  assert.equal(view.ancestors[0].name, "");
  assert.deepEqual(view.ancestors[0].sparks, []);
  assert.deepEqual(view.ancestors[0].unavailableSlots, []);
  assert.equal(view.ancestors[3].sparks[0].stars, null);
  assert.deepEqual(view.ancestors[3].unavailableSlots, ["Pink", "White", "Green", "Race"]);
});

test("catalog labels require exact IDs and preserve literal unknowns and first entries", () => {
  const view = inheritanceView({}, legacy({
    factorIds: ["factor:1", "FACTOR:1", "unknown", "__proto__", "empty-name"],
    sparkCaps: JSON.parse('{"__proto__":4,"constructor":5,"speed":0}'),
  }), [
    { id: "factor:1", name: "<img src=x> Δ" },
    { id: "factor:1", name: "Later duplicate" },
    { id: "empty-name", name: " " },
    { id: "empty-name", name: "Later replacement" },
  ]);
  assert.deepEqual(view.retainedFactors.map((row) => row.name), ["<img src=x> Δ", null, null, null, null]);
  assert.deepEqual(view.capAdditions.map((row) => [row.label, row.value]), [["Speed", 0], ["__proto__", 4], ["constructor", 5]]);
});

test("replacing a career and catalogs replaces the record without mutating either input", () => {
  const meta = freeze({ legacyFactors: ["old@3"], legacyTree: { parentA: ancestor("Old") } });
  const saved = freeze(legacy({ parentNames: ["Old"], factorIds: ["old"], sparkCaps: { speed: 16 } }));
  const catalog = freeze([{ id: "old", name: "Old factor" }]);
  const before = JSON.stringify({ meta, saved, catalog });
  const first = inheritanceView(meta, saved, catalog);
  first.parentNames[0] = "only the view copy";
  const next = inheritanceView({ legacyFactors: ["new@1"] }, legacy({ parentNames: ["New"], factorIds: ["new"] }), [{ id: "new", name: "New factor" }]);
  assert.equal(JSON.stringify({ meta, saved, catalog }), before);
  assert.equal(next.ancestors, null);
  assert.deepEqual(next.parentNames, ["New"]);
  assert.deepEqual(next.retainedFactors, [{ id: "new", name: "New factor" }]);
  assert.deepEqual(next.capAdditions, []);
  assert.ok(!JSON.stringify(next).includes("Old"));
});

test("unavailable scalar and collection values never become invented zeroes or bonuses", () => {
  const view = inheritanceView({ compatibilityScore: "0", legacyFactors: null, legacyTree: [] }, {
    parentNames: [1], factorIds: "factor:1", sparkCaps: { speed: null }, blueStartBonuses: [],
    inheritanceComplete: "true", inspirationEventsDone: Number.NaN,
  }, []);
  assert.equal(view.compatibility, null);
  assert.equal(view.flatEntries, null);
  assert.equal(view.ancestors, null);
  assert.equal(view.parentNames, null);
  assert.equal(view.retainedFactors, null);
  assert.deepEqual(view.capAdditions, [{ key: "speed", label: "Speed", value: null }]);
  assert.equal(view.startingBonuses, null);
  assert.equal(view.choiceComplete, null);
  assert.equal(view.inspirations, null);
});
