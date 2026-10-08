import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import ts from "typescript";

// Use the package's existing compiler so the same test runs on supported Node20.
const source = await readFile(new URL("../src/components/logView.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
}).outputText;
const { historyChange, searchLog } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`
);

function reconstructed(entry) {
  return entry.parts.map((part) => part.text).join("");
}

function actualSpans(entry) {
  const spans = [];
  let offset = 0;
  for (const part of entry.parts) {
    if (part.matched) spans.push([offset, offset + part.text.length]);
    offset += part.text.length;
  }
  return spans;
}

test("empty query exposes every retained entry verbatim, including empty and duplicate entries", () => {
  const history = Object.freeze(["", "Rested", "Rested", " x \n y ", "ウマ娘 🏇"]);
  const view = searchLog(history, "");
  assert.equal(view.totalEntries, 5);
  assert.equal(view.matchCount, 0);
  assert.deepEqual(view.entries.map((entry) => entry.index), [0, 1, 2, 3, 4]);
  assert.deepEqual(view.entries.map(reconstructed), history);
  assert.ok(view.entries.every((entry) => entry.matches === 0));
  assert.ok(view.entries.every((entry) => entry.parts.every((part) => !part.matched)));
});

test("matching entries and occurrences are counted separately without crossing entry boundaries", () => {
  const view = searchLog(["Rested, rested", "Race", "Rested", "Rest", "ed"], "rested");
  assert.equal(view.totalEntries, 5);
  assert.equal(view.matchCount, 3);
  assert.deepEqual(view.entries.map((entry) => [entry.index, entry.matches]), [[0, 2], [2, 1]]);
  assert.deepEqual(view.entries.map(reconstructed), ["Rested, rested", "Rested"]);
  assert.deepEqual(searchLog([], "rested"), { entries: [], matchCount: 0, totalEntries: 0 });
  assert.deepEqual(searchLog(["Career complete"], "absent"), {
    entries: [], matchCount: 0, totalEntries: 1,
  });
});

test("case-insensitive highlighting retains original UTF-16 spans after length-changing lowercase characters", () => {
  const line = "İrace RACE";
  const [entry] = searchLog([line], "race").entries;
  assert.equal(reconstructed(entry), line);
  assert.deepEqual(actualSpans(entry), [[1, 5], [6, 10]]);
  assert.deepEqual(entry.parts.filter((part) => part.matched).map((part) => part.text), ["race", "RACE"]);
});

test("Unicode simple folding handles sigma, Kelvin sign, and long s without expanding sharp s", () => {
  const cases = [
    ["ΟΣ ος οσ", "σ", [[1, 2], [4, 5], [7, 8]]],
    ["k K K", "k", [[0, 1], [2, 3], [4, 5]]],
    ["s S ſ", "s", [[0, 1], [2, 3], [4, 5]]],
    ["Straße STRASSE", "straße", [[0, 6]]],
    ["i I İ ı", "i", [[0, 1], [2, 3]]],
  ];
  for (const [line, query, spans] of cases) {
    const [entry] = searchLog([line], query).entries;
    assert.equal(reconstructed(entry), line);
    assert.deepEqual(actualSpans(entry), spans, JSON.stringify({ line, query }));
  }
});

test("Japanese, astral symbols, combining characters, and isolated surrogates preserve original text", () => {
  const cases = [
    ["ウマ娘 🏇 ウマ娘", "ウマ娘", [[0, 3], [7, 10]]],
    ["x🏇🏇y", "🏇", [[1, 3], [3, 5]]],
    ["e\u0301 é", "é", [[3, 4]]],
    ["e\u0301 é", "e\u0301", [[0, 2]]],
    ["x\ud83dy🏇", "\ud83d", [[1, 2]]],
  ];
  for (const [line, query, spans] of cases) {
    const [entry] = searchLog([line], query).entries;
    assert.equal(reconstructed(entry), line);
    assert.deepEqual(actualSpans(entry), spans, JSON.stringify({ line, query }));
  }
});

test("all regexp metacharacters, whitespace, and markup-like text remain literal search data", () => {
  const queries = [
    ".", "*", "+", "?", "^", "$", "{", "}", "(", ")", "|", "[", "]", "\\",
    "/", "-", "[123].(x)+", "(?=Rested)", "\\d+", "^.*$", " ", "\n", "\t", "\0",
    "<img src=x onerror=alert(1)>",
  ];
  for (const query of queries) {
    const line = `前${query}後`;
    const view = searchLog([line, "plain Rested 123"], query);
    assert.equal(reconstructed(view.entries[0]), line);
    assert.deepEqual(actualSpans(view.entries[0]), [[1, 1 + query.length]], JSON.stringify(query));
    assert.equal(view.entries[0].matches, 1);
    if (query !== " ") assert.equal(view.entries.length, 1, JSON.stringify(query));
  }
});

test("matches are non-overlapping and global matching restarts independently for each entry", () => {
  const view = searchLog(["aaaaa", "AAAAA", "aaaaa"], "aa");
  assert.equal(view.matchCount, 6);
  assert.deepEqual(view.entries.map((entry) => actualSpans(entry)), [
    [[0, 2], [2, 4]], [[0, 2], [2, 4]], [[0, 2], [2, 4]],
  ]);
  assert.deepEqual(view.entries.map(reconstructed), ["aaaaa", "AAAAA", "aaaaa"]);
});

// Independent search oracle: scan Unicode scalar tokens using explicitly known
// equivalence classes. It does not use RegExp, matchAll, lowercasing, or helpers
// from the implementation. The generated alphabet stays within these classes.
const foldGroups = ["aA", "rR", "eE", "sSſ", "kKK", "Σσς"];
const folds = new Map(foldGroups.flatMap((group, index) =>
  Array.from(group, (character) => [character, `group:${index}`])));
const tokenKey = (character) => folds.get(character) ?? `literal:${character}`;

function oracleSpans(line, query) {
  const haystack = Array.from(line);
  const needle = Array.from(query);
  const offsets = [0];
  for (const point of haystack) offsets.push(offsets.at(-1) + point.length);
  const spans = [];
  for (let start = 0; start + needle.length <= haystack.length;) {
    const equal = needle.every((point, index) =>
      tokenKey(point) === tokenKey(haystack[start + index]));
    if (equal) {
      spans.push([offsets[start], offsets[start + needle.length]]);
      start += needle.length;
    } else {
      start += 1;
    }
  }
  return spans;
}

test("2,000 deterministic mixed-Unicode cases agree with an independent scalar-token oracle", () => {
  const alphabet = Array.from("aArReEsSſkKKΣσςİıウマ娘🏇.*+?^${}()|[]\\- /é\u0301\n\t");
  let seed = 0x5e652aa3;
  const random = (bound) => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed % bound;
  };
  const make = (length) => Array.from({ length }, () => alphabet[random(alphabet.length)]).join("");
  for (let iteration = 0; iteration < 2000; iteration += 1) {
    const query = make(1 + random(4));
    const inserted = make(random(10)) + query + make(random(10)) + query;
    const history = Object.freeze([make(random(24)), inserted, inserted, make(random(24))]);
    const expected = history.flatMap((line, index) => {
      const spans = oracleSpans(line, query);
      return spans.length ? [{ index, spans }] : [];
    });
    const view = searchLog(history, query);
    const context = JSON.stringify({ iteration, history, query });
    assert.equal(view.totalEntries, history.length, context);
    assert.equal(view.matchCount, expected.reduce((sum, entry) => sum + entry.spans.length, 0), context);
    assert.deepEqual(view.entries.map((entry) => entry.index), expected.map((entry) => entry.index), context);
    for (let i = 0; i < expected.length; i += 1) {
      const entry = view.entries[i];
      assert.equal(reconstructed(entry), history[entry.index], context);
      assert.deepEqual(actualSpans(entry), expected[i].spans, context);
      assert.equal(entry.matches, expected[i].spans.length, context);
      assert.ok(entry.parts.every((part) => part.text.length > 0), context);
    }
  }
});

test("history changes distinguish a strict prefix append from trimming, replacement, and reordering", () => {
  const cases = [
    [[], [], "unchanged"],
    [[], ["Career started"], "appended"],
    [["Rested", "Rested"], ["Rested", "Rested"], "unchanged"],
    [["Rested", "Rested"], ["Rested", "Rested", "Rested"], "appended"],
    [["a", "b", "c"], ["b", "c", "d"], "replaced"],
    [["a", "b", "c"], ["a", "b"], "replaced"],
    [["a", "b"], ["b", "a"], "replaced"],
    [["old"], ["new", "entry"], "replaced"],
    [["Rested"], [], "replaced"],
    [["Σ"], ["σ"], "replaced"],
    [["a\nb"], ["a", "b"], "replaced"],
    [["a", "b"], ["a\nb"], "replaced"],
  ];
  for (const [previous, current, expected] of cases) {
    assert.equal(historyChange(Object.freeze(previous), Object.freeze(current)), expected);
  }
});

test("14,641 sequence pairs agree with an independent serialized-prefix oracle without mutating input", () => {
  const sequences = [[]];
  let frontier = [[]];
  for (let depth = 1; depth <= 4; depth += 1) {
    frontier = frontier.flatMap((prefix) => ["Rested", "Race", ""].map((line) => [...prefix, line]));
    sequences.push(...frontier);
  }
  assert.equal(sequences.length, 121);
  for (const previous of sequences) {
    Object.freeze(previous);
    for (const current of sequences) {
      Object.freeze(current);
      const same = JSON.stringify(current) === JSON.stringify(previous);
      const hasPrefix = current.length > previous.length &&
        JSON.stringify(current.slice(0, previous.length)) === JSON.stringify(previous);
      const expected = same ? "unchanged" : hasPrefix ? "appended" : "replaced";
      assert.equal(historyChange(previous, current), expected, JSON.stringify({ previous, current }));
    }
  }
});
