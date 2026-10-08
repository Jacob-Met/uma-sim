import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { toolJson, toolText, paramsError, executionError } from './helpers/mcp-client.mjs';
import { actualServer } from './helpers/native-api.mjs';

const skip = process.env.UMA_SIM_TEST_API_BIN === undefined
  ? 'Set UMA_SIM_TEST_API_BIN to exercise the real Rust receiver' : false;
const stateHash = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const value = async (client, name, args) => toolJson(await client.call(name, args));
const state = (client, session) => value(client, 'sim_state', { session });

async function record(t, cwd, evidence) {
  await writeFile(join(cwd, 'sessions-receiving-result.json'), JSON.stringify(evidence, null, 2) + '\n');
  t.diagnostic(JSON.stringify(evidence));
}

test('native receiving: fork contention has one winner and closing it without main preserves surviving careers', { timeout: 30000, skip }, async t => {
  const { client, cwd } = await actualServer(t);
  const a = await value(client, 'sim_start', { session: 'receiving-a', seed: 11 });
  const b = await value(client, 'sim_start', { session: 'receiving-b', seed: 29 });
  const contenders = await Promise.all([
    client.request('tools/call', { name: 'sim_session_fork', arguments: { session: 'receiving-a', id: 'one-target', label: 'Candidate A' } }, { id: 'receiving-fork-a' }),
    client.request('tools/call', { name: 'sim_session_fork', arguments: { session: 'receiving-b', id: 'one-target', label: 'Candidate B' } }, { id: 'receiving-fork-b' }),
  ]);
  const winners = contenders.filter(response => !response.error && response.result?.isError !== true);
  assert.equal(winners.length, 1, 'concurrent forks must not both replace the same target');
  const winner = winners[0];
  toolJson(winner);
  const loser = contenders.find(response => response !== winner);
  executionError(loser, 'HTTP 409');
  assert.deepEqual(await state(client, 'one-target'), winner.id === 'receiving-fork-a' ? a : b);
  assert.deepEqual(await state(client, 'receiving-a'), a);
  assert.deepEqual(await state(client, 'receiving-b'), b);
  const list = await value(client, 'sim_sessions');
  assert.equal(list.sessions.filter(session => session.id === 'one-target').length, 1);
  assert.equal(list.active, 'one-target');
  assert.deepEqual(await value(client, 'sim_session_close', { session: 'one-target' }), { active: '', closed: 'one-target' });
  const noMain = await client.call('sim_state');
  executionError(noMain, 'HTTP 404');
  assert.match(toolText(noMain), /no active run/);
  executionError(await client.call('sim_session_close', { session: 'one-target' }), 'HTTP 404');
  const after = await value(client, 'sim_sessions');
  assert.equal(after.active, '');
  assert.deepEqual(after.sessions.map(session => session.id).sort(), ['receiving-a', 'receiving-b']);
  assert.deepEqual(await state(client, 'receiving-a'), a);
  assert.deepEqual(await state(client, 'receiving-b'), b);
  await record(t, cwd, { case: 'concurrent-fork-and-no-main', winner: winner.id, surviving: after.sessions.map(session => session.id), a: stateHash(a), b: stateHash(b) });
});

test('native receiving: external-policy refusal fully restores a non-active target and permits later continuation', { timeout: 30000, skip }, async t => {
  const { client, cwd } = await actualServer(t);
  const a = await value(client, 'sim_start', { session: 'isolated-a', seed: 13, speed: 3 });
  const b = await value(client, 'sim_start', { session: 'active-b', seed: 31, speed: 7 });
  const sessions = await value(client, 'sim_sessions');
  const refused = await client.call('sim_fast_forward', { session: 'isolated-a', policy: 'external', multiplier: 99 });
  executionError(refused, 'HTTP 503');
  assert.match(toolText(refused), /external policy unavailable/);
  assert.match(toolText(refused), /UMA_POLICY_CMD/);
  assert.deepEqual(await state(client, 'isolated-a'), a);
  assert.deepEqual(await state(client, 'active-b'), b);
  assert.deepEqual(await value(client, 'sim_state'), b);
  assert.deepEqual(await value(client, 'sim_sessions'), sessions);
  await value(client, 'sim_auto', { session: 'isolated-a', policy: 'default' });
  assert.notDeepEqual(await state(client, 'isolated-a'), a, 'a refused fast-forward must not strand the target');
  assert.deepEqual(await state(client, 'active-b'), b);
  assert.equal((await value(client, 'sim_sessions')).active, 'active-b');
  await record(t, cwd, { case: 'non-active-503-rollback-and-continuation', unchangedTarget: stateHash(a), unchangedActive: stateHash(b), targetSpeed: a.settings.speedMultiplier, activeSpeed: b.settings.speedMultiplier });
});

test('native receiving: global content reaches both existing careers and differs from an unloaded API control', { timeout: 60000, skip }, async t => {
  const loadedServer = await actualServer(t);
  const control = await actualServer(t);
  const { client, cwd } = loadedServer;
  const before = await value(client, 'sim_start', { session: 'pack-a', seed: 101, speed: 10 });
  await value(client, 'sim_session_fork', { session: 'pack-a', id: 'pack-b' });
  assert.deepEqual(await value(control.client, 'sim_start', { session: 'control', seed: 101, speed: 10 }), before);
  const beacon = 'Independent global content beacon';
  const events = Array.from({ length: 500 }, (_, index) => ({
    id: `000000-independent-content-${String(index).padStart(3, '0')}`,
    title: beacon,
    payload: { owner_kind: 'shared', owner_name: 'QA', options: ['Speed +1'] },
  }));
  const path = join(cwd, 'independent-content.json');
  await writeFile(path, JSON.stringify({ kinds: { event_local: events } }));
  paramsError(await client.call('sim_load_content_pack', { path, session: 'pack-a' }));
  const loaded = await value(client, 'sim_load_content_pack', { path });
  assert.equal(loaded.loaded, 500);
  assert.equal(loaded.totalRegistered, 500, 'refused targeted load must not register content');
  assert.deepEqual(await state(client, 'pack-a'), before, 'catalog rebuild changed the source snapshot');
  assert.deepEqual(await state(client, 'pack-b'), before, 'catalog rebuild changed the sibling snapshot');
  let observed;
  for (let step = 1; step <= 32; step++) {
    await Promise.all([
      value(client, 'sim_auto', { session: 'pack-a', policy: 'default' }),
      value(client, 'sim_auto', { session: 'pack-b', policy: 'default' }),
      value(control.client, 'sim_auto', { session: 'control', policy: 'default' }),
    ]);
    const [a, b, negative] = await Promise.all([state(client, 'pack-a'), state(client, 'pack-b'), state(control.client, 'control')]);
    assert.deepEqual(a, b, 'the two pre-existing careers must receive the same global catalog');
    assert.ok(!negative.state.log.some(line => line.includes(beacon)), 'the separate unloaded API saw injected content');
    if (a.state.log.some(line => line.includes(beacon))) {
      assert.notDeepEqual(a, negative, 'global content did not produce an observable state difference');
      observed = { step, turn: a.state.turn, a: stateHash(a), b: stateHash(b), unloaded: stateHash(negative), active: (await value(client, 'sim_sessions')).active };
      break;
    }
  }
  assert.ok(observed, 'fixed-seed continuation never encountered the injected content');
  assert.equal(observed.active, 'pack-b');
  await record(t, cwd, { case: 'global-content-native-control', loaded: loaded.loaded, ...observed });
});
