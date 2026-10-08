import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/components/runSetupFile.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const {
  RUN_SETUP_SCHEMA, RUN_SETUP_VERSION, RUN_SETUP_MAX_BYTES,
  ANCESTOR_KEYS, SPARK_KEYS, encodeRunSetup, decodeRunSetup,
  validateRunSetup, validateSeedText,
} = await import("data:text/javascript;base64," + Buffer.from(outputText).toString("base64"));

const catalogs = {
  scenarios: [{ id: "ura", name: "URA" }, { id: "grand_live", name: "グランドライブ" }],
  trainees: [{ id: "trainee:a", name: "Alpha" }, { id: "trainee:b", name: "Bravo" }, { id: "trainee:c", name: "家族 <Uma>" }],
  supports: [
    ...Array.from({ length: 7 }, (_, i) => ({ id: "support:" + (i + 1), name: "Support " + (i + 1) })),
    { id: "support:trainee", name: "aLPHA" },
  ],
  factors: [
    { id: "factor:blue", name: "速度", kind: "blue" },
    { id: "factor:pink", name: "Turf", kind: "pink" },
    { id: "factor:white", name: "Skill", kind: "skill" },
    { id: "factor:green", name: "Scenario", kind: "scenario" },
    { id: "factor:race", name: "Race", kind: "RACE" },
  ],
};

function completeSetup() {
  const legacyTree = Object.fromEntries(ANCESTOR_KEYS.map((key, i) => [key, {
    uma: key.startsWith("parent") ? "Bravo" : "家族 <Uma>",
    ...Object.fromEntries(SPARK_KEYS.map((kind, j) => [kind, {
      factorId: "factor:" + kind, stars: (i + j) % 3 + 1,
    }])),
  }]));
  return {
    seed: "-9223372036854775808",
    scenario: "grand_live",
    trainee: "Alpha",
    speed: 17,
    dialogue: "full",
    raceModel: "stub",
    policy: "default",
    deck: ["support:6", "support:2", "support:5", "support:1", "support:4", "support:3"],
    legacyEnabled: true,
    legacyTree,
    compatibilityScore: 413,
  };
}
const rawFile = (setup) => new TextEncoder().encode(JSON.stringify({
  schema: RUN_SETUP_SCHEMA, version: RUN_SETUP_VERSION, setup,
}));

test("complete setup preserves all choices, six supports in order, six ancestors and all 30 spark stars", () => {
  const setup = completeSetup();
  const text = encodeRunSetup(setup, catalogs);
  assert.equal(text.endsWith("\n"), true);
  assert.deepEqual(decodeRunSetup(new TextEncoder().encode(text), catalogs), setup);
  assert.equal(JSON.parse(text).setup.seed, "-9223372036854775808");
  assert.equal(JSON.parse(text).schema, "uma-sim.run-setup");
  assert.equal(JSON.parse(text).version, 1);
});

test("disabled inheritance retains all hidden choices, compatibility, and empty-slot stars", () => {
  const setup = completeSetup();
  setup.legacyEnabled = false;
  setup.legacyTree.parentA.uma = setup.trainee;
  setup.legacyTree.gpB2.race = { factorId: "", stars: 3 };
  assert.deepEqual(decodeRunSetup(new TextEncoder().encode(encodeRunSetup(setup, catalogs)), catalogs), setup);
});

test("unfilled inheritance and an empty deck remain valid editable choices", () => {
  const setup = completeSetup();
  setup.deck = [];
  setup.compatibilityScore = 0;
  for (const node of Object.values(setup.legacyTree)) {
    node.uma = "";
    for (const slot of Object.values(node).filter((x) => typeof x === "object")) slot.factorId = "";
  }
  assert.deepEqual(validateRunSetup(setup, catalogs), setup);
});

test("read returns detached selections and never mutates its input or current catalogs", () => {
  const setup = completeSetup();
  const original = structuredClone(setup);
  const catalogBefore = structuredClone(catalogs);
  const admitted = validateRunSetup(setup, catalogs);
  admitted.deck.reverse();
  admitted.legacyTree.parentA.blue.stars = 1;
  admitted.legacyTree.gpB2.uma = "";
  assert.deepEqual(setup, original);
  assert.deepEqual(catalogs, catalogBefore);
});

