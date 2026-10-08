import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const root = 'D:/Hamon/worktrees/uma-sim-discovery-0378a7b6';
const proof = 'D:/Hamon/worktrees/uma-sim-career-controls-0378a7b6-proof/root-independent-20261008T2020Z';
const source = root + '/packages/uma-sim-mcp/mcp-stdio.js';
const binary = 'D:/Hamon/worktrees/uma-sim-career-controls-0378a7b6-proof/target/debug/uma-sim-api.exe';
const expectedSource = 'a6e5ca16f5bf411a7cacc5d43cb944644c375f47fbdb5ce039e13edc2103e4ea';
const expectedBinary = '5ee4f7a6792c7d27930dbb9948f44631814a4d710f4dca0085357c07b46234d7';
const sha = data => createHash('sha256').update(data).digest('hex');
process.env.UMA_SIM_TEST_API_BIN = binary;
process.env.UMA_SIM_TEST_OUTPUT_DIR = proof;
process.env.UMA_SIM_TEST_REPO_ROOT = root;
process.env.UMA_MCP_REVIEW_SOURCE = source;
process.env.UMA_SIM_TEST_RECEIVING_COMMIT = 'aff7450bb97f3ba85e0c637d3c31363282596a57';
const helpers = 'file:///D:/Hamon/worktrees/uma-sim-discovery-0378a7b6/packages/uma-sim-mcp/tests/helpers/';
const { actualServer } = await import(helpers + 'native-api.mjs');
const { toolJson, paramsError, executionError } = await import(helpers + 'mcp-client.mjs');

