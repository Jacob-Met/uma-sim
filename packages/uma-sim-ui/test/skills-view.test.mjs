import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/components/skillsView.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
}).outputText;
const { parseSkillCatalog, skillRows } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`
);

test("learned names and descriptions use exact IDs with a literal unknown fallback", () => {
  const catalog = parseSkillCatalog({ items: [
    { id: "skill:1", name: "Known skill", description: "Catalog description." },
  ] });
  const view = skillRows(["skill:1", "skill:unknown"], {}, catalog);
  assert.deepEqual(view.learned, [
    { id: "skill:1", name: "Known skill", description: "Catalog description.", kind: "skill" },
    { id: "skill:unknown", name: null, description: null, kind: "skill" },
  ]);
  assert.deepEqual(view.skillHints, []);
  assert.deepEqual(view.otherHints, []);
});

test("all retained learned entries survive in order, including duplicates beyond 24", () => {
  const ids = Object.freeze([...Array.from({ length: 32 }, (_, i) => `skill:${i}`), "skill:0"]);
  const view = skillRows(ids, Object.freeze({}), new Map());
  assert.equal(view.learned.length, 33);
  assert.deepEqual(view.learned.map(row => row.id), ids);
});

test("only skill namespace hints are skills; raw names and training keys remain separate", () => {
  const catalog = parseSkillCatalog({ items: [
    { id: "skill:1", name: "Hydrate", description: "A skill." },
  ] });
  const hints = Object.freeze({ "skill:1": 2, speed: 1, Hydrate: 3, generic: 0, "skill:future": 4 });
  const view = skillRows([], hints, catalog);
  assert.deepEqual(view.learned, []);
  assert.deepEqual(view.skillHints.map(row => [row.id, row.level]), [["skill:1", 2], ["skill:future", 4]]);
  assert.deepEqual(view.otherHints.map(row => [row.id, row.name, row.level]), [
    ["Hydrate", "Hydrate", 3], ["generic", "Generic hint", 0], ["speed", "Speed training", 1],
  ]);
});

test("hint levels are retained values, with no cap, discount, or purchase inference", () => {
  const view = skillRows(["skill:2"], { "skill:2": 0, "skill:3": 9, wit: -0.5 }, new Map());
  assert.equal(view.learned.length, 1);
  assert.deepEqual(view.skillHints.map(row => row.level), [0, 9]);
  assert.equal(view.otherHints[0].level, -0.5);
  assert.equal(view.learned[0].level, undefined);
});

test("catalog failure and malformed entries do not manufacture names or accept an HTML fallback", () => {
  for (const value of [null, "<html>old server</html>", {}, { items: {} }]) {
    assert.throws(() => parseSkillCatalog(value));
  }
  const view = parseSkillCatalog({ items: [
    null, false, { id: 1 }, { id: "speed", name: "Not a skill" },
    { id: "skill:1", name: " ", description: 4 },
    { id: "skill:1", name: "Later duplicate" },
  ] });
  assert.deepEqual([...view.values()], [{ id: "skill:1", name: null, description: null }]);
  assert.equal(parseSkillCatalog({ items: [] }).size, 0);
});

test("markup-like text and object-property keys remain literal, without changing snapshot inputs", () => {
  const name = '<img src=x onerror="throw 1">';
  const data = Object.freeze({ items: Object.freeze([
    Object.freeze({ id: "skill:__proto__", name, description: "<script>literal</script>" }),
  ]) });
  const hints = Object.freeze(JSON.parse('{"__proto__":2,"constructor":3,"skill:__proto__":4}'));
  const before = JSON.stringify({ data, hints });
  const view = skillRows(["skill:__proto__"], hints, parseSkillCatalog(data));
  assert.equal(view.learned[0].name, name);
  assert.equal(view.learned[0].description, "<script>literal</script>");
  assert.deepEqual(view.otherHints.map(row => row.name), ["__proto__", "constructor"]);
  assert.equal(JSON.stringify({ data, hints }), before);
});
