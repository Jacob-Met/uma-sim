import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFile, readFile, readdir } from 'node:fs/promises';
import { toolJson, toolText, paramsError, executionError, comparableComparison, comparableReport } from './helpers/mcp-client.mjs';
import { actualServer } from './helpers/native-api.mjs';

test('native receiving contract: independent forks, immutable checkpoints, typed errors, isolated branches and reports', { timeout: 120000, skip: process.env.UMA_SIM_TEST_API_BIN === undefined ? 'Set UMA_SIM_TEST_API_BIN to exercise the real Rust receiver' : false }, async t => {
  const { client, direct, state, cwd } = await actualServer(t);
  const baselineDefault = await direct('POST', '/v1/run/start', { seed: 3, raceModel: 'stub', speed: 3 });
  assert.equal(baselineDefault.status, 200, baselineDefault.text);
  const defaultBefore = baselineDefault.data;
  toolJson(await client.call('sim_start', { session: 'source', seed: 7, scenario: 'ura', trainee: 'Special Week', raceModel: 'stub', speed: 3, label: 'Review source' }));
  for (let step = 0; step < 4; step++) toolJson(await client.call('sim_auto', { session: 'source', policy: 'bot' }));
  const sourceBefore = await state('source');
  const saved = toolJson(await client.call('sim_library_save', { session: 'source', name: 'review-mid', label: 'Review midpoint', note: 'Independent receiver test' }));
  assert.equal(saved.entry.name, 'review-mid');
  assert.equal(saved.entry.seed, 7);
  const turn = saved.entry.turn;
  const checkpoint = await direct('GET', '/v1/library/export?name=review-mid');
  assert.equal(checkpoint.status, 200, checkpoint.text);

  await t.test('save/export/import preserve the native snapshot, and conflicting writes preserve previous bytes', async () => {
    assert.deepEqual(toolJson(await client.call('sim_library_export', { name: 'review-mid' })), checkpoint.data);
    executionError(await client.call('sim_library_save', { session: 'source', name: 'review-mid' }), 'already exists');
    assert.equal((await direct('GET', '/v1/library/export?name=review-mid')).text, checkpoint.text);
    executionError(await client.call('sim_library_import', { name: 'review-mid', snapshot: { nope: true }, overwrite: true }), 'not imported');
    assert.equal((await direct('GET', '/v1/library/export?name=review-mid')).text, checkpoint.text);
    const imported = toolJson(await client.call('sim_library_import', { name: 'review-imported', snapshot: checkpoint.data }));
    assert.equal(imported.entry.name, 'review-imported');
    assert.deepEqual((await direct('GET', '/v1/library/export?name=review-imported')).data, checkpoint.data);
    const listing = toolJson(await client.call('sim_library_list'));
    assert.deepEqual(listing.entries.map(entry => entry.name).sort(), ['review-imported', 'review-mid']);
    toolJson(await client.call('sim_library_delete', { name: 'review-imported' }));
    assert.equal((await direct('GET', '/v1/library/export?name=review-imported')).status, 404);
  });

  await t.test('forking and named actions do not mutate the source, sibling or saved checkpoint', async () => {
    for (const id of ['fork-a', 'fork-b']) {
      const forked = toolJson(await client.call('sim_session_fork', { checkpoint: 'review-mid', id, label: id }));
      assert.equal(forked.session.id, id);
      assert.equal(forked.session.turn, turn);
    }
    const siblingBefore = await state('fork-b');
    toolJson(await client.call('sim_act', { session: 'fork-a', action: 'rest' }));
    assert.notDeepEqual(await state('fork-a'), siblingBefore);
    assert.deepEqual(await state('fork-b'), siblingBefore);
    assert.deepEqual(await state('source'), sourceBefore);
    assert.equal((await direct('GET', '/v1/library/export?name=review-mid')).text, checkpoint.text);
    const sessionsBefore = (await direct('GET', '/v1/sessions')).data;
    executionError(await client.call('sim_session_fork', { checkpoint: 'review-mid', id: 'fork-a' }), 'already exists');
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, sessionsBefore);
    executionError(await client.call('sim_act', { session: 'missing-session', action: 'rest' }), 'no such session');
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, sessionsBefore);
    assert.deepEqual(await state('fork-b'), siblingBefore);
    const loaded = toolJson(await client.call('sim_library_load', { name: 'review-mid', session: 'restored', label: 'Restored' }));
    assert.equal(loaded.state.state.turn, turn);
    assert.deepEqual(await state('restored'), checkpoint.data);
  });

  await t.test('incompatible checkpoint metadata returns real 422 and preserves the target session', async () => {
    const metaPath = `${cwd}/.uma-sim/library/review-mid.meta.json`;
    const originalMeta = await readFile(metaPath, 'utf8');
    const futureMeta = JSON.parse(originalMeta);
    futureMeta.fingerprint.snapshotSchema = 999;
    const beforeState = await state('source');
    const beforeSessions = (await direct('GET', '/v1/sessions')).data;
    try {
      await writeFile(metaPath, JSON.stringify(futureMeta));
      const native = await direct('POST', '/v1/library/load', { name: 'review-mid', session: 'source' });
      assert.equal(native.status, 422, native.text);
      executionError(await client.call('sim_library_load', { name: 'review-mid', session: 'source' }), native.data.error);
      assert.deepEqual(await state('source'), beforeState);
      assert.deepEqual((await direct('GET', '/v1/sessions')).data, beforeSessions);
      assert.equal((await direct('GET', '/v1/library/export?name=review-mid')).text, checkpoint.text);
    } finally {
      await writeFile(metaPath, originalMeta);
    }
  });

  await t.test('known empty-session ambiguity is stopped while explicit activation/fork retain correct default semantics', async () => {
    toolJson(await client.call('sim_session_activate', { session: 'source' }));
    const before = await state('source');
    for (const [name, args] of [
      ['sim_act', { session: '', action: 'rest' }],
      ['sim_library_save', { session: '', name: 'wrong-target' }],
      ['sim_library_load', { session: '', name: 'review-mid' }],
    ]) paramsError(await client.call(name, args));
    assert.deepEqual(await state('source'), before);
    assert.equal((await direct('GET', '/v1/library/export?name=wrong-target')).status, 404);
    toolJson(await client.call('sim_session_activate', { session: '' }));
    assert.deepEqual(toolJson(await client.call('sim_state')), defaultBefore);
    for (const endpoint of ['state', 'telemetry']) {
      const uri = `uma-sim://run/${endpoint}`;
      const resource = await client.request('resources/read', { uri });
      assert.equal(resource.error, undefined);
      assert.equal(resource.result.contents[0].uri, uri);
      assert.equal(resource.result.contents[0].mimeType, 'application/json');
      assert.deepEqual(JSON.parse(resource.result.contents[0].text), (await direct('GET', `/v1/run/${endpoint}`)).data);
    }
    toolJson(await client.call('sim_session_fork', { session: '', id: 'from-default' }));
    assert.deepEqual(await state('from-default'), defaultBefore);
    toolJson(await client.call('sim_session_activate', { session: 'source' }));
    toolJson(await client.call('sim_start', { seed: 19, raceModel: 'stub' }));
    assert.deepEqual(await state('source'), before, 'omitted start incorrectly overwrote named active session');
    assert.equal((await direct('GET', '/v1/sessions')).data.active, '');
    toolJson(await client.call('sim_session_activate', { session: 'source' }));
    toolJson(await client.call('sim_start', { session: '', seed: 23, raceModel: 'stub' }));
    assert.deepEqual(await state('source'), before, 'explicit default start incorrectly overwrote named active session');
    assert.equal((await direct('GET', '/v1/run/state')).data.meta.seed, 23);
  });

  await t.test('external-policy 503 survives MCP and leaves turn, RNG, speed and active session unchanged', async () => {
    toolJson(await client.call('sim_session_activate', { session: 'source' }));
    const before = await state('source');
    const beforeSessions = (await direct('GET', '/v1/sessions')).data;
    executionError(await client.call('sim_fast_forward', { session: 'source', multiplier: 9, policy: 'external' }), 'UMA_POLICY_CMD');
    assert.deepEqual(await state('source'), before);
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, beforeSessions);
  });

  let a;
  let b;
  await t.test('branches compare actual decisions while preserving live sessions and checkpoint bytes', async () => {
    const beforeSessions = (await direct('GET', '/v1/sessions')).data;
    const beforeSource = await state('source');
    const branch = async (name, actionId) => toolJson(await client.call('sim_lab_branch', { checkpoint: 'review-mid', name, policy: 'bot', maxActions: 2, overrides: [{ turn, actionId }] }));
    a = (await branch('review-rest', 'rest')).branch.id;
    b = (await branch('review-train', 'train_speed')).branch.id;
    assert.notEqual(a, b);
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, beforeSessions);
    assert.deepEqual(await state('source'), beforeSource);
    assert.equal((await direct('GET', '/v1/library/export?name=review-mid')).text, checkpoint.text);
    const branches = toolJson(await client.call('sim_lab_branches'));
    assert.deepEqual(branches.branches.map(branch => branch.id).sort(), [a, b].sort());
    assert.deepEqual(toolJson(await client.call('sim_lab_branch_get', { id: a })), (await direct('GET', `/v1/lab/branch?id=${encodeURIComponent(a)}`)).data);
    const comparison = toolJson(await client.call('sim_lab_compare', { a, b }));
    assert.deepEqual(comparableComparison(comparison), comparableComparison((await direct('POST', '/v1/lab/compare', { a, b })).data));
    assert.equal(comparison.firstDivergence.kind, 'decision');
    assert.equal(comparison.firstDivergence.turn, turn);
    assert.equal(comparison.sameCheckpoint, true);
    assert.ok(comparison.caveats.length >= 3);
    const report = await direct('GET', `/v1/lab/report?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
    assert.equal(report.status, 200);
    assert.match(report.type, /^text\/markdown/);
    assert.match(report.text, /# Branch comparison: review\\-rest vs review\\-train/);
    assert.equal(comparableReport(toolText(await client.call('sim_lab_report', { a, b }))), comparableReport(report.text));
    assert.deepEqual(comparableComparison(toolJson(await client.call('sim_lab_report', { a, b, format: 'json' }))), comparableComparison(comparison));
    await writeFile(`${cwd}/comparison.json`, JSON.stringify(comparison, null, 2) + '\n');
    await writeFile(`${cwd}/comparison.md`, report.text);
  });

  await t.test('invalid branch/import operations and deletions preserve unrelated native state', async () => {
    assert.ok(a && b, 'branch creation prerequisite failed');
    const beforeSessions = (await direct('GET', '/v1/sessions')).data;
    const beforeLibrary = (await direct('GET', '/v1/library')).data;
    const beforeBranches = (await direct('GET', '/v1/lab/branches')).data;
    paramsError(await client.call('sim_lab_branch', { checkpoint: 'review-mid', maxActions: 1.5 }));
    paramsError(await client.call('sim_lab_branch', { checkpoint: 'review-mid', overrides: [{ turn: '4', actionId: 'rest' }] }));
    paramsError(await client.call('sim_library_save', { name: 'review-mid', overwrite: 'true' }));
    executionError(await client.call('sim_library_load', { name: 'missing-checkpoint', session: 'source' }), 'no checkpoint named');
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, beforeSessions);
    assert.deepEqual((await direct('GET', '/v1/library')).data, beforeLibrary);
    assert.deepEqual((await direct('GET', '/v1/lab/branches')).data, beforeBranches);
    toolJson(await client.call('sim_lab_branch_delete', { id: a }));
    assert.equal((await direct('GET', `/v1/lab/branch?id=${encodeURIComponent(a)}`)).status, 404);
    assert.equal((await direct('GET', `/v1/lab/branch?id=${encodeURIComponent(b)}`)).status, 200);
    toolJson(await client.call('sim_session_close', { session: 'fork-a' }));
    assert.equal((await direct('GET', '/v1/run/state?session=fork-a')).status, 404);
    assert.equal((await direct('GET', '/v1/run/state?session=fork-b')).status, 200);
    assert.deepEqual((await direct('GET', '/v1/library')).data, beforeLibrary);
  });
  const artifacts = await readdir(`${cwd}/.uma-sim`);
  assert.ok(artifacts.includes('library') && artifacts.includes('lab'));
});
