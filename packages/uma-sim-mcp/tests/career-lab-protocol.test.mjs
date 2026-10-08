import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, toolJson, toolText, paramsError, executionError } from './helpers/mcp-client.mjs';

const oldNames = ['sim_start', 'sim_state', 'sim_text', 'sim_choices', 'sim_act', 'sim_auto', 'sim_fast_forward', 'sim_export_telemetry', 'sim_load_content_pack', 'sim_deck_place'];
const newNames = ['sim_catalog', 'sim_style', 'sim_sessions', 'sim_session_fork', 'sim_session_activate', 'sim_session_close', 'sim_library_list', 'sim_library_save', 'sim_library_load', 'sim_library_delete', 'sim_library_import', 'sim_library_export', 'sim_lab_branch', 'sim_lab_branches', 'sim_lab_branch_get', 'sim_lab_branch_delete', 'sim_lab_compare', 'sim_lab_report'];

test('discovery preserves existing tool and resource identities and publishes the full lab boundary', async t => {
  const { client, requests } = await fixture(t);
  const init = await client.request('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'independent-review', version: '1' } });
  assert.equal(init.result.protocolVersion, '2025-11-25');
  const listed = await client.request('tools/list');
  assert.deepEqual(listed.result.tools.map(tool => tool.name).sort(), [...oldNames, ...newNames].sort());
  const resources = await client.request('resources/list');
  assert.deepEqual(resources.result.resources.map(resource => resource.uri).sort(), ['uma-sim://run/state', 'uma-sim://run/telemetry', 'uma-sim://run/text']);
  for (const tool of listed.result.tools) {
    assert.equal(tool.inputSchema.type, 'object', tool.name);
    assert.equal(tool.inputSchema.additionalProperties, false, `${tool.name} must not silently accept misspelled selectors`);
  }
  assert.equal(requests.length, 0, 'discovery must not run simulation operations');
});

test('named run calls and the process-wide content operation retain REST identities', async t => {
  const { client, requests } = await fixture(t);
  const cases = [
    ['sim_state', { session: 'career-a' }, 'GET', '/v1/run/state'],
    ['sim_text', { session: 'career-a' }, 'GET', '/v1/run/text'],
    ['sim_choices', { session: 'career-a' }, 'GET', '/v1/run/choices'],
    ['sim_export_telemetry', { session: 'career-a' }, 'GET', '/v1/run/telemetry'],
    ['sim_act', { session: 'career-a', action: 'rest' }, 'POST', '/v1/run/action'],
    ['sim_auto', { session: 'career-a', policy: 'default' }, 'POST', '/v1/run/auto'],
    ['sim_fast_forward', { session: 'career-a', multiplier: 3, policy: 'external' }, 'POST', '/v1/run/fast'],
    ['sim_deck_place', { session: 'career-a', supportId: 'card-1', facility: 'speed' }, 'POST', '/v1/run/deck/place'],
    ['sim_load_content_pack', { path: 'content_packs/example.json' }, 'POST', '/v1/run/load_content_pack'],
  ];
  for (const [name, args, method, path] of cases) await t.test(name, async () => {
    const count = requests.length;
    toolJson(await client.call(name, args));
    assert.equal(requests.length, count + 1);
    const sent = requests.at(-1);
    assert.equal(sent.method, method);
    assert.equal(sent.path, path);
    if (method === 'GET') assert.deepEqual(sent.query, [['session', 'career-a']]);
    else {
      for (const [key, value] of Object.entries(args)) {
        assert.equal(String(sent.body[key]), String(value), `${name}.${key}`);
      }
      assert.deepEqual(sent.query, []);
    }
  });
});

test('start preserves default-session creation and explicit configuration', async t => {
  const { client, requests } = await fixture(t);
  toolJson(await client.call('sim_start'));
  const omitted = requests.at(-1).body;
  assert.ok(!Object.hasOwn(omitted, 'session') || omitted.session === '', 'omitted start must create the REST default session');
  assert.equal(String(omitted.seed), '42');
  const args = { session: 'named-run', seed: 17, label: '名前 🐎', policy: 'default', raceModel: 'stub', speed: 3, scenario: 'ura', trainee: 'Special Week' };
  toolJson(await client.call('sim_start', args));
  for (const [key, value] of Object.entries(args)) assert.equal(String(requests.at(-1).body[key]), String(value), key);
  toolJson(await client.call('sim_start', { session: 'zero-seed', seed: 0, speed: 1 }));
  assert.equal(String(requests.at(-1).body.seed), '0', 'zero must not silently select the default seed');
});

test('all added operations preserve their method, route, selectors and payload values', async t => {
  const { client, requests } = await fixture(t);
  const snapshot = { schemaVersion: 1, arbitrary: { text: '雪 🐎', values: [0, false, null] } };
  const cases = [
    ['sim_sessions', {}, 'GET', '/v1/sessions'],
    ['sim_session_fork', { checkpoint: 'mid', id: 'fork-a', label: 'Fork A' }, 'POST', '/v1/session/fork'],
    ['sim_session_fork', { session: 'source', id: 'fork-b', label: 'Fork B' }, 'POST', '/v1/session/fork'],
    ['sim_session_activate', { session: 'fork-a' }, 'POST', '/v1/session/activate'],
    ['sim_session_close', { session: 'fork-a' }, 'POST', '/v1/session/close'],
    ['sim_library_list', {}, 'GET', '/v1/library'],
    ['sim_library_save', { name: 'mid', session: 'source', label: 'Mid', note: 'Review note', overwrite: false }, 'POST', '/v1/library/save'],
    ['sim_library_load', { name: 'mid', session: 'restored', label: 'Restored' }, 'POST', '/v1/library/load'],
    ['sim_library_delete', { name: 'mid' }, 'POST', '/v1/library/delete'],
    ['sim_library_import', { name: 'imported', snapshot, overwrite: true }, 'POST', '/v1/library/import'],
    ['sim_library_export', { name: 'mid' }, 'GET', '/v1/library/export'],
    ['sim_lab_branch', { checkpoint: 'session:source', name: 'rest-branch', policy: 'bot', maxActions: 2, overrides: [{ turn: 4, actionId: 'rest' }] }, 'POST', '/v1/lab/branch'],
    ['sim_lab_branches', {}, 'GET', '/v1/lab/branches'],
    ['sim_lab_branch_get', { id: 'branch-a' }, 'GET', '/v1/lab/branch'],
    ['sim_lab_branch_delete', { id: 'branch-a' }, 'POST', '/v1/lab/branch/delete'],
    ['sim_lab_compare', { a: 'branch-a', b: 'branch-b' }, 'POST', '/v1/lab/compare'],
    ['sim_lab_report', { a: 'branch-a', b: 'branch-b', format: 'json' }, 'GET', '/v1/lab/report'],
  ];
  for (const [name, args, method, path] of cases) await t.test(name, async () => {
    const count = requests.length;
    toolJson(await client.call(name, args));
    assert.equal(requests.length, count + 1);
    const sent = requests.at(-1);
    assert.equal(sent.method, method);
    assert.equal(sent.path, path);
    if (method === 'GET') assert.deepEqual(Object.fromEntries(sent.query), args);
    else for (const [key, value] of Object.entries(args)) assert.deepEqual(sent.body[key], value, `${name}.${key}`);
  });
});

test('empty default-session selectors are permitted only on endpoints with correct native semantics', async t => {
  const { client, requests } = await fixture(t);
  for (const [name, args] of [
    ['sim_state', { session: '' }], ['sim_text', { session: '' }],
    ['sim_choices', { session: '' }], ['sim_export_telemetry', { session: '' }],
    ['sim_act', { session: '', action: 'rest' }], ['sim_auto', { session: '' }],
    ['sim_fast_forward', { session: '' }], ['sim_deck_place', { session: '', supportId: 'card', facility: 'speed' }],
    ['sim_library_save', { session: '', name: 'mid' }], ['sim_library_load', { session: '', name: 'mid' }],
    ['sim_session_close', { session: '' }],
  ]) await t.test(name, async () => {
    const count = requests.length;
    paramsError(await client.call(name, args));
    assert.equal(requests.length, count, 'ambiguous selector reached native API');
  });
  for (const [name, args] of [
    ['sim_start', { session: '' }], ['sim_session_activate', { session: '' }], ['sim_session_fork', { session: '', id: 'from-default' }],
  ]) await t.test(`${name} explicit default`, async () => {
    toolJson(await client.call(name, args));
    assert.equal(requests.at(-1).body.session, '');
  });
});

test('query values cannot inject a second selector or change the endpoint', async t => {
  const { client, requests } = await fixture(t);
  for (const [name, key, args] of [
    ['sim_state', 'session', {}], ['sim_library_export', 'name', {}],
    ['sim_lab_branch_get', 'id', {}], ['sim_lab_report', 'a', { b: 'second', format: 'json' }],
  ]) await t.test(name, async () => {
    const value = 'literal&session=other?x=1#fragment雪';
    const count = requests.length;
    const result = await client.call(name, { ...args, [key]: value });
    if (result.error) {
      paramsError(result);
      assert.equal(requests.length, count);
    } else {
      toolJson(result);
      assert.equal(requests.length, count + 1);
      assert.deepEqual(Object.fromEntries(requests.at(-1).query), { ...args, [key]: value });
    }
  });
});

test('invalid containers and mutation arguments fail before HTTP dispatch', async t => {
  const { client, requests } = await fixture(t);
  const invalid = [
    ...[null, [], false, 'text', 4].map(args => ['sim_state', args]),
    ['sim_act', {}], ['sim_act', { action: 1 }], ['sim_act', { action: 'rest', session: null }],
    ['sim_act', { action: 'rest', sessionId: 'sibling' }], ['sim_start', { seed: '7' }],
    ['sim_start', { seed: 9007199254740992 }], ['sim_start', { speed: 0 }], ['sim_fast_forward', { multiplier: 101 }],
    ['sim_start', { raceModel: false }], ['sim_library_save', JSON.parse('{"name":"mid","__proto__":{"overwrite":true}}')],
    ['sim_library_save', { name: 'mid', constructor: true }],
    ['sim_start', { session: false }], ['sim_fast_forward', { multiplier: '3' }],
    ['sim_load_content_pack', { path: 'pack.json', session: 'sibling' }],
    ['sim_session_fork', { id: 5 }], ['sim_session_fork', { checkpoint: 'mid', session: 'source' }],
    ['sim_session_activate', {}], ['sim_session_close', { session: 5 }],
    ['sim_library_save', { overwrite: 'true' }], ['sim_library_save', { overwrite: null }],
    ['sim_library_save', { name: 'mid', overwritee: true }], ['sim_library_load', {}],
    ['sim_library_import', {}], ['sim_library_import', { snapshot: [] }], ['sim_library_import', { snapshot: 'raw' }],
    ['sim_library_import', { snapshot: {}, overwrite: 1 }], ['sim_library_export', { name: 4 }],
    ['sim_lab_compare', { a: 'a' }], ['sim_lab_compare', { a: 'a', b: false }],
    ['sim_lab_report', { a: 'a', b: 'b', format: 'html' }], ['sim_lab_branch_delete', { id: null }],
    ['sim_lab_branch', { policy: 'external' }],
    ...[0, 501, 1.5, '2', null].map(maxActions => ['sim_lab_branch', { maxActions }]),
    ...[{}, 'rest', [null], [{ turn: 1 }], [{ turn: '1', actionId: 'rest' }], [{ turn: 1.5, actionId: 'rest' }], [{ turn: 4294967297, actionId: 'rest' }], [{ turn: 1, actionId: 3 }], [{ turn: 1, actionId: 'rest', typo: true }]].map(overrides => ['sim_lab_branch', { overrides }]),
  ];
  for (let i = 0; i < invalid.length; i++) {
    const [name, invalidArgs] = invalid[i];
    // A valid branch source prevents the required-checkpoint guard from
    // masking malformed range/override validation.
    const args = name === 'sim_lab_branch' ? { checkpoint: 'mid', ...invalidArgs } : invalidArgs;
    await t.test(`${i + 1}: ${name} ${JSON.stringify(args)}`, async () => {
      const count = requests.length;
      paramsError(await client.call(name, args));
      assert.equal(requests.length, count, 'invalid input caused a backend effect');
    });
  }
});

test('valid numeric edges survive validation without changing the requested experiment', async t => {
  const { client, requests } = await fixture(t);
  for (const maxActions of [1, 500]) {
    const args = { checkpoint: 'session:source', maxActions, overrides: [{ turn: -2147483648, actionId: 'rest' }, { turn: 2147483647, actionId: 'train_speed' }] };
    toolJson(await client.call('sim_lab_branch', args));
    assert.deepEqual(requests.at(-1).body, args);
  }
  for (const seed of [Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER]) {
    toolJson(await client.call('sim_start', { seed }));
    assert.equal(requests.at(-1).body.seed, String(seed));
  }
});

test('HTTP failures retain the native diagnostic and never retry the mutation', async t => {
  let status = 400;
  let marker = '';
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(status, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: marker, details: { preserved: true } }));
  });
  for (const code of [400, 404, 409, 422, 503]) await t.test(String(code), async () => {
    status = code;
    marker = `native-${code}-unique-雪`;
    const count = requests.length;
    executionError(await client.call('sim_act', { action: 'rest' }), marker);
    assert.equal(requests.length, count + 1, 'backend error unexpectedly retried');
  });
  const resource = await client.request('resources/read', { uri: 'uma-sim://run/state' });
  assert.equal(resource.error?.code, -32603);
  assert.equal(resource.result, undefined, 'failed resource read reported successful contents');
  assert.ok(resource.error.message.includes(marker));
});

