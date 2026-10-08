import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, toolJson, toolText, paramsError, executionError } from './helpers/mcp-client.mjs';

test('training discovery exposes the existing immutable sample contract without an HTTP call', async t => {
  const { client, requests } = await fixture(t);
  const listed = await client.request('tools/list');
  const tools = listed.result.tools.filter(tool => tool.name === 'sim_training');
  assert.equal(tools.length, 1);
  const tool = tools[0];
  assert.match(tool.description, /sample/i);
  assert.match(tool.description, /read.only/i);
  assert.equal(tool.inputSchema.type, 'object');
  assert.equal(tool.inputSchema.additionalProperties, false);
  assert.deepEqual(Object.keys(tool.inputSchema.properties), ['session']);
  assert.deepEqual(tool.inputSchema.properties.session, { type: 'string', minLength: 1 });
  assert.equal(requests.length, 0);
});

test('training dispatch preserves active omission and encoded explicit session through a real MCP process', async t => {
  const expected = { schemaVersion: 1, sampleKind: 'deterministic_seed_0',
    notes: ['Native sample; not a guaranteed next result'], unavailableReason: null,
    rows: [{ actionId: 'train_wit', energyDelta: 5, failureChancePct: 30, available: true }] };
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(expected));
  });
  for (const args of [{}, { session: 'career A/β?x=1&next=other+plus' }]) {
    assert.deepEqual(toolJson(await client.call('sim_training', args)), expected);
    assert.deepEqual(requests.at(-1), { method: 'GET', path: '/v1/run/training',
      query: args.session ? [['session', args.session]] : [], body: undefined });
  }
  assert.equal(requests.length, 2);
});

test('training refuses malformed selectors without dispatch and keeps the bridge usable', async t => {
  const { client, requests } = await fixture(t);
  for (const args of [{ session: '' }, { session: null }, { session: 1 },
    { session: [] }, { session: {} }, { unexpected: true }, { session: 'a', policy: 'bot' }]) {
    paramsError(await client.call('sim_training', args));
  }
  assert.equal(requests.length, 0);
  assert.deepEqual((await client.request('ping')).result, {});
});

test('training retains backend refusal text and never retries a failed read', async t => {
  const raw = '  {"error":"no such session \'retired\'"}\n';
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(raw);
  });
  const response = await client.call('sim_training', { session: 'retired' });
  executionError(response, 'HTTP 404');
  assert.equal(toolText(response), 'uma-sim API returned HTTP 404: ' + raw);
  assert.equal(requests.length, 1);
});

test('training treats malformed successful JSON as an error without dropping the MCP process', async t => {
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end('{incomplete');
  });
  assert.equal((await client.call('sim_training')).error?.code, -32603);
  assert.equal(requests.length, 1);
  assert.deepEqual((await client.request('ping')).result, {});
});
