
import { createRequire } from "node:module";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import crypto from "node:crypto";
const root = "/workspace/scratch/69570d292200/root-receiving";
const ui = join(root, "packages/uma-sim-ui");
const depsUi = "/workspace/scratch/69570d292200/agents/production/uma-sim/packages/uma-sim-ui";
const dependencies = createRequire(join(depsUi, "package.json"));
const sourceRequire = createRequire(join(ui, "package.json"));
const ts = dependencies("typescript");
const digest = path => crypto.createHash("sha256").update(readFileSync(path)).digest("hex");
const sourceFiles = [];
function collect(dir) {
 for(const entry of readdirSync(dir, {withFileTypes:true})) {
  const path=join(dir,entry.name);
  if(entry.isDirectory()) collect(path); else sourceFiles.push(path);
 }
}
collect(join(ui, "src"));
for(const file of ["package.json","package-lock.json","tests/run-session.test.mjs","tests/compile-ui.mjs"]) sourceFiles.push(join(ui,file));
sourceFiles.push(join(root, "uma-sim-core/src/api.rs"), join(root, "uma-sim-core/tests/session_targeting.rs"));
const signatures = new Map(sourceFiles.map(path => [path,digest(path)]));
globalThis.__reviewVerifySources = () => {
 for(const [path,hash] of signatures) if(digest(path)!==hash) throw new Error("Source moved during review: "+path);
 const hashList = [...signatures].sort().map(([path,hash])=>relative(root,path)+" "+hash).join("\n");
 console.log("RECEIVING_REVIEW_SOURCE_RECEIPT",JSON.stringify({
 baseline:"8d74413d7dbc3caf1745779c92d56b6eb7feea42", node:process.version, files:signatures.size,
 sourceSetSha256:crypto.createHash("sha256").update(hashList).digest("hex"),
 app:signatures.get(join(ui,"src/App.tsx")),
 runStore:signatures.get(join(ui,"src/state/runStore.ts")),
 client:signatures.get(join(ui,"src/api/client.ts")),
 labStore:signatures.get(join(ui,"src/state/labStore.ts")),
 nativeApi:signatures.get(join(root,"uma-sim-core/src/api.rs")),
 nativeTest:signatures.get(join(root,"uma-sim-core/tests/session_targeting.rs")),
 changedDuringExecution:false, nativeRerun:false, cssContentIgnored:true
 }));
};
for(const ext of [".ts",".tsx"]) sourceRequire.extensions[ext]=(module,filename)=>{
 module.paths.unshift(join(depsUi,"node_modules"));
 const output=ts.transpileModule(readFileSync(filename,"utf8"),{compilerOptions:{
 target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS,
 jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true},fileName:filename});
 module._compile(output.outputText,filename);
};
sourceRequire.extensions[".css"]=(module)=>{module.exports={};};
globalThis.__reviewRequire=dependencies;
globalThis.__reviewLoad=path=>sourceRequire(join(ui,"src",path));
let tests=readFileSync(join(ui,"tests/run-session.test.mjs"),"utf8")
 .replace('import React from "react";','const React = globalThis.__reviewRequire("react");')
 .replace('import { create, act } from "react-test-renderer";','const { create, act } = globalThis.__reviewRequire("react-test-renderer");')
 .replace('import { loadUiModule, cleanCompiledUi } from "./compile-ui.mjs";','const loadUiModule=globalThis.__reviewLoad; const cleanCompiledUi=()=>{};');