test('non-JSON HTTP errors and Markdown reports preserve readable text', async t => {
  const markdown = '# Branch comparison: 雪 vs horse\n\n| Turn | Difference |\n| --- | --- |\n| 4 | Rest 🐎 |\n';
  const { client } = await fixture(t, (entry, _req, res) => {
    if (entry.path === '/v1/lab/report') {
      res.writeHead(200, { 'Content-Type': 'text/markdown; charset=utf-8' });
      res.end(markdown);
    } else {
      res.writeHead(503, { 'Content-Type': 'text/plain' });
      res.end('native-proxy-unavailable-雪');
    }
  });
  const report = await client.call('sim_lab_report', { a: 'a', b: 'b' });
  assert.equal(toolText(report), markdown, 'Markdown must be returned as literal human-readable text');
  assert.notEqual(report.result.isError, true);
  executionError(await client.call('sim_state'), 'native-proxy-unavailable-雪');
});

test('connection loss after a mutation is not automatically replayed and stdio stays usable', async t => {
  const { client, requests } = await fixture(t, (_entry, req) => { req.socket.destroy(); });
  const result = await client.call('sim_act', { action: 'rest' });
  assert.equal(result.error?.code, -32603);
  assert.equal(requests.length, 1, 'transport uncertainty replayed a mutation');
  assert.deepEqual((await client.request('ping')).result, {});
});

