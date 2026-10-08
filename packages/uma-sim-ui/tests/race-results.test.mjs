// SPDX-License-Identifier: GPL-3.0-only
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { after, test } from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import ts from "typescript";

const require = createRequire(import.meta.url);
const temporary = await mkdtemp(join(tmpdir(), "uma-race-results-"));
after(() => rm(temporary, { recursive: true, force: true }));
await symlink(dirname(dirname(require.resolve("react/package.json"))), join(temporary, "node_modules"), "dir");
for (const [sourceName, outputName] of [["raceResults.ts", "raceResults.js"], ["RacePanel.tsx", "RacePanel.cjs"]]) {
  const source = await readFile(new URL("../src/components/" + sourceName, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
  });
  await writeFile(join(temporary, outputName), outputText);
}
// Real browser receiving loads the actual component stylesheet.
await writeFile(join(temporary, "race-results.css"), "");
const { parseRaceResult, parseRaceResults } = require(join(temporary, "raceResults.js"));
const { RacePanel } = require(join(temporary, "RacePanel.cjs"));

// Retained verbatim from an isolated real 4dc00f API career, seed29, physics.
const physics = "Race debut 9th +321 fans [physics t=75.400s course=10601 seed=230167224 field=9 margin_win=7.867s margin_ahead=0.000s]";
// Separate actual seed29 run with the explicitly selected stub backend.
const fanOnly = "Race debut +1072 fans";
const render = (lines, options = {}) => renderToStaticMarkup(React.createElement(RacePanel, {
  lines, mandatory: false, busy: false, onRace: () => {}, ...options,
}));

test("native physics record keeps the observed place, field, precision and source metadata", () => {
  assert.deepEqual(parseRaceResults(["Ordinary training", physics]), [{
    kind: "physics", logIndex: 1, summary: physics, raceId: "debut", place: 9,
    placeLabel: "9th", fieldSize: 9, finishTime: "75.400", fanGain: 321,
    courseId: "10601", seed: "230167224", marginToWinner: "7.867", marginAhead: "0.000",
  }]);
});

test("fan-only records retain zero and never invent missing race statistics", () => {
  for (const line of [fanOnly, "Race optional +0 fans"]) {
    const result = parseRaceResult([line]);
    assert.equal(result.kind, "fan-only");
    assert.deepEqual(Object.keys(result).sort(), ["fanGain", "kind", "logIndex", "raceId", "summary"]);
  }
  assert.equal(parseRaceResult(["Race optional +0 fans"]).fanGain, 0);
});

test("repeated race IDs and identical occurrences retain separate source positions", () => {
  const lines = Object.freeze([physics, "Rest", fanOnly, physics]);
  const results = parseRaceResults(lines);
  assert.deepEqual(results.map(result => result.logIndex), [0, 2, 3]);
  assert.deepEqual(results.map(result => result.summary), [physics, fanOnly, physics]);
  assert.equal(parseRaceResult(lines).logIndex, 3);
  results[0].raceId = "changed result";
  assert.equal(parseRaceResults(lines)[0].raceId, "debut");
  assert.deepEqual(lines, [physics, "Rest", fanOnly, physics]);
});

test("non-race entries and action narration are not retained race records", () => {
  const lines = ["Racetrack opened", "Event: Race debut +500 fans", "Mandatory race (debut): 1st! +500 fans", "  Race debut +500 fans"];
  assert.deepEqual(parseRaceResults(lines), []);
  assert.equal(parseRaceResult(lines), null);
});

test("a newest unrecognized record stays latest even when a valid older result exists", () => {
  for (const newest of ["Race next result withheld", "Race", "Race\tdebut +1072 fans", physics + " later text"]) {
    const result = parseRaceResult([physics, newest, "Ordinary rest"]);
    assert.deepEqual(result, { kind: "unknown", logIndex: 1, summary: newest });
    assert.equal(parseRaceResults([physics, newest]).length, 2);
  }
});

