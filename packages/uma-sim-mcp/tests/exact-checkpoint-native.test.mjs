import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { actualServer } from './helpers/native-api.mjs';
import { McpClient, toolText, toolJson, paramsError, executionError } from './helpers/mcp-client.mjs';

const sha = text => createHash('sha256').update(text).digest('hex');
const literals = text => [...text.matchAll(/"(seed|rngSeed)"\s*:\s*(-?\d+)/g)].map(match => ({ field: match[1], literal: match[2] }));


// Native HashMap key order can change when a checkpoint is deserialized.
// Compare structure AND original number literals, so i64 rounding cannot hide
// inside a JavaScript-number deep comparison. This view is test-only.
function assertSameJson(actual, expected, message) {
  assert.deepEqual(JSON.parse(actual), JSON.parse(expected), message);
  const numbers = text => JSON.parse(text.replace(/"(?:[^"\\]|\\[\s\S])*"|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (token, number) => number === undefined ? token : JSON.stringify(number)));
  assert.deepEqual(numbers(actual), numbers(expected), message + ' (exact number literals)');
}

// Record the very response forwarded from the real API to MCP. Two separate
// native exports may legitimately use different HashMap key orders.
async function recordingClient(t, api, cwd) {
  const responses = [];
  const proxy = createServer(async (req, res) => {
    try {
      const chunks = [];
      for await (const chunk of req) chunks.push(chunk);
      const body = Buffer.concat(chunks);
      const upstream = await fetch(api + req.url, {
        method: req.method,
        ...(body.length ? { headers: { 'Content-Type': 'application/json' }, body } : {}),
        signal: AbortSignal.timeout(15000),
      });
      const bytes = Buffer.from(await upstream.arrayBuffer());
      responses.push({ path: req.url, status: upstream.status, text: bytes.toString('utf8') });
      res.writeHead(upstream.status, { 'Content-Type': upstream.headers.get('content-type') ?? 'application/json' });
      res.end(bytes);
    } catch (error) {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('Native recording transport failed: ' + error.message);
    }
  });
  proxy.listen(0, '127.0.0.1');
  await once(proxy, 'listening');
  const client = new McpClient('http://127.0.0.1:' + proxy.address().port);
  t.after(async () => {
    await client.stop();
    proxy.closeAllConnections();
    await new Promise(resolve => proxy.close(resolve));
    await writeFile(join(cwd, 'exact-native-wire.json'), JSON.stringify(responses, null, 2) + '\n');
  });
  return { client, responses };
}

test('native comparison keeps exact number tokens while ignoring only object key order', () => {
  assertSameJson('{"b":[1.25,{"seed":9007199254740993}],"a":"9007199254740993"}',
    '{"a":"9007199254740993","b":[1.25,{"seed":9007199254740993}]}', 'key order');
  assert.throws(() => assertSameJson('{"seed":9007199254740993}', '{"seed":9007199254740992}', 'rounded i64'));
  assert.throws(() => assertSameJson('{"seed":9223372036854775807}', '{"seed":9223372036854775806}', 'adjacent i64'));
  assert.throws(() => assertSameJson('{"seed":42}', '{"seed":"42"}', 'number type'));
  assert.throws(() => assertSameJson('{"x":-0.0}', '{"x":0.0}', 'number spelling'));
});

