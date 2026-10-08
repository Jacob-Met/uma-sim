#!/usr/bin/env node
// Independent receiving of recorded native results. No simulator rules are implemented here.
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { once } from 'node:events';

const usage = 'node verify-native.mjs --fixture FILE --source-root CHECKOUT --binary API_BINARY --output NEW_DIRECTORY [--fixture-sha256 HEX] [--binary-sha256 HEX]';
const flags = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
  const flag = process.argv[i];
  if (flag === '--help') { console.log(usage); process.exit(0); }
  if (!['--fixture', '--source-root', '--binary', '--output', '--fixture-sha256', '--binary-sha256'].includes(flag) || flags.has(flag) || !process.argv[i + 1]) throw new Error(usage);
  flags.set(flag, process.argv[i + 1]);
}
for (const flag of ['--fixture', '--source-root', '--binary', '--output']) if (!flags.has(flag)) throw new Error(usage);
const sourceRoot = fs.realpathSync(flags.get('--source-root'));
const binary = fs.realpathSync(flags.get('--binary'));
const fixturePath = fs.realpathSync(flags.get('--fixture'));
const output = path.resolve(flags.get('--output'));
if (fs.existsSync(output)) throw new Error('Use a new receiving directory; previous evidence is retained.');
fs.mkdirSync(output, { recursive: true, mode: 0o700 });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => {
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (value !== null && typeof value === 'object') return '{' + Object.keys(value).sort().map(key => JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}';
  return JSON.stringify(value);
};
const digest = value => hash(Buffer.from(canonical(value)));
const git = (...args) => execFileSync('git', ['-C', sourceRoot, ...args], { encoding: 'utf8' }).trim();
const assert = (condition, message) => { if (!condition) throw new Error(message); };
class Mismatch extends Error {
  constructor(label, actual, expected) {
    super(label + ' differs from the frozen native record');
    this.label = label; this.actualSha256 = digest(actual); this.expectedSha256 = digest(expected);
  }
}
function equal(actual, expected, label) {
  if (canonical(actual) !== canonical(expected)) throw new Mismatch(label, actual, expected);
}
const fixtureBytes = fs.readFileSync(fixturePath);
const fixture = JSON.parse(fixtureBytes);
const binaryBytes = fs.readFileSync(binary);
const provenance = {
  fixture: { path: fixturePath, bytes: fixtureBytes.length, sha256: hash(fixtureBytes) },
  verifier_sha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))),
  binary: { path: binary, bytes: binaryBytes.length, sha256: hash(binaryBytes) },
  source: { path: sourceRoot, commit: git('rev-parse', 'HEAD'), tree: git('rev-parse', 'HEAD^{tree}') },
};
if (flags.has('--fixture-sha256')) assert(provenance.fixture.sha256 === flags.get('--fixture-sha256'), 'Fixture byte identity differs');
if (flags.has('--binary-sha256')) assert(provenance.binary.sha256 === flags.get('--binary-sha256'), 'Receiver binary identity differs');
assert(fixture.schema === 'uma.training-demo.v1', 'Unsupported fixture schema');
assert(fixture.source.commit === provenance.source.commit && fixture.source.tree === provenance.source.tree, 'Native checkout does not match recorded source');
assert(git('status', '--porcelain', '--untracked-files=no') === '', 'Native checkout has tracked changes');
assert(fixture.setup.policy === 'default' || fixture.setup.policy === 'bot', 'Receiving requires an existing internal policy');
assert(!('session' in fixture.setup) && !('seed' in fixture.setup), 'Seed and session must be explicit per replay');
assert(Array.isArray(fixture.checkpoints) && fixture.checkpoints.length > 0 && fixture.checkpoints.length <= 100, 'Invalid bounded checkpoint list');
const uniqueIds = new Set();
for (const point of fixture.checkpoints) {
  assert(!uniqueIds.has(point.id), 'Duplicate checkpoint ID'); uniqueIds.add(point.id);
  assert(fixture.seeds.includes(point.seed), 'Checkpoint seed is not declared');
  assert(Array.isArray(point.preparation) && point.preparation.length <= 200 && point.preparation.every(action => typeof action === 'string' && action.length), 'Invalid preparation history');
  assert(point.snapshot?.state?.phase === 'FREE', 'Recorded training checkpoint is not FREE');
  assert(Array.isArray(point.choices) && Array.isArray(point.outcomes) && point.outcomes.length > 0, 'Missing recorded choices/outcomes');
  const legalIds = point.choices.map(choice => choice.id);
  assert(new Set(legalIds).size === legalIds.length, 'Duplicate legal choice');
  const ids = point.outcomes.map(outcome => outcome.actionId);
  assert(new Set(ids).size === ids.length && ids.every(id => legalIds.includes(id)), 'Recorded outcome is duplicated or not legal');
}

