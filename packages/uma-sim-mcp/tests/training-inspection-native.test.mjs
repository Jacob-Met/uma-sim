import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, writeFile, readFile, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { actualServer } from './helpers/native-api.mjs';
import { BASELINE, toolJson, executionError } from './helpers/mcp-client.mjs';

const runFile = promisify(execFile);
const configured = process.env.UMA_SIM_TEST_API_BIN;
const nativeOptions = { skip: !configured, timeout: 120000 };
const query = session => '?session=' + encodeURIComponent(session);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

async function filesAt(root) {
  const result = {};
  async function visit(directory, prefix = '') {
    let entries;
    try { entries = await readdir(directory, { withFileTypes: true }); }
    catch (error) { if (error.code === 'ENOENT') return; throw error; }
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const name = prefix + entry.name;
      if (entry.isDirectory()) await visit(join(directory, entry.name), name + '/');
      else if (entry.isFile()) result[name] = hash(await readFile(join(directory, entry.name)));
      else assert.fail('Unexpected nonregular fixture entry: ' + name);
    }
  }
  await visit(root);
  return result;
}

test('native REST/MCP training inspection uses retained career state without spending a turn', nativeOptions, async t => {
  const server = await actualServer(t);
  const { direct, state, client, cwd } = server;
  const oracleDirectory = join(cwd, 'training-oracle');
  await mkdir(join(oracleDirectory, '.uma-sim'), { recursive: true });
  const cli = process.env.UMA_SIM_TEST_CLI_BIN ??
    join(dirname(resolve(configured)), process.platform === 'win32' ? 'uma-sim.exe' : 'uma-sim');
  const observations = [];
  t.after(async () => {
    const body = JSON.stringify({ version: 1, receivingCommit: process.env.UMA_SIM_TEST_RECEIVING_COMMIT ?? null,
      cli, cliSha256: hash(await readFile(cli)), observations }, null, 2) + '\n';
    await writeFile(join(cwd, 'training-receiving.json'), body);
    console.log('TRAINING_RECEIVING ' + body.trim().replaceAll('\n', ' '));
  });
  async function getSnapshot(session) {
    const response = await direct('GET', '/v1/run/state' + query(session));
    assert.equal(response.status, 200, response.text);
    return response;
  }
  async function nativeInspection(snapshotText) {
    const path = join(oracleDirectory, '.uma-sim/session.json');
    await writeFile(path, snapshotText);
    const env = { ...process.env, UMA_REPO_ROOT: BASELINE };
    delete env.UMA_POLICY_CMD;
    const result = await runFile(cli, ['training', '--format=json'], {
      cwd: oracleDirectory, env, timeout: 15000, maxBuffer: 262144,
    });
    assert.equal(await readFile(path, 'utf8'), snapshotText, 'native oracle must not rewrite the saved career');
    return JSON.parse(result.stdout);
  }
  async function start(session, scenario = 'ura', seed = '701') {
    const result = await direct('POST', '/v1/run/start', { session, scenario, seed,
      trainee: 'Special Week', raceModel: 'stub', traceTelemetry: 'true' });
    assert.equal(result.status, 200, result.text);
    return getSnapshot(session);
  }
  async function enterFree(session) {
    for (let step = 0; step < 12; step++) {
      const choices = await direct('GET', '/v1/run/choices' + query(session));
      assert.equal(choices.status, 200, choices.text);
      if (choices.data.choices.some(choice => choice.id.startsWith('train_'))) return getSnapshot(session);
      const choice = choices.data.choices.find(row => row.id.startsWith('event_')) ??
        choices.data.choices.find(row => row.id === 'race');
      assert.ok(choice, 'native setup must follow an offered pre-training choice');
      assert.equal((await direct('POST', '/v1/run/action', { session, action: choice.id })).status, 200);
    }
    assert.fail('native setup did not reach training within 12 offered actions');
  }
  async function readWithoutMutation(read) {
    async function footprint() {
      const sessions = await direct('GET', '/v1/sessions');
      assert.equal(sessions.status, 200, sessions.text);
      // The existing resolver cannot independently address an inactive default
      // career through ?session=. Preserve every named career and the default
      // when active without activating anything to inspect it.
      const ids = [...new Set(sessions.data.sessions.map(row => row.id)
        .filter(id => id !== '' || sessions.data.active === ''))].sort();
      const snapshots = {};
      for (const id of ids) snapshots[id] = (await getSnapshot(id)).text;
      return { sessions: sessions.data, snapshots, files: await filesAt(join(cwd, '.uma-sim')) };
    }
    const before = await footprint();
    const response = await read();
    assert.deepEqual(await footprint(), before,
      'each inspection must preserve every addressable career, active selection and checkpoint file');
    return response;
  }
  const inspectRest = session => readWithoutMutation(() =>
    direct('GET', '/v1/run/training' + (session === undefined ? '' : query(session))));
  const inspectMcp = session => readWithoutMutation(() =>
    client.call('sim_training', session === undefined ? {} : { session }));

  async function assertInspection(session, label) {
    const before = await getSnapshot(session);
    const expected = await nativeInspection(before.text);
    const persisted = await filesAt(join(cwd, '.uma-sim'));
    const sessions = await direct('GET', '/v1/sessions');
    const response = await inspectRest(session);
    assert.equal(response.status, 200, response.text);
    assert.match(response.type, /application\/json/);
    assert.deepEqual(response.data, expected, 'REST must return the complete unchanged native inspection');
    assert.deepEqual(toolJson(await inspectMcp(session)), expected);
    assert.deepEqual((await inspectRest(session)).data, expected);
    assert.equal((await getSnapshot(session)).text, before.text, 'complete snapshot including RNG must be unchanged');
    assert.deepEqual((await direct('GET', '/v1/sessions')).data, sessions.data, 'inspection must not activate a different session');
    assert.deepEqual(await filesAt(join(cwd, '.uma-sim')), persisted, 'inspection must not write checkpoint or session files');
    observations.push({ label, session, snapshotSha256: hash(before.text), inspection: expected });
    return expected;
  }
  async function install(snapshot, session, name) {
    const imported = await direct('POST', '/v1/library/import', { name, snapshot, overwrite: false });
    assert.equal(imported.status, 200, imported.text);
    const loaded = await direct('POST', '/v1/library/load', { name, session });
    assert.equal(loaded.status, 200, loaded.text);
  }

  await t.test('missing career and unsupported methods refuse through existing REST semantics', async () => {
    const missing = await inspectRest();
    assert.equal(missing.status, 404);
    assert.equal(missing.data?.error, 'no active run', missing.text);
    const absent = await inspectRest('unknown');
    assert.equal(absent.status, 404);
    assert.equal(absent.data?.error, "no such session 'unknown'", absent.text);
    assert.equal((await direct('POST', '/v1/run/training', {})).status, 405);
    assert.equal((await direct('DELETE', '/v1/run/training')).status, 405);
    assert.deepEqual((await direct('GET', '/v1/sessions')).data.sessions, []);
  });

  await t.test('default and named selection preserve active fallback and sibling careers', async () => {
    const defaultSnapshot = await start('');
    const defaultExpected = await nativeInspection(defaultSnapshot.text);
    assert.deepEqual((await inspectRest()).data, defaultExpected);
    assert.deepEqual(toolJson(await inspectMcp()), defaultExpected);
    await start('training-sibling', 'unity', '702');
    const siblingBefore = await getSnapshot('training-sibling');
    await start('training-target', 'ura', '703');
    const named = await assertInspection('training-sibling', 'named-mandatory');
    assert.equal(named.unavailableReason, 'mandatory_race');
    assert.deepEqual(named.rows, []);
    assert.equal((await getSnapshot('training-sibling')).text, siblingBefore.text);
    const targetBefore = await getSnapshot('training-target');
    const active = await nativeInspection(targetBefore.text);
    assert.deepEqual(toolJson(await inspectMcp()), active);
    assert.equal((await direct('GET', '/v1/sessions')).data.active, 'training-target');
    executionError(await inspectMcp('missing'), 'no such session');
    assert.equal((await getSnapshot('training-target')).text, targetBefore.text);
  });

  for (const scenario of ['ura', 'unity', 'trackblazer', 'grand_concert']) {
    await t.test('native samples and next action remain exact in ' + scenario, async () => {
      const session = 'training-' + scenario;
      await start(session, scenario);
      await enterFree(session);
      const before = await getSnapshot(session);
      const fork = await direct('POST', '/v1/session/fork', { session, id: session + '-control' });
      assert.equal(fork.status, 200, fork.text);
      const view = await assertInspection(session, scenario + '-free');
      assert.equal(view.schemaVersion, 1);
      assert.equal(view.sampleKind, 'deterministic_seed_0');
      assert.deepEqual(view.rows.map(row => row.actionId),
        ['train_speed', 'train_stamina', 'train_power', 'train_guts', 'train_wit']);
      assert.ok(view.notes.some(note => note.includes('not guaranteed')));
      assert.equal((await getSnapshot(session)).text, before.text);
      for (const target of [session, session + '-control'])
        assert.equal((await direct('POST', '/v1/run/action', { session: target, action: 'train_speed' })).status, 200);
      assert.deepEqual(await state(session), await state(session + '-control'),
        'inspection must preserve the next actual native action and RNG continuation');
    });
  }

  await t.test('admitted synthetic checkpoint states retain exact unavailable and blocked meanings', async () => {
    await start('training-fixture');
    const free = (await enterFree('training-fixture')).data;
    const cases = [
      ['low-energy', snapshot => { snapshot.state.energy = 10; }, null, 'insufficient_energy'],
      ['injured', snapshot => { snapshot.state.statuses.push('injured'); }, null, 'injured'],
      ['pending-event', snapshot => { snapshot.state.phase = 'EVENT'; snapshot.state.pendingEventOptions = ['Choose']; }, 'pending_event', null],
      ['complete', snapshot => { snapshot.state.phase = 'COMPLETE'; snapshot.state.careerComplete = true; }, 'career_complete', null],
    ];
    for (const [name, change, unavailable, blocked] of cases) {
      const snapshot = structuredClone(free);
      change(snapshot);
      const session = 'training-' + name;
      await install(snapshot, session, session);
      const view = await assertInspection(session, 'synthetic-' + name);
      assert.equal(view.unavailableReason, unavailable);
      if (unavailable) assert.deepEqual(view.rows, []);
      if (blocked) {
        const rows = blocked === 'injured' ? view.rows : view.rows.slice(0, 4);
        assert.ok(rows.every(row => !row.available && row.blockedReason === blocked &&
          row.energyDelta === null && row.failureChancePct === null));
        if (blocked === 'insufficient_energy') assert.equal(view.rows[4].available, true);
      }
    }
  });
});
