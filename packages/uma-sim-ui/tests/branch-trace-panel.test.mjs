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
const temporary = await mkdtemp(join(tmpdir(), "uma-branch-trace-panel-"));
after(() => rm(temporary, { recursive: true, force: true }));
await symlink(dirname(dirname(require.resolve("react/package.json"))), join(temporary, "node_modules"), "dir");
const source = await readFile(new URL("../src/components/BranchTracePanel.tsx", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
});
await writeFile(join(temporary, "BranchTracePanel.cjs"), outputText);
// Node rendering does not load stylesheets; the real-browser receiver does.
await writeFile(join(temporary, "branch-trace.css"), "");
const { BranchTracePanel } = require(join(temporary, "BranchTracePanel.cjs"));

const stats = { speed: 10, stamina: 11, power: 12, guts: 13, wit: 14 };
const step = (stepIndex, actionId, overrides = {}) => ({
  stepIndex, turn: 8, dateLabel: "Fixture date", phase: "event", actionId,
  actionLabel: "Action " + actionId, overrideApplied: false, overrideRejected: false,
  rngCallsBefore: stepIndex * 3, rngCallsAfter: stepIndex * 3 + 3,
  energy: 80, mood: "NORMAL", fans: 1200, skillPoints: 12, stats,
  newRaces: [], totalRaces: 1, totalSkills: 0, ...overrides,
});
const result = (changes = {}) => ({
  id: "alpha", name: "Alpha", checkpointName: "fixture-checkpoint", checkpointTurn: 8,
  seed: 42, scenarioId: "ura", traineeName: "Fixture trainee",
  config: { policy: "bot", maxActions: 120, overrides: [{ turn: 8, actionId: "train_speed" }, { turn: 99, actionId: "rest" }] },
  startedAt: "2026-10-08T00:00:00Z",
  timeline: [
    step(0, "event_0", { overrideRejected: true }),
    step(1, "train_speed", { phase: "training", overrideApplied: true }),
    step(2, "policy_train_power", { turn: 9, phase: "training" }),
  ],
  outcome: { steps: 3, finalTurn: 9, careerComplete: false, completedRaces: [], stats, energy: 80, mood: "NORMAL", fans: 1200, skillPoints: 12, learnedSkills: [], sparks: [], scenarioResources: {}, totalRngCalls: 9, telemetryRecords: 3 },
  ...changes,
});
const markup = state => renderToStaticMarkup(React.createElement(BranchTracePanel, {
  trace: { state, open: async () => {}, retry: async () => {}, close: () => {} },
}));
const ready = branch => ({ status: "ready", requestedId: branch.id, branch });

test("an unselected inspector explains how to read a saved trace", () => {
  const html = markup({ status: "idle", requestedId: null });
  assert.match(html, /aria-labelledby=/);
  assert.match(html, /Inspect a saved branch/);
  assert.doesNotMatch(html, /Download full trace JSON|Close trace/);
});

test("loading identifies the requested branch and exposes only a close action", () => {
  const html = markup({ status: "loading", requestedId: "pending-beta" });
  assert.match(html, /role="status"/);
  assert.match(html, /pending-beta/);
  assert.match(html, /Close trace/);
  assert.doesNotMatch(html, /Recorded action|Download full trace JSON/);
});

test("an error identifies its branch, is announced and supports explicit retry", () => {
  const html = markup({ status: "error", requestedId: "beta", error: "Authored network refusal" });
  assert.match(html, /role="alert"/);
  assert.match(html, /beta/);
  assert.match(html, /Authored network refusal/);
  assert.match(html, /Retry trace/);
  assert.doesNotMatch(html, /Download full trace JSON/);
});

test("same-turn rejection and later application remain separate recorded steps", () => {
  const html = markup(ready(result()));
  assert.match(html, /Recorded action/);
  assert.match(html, /action selected for each recorded step/);
  assert.match(html, /Action event_0/);
  assert.match(html, /Action train_speed/);
  assert.match(html, />Rejected</);
  assert.match(html, />Applied</);
  assert.match(html, /2 steps; 1 applied flags; 1 rejected flags/);
  assert.match(html, /both applied and rejected attempts/);
  assert.match(html, /Showing 2 of 3 recorded steps/);
  assert.doesNotMatch(html, /Action policy_train_power/);
  assert.match(html, /Download full trace JSON/);
});

test("a configured but unobserved turn has no invented rejection outcome", () => {
  const html = markup(ready(result({
    config: { policy: "default", maxActions: 1, overrides: [{ turn: 99, actionId: "rest" }] },
    timeline: [step(0, "event_0")],
  })));
  assert.match(html, /No step recorded at this turn/);
  assert.match(html, /no observed override outcome/);
  assert.doesNotMatch(html, />Rejected</);
});

test("a branch with no recorded steps has an explicit empty state", () => {
  const html = markup(ready(result({
    config: { policy: "bot", maxActions: 1, overrides: [] },
    timeline: [],
  })));
  assert.match(html, /No overrides were configured/);
  assert.match(html, /No simulation steps were recorded for this branch/);
  assert.doesNotMatch(html, /aria-label="Recorded branch steps"/);
});

test("a policy-only timeline offers all steps when the override filter is empty", () => {
  const html = markup(ready(result({
    config: { policy: "bot", maxActions: 120, overrides: [] },
    timeline: [step(0, "event_0")],
  })));
  assert.match(html, /Only steps with override flags/);
  assert.match(html, /No applied or rejected override flags were recorded/);
  assert.match(html, /Show all recorded steps/);
});

test("legacy duplicate configuration is retained without dropping either authored action", () => {
  const html = markup(ready(result({
    config: { policy: "bot", maxActions: 120, overrides: [{ turn: 8, actionId: "first_request" }, { turn: 8, actionId: "second_request" }] },
  })));
  assert.match(html, /first_request/);
  assert.match(html, /second_request/);
  assert.equal((html.match(/2 steps; 1 applied flags; 1 rejected flags/g) || []).length, 2);
  assert.match(html, /Flags describe recorded steps at each turn/);
  assert.match(html, /the first entry takes precedence/);
});

test("source text remains literal instead of becoming active markup", () => {
  const html = markup(ready(result({
    name: '<script>window.changed = true</script>',
    config: { policy: "bot", maxActions: 120, overrides: [{ turn: 8, actionId: '<img src=x onerror="bad()">' }] },
    timeline: [step(0, '<svg onload="bad()">', { overrideApplied: true })],
  })));
  assert.doesNotMatch(html, /<script>|<img src=|<svg onload=/);
  assert.match(html, /&lt;script&gt;/);
  assert.match(html, /&lt;img src=x/);
  assert.match(html, /&lt;svg onload=/);
});