const started = new Date().toISOString();
const startedClock = performance.now();
let child, base, requestCount = 0;
const trials = [], counterexamples = [];
const session = 'training-independent-e137-receiver';
async function request(route, body) {
  const url = new URL(route, base);
  if (!body) url.searchParams.set('session', session);
  const response = await fetch(url, body ? {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ ...body, session }), signal: AbortSignal.timeout(15000),
  } : { signal: AbortSignal.timeout(15000) });
  requestCount += 1;
  const raw = await response.text();
  assert(response.ok, route + ' returned HTTP ' + response.status + ': ' + raw.slice(0, 160));
  const value = JSON.parse(raw);
  assert(!value?.error, route + ' returned an API error');
  return value;
}
const legalChoices = async () => (await request('/v1/run/choices')).choices;
async function replay(point, preparation = point.preparation) {
  let snapshot = await request('/v1/run/start', { ...fixture.setup, seed: point.seed });
  let legal = await legalChoices();
  for (const action of preparation) {
    assert(legal.some(choice => choice.id === action), 'Preparation action is not legal: ' + point.id + '/' + action);
    const response = await request('/v1/run/action', { action });
    snapshot = response.state; legal = response.choices;
  }
  return { snapshot, legal };
}
function expectMismatch(actual, expected, label) {
  try { equal(actual, expected, label); }
  catch (error) {
    if (!(error instanceof Mismatch)) throw error;
    return { rejected: true, comparison: label, actual_sha256: error.actualSha256, expected_sha256: error.expectedSha256 };
  }
  throw new Error('Negative control was incorrectly accepted: ' + label);
}
let failure;
try {
  const reservation = net.createServer();
  await new Promise((resolve, reject) => { reservation.once('error', reject); reservation.listen(0, '127.0.0.1', resolve); });
  const port = reservation.address().port;
  await new Promise((resolve, reject) => reservation.close(error => error ? reject(error) : resolve()));
  base = 'http://127.0.0.1:' + port;
  const log = fs.openSync(path.join(output, 'api.log'), 'wx', 0o600);
  child = spawn(binary, [String(port)], { cwd: sourceRoot, stdio: ['ignore', log, log] });
  fs.closeSync(log);
  child.on('error', error => { failure = error; });
  let health;
  for (let attempt = 0; attempt < 70; attempt += 1) {
    if (failure) throw failure;
    assert(child.exitCode === null, 'Native API exited during startup');
    try {
      const response = await fetch(base + '/v1/health', { signal: AbortSignal.timeout(500) });
      if (response.ok) { health = await response.json(); break; }
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert(health?.ok && health.repoRootPath === sourceRoot, 'Fresh API did not discover the exact source checkout');
  for (const point of fixture.checkpoints) {
    for (const outcome of point.outcomes) {
      const before = await replay(point);
      equal(before.snapshot, point.snapshot, point.id + '/baseline');
      equal(before.legal, point.choices, point.id + '/legal-choices');
      const actual = await request('/v1/run/action', { action: outcome.actionId });
      equal(actual, outcome.response, point.id + '/' + outcome.actionId);
      trials.push({ checkpoint: point.id, seed: point.seed, turn: before.snapshot.state.turn,
        action: outcome.actionId, preparation_actions: point.preparation.length,
        baseline_sha256: digest(before.snapshot), legal_choices_sha256: digest(before.legal),
        expected_response_sha256: digest(outcome.response), actual_response_sha256: digest(actual), match: true });
    }
    console.log(JSON.stringify({ checkpoint: point.id, outcomes: point.outcomes.length, result: 'exact-native-match' }));
  }

  const point = fixture.checkpoints.find(candidate =>
    candidate.preparation.some(action => action.startsWith('train_')) &&
    candidate.outcomes.some(outcome => outcome.actionId === 'rest') &&
    candidate.outcomes.some(outcome => outcome.actionId === 'train_speed'));
  assert(point, 'Fixture lacks a training-history/recovery checkpoint for the receiving controls');
  const omittedAt = point.preparation.findLastIndex(action => action.startsWith('train_'));
  const shortened = point.preparation.slice(0, omittedAt);
  const wrongBefore = await replay(point, shortened);
  counterexamples.push({ kind: 'omitted-preparation-training', checkpoint: point.id,
    original_preparation: point.preparation, actual_preparation: shortened,
    ...expectMismatch(wrongBefore.snapshot, point.snapshot, 'omitted-preparation-training'),
    expected_snapshot: point.snapshot, actual_snapshot: wrongBefore.snapshot });

  const correctBefore = await replay(point);
  equal(correctBefore.snapshot, point.snapshot, 'substituted-result-control/baseline');
  const actualTraining = await request('/v1/run/action', { action: 'train_speed' });
  const recordedRest = point.outcomes.find(outcome => outcome.actionId === 'rest').response;
  counterexamples.push({ kind: 'substituted-action-result', checkpoint: point.id,
    expected_action: 'rest', actual_action: 'train_speed',
    ...expectMismatch(actualTraining, recordedRest, 'substituted-action-result'),
    expected_response: recordedRest, actual_response: actualTraining });

  assert(hash(fs.readFileSync(fixturePath)) === provenance.fixture.sha256, 'Fixture changed during receiving');
  assert(hash(fs.readFileSync(binary)) === provenance.binary.sha256, 'Receiver binary changed during receiving');
  assert(git('rev-parse', 'HEAD') === provenance.source.commit && git('rev-parse', 'HEAD^{tree}') === provenance.source.tree &&
    git('status', '--porcelain', '--untracked-files=no') === '', 'Source changed during receiving');
} catch (error) {
  failure = error;
} finally {
  if (child && child.exitCode === null && child.signalCode === null) {
    child.kill('SIGTERM');
    await Promise.race([once(child, 'exit'), new Promise(resolve => setTimeout(resolve, 2000))]);
    if (child.exitCode === null && child.signalCode === null) { child.kill('SIGKILL'); await once(child, 'exit'); }
  }
}
const controlBytes = Buffer.from(JSON.stringify(counterexamples, null, 2) + '\n');
fs.writeFileSync(path.join(output, 'counterexamples.json'), controlBytes, { flag: 'wx', mode: 0o600 });
const receipt = {
  schema: 'uma.training-demo.native-receiving.v1', status: failure ? 'FAILED' : 'QUALIFIED',
  started_at: started, finished_at: new Date().toISOString(), elapsed_ms: performance.now() - startedClock,
  ...provenance, fixture_recorded_binary_sha256: fixture.source.binarySha256,
  receiver_binary_matches_recorded_binary: provenance.binary.sha256 === fixture.source.binarySha256,
  fresh_native_api: { base, pid: child?.pid, exit_code: child?.exitCode, signal: child?.signalCode, retained_user_sessions: false },
  request_count: requestCount, checkpoint_count: uniqueIds.size, outcome_count: trials.length,
  complete_snapshot_choice_and_step_response_comparison: true,
  canonicalization: 'Sort object keys recursively; preserve all array order and every primitive value.',
  counterexamples_sha256: hash(controlBytes), negative_controls: counterexamples.map(({ expected_snapshot, actual_snapshot, expected_response, actual_response, ...summary }) => summary),
  trials,
  ...(failure ? { failure: { name: failure.name, message: failure.message, actual_sha256: failure.actualSha256, expected_sha256: failure.expectedSha256 } } : {}),
};
const receiptBytes = Buffer.from(JSON.stringify(receipt, null, 2) + '\n');
fs.writeFileSync(path.join(output, 'receipt.json'), receiptBytes, { flag: 'wx', mode: 0o600 });
console.log(JSON.stringify({ status: receipt.status, output, outcomes: trials.length, negative_controls: counterexamples.length,
  requests: requestCount, elapsed_ms: receipt.elapsed_ms, receipt_sha256: hash(receiptBytes),
  ...(failure ? { error: failure.message } : {}) }));
if (failure) process.exitCode = 1;