const marker='return {\n    async click(text)';
if(!tests.includes(marker)) throw new Error("App helper changed; review receiver must be adapted explicitly");
tests=tests.replace(marker,'return { renderer,\n    async click(text)');
tests+="\nfor (const method of [\"act\", \"autoStep\", \"fastForward\", \"placeDeck\"]) {\n test(\"independent receiver: delayed follow-up read is discarded for \" + method, async (t) => {\n  const server = fixtureServer(), store = await mountStore(t, server);\n  await store.call(\"refreshActive\");\n  const holdPath = method === \"placeDeck\" ? \"/v1/run/choices\" : \"/v1/run/text\";\n  const gate = server.hold(\"GET\", holdPath);\n  const args = method === \"act\" ? [\"rest\"] : method === \"autoStep\" ? [\"default\"] :\n    method === \"fastForward\" ? [20, \"default\"] : [\"support:1\", \"speed\"];\n  const pending = await store.begin(method, ...args);\n  assert.ok(server.calls.some(call => call.path === holdPath));\n  server.setActive(\"b\");\n  await store.call(\"refreshActive\");\n  await pending.finish(gate);\n  assert.equal(store.value.state.sessionId, \"b\");\n  assert.equal(store.value.state.snapshot.state.meta.traineeName, \"b\");\n  assert.deepEqual(store.value.state.textLines, [\"Career b\"]);\n  assert.equal(store.value.state.toast, null);\n  assert.equal(store.value.state.busy, false);\n });\n}\ntest(\"independent receiver: unmount blocks follow-up reads from an accepted action\", async () => {\n const server = fixtureServer(), previous = globalThis.fetch;\n globalThis.fetch = server.fetch;\n let current, renderer, pending;\n function Probe() { current = useRunStore(); return null; }\n try {\n  await act(async () => { renderer = create(React.createElement(Probe)); });\n  await act(async () => current.refreshActive());\n  const gate = server.hold(\"POST\", \"/v1/run/action\");\n  await act(async () => { pending = current.act(\"rest\"); });\n  await act(async () => renderer.unmount());\n  const before = server.calls.length;\n  await act(async () => { gate.resolve(); await pending; });\n  assert.equal(server.calls.length, before);\n  await current.refreshActive();\n  await current.startRun({seed: 10});\n  assert.equal(server.calls.length, before);\n } finally { globalThis.fetch = previous; }\n});\ntest(\"independent receiver: selection supersedes start before hydration\", async (t) => {\n const server = fixtureServer(), store = await mountStore(t, server);\n const gate = server.hold(\"POST\", \"/v1/run/start\");\n const pending = await store.begin(\"startRun\", {seed: 10});\n server.setActive(\"b\");\n await store.call(\"refreshActive\");\n const before = server.calls.length;\n await pending.finish(gate);\n assert.equal(server.calls.length, before);\n assert.equal(store.value.state.sessionId, \"b\");\n assert.deepEqual(store.value.state.textLines, [\"Career b\"]);\n});\ntest(\"independent receiver: Lab selection preserves receiving panels and rejects an old action\", async (t) => {\n const server = fixtureServer(), baseFetch = server.fetch;\n server.fetch = async (path, options = {}) => {\n  const pathname = new URL(path, \"http://fixture.invalid\").pathname;\n  if (pathname === \"/v1/session/activate\") {\n   const { session: id } = JSON.parse(options.body);\n   server.setActive(id);\n   return new Response(JSON.stringify({session:{id,label:id}}), {status:200});\n  }\n  const response = await baseFetch(path, options);\n  if ([\"/v1/run/start\", \"/v1/run/state\", \"/v1/run/action\"].includes(pathname)) {\n   const result = await response.json();\n   const snapshot = pathname === \"/v1/run/action\" ? result.state : result;\n   const name = snapshot.state.meta.traineeName;\n   snapshot.state.log = [\"retained:\" + name];\n   snapshot.state.statuses = [\"condition:\" + name];\n   snapshot.state.learnedSkillIds = [\"skill:\" + name];\n   return new Response(JSON.stringify(result), {status:200});\n  }\n  return response;\n };\n const app = await mountApp(t, server);\n const { ChoicePanel } = loadUiModule(\"components/ChoicePanel\");\n const { SessionPanel } = loadUiModule(\"components/SessionPanel\");\n const { ConditionsPanel } = loadUiModule(\"components/ConditionsPanel\");\n const { SkillsPanel } = loadUiModule(\"components/SkillsPanel\");\n const { LogPanel } = loadUiModule(\"components/LogPanel\");\n const { RacePanel } = loadUiModule(\"components/RacePanel\");\n const gate = server.hold(\"POST\", \"/v1/run/action\");\n await act(async () => { app.renderer.root.findByType(ChoicePanel).props.onChoose(\"rest\"); });\n await app.click(\"Library & lab\");\n const actionsBeforeKey = server.calls.filter(call => call.path === \"/v1/run/action\").length;\n await app.key();\n assert.equal(server.calls.filter(call => call.path === \"/v1/run/action\").length, actionsBeforeKey);\n await act(async () => { await app.renderer.root.findByType(SessionPanel).props.lab.activateSession(\"b\"); });\n await act(async () => { gate.resolve(); });\n await app.click(\"Run\");\n const conditionState = app.renderer.root.findByType(ConditionsPanel).props.state;\n assert.equal(conditionState.meta.traineeName, \"b\");\n assert.deepEqual(conditionState.statuses, [\"condition:b\"]);\n const skillsState = app.renderer.root.findByType(SkillsPanel).props.state;\n assert.equal(skillsState.meta.traineeName, \"b\");\n assert.deepEqual(skillsState.learnedSkillIds, [\"skill:b\"]);\n assert.deepEqual(app.renderer.root.findByType(LogPanel).props.history, [\"retained:b\"]);\n assert.deepEqual(app.renderer.root.findByType(LogPanel).props.lines, [\"Career b\"]);\n assert.deepEqual(app.renderer.root.findByType(RacePanel).props.lines, [\"retained:b\"]);\n await app.key({ target:{tagName:\"INPUT\",type:\"search\"} });\n assert.equal(server.calls.filter(call => call.path === \"/v1/run/action\").length, actionsBeforeKey);\n});\nafter(() => globalThis.__reviewVerifySources());\n";
await import("data:text/javascript;base64,"+Buffer.from(tests).toString("base64"));
