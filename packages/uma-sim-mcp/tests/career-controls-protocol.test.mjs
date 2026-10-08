import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, toolJson, paramsError, executionError } from './helpers/mcp-client.mjs';

const kinds = ['scenarios', 'trainees', 'supports', 'factors', 'skills'];
const styles = ['auto', 'front', 'pace', 'late', 'end'];

test('catalog and style discovery states their required enums and dispatches nothing', async t => {
  const { client, requests } = await fixture(t);
  const listed = (await client.request('tools/list')).result.tools;
  const catalog = listed.find(tool => tool.name === 'sim_catalog');
  const style = listed.find(tool => tool.name === 'sim_style');
  assert.deepEqual(catalog.inputSchema.properties.kind.enum, kinds);
  assert.deepEqual(catalog.inputSchema.required, ['kind']);
  assert.deepEqual(style.inputSchema.properties.style.enum, styles);
  assert.deepEqual(style.inputSchema.required, ['style']);
  assert.equal(style.inputSchema.properties.session.minLength, 1);
  assert.equal(catalog.inputSchema.additionalProperties, false);
  assert.equal(style.inputSchema.additionalProperties, false);
  assert.equal(requests.length, 0);
});

test('all catalog kinds use their existing GET and preserve returned values', async t => {
  const returned = { items: [{ id: 'exact:<ID>&雪', name: 'Literal <img> & text', zero: 0, absent: null }], extra: false };
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(returned));
  });
  for (const kind of kinds) {
    assert.deepEqual(toolJson(await client.call('sim_catalog', { kind })), returned);
    assert.deepEqual(requests.at(-1), { method: 'GET', path: '/v1/catalog/' + kind, query: [], body: undefined });
  }
  assert.equal(requests.length, kinds.length);
});

test('style dispatch keeps explicit named identity and maps only Auto to the native empty key', async t => {
  const { client, requests } = await fixture(t);
  const session = 'strategy & <literal> 雪';
  for (const style of styles) {
    toolJson(await client.call('sim_style', { style, session }));
    assert.deepEqual(requests.at(-1), {
      method: 'POST', path: '/v1/run/style', query: [],
      body: { style: style === 'auto' ? '' : style, session },
    });
  }
  toolJson(await client.call('sim_style', { style: 'pace' }));
  assert.deepEqual(requests.at(-1).body, { style: 'pace' });
  assert.equal(requests.length, styles.length + 1);
});

test('invalid catalog/style inputs and empty session never dispatch HTTP', async t => {
  const { client, requests } = await fixture(t);
  const invalid = [
    ['sim_catalog', {}], ['sim_catalog', { kind: '' }],
    ['sim_catalog', { kind: 'SCENARIOS' }], ['sim_catalog', { kind: '../run/state' }],
    ['sim_catalog', { kind: null }], ['sim_catalog', { kind: 1 }],
    ['sim_catalog', { kind: 'trainees', session: 'x' }],
    ['sim_catalog', { kind: 'supports', query: 'unknown feature' }],
    ['sim_style', {}], ['sim_style', { style: '' }],
    ['sim_style', { style: 'FRONT' }], ['sim_style', { style: 'auto ' }],
    ['sim_style', { style: 'sprint' }], ['sim_style', { style: null }],
    ['sim_style', { style: 1 }], ['sim_style', { style: 'pace', session: '' }],
    ['sim_style', { style: 'pace', session: null }],
    ['sim_style', { style: 'pace', session: 0 }],
    ['sim_style', { style: 'pace', overwrite: true }],
  ];
  for (const [name, args] of invalid) paramsError(await client.call(name, args));
  assert.equal(requests.length, 0);
});

test('native HTTP refusals remain unsuccessful tool results without automatic retries', async t => {
  const { client, requests } = await fixture(t, (_entry, _req, res) => {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end('{"error":"literal missing <target> & 雪"}');
  });
  for (const [name, args] of [
    ['sim_style', { style: 'front', session: 'absent' }],
    ['sim_catalog', { kind: 'skills' }],
  ]) executionError(await client.call(name, args), 'HTTP 404: {"error":"literal missing <target> & 雪"}');
  assert.equal(requests.length, 2);
});
