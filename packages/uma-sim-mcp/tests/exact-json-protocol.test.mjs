import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { McpClient, toolText, toolJson, paramsError } from './helpers/mcp-client.mjs';

// Capture the actual UTF-8 request body; parsing it first would hide the bug.
async function rawFixture(t, reply = '{}') {
  const requests = [];
  const server = createServer(async (req, res) => {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    requests.push({ method: req.method, url: req.url, text: Buffer.concat(chunks).toString('utf8') });
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(reply);
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const client = new McpClient('http://127.0.0.1:' + server.address().port);
  t.after(async () => {
    await client.stop();
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  });
  return { client, requests };
}

function rawRequest(client, id, wire) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      client.pending.delete(id);
      reject(new Error('Raw MCP request timed out'));
    }, 15000);
    client.pending.set(id, { resolve, reject, timeout });
    client.child.stdin.write(wire + '\n');
  });
}

test('exact seed discovery and canonical signed-i64 inputs retain safe numeric defaults', async t => {
  const { client, requests } = await rawFixture(t);
  const listed = await client.request('tools/list');
  const start = listed.result.tools.find(item => item.name === 'sim_start');
  assert.ok(start.inputSchema.properties.seed.anyOf.some(schema => schema.type === 'string'));
  assert.ok(start.inputSchema.properties.seed.anyOf.some(schema => schema.type === 'integer'));
  const importer = listed.result.tools.find(item => item.name === 'sim_library_import_json');
  assert.deepEqual(importer.inputSchema.required, ['snapshotJson']);
  assert.equal(importer.inputSchema.additionalProperties, false);
  assert.equal(requests.length, 0);
  const strings = ['0', '1', '-1', '9007199254740991', '9007199254740992', '9007199254740993',
    '-9007199254740991', '-9007199254740992', '-9007199254740993',
    '9223372036854775806', '9223372036854775807', '-9223372036854775807', '-9223372036854775808'];
  for (const seed of strings) {
    toolJson(await client.call('sim_start', { seed, session: 'exact-seed' }));
    assert.equal(JSON.parse(requests.at(-1).text).seed, seed);
  }
  for (const seed of [0, -0, 42, Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER]) {
    toolJson(await client.call('sim_start', { seed }));
    assert.equal(JSON.parse(requests.at(-1).text).seed, String(seed));
  }
  toolJson(await client.call('sim_start'));
  assert.equal(JSON.parse(requests.at(-1).text).seed, '42');
});

test('malformed, noncanonical, out-of-range and unsafe numeric seeds have zero HTTP effects', async t => {
  const { client, requests } = await rawFixture(t);
  const invalid = ['', ' ', '01', '-01', '-0', '+1', ' 1', '1 ', '1\n', '1\r\n',
    '1.0', '1e3', '0x10', 'NaN', 'Infinity', '１２', '9223372036854775808',
    '-9223372036854775809', '100000000000000000000', null, false, [], {}, 0.5,
    9007199254740992, -9007199254740992, Number.MAX_VALUE];
  for (const seed of invalid) paramsError(await client.call('sim_start', { seed, session: 'must-not-start' }));
  assert.equal(requests.length, 0);
});

test('native JSON text and all number lexemes survive JSON tool and structured resource replies', async t => {
  const raw = ' {\n  "seed": 9223372036854775807, "rngSeed": -9223372036854775808,\n  "values": [9007199254740993, -9007199254740993, -0, 1.2300, 1e+2], "text": "雪 🐎"\n}\n';
  const { client } = await rawFixture(t, raw);
  for (const [name, args] of [
    ['sim_state', { session: 'named' }], ['sim_sessions', {}],
    ['sim_library_export', { name: 'kept' }], ['sim_export_telemetry', {}],
    ['sim_lab_report', { a: 'a', b: 'b', format: 'json' }],
  ]) assert.equal(toolText(await client.call(name, args)), raw, name);
  for (const uri of ['uma-sim://run/state', 'uma-sim://run/telemetry']) {
    const response = await client.request('resources/read', { uri });
    assert.equal(response.error, undefined);
    assert.equal(response.result.contents[0].text, raw);
  }
});