test("inheritance property order is canonical for the existing native flattening order", () => {
  const setup = completeSetup();
  setup.legacyTree = Object.fromEntries(Object.entries(setup.legacyTree).reverse());
  const result = decodeRunSetup(rawFile(setup), catalogs);
  assert.deepEqual(Object.keys(result.legacyTree), [...ANCESTOR_KEYS]);
  for (const node of Object.values(result.legacyTree)) assert.deepEqual(Object.keys(node), ["uma", ...SPARK_KEYS]);
});

for (const seed of ["0", "-0", "+42", "9007199254740993", "9223372036854775807", "-9223372036854775808", "000000000000000000000000000042"]) {
  test("decimal seed survives as exact text: " + seed, () => {
    const setup = { ...completeSetup(), seed };
    assert.equal(validateSeedText(seed), seed);
    assert.equal(decodeRunSetup(rawFile(setup), catalogs).seed, seed);
    assert.equal(JSON.parse(encodeRunSetup(setup, catalogs)).setup.seed, seed);
  });
}

for (const seed of [42, 9007199254740992, null, true, "", " ", " 42", "42 ", "1e3", "1.0", "0x2a", "１２", "+", "--1", "9223372036854775808", "-9223372036854775809"]) {
  test("refuses non-decimal, numeric or out-of-range seed " + JSON.stringify(seed), () => {
    assert.throws(() => validateSeedText(seed), /Seed/);
    assert.throws(() => decodeRunSetup(rawFile({ ...completeSetup(), seed }), catalogs), /Seed/);
  });
}

const invalidSetups = [
  ["missing seed", (s) => { delete s.seed; }, /missing or unsupported/],
  ["unknown setup field", (s) => { s.autostart = true; }, /missing or unsupported/],
  ["unknown scenario", (s) => { s.scenario = "future"; }, /Scenario is unavailable/],
  ["unknown trainee", (s) => { s.trainee = "Former name"; }, /Trainee is unavailable/],
  ["fractional speed", (s) => { s.speed = 1.5; }, /Speed/],
  ["speed too low", (s) => { s.speed = 0; }, /Speed/],
  ["speed too high", (s) => { s.speed = 101; }, /Speed/],
  ["coerced speed", (s) => { s.speed = "2"; }, /Speed/],
  ["unknown dialogue", (s) => { s.dialogue = "all"; }, /Dialogue is not supported/],
  ["unknown race model", (s) => { s.raceModel = "future"; }, /Race model is not supported/],
  ["unsupported form policy", (s) => { s.policy = "external"; }, /Policy is not supported/],
  ["deck is not a list", (s) => { s.deck = "support:1"; }, /Deck/],
  ["seven supports", (s) => { s.deck = catalogs.supports.slice(0, 7).map((x) => x.id); }, /Deck/],
  ["duplicate support", (s) => { s.deck[1] = s.deck[0]; }, /only once/],
  ["unknown support", (s) => { s.deck[0] = "support:missing"; }, /Support is unavailable/],
  ["support/trainee collision", (s) => { s.deck[0] = "support:trainee"; }, /Trainee cannot also/],
  ["non-boolean inheritance", (s) => { s.legacyEnabled = 0; }, /true or false/],
  ["missing ancestor", (s) => { delete s.legacyTree.gpB2; }, /missing or unsupported/],
  ["unknown ancestor field", (s) => { s.legacyTree.parentB.note = "extra"; }, /missing or unsupported/],
  ["missing spark", (s) => { delete s.legacyTree.gpA1.white; }, /missing or unsupported/],
  ["missing star count", (s) => { delete s.legacyTree.parentA.blue.stars; }, /missing or unsupported/],
  ["zero stars", (s) => { s.legacyTree.parentA.blue.stars = 0; }, /stars/],
  ["fractional stars", (s) => { s.legacyTree.parentA.pink.stars = 2.5; }, /stars/],
  ["coerced stars", (s) => { s.legacyTree.parentA.white.stars = "3"; }, /stars/],
  ["too many stars", (s) => { s.legacyTree.parentA.race.stars = 4; }, /stars/],
  ["unknown hidden factor", (s) => { s.legacyEnabled = false; s.legacyTree.gpB2.race.factorId = "factor:removed"; }, /factor is unavailable/],
  ["wrong factor kind", (s) => { s.legacyTree.gpA1.blue.factorId = "factor:white"; }, /not a blue factor/],
  ["unknown ancestor character", (s) => { s.legacyTree.gpB1.uma = "Former name"; }, /character is unavailable/],
  ["direct-parent/trainee collision", (s) => { s.legacyTree.parentB.uma = "Alpha"; }, /direct parent/],
  ["fractional compatibility", (s) => { s.compatibilityScore = 0.5; }, /Compatibility score/],
  ["negative compatibility", (s) => { s.compatibilityScore = -1; }, /Compatibility score/],
  ["excess compatibility", (s) => { s.compatibilityScore = 501; }, /Compatibility score/],
];
for (const [name, change, pattern] of invalidSetups) {
  test("whole-file refusal without partial values: " + name, () => {
    const setup = completeSetup();
    change(setup);
    const before = structuredClone(setup);
    assert.throws(() => encodeRunSetup(setup, catalogs), pattern);
    assert.throws(() => decodeRunSetup(rawFile(setup), catalogs), pattern);
    assert.deepEqual(setup, before);
  });
}