test('native exact-seed checkpoints export, reimport and resume without changing RNG or siblings', {
  timeout: 120000,
  skip: process.env.UMA_SIM_TEST_API_BIN === undefined ? 'Set UMA_SIM_TEST_API_BIN to exercise the real Rust receiver' : false,
}, async t => {
  const { api, direct, cwd } = await actualServer(t);
  const { client, responses } = await recordingClient(t, api, cwd);
  toolJson(await client.call('sim_start', { seed: 17, session: 'exact-sibling', raceModel: 'stub' }));
  const siblingBefore = await direct('GET', '/v1/run/state?session=exact-sibling');
  assert.equal(siblingBefore.status, 200);
  const evidence = [];
  const seeds = ['9223372036854775807', '-9223372036854775808', '9007199254740993', '-9007199254740993'];
  for (let index = 0; index < seeds.length; index++) await t.test(seeds[index], async () => {
    const seed = seeds[index];
    const session = 'exact-source-' + index;
    const restored = 'exact-restored-' + index;
    const checkpoint = 'exact-checkpoint-' + index;
    const importedName = 'exact-import-' + index;
    const started = toolText(await client.call('sim_start', { seed, session, raceModel: 'stub' }));
    assert.ok(literals(started).some(item => item.field === 'seed' && item.literal === seed));
    toolJson(await client.call('sim_auto', { session, policy: 'bot' }));
    toolJson(await client.call('sim_library_save', { session, name: checkpoint }));
    const nativeExport = await direct('GET', '/v1/library/export?name=' + checkpoint);
    assert.equal(nativeExport.status, 200, nativeExport.text);
    const snapshotJson = toolText(await client.call('sim_library_export', { name: checkpoint }));
    assert.equal(responses.at(-1).path, '/v1/library/export?name=' + checkpoint);
    assert.equal(snapshotJson, responses.at(-1).text, 'MCP changed its actual native response text');
    assertSameJson(snapshotJson, nativeExport.text, 'native checkpoint export identity');
    const seedFields = literals(snapshotJson);
    assert.ok(seedFields.some(item => item.field === 'seed'));
    assert.ok(seedFields.some(item => item.field === 'rngSeed'));
    assert.ok(seedFields.every(item => item.literal === seed), JSON.stringify(seedFields));
    const checkpointPath = join(cwd, '.uma-sim/library', checkpoint + '.snapshot.json');
    const checkpointBytes = await readFile(checkpointPath, 'utf8');
    assertSameJson(checkpointBytes, snapshotJson, 'stored checkpoint identity');

    // The old object path cannot recover a rounded JavaScript number.
    const refusedObject = await client.call('sim_library_import', { name: 'lossy-' + index, snapshot: JSON.parse(snapshotJson) });
    paramsError(refusedObject);
    assert.match(refusedObject.error.message, /sim_library_import_json/);
    assert.equal((await direct('GET', '/v1/library/export?name=lossy-' + index)).status, 404);

    toolJson(await client.call('sim_library_import_json', { snapshotJson, name: importedName }));
    const imported = await direct('GET', '/v1/library/export?name=' + importedName);
    assert.equal(imported.status, 200, imported.text);
    assertSameJson(imported.text, snapshotJson, 'imported checkpoint identity');
    const importedPath = join(cwd, '.uma-sim/library', importedName + '.snapshot.json');
    const importedBytes = await readFile(importedPath, 'utf8');
    executionError(await client.call('sim_library_import_json', { snapshotJson, name: importedName }), 'already exists');
    executionError(await client.call('sim_library_import_json', { snapshotJson: '{}', name: importedName, overwrite: true }), 'not imported');
    assertSameJson((await direct('GET', '/v1/library/export?name=' + importedName)).text, snapshotJson, 'refused import preserves checkpoint');
    assert.equal(await readFile(importedPath, 'utf8'), importedBytes);

    toolJson(await client.call('sim_library_load', { name: importedName, session: restored }));
    const initialRestored = await direct('GET', '/v1/run/state?session=' + restored);
    const initialSource = await direct('GET', '/v1/run/state?session=' + session);
    assert.equal(initialRestored.status, 200);
    assertSameJson(initialRestored.text, initialSource.text, 'restored starting state');
    for (const target of [session, restored]) toolJson(await client.call('sim_auto', { session: target, policy: 'bot' }));
    const continuedSource = await direct('GET', '/v1/run/state?session=' + session);
    const continuedRestored = await direct('GET', '/v1/run/state?session=' + restored);
    assert.equal(continuedSource.status, 200);
    assert.equal(continuedRestored.status, 200);
    assert.notDeepEqual(JSON.parse(continuedSource.text), JSON.parse(initialSource.text), 'the continuation must take an actual step');
    assertSameJson(continuedRestored.text, continuedSource.text, 'restored RNG continuation');
    assertSameJson((await direct('GET', '/v1/run/state?session=exact-sibling')).text, siblingBefore.text, 'sibling state');
    assertSameJson((await direct('GET', '/v1/library/export?name=' + checkpoint)).text, snapshotJson, 'original checkpoint identity');
    assert.equal(await readFile(checkpointPath, 'utf8'), checkpointBytes);
    await writeFile(join(cwd, checkpoint + '.export.json'), snapshotJson);
    await writeFile(join(cwd, checkpoint + '.continued.json'), continuedSource.text);
    evidence.push({ seed, session, restored, checkpoint, importedName, seedFields, snapshotSha256: sha(snapshotJson), continuationSha256: sha(continuedSource.text), siblingPreserved: true, checkpointPreserved: true });
  });
  const sessionsBefore = await direct('GET', '/v1/sessions');
  paramsError(await client.call('sim_start', { seed: '9223372036854775808', session: 'must-not-fallback' }));
  assertSameJson((await direct('GET', '/v1/sessions')).text, sessionsBefore.text, 'rejected seed preserves sessions');
  assert.equal((await direct('GET', '/v1/run/state?session=must-not-fallback')).status, 404);
  await writeFile(join(cwd, 'exact-checkpoint-receiving.json'), JSON.stringify(evidence, null, 2) + '\n');
});