test('independent native MCP receiver preserves inherited style, siblings and active targeting', { timeout: 120000 }, async t => {
  await mkdir(proof, { recursive: true });
  const sourceBytes = await readFile(source);
  assert.equal(sha(sourceBytes), expectedSource);
  assert.equal(sha(await readFile(binary)), expectedBinary);
  const head = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.equal(head, 'aff7450bb97f3ba85e0c637d3c31363282596a57');
  const { client, direct, state, cwd } = await actualServer(t);
  const originalSessions = (await direct('GET', '/v1/sessions')).data;
  const catalog = {};
  for (const kind of ['scenarios', 'trainees', 'supports', 'factors']) {
    catalog[kind] = toolJson(await client.call('sim_catalog', { kind }));
    assert.deepEqual(catalog[kind], (await direct('GET', '/v1/catalog/' + kind)).data);
  }
  assert.deepEqual((await direct('GET', '/v1/sessions')).data, originalSessions);
  const trainee = [...catalog.trainees.items].reverse().find(x => x.name !== 'Special Week');
  const support = [...catalog.supports.items].reverse().find(x => x.name !== trainee.name);
  const factor = catalog.factors.items.at(-1);
  assert.ok(trainee && support && factor);
  assert.ok(catalog.scenarios.items.some(x => x.id === 'ura'));
  toolJson(await client.call('sim_start', {
    session: 'root-source', seed: -17, scenario: 'ura', trainee: trainee.name,
    deckSupports: support.id, legacyFactors: factor.id, raceModel: 'stub',
  }));
  const initial = await state('root-source');
  assert.equal(initial.meta.traineeName, trainee.name);
  assert.ok(initial.meta.deckSupports.includes(support.id));
  assert.ok(initial.meta.legacyFactors.includes(factor.id));
  const setup = await direct('POST', '/v1/run/style', { session: 'root-source', style: 'late' });
  assert.equal(setup.status, 200, setup.text);
  const sourceState = await state('root-source');
  assert.equal(sourceState.state.preferredRunningStyle, 'late');
  toolJson(await client.call('sim_library_save', { session: 'root-source', name: 'root-style-checkpoint' }));
  const checkpoint = (await direct('GET', '/v1/library/export?name=root-style-checkpoint')).text;
  for (const id of ['root-fork-a', 'root-fork-b']) {
    toolJson(await client.call('sim_session_fork', { session: 'root-source', id }));
    assert.deepEqual(await state(id), sourceState);
  }
  const activeBefore = (await direct('GET', '/v1/sessions')).data;
  assert.equal(activeBefore.active, 'root-fork-b');
  const automatic = structuredClone(sourceState);
  delete automatic.state.preferredRunningStyle;
  const resetA = toolJson(await client.call('sim_style', { session: 'root-fork-a', style: 'auto' }));
  assert.deepEqual(resetA, automatic, 'Auto clears an inherited explicit preference instead of restoring late');
  assert.deepEqual(await state('root-fork-a'), automatic);
  assert.deepEqual(await state('root-fork-b'), sourceState);
  assert.deepEqual(await state('root-source'), sourceState);
  assert.deepEqual((await direct('GET', '/v1/sessions')).data, activeBefore);

  const front = structuredClone(sourceState);
  front.state.preferredRunningStyle = 'front';
  assert.deepEqual(toolJson(await client.call('sim_style', { style: 'front' })), front);
  assert.deepEqual(await state('root-fork-b'), front);
  assert.deepEqual(await state('root-fork-a'), automatic);
  const end = structuredClone(sourceState);
  end.state.preferredRunningStyle = 'end';
  assert.deepEqual(toolJson(await client.call('sim_style', { session: 'root-fork-a', style: 'end' })), end);
  assert.deepEqual(await state('root-fork-b'), front);
  const allBeforeRefusal = {};
  for (const id of ['root-source', 'root-fork-a', 'root-fork-b']) allBeforeRefusal[id] = await state(id);
  executionError(await client.call('sim_style', { session: 'root-never-created', style: 'pace' }), 'no such session');
  paramsError(await client.call('sim_style', { session: '', style: 'pace' }));
  paramsError(await client.call('sim_catalog', { kind: 'trainees/../run/state' }));
  for (const [id, value] of Object.entries(allBeforeRefusal)) assert.deepEqual(await state(id), value);
  assert.deepEqual((await direct('GET', '/v1/sessions')).data, activeBefore);
  for (let i = 0; i < 2; i++) {
    assert.deepEqual(toolJson(await client.call('sim_style', { session: 'root-fork-b', style: 'auto' })), automatic);
  }
  assert.deepEqual(await state('root-source'), sourceState);
  assert.deepEqual(await state('root-fork-a'), end);
  assert.equal((await direct('GET', '/v1/library/export?name=root-style-checkpoint')).text, checkpoint);
  assert.equal(sha(await readFile(source)), expectedSource);
  const receipt = {
    at: new Date().toISOString(), result: 'pass', worker: 'chatgpt-0378a7b6b7c2/root',
    sourceHead: head, sourceSha256: expectedSource, nativeApiSha256: expectedBinary,
    fixture: { seed: -17, trainee: trainee.name, support: support.id, factor: factor.id,
      originalPreference: 'late', initialActive: 'root-fork-b' },
    groups: [
      'four catalog values equal native records and leave empty sessions intact',
      'catalog identities create a different native career',
      'Auto clears a fork inherited explicit late preference without reverting to it',
      'named request preserves different active career and all complete sibling state',
      'omitted session targets active fork only',
      'missing target and invalid schema preserve all state and active identity',
      'repeated Auto is idempotent while another fork retains end preference',
      'saved checkpoint remains byte-identical and native source pins remain exact'
    ],
    fullStateChecks: true, checkpointSha256: sha(checkpoint), nativeRun: cwd,
    limits: 'No race outcomes or better-strategy claim. Isolated native API and actual stdio MCP only.'
  };
  await writeFile(proof + '/receipt.json', JSON.stringify(receipt, null, 2) + '\n');
  await writeFile(proof + '/complete-states.json', JSON.stringify({ sourceState, automatic, end, front }, null, 2) + '\n');
  console.log(JSON.stringify(receipt));
});
