import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import vm from "node:vm";
import ts from "typescript";

// Execute the production TypeScript with the project's locked compiler.
// Each receiver is private and inert; no actual network or native API is used.
const [helperSource, clientSource] = await Promise.all([
  readFile(new URL("../src/api/checkpointJson.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/api/client.ts", import.meta.url), "utf8"),
]);

function loadTs(source, bindings = {}) {
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
  });
  const exports = {};
  new vm.Script(outputText).runInNewContext({ exports, ...bindings }, { timeout: 1000 });
  return exports;
}

function receiver(response = {}) {
  const calls = [];
  const helper = loadTs(helperSource);
  const { api } = loadTs(clientSource, {
    require: (id) => {
      assert.equal(id, "./checkpointJson");
      return helper;
    },
    fetch: async (path, options) => {
      calls.push({ path, options });
      if (response.failure) throw response.failure;
      return {
        ok: response.ok ?? true,
        status: response.status ?? 200,
        statusText: response.statusText ?? "OK",
        text: async () => response.text ?? '{"entry":{"name":"received"}}',
      };
    },
  });
  return { api, calls };
}

for (const seed of [
  "-9223372036854775808",
  "-9007199254740993",
  "-42",
  "0",
  "42",
  "9007199254740993",
  "9223372036854775807",
]) {
  test("import preserves the complete snapshot text for signed seed " + seed, async () => {
    const { api, calls } = receiver();
    // These are transport fixtures, not claims of valid native snapshot fields.
    const raw = '\n \t{"meta":{"seed":' + seed + '},"state":{"meta":{"seed":' +
      seed + '}},"rngSeed":' + seed + ',"literal":"' + seed + '"}\r\n';
    const entry = await api.libraryImportJson(raw);
    assert.equal(entry.name, "received");
    assert.equal(calls.length, 1);
    assert.equal(calls[0].path, "/v1/library/import");
    assert.equal(calls[0].options.method, "POST");
    assert.equal(calls[0].options.headers["Content-Type"], "application/json");
    assert.equal(calls[0].options.body, '{"snapshot":' + raw + '}');
    assert.equal(Object.hasOwn(JSON.parse(calls[0].options.body), "name"), false);
    assert.equal(Object.hasOwn(JSON.parse(calls[0].options.body), "overwrite"), false);
  });
}

test("optional names remain one escaped string and cannot add envelope fields", async () => {
  for (const name of ["", "夏のウマ Café 🐎", 'quote" slash\\ line\nend', '"},"overwrite":true,"snapshot":{}']) {
    const { api, calls } = receiver();
    const raw = ' { "rngSeed" : 9223372036854775807 } ';
    await api.libraryImportJson(raw, name);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].options.body, '{"snapshot":' + raw + ',"name":' + JSON.stringify(name) + '}');
    const envelope = JSON.parse(calls[0].options.body);
    assert.deepEqual(Object.keys(envelope), ["snapshot", "name"]);
    assert.equal(envelope.name, name);
  }
});

test("malformed JSON is refused before any request", async () => {
  for (const raw of ["", " \r\n", "{", '{"x":}', '{"seed":01}', '{} {}', '{"x":NaN}', '{"x":Infinity}', '{"x":1,}']) {
    const { api, calls } = receiver();
    await assert.rejects(async () => api.libraryImportJson(raw), /Import: not valid JSON/);
    assert.equal(calls.length, 0);
  }
});

test("valid JSON outside the object shape is refused before any request", async () => {
  for (const raw of ["null", "[]", "[{}]", "42", '"text"', "true", "false"]) {
    const { api, calls } = receiver();
    await assert.rejects(async () => api.libraryImportJson(raw), /Import: checkpoint must be a JSON object/);
    assert.equal(calls.length, 0);
  }
});

test("the existing object-import route retains its serialization and entry projection", async () => {
  const { api, calls } = receiver();
  const body = { snapshot: { meta: { seed: 42 } }, name: "saved", overwrite: true };
  const entry = await api.libraryImport(body);
  assert.equal(entry.name, "received");
  assert.equal(calls.length, 1);
  assert.equal(calls[0].path, "/v1/library/import");
  assert.equal(calls[0].options.method, "POST");
  assert.equal(calls[0].options.headers["Content-Type"], "application/json");
  assert.equal(calls[0].options.body, JSON.stringify(body));
});

test("ordinary GET and object POST defaults remain on the same transport", async () => {
  const get = receiver({ text: '{"status":"ok"}' });
  assert.equal((await get.api.health()).status, "ok");
  assert.equal(get.calls[0].path, "/v1/health");
  assert.equal(get.calls[0].options.method, "GET");
  assert.equal(get.calls[0].options.headers, undefined);
  assert.equal(get.calls[0].options.body, undefined);

  const post = receiver({ text: '{"turn":2}' });
  assert.equal((await post.api.auto()).turn, 2);
  assert.equal(post.calls[0].path, "/v1/run/auto");
  assert.equal(post.calls[0].options.body, '{"policy":"bot"}');
  assert.equal(post.calls[0].options.headers["Content-Type"], "application/json");
});

test("server errors retain their status, body diagnostics and single request", async () => {
  for (const response of [
    { ok: false, status: 400, statusText: "Bad Request", text: '{"error":"snapshot rejected"}', expected: "snapshot rejected" },
    { ok: false, status: 409, statusText: "Conflict", text: '{"error":73}', expected: "73" },
    { ok: false, status: 503, statusText: "Unavailable", text: '{}', expected: "Unavailable" },
  ]) {
    for (const raw of [false, true]) {
      const { api, calls } = receiver(response);
      await assert.rejects(
        async () => raw ? api.libraryImportJson("{}") : api.libraryImport({ snapshot: {} }),
        (error) => error.message === "POST /v1/library/import: " + response.expected,
      );
      assert.equal(calls.length, 1);
    }
  }
});

test("invalid response JSON uses the shared diagnostic for success and error statuses", async () => {
  for (const status of [200, 502]) {
    const { api, calls } = receiver({ ok: status === 200, status, text: "<html>unavailable</html>" });
    await assert.rejects(
      async () => api.libraryImportJson("{}"),
      (error) => error.message === "POST /v1/library/import: invalid JSON (" + status + ")",
    );
    assert.equal(calls.length, 1);
  }
});

test("network failure is preserved without retry", async () => {
  const failure = new Error("inert connection failure");
  const { api, calls } = receiver({ failure });
  await assert.rejects(async () => api.libraryImportJson("{}"), (error) => error === failure);
  assert.equal(calls.length, 1);
});
