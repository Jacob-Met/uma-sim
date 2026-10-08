"use strict";
const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const nodeTest = require("node:test");
const input = JSON.parse(process.argv[1]);
const sha = value => crypto.createHash("sha256").update(value).digest("hex");
const blob = value => crypto.createHash("sha1").update("blob " + Buffer.byteLength(value) + "\0").update(value).digest("hex");
const compilerFile = path.join(input.compiler_path, "lib/typescript.js");
assert.equal(sha(fs.readFileSync(compilerFile)), input.compiler_file_sha256);
const ts = require(input.compiler_path);
assert.equal(ts.version, "5.9.3");
assert.equal(blob(input.capsule.baseClient), "bed38774d5dcd48af5b964d97472a76db92f78e7");
assert.equal(blob(input.capsule.baseStore), "6cc39e49150ab6774cb78aac1b89676b704418f8");
const production = input.capsule.files.map(file => {
  assert.equal(sha(file.content), file.expected, file.path);
  return {path:"packages/uma-sim-ui/"+file.path,bytes:Buffer.byteLength(file.content),sha256:sha(file.content),git_blob:blob(file.content)};
});
const sourceMap = new Map(input.capsule.files.map(file => ["file:///recovery/packages/uma-sim-ui/" + file.path, file.content]));
const readLog = [];
const context = vm.createContext({URL, Buffer, console});
const registry = {
  "node:assert/strict": {default:assert},
  "node:fs/promises": {readFile:async (url, encoding) => {
    assert.equal(encoding, "utf8");
    const key = String(url);
    assert.ok(sourceMap.has(key), "only declared production source may be read: " + key);
    readLog.push(key);
    return sourceMap.get(key);
  }},
  "node:test": {default:nodeTest},
  "node:vm": {default:vm},
  "typescript": {default:ts},
};
const testPath = "file:///recovery/packages/uma-sim-ui/tests/checkpoint-json.test.mjs";
const module = new vm.SourceTextModule(input.test_source, {
  context, identifier:testPath,
  initializeImportMeta(meta) {meta.url=testPath;},
});
const started = new Date().toISOString();
nodeTest.after(() => {
  assert.equal(sha(fs.readFileSync(compilerFile)), input.compiler_file_sha256);
  assert.deepEqual(readLog.slice().sort(), [
    "file:///recovery/packages/uma-sim-ui/src/api/checkpointJson.ts",
    "file:///recovery/packages/uma-sim-ui/src/api/client.ts",
  ]);
  process.stdout.write("# SOURCE_BINDING " + JSON.stringify({
    schema:"uma.browser-checkpoint.recovery-focused-gate.v1",started,completed:new Date().toISOString(),
    current_main:input.current_main,independent_commit:input.independent_commit,
    node:process.version,platform:process.platform,arch:process.arch,typescript:ts.version,
    compiler_sha256:input.compiler_file_sha256,
    source:production,test:{path:"packages/uma-sim-ui/tests/checkpoint-json.test.mjs",bytes:Buffer.byteLength(input.test_source),sha256:sha(input.test_source),git_blob:blob(input.test_source)},
    loader:"Exact maintained test module via vm.SourceTextModule; two fs readFile calls bound to exact source strings; same locked compiler version; standard node:test and inert fetch.",
    native_api:false,browser:false,network:false,filesystem_writes:false,
    limitations:["This newly authored focused test is not the unavailable original author test.","No full application typecheck/build or native browser completion is established by this gate."]
  }) + "\n");
});
(async () => {
  await module.link(async specifier => {
    assert.ok(Object.hasOwn(registry, specifier), "unlisted dependency: " + specifier);
    const entries=registry[specifier];
    return new vm.SyntheticModule(Object.keys(entries), function(){
      for (const [name,value] of Object.entries(entries)) this.setExport(name,value);
    }, {context});
  });
  await module.evaluate();
})().catch(error => {process.stderr.write(error.stack+"\n");process.exitCode=1;});