test('malformed JSON successes remain internal failures, including resource reads', async t => {
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
    res.end('{"state": malformed JSON');
  });
  const tool = await client.call('sim_state');
  assert.equal(tool.error?.code, -32603, 'malformed successful JSON was presented as valid tool data');
  assert.equal(tool.result, undefined);
  const resource = await client.request('resources/read', { uri: 'uma-sim://run/state' });
  assert.equal(resource.error?.code, -32603);
  assert.equal(resource.result, undefined);
  assert.equal(requests.length, 2);
  assert.deepEqual((await client.request('ping')).result, {});
});

test('concurrent response IDs, fragmented UTF-8, notifications and parse errors preserve the protocol boundary', async t => {
  const { client, requests } = await fixture(t, async (entry, _req, res) => {
    if (entry.body?.supportId === 'slow') await new Promise(resolve => setTimeout(resolve, 40));
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ supportId: entry.body?.supportId }));
  });
  const [slow, fast] = await Promise.all([
    client.request('tools/call', { name: 'sim_deck_place', arguments: { supportId: 'slow', facility: 'speed' } }, { id: 'slow-id' }),
    client.request('tools/call', { name: 'sim_deck_place', arguments: { supportId: '雪 🐎', facility: 'speed' } }, { id: 'fast-id', fragment: true }),
  ]);
  assert.equal(slow.id, 'slow-id');
  assert.equal(fast.id, 'fast-id');
  assert.equal(toolJson(slow).supportId, 'slow');
  assert.equal(toolJson(fast).supportId, '雪 🐎');
  const count = requests.length;
  client.child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'tools/call', params: { name: 'sim_library_delete', arguments: { name: 'should-survive' } } }) + '\n');
  client.child.stdin.write('{\n');
  assert.deepEqual((await client.request('ping')).result, {});
  assert.equal(requests.length, count, 'notification dispatched a mutation');
  assert.deepEqual(client.unclaimed, [{ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } }]);
});