test('JSON string tool results remain quoted while text resources decode the string', async t => {
  const raw = '"snow: 雪\\nseed: 9007199254740993\\nquote: \\"kept\\""\n';
  const { client } = await rawFixture(t, raw);
  assert.equal(toolText(await client.call('sim_text')), raw);
  const response = await client.request('resources/read', { uri: 'uma-sim://run/text' });
  assert.equal(response.error, undefined);
  assert.equal(response.result.contents[0].text, JSON.parse(raw));
  assert.equal(response.result.contents[0].mimeType, 'text/plain');
});

test('JSON-text import keeps the original nested object text and encodes optional metadata safely', async t => {
  const { client, requests } = await rawFixture(t);
  const snapshotJson = ' {\n "meta": {"seed":9223372036854775807}, "rngSeed":-9223372036854775808,\n "nested": [9007199254740993, -0, 1.2300, 1e+2, {"text":"雪 🐎"}]\n } ';
  const name = 'quote " brace } snow 雪';
  toolJson(await client.call('sim_library_import_json', { snapshotJson, name, overwrite: true }));
  const sent = requests.at(-1);
  assert.equal(sent.method, 'POST');
  assert.equal(sent.url, '/v1/library/import');
  assert.ok(sent.text.includes(snapshotJson), 'snapshot text was parsed and re-encoded');
  const envelope = JSON.parse(sent.text);
  assert.ok(envelope.snapshot && !Array.isArray(envelope.snapshot));
  assert.equal(envelope.name, name);
  assert.equal(envelope.overwrite, true);
  assert.deepEqual(Object.keys(envelope).sort(), ['name', 'overwrite', 'snapshot']);
  toolJson(await client.call('sim_library_import_json', { snapshotJson }));
  const minimal = JSON.parse(requests.at(-1).text);
  assert.deepEqual(Object.keys(minimal), ['snapshot']);
  assert.ok(requests.at(-1).text.includes(snapshotJson));
});

test('invalid JSON-text imports and literal malformed Unicode fail before HTTP', async t => {
  const { client, requests } = await rawFixture(t);
  const strings = ['', ' ', '{', '{} {}', '{"a":1,}', '[]', 'null', 'true', '42', '"object"',
    '{"text":"' + String.fromCharCode(0xd800) + '"}',
    '{"' + String.fromCharCode(0xdc00) + '": 1}',
    '{"text":"' + String.fromCharCode(0xdc00, 0xd800) + '"}'];
  for (const snapshotJson of strings) paramsError(await client.call('sim_library_import_json', { snapshotJson }));
  for (const args of [{}, { snapshotJson: {} }, { snapshotJson: null },
    { snapshotJson: '{}', name: '' }, { snapshotJson: '{}', overwrite: 'true' },
    { snapshotJson: '{}', snapshot: {} }]) paramsError(await client.call('sim_library_import_json', args));
  assert.equal(requests.length, 0);
  for (const snapshotJson of ['{"text":"🐎"}', '{"text":"�"}', '{"text":"\\ud800"}', '{"text":"\\ud83d\\udc0e"}']) {
    toolJson(await client.call('sim_library_import_json', { snapshotJson }));
    assert.ok(requests.at(-1).text.includes(snapshotJson), 'valid or escaped Unicode text changed');
  }
});

test('object import refuses unsafe and nonfinite number leaves but retains ordinary object inputs', async t => {
  const { client, requests } = await rawFixture(t);
  for (const value of [9007199254740992, -9007199254740992, Number.MAX_VALUE]) {
    const response = await client.call('sim_library_import', { snapshot: { nested: [null, { value }] } });
    paramsError(response);
    assert.match(response.error.message, /sim_library_import_json/);
  }
  const overflow = await rawRequest(client, 'overflow-leaf',
    '{"jsonrpc":"2.0","id":"overflow-leaf","method":"tools/call","params":{"name":"sim_library_import","arguments":{"snapshot":{"nested":[{"value":1e400}]}}}}');
  paramsError(overflow);
  assert.match(overflow.error.message, /sim_library_import_json/);
  assert.equal(requests.length, 0);
  const snapshot = { nested: [null, false, 0, 1.25, Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER, { text: '雪 🐎' }] };
  toolJson(await client.call('sim_library_import', { snapshot, name: 'ordinary', overwrite: false }));
  assert.deepEqual(JSON.parse(requests.at(-1).text), { snapshot, name: 'ordinary', overwrite: false });
});
