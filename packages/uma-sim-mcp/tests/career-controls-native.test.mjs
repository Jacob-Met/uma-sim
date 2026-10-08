import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { toolJson, executionError } from './helpers/mcp-client.mjs';
import { actualServer } from './helpers/native-api.mjs';

test('MCP catalog selection and race style use the real native career without advancing it', {
  timeout: 120000,
  skip: process.env.UMA_SIM_TEST_API_BIN === undefined ? 'Set UMA_SIM_TEST_API_BIN to exercise the real Rust receiver' : false,
}, async t => {
  const { client, direct, state, cwd } = await actualServer(t);
  const catalogs = {};
  const originalSessions = (await direct('GET', '/v1/sessions')).data;
  await t.test('all five catalogs are available before a career and equal native records', async () => {
    for (const kind of ['scenarios', 'trainees', 'supports', 'factors', 'skills']) {
      const native = await direct('GET', '/v1/catalog/' + kind);
      assert.equal(native.status, 200, native.text);
      catalogs[kind] = toolJson(await client.call('sim_catalog', { kind }));
      assert.deepEqual(catalogs[kind], native.data);
      assert.ok(catalogs[kind].items.length > 0);
    }
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, originalSessions);
    executionError(await client.call('sim_style', { style: 'front' }), 'HTTP 404');
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, originalSessions);
  });

  const trainee = catalogs.trainees.items.find(item => item.name === 'Special Week') ?? catalogs.trainees.items[0];
  const support = catalogs.supports.items.find(item => item.name !== trainee.name);
  const factor = catalogs.factors.items[0];
  assert.ok(support && factor);
  assert.ok(catalogs.scenarios.items.some(item => item.id === 'ura'));
  const started = toolJson(await client.call('sim_start', {
    session: 'source', seed: 42, scenario: 'ura', trainee: trainee.name,
    deckSupports: support.id, legacyFactors: factor.id, raceModel: 'stub',
  }));
  const source = await state('source');
  assert.equal(source.meta.traineeName, trainee.name);
  assert.ok(source.meta.deckSupports.includes(support.id));
  assert.ok(source.meta.legacyFactors.includes(factor.id));
  toolJson(await client.call('sim_library_save', { session: 'source', name: 'before-style' }));
  const checkpoint = (await direct('GET', '/v1/library/export?name=before-style')).text;
  const target = 'strategy-named';
  toolJson(await client.call('sim_session_fork', { session: 'source', id: target }));
  const originalTarget = await state(target);
  assert.deepEqual(originalTarget, source);

  await t.test('catalog-chosen identities create a native career and further catalog reads preserve it', async () => {
    assert.ok(started);
    for (const kind of Object.keys(catalogs)) {
      assert.deepEqual(toolJson(await client.call('sim_catalog', { kind })), catalogs[kind]);
    }
    assert.deepEqual(await state('source'), source);
    assert.deepEqual(await state(target), originalTarget);
    assert.equal((await direct('GET', '/v1/library/export?name=before-style')).text, checkpoint);
  });

  await t.test('every explicit style and Auto change only the named target preference', async () => {
    for (const style of ['front', 'pace', 'late', 'end', 'auto']) {
      const result = toolJson(await client.call('sim_style', { session: target, style }));
      const expected = structuredClone(originalTarget);
      if (style !== 'auto') expected.state.preferredRunningStyle = style;
      assert.deepEqual(result, expected);
      assert.deepEqual(await state(target), expected);
      assert.deepEqual(await state('source'), source);
      assert.equal((await direct('GET', '/v1/library/export?name=before-style')).text, checkpoint);
    }
  });

  await t.test('omitted session uses active target while a named request ignores a different active career', async () => {
    toolJson(await client.call('sim_session_activate', { session: 'source' }));
    const named = toolJson(await client.call('sim_style', { session: target, style: 'end' }));
    assert.equal(named.state.preferredRunningStyle, 'end');
    assert.deepEqual(await state('source'), source);
    const active = toolJson(await client.call('sim_style', { style: 'pace' }));
    const expected = structuredClone(source);
    expected.state.preferredRunningStyle = 'pace';
    assert.deepEqual(active, expected);
    assert.deepEqual(await state(target), named);
    toolJson(await client.call('sim_style', { style: 'auto' }));
    assert.deepEqual(await state('source'), source);
    assert.equal((await direct('GET', '/v1/sessions')).data.active, 'source');
  });

  await t.test('missing target refusal leaves complete careers, checkpoint and active session unchanged', async () => {
    const beforeTarget = await state(target);
    const beforeSessions = (await direct('GET', '/v1/sessions')).data;
    executionError(await client.call('sim_style', { style: 'front', session: 'missing-career' }), 'no such session');
    assert.deepEqual(await state(target), beforeTarget);
    assert.deepEqual(await state('source'), source);
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, beforeSessions);
    assert.equal((await direct('GET', '/v1/library/export?name=before-style')).text, checkpoint);
  });
  await writeFile(cwd + '/career-controls.json', JSON.stringify({
    catalogs: Object.fromEntries(Object.entries(catalogs).map(([kind, data]) => [kind, data.items.length])),
    selected: { trainee: trainee.name, support: support.id, factor: factor.id },
    source, target: await state(target), checkpointPreserved: true,
    result: 'pass; only requested preferredRunningStyle changes',
  }, null, 2) + '\n');
});