test("invalid numeric fields or inconsistent place and field remain literal unknown records", () => {
  const variants = [
    physics.replace("9th", "0th"), physics.replace("9th", "9st"),
    physics.replace("field=9", "field=8"), physics.replace("field=9", "field=0"),
    physics.replace("75.400s", "0.000s"), physics.replace("75.400s", "NaNs"),
    physics.replace("75.400s", "Infinitys"), physics.replace("75.400s", "1e3s"),
    physics.replace("75.400s", "-1.000s"), physics.replace("75.400s", ".5s"),
    physics.replace("+321", "+9007199254740993"), physics.replace("+321", "+-1"),
    physics.replace("field=9", "field=9007199254740993"),
    physics.replace("seed=230167224", "seed=4294967296"),
    physics.replace("course=10601", "course=4294967296"),
    physics.replace("margin_win=7.867s", "margin_win=-1.000s"),
    physics.replace("margin_ahead=0.000s", "margin_ahead=NaNs"),
    physics.replace("[physics", "[different-model"), physics + "\n",
    "Race optional +9007199254740993 fans",
  ];
  for (const line of variants) {
    assert.deepEqual(parseRaceResult([line]), { kind: "unknown", logIndex: 0, summary: line }, line);
  }
});

test("native ordinal suffixes keep 11th to 13th distinct from 21st to 23rd", () => {
  for (const place of ["1st", "2nd", "3rd", "11th", "12th", "13th", "21st", "22nd", "23rd", "111th"]) {
    const line = physics.replace("9th", place).replace("field=9", "field=200");
    assert.equal(parseRaceResult([line]).placeLabel, place);
  }
});

test("rendered physics facts show exact native values and the full original record", () => {
  const html = render(["Training", physics]);
  assert.match(html, /aria-labelledby=/);
  assert.match(html, /Latest retained race/);
  for (const value of ["9th", "9", "75.400 s", "+321"]) assert.ok(html.includes(`<dd>${value}</dd>`));
  assert.match(html, /Original race record · log entry 2/);
  assert.ok(html.includes(physics));
  assert.match(html, /1 retained race record in the loaded career log/);
  assert.doesNotMatch(html, /Earlier retained races/);
});

test("rendered fan-only facts state the missing details without inventing first place", () => {
  const html = render([fanOnly]);
  assert.match(html, /<dt>Fan gain<\/dt><dd>\+1072<\/dd>/);
  assert.match(html, /Place, finish time and field size were not recorded/);
  assert.doesNotMatch(html, /<dt>Place|<dt>Field size|<dt>Finish time|1st|1 place/);
});

test("unknown latest is visible while earlier recognized occurrences stay in history", () => {
  const html = render([physics, fanOnly, physics, "Race novel-format <unknown>"]);
  const latest = html.slice(html.indexOf('class="race-latest"'), html.indexOf('class="race-history"'));
  assert.match(latest, /Unrecognized race record/);
  assert.doesNotMatch(latest, /<dt>Place|<dt>Fan gain|<dt>Finish time/);
  assert.match(html, /Earlier retained races \(3\)/);
  assert.equal((html.match(/data-race-kind="physics"/g) ?? []).length, 2);
  assert.match(html, /4 retained race records/);
});

test("mandatory entry remains explicit and disabled while busy", () => {
  const html = render([], { mandatory: true, pendingRaceId: "debut", busy: true });
  assert.match(html, /Mandatory race: debut/);
  assert.match(html, /<button class="primary" disabled="">Enter race<\/button>/);
  assert.match(html, /No race result recorded yet/);
});

test("replacing the supplied history cannot retain an old result", () => {
  assert.match(render([physics]), /75.400 s/);
  assert.equal(render([]), "");
  const replacement = render(["Race second +0 fans"]);
  assert.match(replacement, />second<\/h3>/);
  assert.doesNotMatch(replacement, /debut|75.400|9th/);
});

test("unusual race IDs and unrecognized source text are rendered literally", () => {
  const html = render(["Race <svg/onload=alert(1)> +0 fans", "Race 比較🦄 <img src=x onerror=alert(1)>"]);
  assert.match(html, /&lt;svg\/onload=alert\(1\)&gt;/);
  assert.match(html, /比較🦄 &lt;img src=x onerror=alert\(1\)&gt;/);
  assert.doesNotMatch(html, /<svg|<img|<script/);
});
