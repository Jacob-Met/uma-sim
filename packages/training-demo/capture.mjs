#!/usr/bin/env node
// Capture actual native outcomes. This file does not simulate game rules.
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

const SOURCE_COMMIT = '0bc58cb83e89f07c6af3056dd7c37436906d994f';
const SOURCE_TREE = '7c1b145124486f2a7911df3cecb620558abf84e1';
const seeds = ['1', '7', '42'];
const depths = [0, 3, 6];
const setup = { scenario: 'ura', trainee: 'Special Week', raceModel: 'physics', policy: 'default' };

function usage() {
  return 'node capture.mjs --base http://127.0.0.1:PORT --source-root PINNED_CHECKOUT --binary API_BINARY --output fixture.json';
}
const flags = Object.create(null);
for (let i = 2; i < process.argv.length; i += 2) {
  const name = process.argv[i];
  if (name === '--help') { console.log(usage()); process.exit(0); }
  if (!['--base', '--source-root', '--binary', '--output'].includes(name) || flags[name] || !process.argv[i + 1]) {
    throw new Error(usage());
  }
  flags[name] = process.argv[i + 1];
}
for (const name of ['--base', '--source-root', '--binary', '--output']) if (!flags[name]) throw new Error(usage());
const base = new URL(flags['--base']);
if (base.protocol !== 'http:' || !['127.0.0.1', '[::1]', 'localhost'].includes(base.hostname) || base.username || base.password || base.pathname !== '/' || base.search || base.hash) {
  throw new Error('Use an explicitly started native API on the local loopback interface.');
}
const sourceRoot = resolve(flags['--source-root']);
const git = (...args) => execFileSync('git', ['-C', sourceRoot, ...args], { encoding: 'utf8' }).trim();
if (git('rev-parse', 'HEAD') !== SOURCE_COMMIT || git('rev-parse', 'HEAD^{tree}') !== SOURCE_TREE || git('status', '--porcelain', '--untracked-files=no')) {
  throw new Error('The native source checkout must be clean and exactly match the pinned commit and tree.');
}
const binarySha256 = createHash('sha256').update(await readFile(flags['--binary'])).digest('hex');
const ordered = value => Array.isArray(value) ? value.map(ordered) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, ordered(value[key])])) : value;
const canonical = value => JSON.stringify(ordered(value));
let requestCount = 0;
async function request(route, session, body) {
  const url = new URL(route, base);
  if (!body) url.searchParams.set('session', session);
  const response = await fetch(url, body ? {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...body, session }), signal: AbortSignal.timeout(15000),
  } : { signal: AbortSignal.timeout(15000) });
  requestCount += 1;
  const text = await response.text();
  if (!response.ok) throw new Error(`${route} returned ${response.status}: ${text.slice(0, 300)}`);
  const result = JSON.parse(text);
  if (result?.error) throw new Error(`${route} returned an error: ${JSON.stringify(result.error)}`);
  return result;
}
const start = (seed, session) => request('/v1/run/start', session, { ...setup, seed });
const choices = async session => (await request('/v1/run/choices', session)).choices;
const step = (session, action) => request('/v1/run/action', session, { action });
const checkpoints = [];
for (const seed of seeds) {
  const session = `training-e137-production-${seed}-prepare`;
  let snapshot = await start(seed, session);
  let legal = await choices(session);
  let freeCount = 0;
  const preparation = [];
  for (let guard = 0; guard < 80 && freeCount <= depths.at(-1); guard += 1) {
    if (snapshot.state.careerComplete) throw new Error(`Seed ${seed} ended before all checkpoints.`);
    if (snapshot.state.phase === 'FREE' && depths.includes(freeCount)) {
      const checkpoint = {
        id: `seed-${seed}-training-${freeCount}`,
        seed, trainingActionsBefore: freeCount,
        preparation: [...preparation], snapshot, choices: legal,
        outcomes: [],
      };
      const recorded = legal.filter(choice => choice.id.startsWith('train_') || ['rest', 'recreation'].includes(choice.id));
      if (!recorded.length || !recorded.some(choice => choice.id === 'rest')) throw new Error('No native training/recovery alternatives.');
      for (const [i, choice] of recorded.entries()) {
        const branch = `training-e137-production-${seed}-${freeCount}-${i}`;
        let before = await start(seed, branch);
        for (const action of preparation) before = (await step(branch, action)).state;
        if (canonical(before) !== canonical(snapshot)) throw new Error(`Preparation is not deterministic: ${checkpoint.id}/${choice.id}`);
        if (canonical(await choices(branch)) !== canonical(legal)) throw new Error(`Legal choices changed: ${checkpoint.id}/${choice.id}`);
        const response = await step(branch, choice.id);
        if (!response.state?.state || typeof response.text !== 'string' || !Array.isArray(response.choices) || typeof response.careerEnded !== 'boolean') throw new Error('Unexpected action response shape.');
        checkpoint.outcomes.push({ actionId: choice.id, response });
      }
      checkpoints.push(checkpoint);
      console.log(JSON.stringify({ checkpoint: checkpoint.id, turn: snapshot.state.turn, energy: snapshot.state.energy, preparation: preparation.length, outcomes: checkpoint.outcomes.length }));
      if (freeCount === depths.at(-1)) break;
    }
    let action;
    if (snapshot.state.phase === 'FREE') {
      action = legal.find(choice => choice.id === 'train_speed')?.id;
      freeCount += 1;
    } else {
      action = legal.find(choice => choice.id === 'race')?.id ?? legal.find(choice => choice.id === 'event_0')?.id;
    }
    if (!action) throw new Error(`No explicit preparation action for ${snapshot.state.phase}: ${legal.map(choice => choice.id).join(', ')}`);
    const response = await step(session, action);
    preparation.push(action);
    snapshot = response.state;
    legal = response.choices;
  }
}
if (checkpoints.length !== seeds.length * depths.length) throw new Error('Not all planned checkpoints were captured.');
const fixture = {
  schema: 'uma.training-demo.v1',
  source: { repository: 'https://github.com/Jacob-Met/uma-sim', commit: SOURCE_COMMIT, tree: SOURCE_TREE, binarySha256 },
  setup, seeds, checkpoints,
};
const bytes = `${JSON.stringify(ordered(fixture), null, 2)}\n`;
await writeFile(flags['--output'], bytes, { flag: 'wx' });
console.log(JSON.stringify({ checkpoints: checkpoints.length, outcomes: checkpoints.reduce((n, c) => n + c.outcomes.length, 0), requests: requestCount, bytes: Buffer.byteLength(bytes), sha256: createHash('sha256').update(bytes).digest('hex') }));
