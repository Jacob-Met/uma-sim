// Real API receiving check. Uses only an isolated temporary career library.
// Build the intended API first, then set UMA_SIM_TEST_API_BIN to that binary.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const binary = process.env.UMA_SIM_TEST_API_BIN;
assert.ok(binary, 'UMA_SIM_TEST_API_BIN must identify the intended native API');
const repo = process.env.UMA_SIM_TEST_REPO_ROOT || fileURLToPath(new URL('../../../', import.meta.url));
const output = process.env.UMA_SIM_TEST_OUTPUT_DIR || tmpdir();
const expected = process.env.UMA_SIM_TEST_EXPECT_REPLACEMENT || 'preserved';
assert.ok(['preserved', 'lost'].includes(expected), 'expected outcome must be preserved or lost');
await mkdir(output, { recursive: true });
const cwd = await mkdtemp(path.join(output, 'checkpoint-replacement-'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha256 = hash(await readFile(binary));
const reserver = createServer().listen(0, '127.0.0.1');
await once(reserver, 'listening');
const port = reserver.address().port;
await new Promise(resolve => reserver.close(resolve));
const child = spawn(path.resolve(binary), [String(port)], {
  cwd, env: { ...process.env, UMA_REPO_ROOT: path.resolve(repo) },
  stdio: ['ignore', 'pipe', 'pipe'],
});
let log = '';
let spawnError;
child.on('error', error => { spawnError = error; });
child.stdout.on('data', bytes => { log += bytes; });
child.stderr.on('data', bytes => { log += bytes; });
const call = async (method, route, body) => {
  const response = await fetch(`http://127.0.0.1:${port}${route}`, {
    method, signal: AbortSignal.timeout(10000),
    ...(body === undefined ? {} : {
      headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
    }),
  });
  return { status: response.status, body: await response.json() };
};
try {
  for (let attempt = 0; ; attempt++) {
    if (spawnError) throw spawnError;
    try { await call('GET', '/v1/health'); break; }
    catch (error) { if (attempt === 100 || child.exitCode !== null) throw error; }
    await new Promise(resolve => setTimeout(resolve, 25));
  }
  const start = await call('POST', '/v1/run/start', { session: 'source', seed: 42, raceModel: 'stub' });
  const saved = await call('POST', '/v1/library/save', {
    session: 'source', name: 'keep', label: 'Previous saved career', note: 'Preserve this checkpoint',
  });
  assert.equal(start.status, 200);
  assert.equal(saved.status, 200);
  const library = path.join(cwd, '.uma-sim', 'library');
  const snapshotPath = path.join(library, 'keep.snapshot.json');
  const metadataPath = path.join(library, 'keep.meta.json');
  const previousSnapshot = await readFile(snapshotPath);
  const previousMetadata = await readFile(metadataPath);
  const advance = await call('POST', '/v1/run/auto', { session: 'source', policy: 'bot' });
  assert.equal(advance.status, 200);
  assert.notEqual(advance.body.state.state.turn, saved.body.entry.turn);

  // Refuse metadata staging after the prior saved pair is known to be intact.
  // This uses the real filesystem and unchanged HTTP save handler.
  await mkdir(path.join(library, `.tmp-${child.pid}-keep.meta.json`));
  const replacement = await call('POST', '/v1/library/save', {
    session: 'source', name: 'keep', overwrite: true,
  });
  const snapshotAfter = await readFile(snapshotPath).catch(error => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  const metadataAfter = await readFile(metadataPath);
  const loadAfter = await call('POST', '/v1/library/load', { name: 'keep', session: 'restored' });
  const listingAfter = await call('GET', '/v1/library');
  const recoveryFiles = (await readdir(library)).filter(name => name.startsWith('.rollback-'));
  const receipt = {
    binarySource: process.env.UMA_SIM_TEST_BINARY_SOURCE || 'not supplied',
    binarySha256, expected, cwd,
    savedTurn: saved.body.entry.turn,
    replacementTurn: advance.body.state.state.turn,
    previousSnapshotSha256: hash(previousSnapshot),
    previousMetadataSha256: hash(previousMetadata),
    snapshotAfterSha256: snapshotAfter === null ? null : hash(snapshotAfter),
    snapshotUnchanged: snapshotAfter !== null && previousSnapshot.equals(snapshotAfter),
    metadataUnchanged: previousMetadata.equals(metadataAfter),
    replacement, loadAfter, listingAfter, recoveryFiles,
  };
  await writeFile(path.join(cwd, 'result.json'), JSON.stringify(receipt, null, 2) + '\n');
  assert.equal(replacement.status, 500, 'the injected write refusal must remain an error');
  assert.equal(receipt.metadataUnchanged, true);
  assert.equal(listingAfter.status, 200);
  assert.deepEqual(listingAfter.body.entries, [saved.body.entry]);
  assert.deepEqual(recoveryFiles, [], 'successful restoration must discard the redundant recovery copy');
  if (expected === 'preserved') {
    assert.equal(receipt.snapshotUnchanged, true, 'failed replacement must retain all previous snapshot bytes');
    assert.equal(loadAfter.status, 200, 'the original career must remain loadable');
    assert.deepEqual(loadAfter.body.state, JSON.parse(previousSnapshot));
  } else {
    assert.equal(snapshotAfter, null, 'negative control expects the baseline snapshot deletion');
    assert.equal(loadAfter.status, 404);
  }
  console.log(JSON.stringify({ result: path.join(cwd, 'result.json'), binarySha256, expected, passed: true }));
} finally {
  if (child.exitCode === null && child.signalCode === null && !spawnError) {
    const done = once(child, 'exit');
    child.kill('SIGTERM');
    const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
    try { await done; } finally { clearTimeout(timer); }
  }
  await writeFile(path.join(cwd, 'server.log'), log);
}
