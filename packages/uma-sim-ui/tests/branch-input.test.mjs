import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

// Run the production TypeScript with the project's already locked compiler.
// The type-only import disappears; no copied parser or additional dependency.
const source = await readFile(new URL("../src/components/branchInput.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const { validateBranchDraft } = await import(
  "data:text/javascript;base64," + Buffer.from(outputText).toString("base64")
);

test("preserves the full authored order, blank lines, turn zero and colon action IDs", () => {
  assert.deepEqual(validateBranchDraft(" \r\n 0 : event_0:followup \r\n\r\n8:rest\n10:train_stamina ", "120"), {
    ok: true,
    maxActions: 120,
    overrides: [
      { turn: 0, actionId: "event_0:followup" },
      { turn: 8, actionId: "rest" },
      { turn: 10, actionId: "train_stamina" },
    ],
  });
});

test("a policy-only run is valid with no override text", () => {
  assert.deepEqual(validateBranchDraft("\n \r\n", "1"), {
    ok: true, maxActions: 1, overrides: [],
  });
});

test("accepts only the exact representable integer boundary and allowed maximum budget", () => {
  assert.deepEqual(validateBranchDraft("2147483647:rest", "500"), {
    ok: true, maxActions: 500, overrides: [{ turn: 2147483647, actionId: "rest" }],
  });
});

for (const [name, text] of [
  ["missing colon", "8:rest\nnot-an-override\n10:train_stamina"],
  ["missing turn", ":rest"],
  ["negative turn", "-1:rest"],
  ["fractional turn", "10.5:rest"],
  ["scientific notation", "1e2:rest"],
  ["hexadecimal notation", "0x10:rest"],
  ["non-number turn", "NaN:rest"],
  ["infinite turn", "Infinity:rest"],
  ["signed integer overflow", "2147483648:rest"],
  ["integer that would wrap to turn 8", "4294967304:rest"],
  ["unsafe integer", "9007199254740993:rest"],
  ["empty action", "8: \r\n"],
]) {
  test("refuses the whole draft for " + name, () => {
    const result = validateBranchDraft(text, "120");
    assert.equal(result.ok, false);
    assert.equal("overrides" in result, false, "a failed validation exposes no partial command list");
    assert.equal(result.issues.every(issue => issue.field === "overrides"), true);
    assert.match(result.issues[0].message, /^Line \d+: /);
  });
}

test("duplicate turns report both source lines rather than picking the first silently", () => {
  const result = validateBranchDraft("\n8:rest\n\n008:train_speed", "120");
  assert.equal(result.ok, false);
  assert.deepEqual(result.issues.map(issue => issue.line), [4]);
  assert.match(result.issues[0].message, /turn 8 is already set on line 2/);
});

test("collects actionable line errors without discarding valid intervening input", () => {
  const text = "bad line\n8:rest\n10.5:train_speed\n12:";
  const result = validateBranchDraft(text, "120");
  assert.equal(result.ok, false);
  assert.deepEqual(result.issues.map(issue => issue.line), [1, 3, 4]);
  assert.equal("overrides" in result, false);
});

for (const limit of ["", " ", "0", "501", "1.5", "-1", "NaN", "Infinity", "1e2", "9007199254740993"]) {
  test("refuses invalid action budget " + JSON.stringify(limit), () => {
    const result = validateBranchDraft("8:rest", limit);
    assert.equal(result.ok, false);
    assert.deepEqual(result.issues.map(issue => issue.field), ["maxActions"]);
    assert.match(result.issues[0].message, /whole number from 1 through 500/);
    assert.equal("maxActions" in result, false);
    assert.equal("overrides" in result, false);
  });
}

test("reports both budget and override errors in a single review", () => {
  const result = validateBranchDraft("8:", "");
  assert.equal(result.ok, false);
  assert.deepEqual(result.issues.map(issue => issue.field), ["maxActions", "overrides"]);
});

test("preserves literal action text for the existing simulator legality check", () => {
  const actionId = 'event:<tag>&"choice"';
  assert.deepEqual(validateBranchDraft("008: " + actionId, " 00120 "), {
    ok: true, maxActions: 120, overrides: [{ turn: 8, actionId }],
  });
});