test("grandparents may match the trainee under the existing form's direct-parent rule", () => {
  const setup = completeSetup();
  setup.legacyTree.gpA1.uma = setup.trainee;
  assert.equal(validateRunSetup(setup, catalogs).legacyTree.gpA1.uma, setup.trainee);
});

test("current catalog membership and support exclusion are rechecked instead of substituting a new choice", () => {
  const bytes = rawFile(completeSetup());
  const next = structuredClone(catalogs);
  next.supports.find((s) => s.id === "support:6").name = "ALPHA";
  assert.throws(() => decodeRunSetup(bytes, next), /Trainee cannot also/);
  next.supports = next.supports.filter((s) => s.id !== "support:6");
  assert.throws(() => decodeRunSetup(bytes, next), /Support is unavailable/);
});

test("changed factor category is refused with its saved ID intact", () => {
  const next = structuredClone(catalogs);
  next.factors[0].kind = "skill";
  assert.throws(() => decodeRunSetup(rawFile(completeSetup()), next), /not a blue factor/);
});

for (const envelope of [
  null, [], {}, { setup: completeSetup() },
  { schema: "other-product", version: 1, setup: completeSetup() },
  { schema: RUN_SETUP_SCHEMA, version: 2, setup: completeSetup() },
  { schema: RUN_SETUP_SCHEMA, version: "1", setup: completeSetup() },
  { schema: RUN_SETUP_SCHEMA, version: 1, setup: completeSetup(), extra: true },
]) {
  test("refuses incomplete, foreign or unsupported envelope " + JSON.stringify(envelope).slice(0, 85), () => {
    assert.throws(() => decodeRunSetup(new TextEncoder().encode(JSON.stringify(envelope)), catalogs));
  });
}

test("malformed JSON, invalid UTF-8 and files over the byte limit produce specific refusals", () => {
  assert.throws(() => decodeRunSetup(new Uint8Array(), catalogs), /not valid JSON/);
  assert.throws(() => decodeRunSetup(new TextEncoder().encode("{"), catalogs), /not valid JSON/);
  assert.throws(() => decodeRunSetup(Uint8Array.of(0xc3, 0x28), catalogs), /valid UTF-8/);
  assert.throws(() => decodeRunSetup(new Uint8Array(RUN_SETUP_MAX_BYTES + 1), catalogs), /64 KiB/);
});

test("size limit counts UTF-8 bytes, including at export", () => {
  const setup = completeSetup();
  const next = structuredClone(catalogs);
  const name = "界".repeat(22000);
  next.trainees.push({ id: "trainee:long", name });
  setup.trainee = name;
  assert.throws(() => encodeRunSetup(setup, next), /64 KiB/);
  assert.throws(() => decodeRunSetup(rawFile(setup), next), /64 KiB/);
});

test("unknown prototype-like JSON keys are refused and do not become setup defaults", () => {
  const text = encodeRunSetup(completeSetup(), catalogs).replace('"version": 1,', '"version": 1, "__proto__": {"policy":"external"},');
  assert.throws(() => decodeRunSetup(new TextEncoder().encode(text), catalogs), /missing or unsupported/);
  assert.equal({}.policy, undefined);
});

test("export does not invoke a getter in place of a saved field", () => {
  const setup = completeSetup();
  let invoked = false;
  Object.defineProperty(setup, "seed", { enumerable: true, get() { invoked = true; return "42"; } });
  assert.throws(() => encodeRunSetup(setup, catalogs), /ordinary saved values/);
  assert.equal(invoked, false);
});
