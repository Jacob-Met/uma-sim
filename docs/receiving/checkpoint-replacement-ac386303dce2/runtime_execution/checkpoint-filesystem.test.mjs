// Independent public-REST challenge of checkpoint replacement recovery.
// Only temporary files, a fresh API process and authored filesystem faults are used.
import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:net';
import { promisify } from 'node:util';
import { createHash } from 'node:crypto';
import { access, chmod, mkdir, mkdtemp, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, resolve } from 'node:path';

const binary = process.env.UMA_SIM_TEST_API_BIN;
const repo = resolve(process.env.UMA_SIM_TEST_REPO_ROOT || '.');
const output = resolve(process.env.UMA_SIM_TEST_OUTPUT || 'checkpoint-filesystem-results');
const receipts = [];
const run = promisify(execFile);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const uid = process.getuid?.();
const gid = process.getgid?.();
const skip = binary === undefined ? 'set UMA_SIM_TEST_API_BIN to run native API qualification'
  : process.platform !== 'linux' ? 'Linux filesystem fault qualification' : false;

async function startServer(cwd, { maxFileBytes } = {}) {
  assert.ok(binary, 'an explicitly empty API binary is invalid');
  await access(binary, constants.X_OK);
  const reserver = createServer().listen(0, '127.0.0.1');
  await once(reserver, 'listening');
  const port = reserver.address().port;
  await new Promise(resolve => reserver.close(resolve));
  // The child retains the current identity. Qualification requires directory
  // permissions to be enforced; no account or user namespace is changed.
  const env = { PATH: process.env.PATH, UMA_REPO_ROOT: repo };
  const python = process.env.UMA_SIM_TEST_PYTHON || 'python3';
  const wrapper = 'import os,resource,signal,sys; '
    + 'signal.signal(signal.SIGXFSZ,signal.SIG_IGN); '
    + 'limit=int(sys.argv[1]); resource.setrlimit(resource.RLIMIT_FSIZE,(limit,limit)); '
    + 'os.execv(sys.argv[2],[sys.argv[2],sys.argv[3]])';
  const command = maxFileBytes === undefined ? binary : python;
  const args = maxFileBytes === undefined ? [String(port)]
    : ['-c', wrapper, String(maxFileBytes), binary, String(port)];
  const child = spawn(command, args, { cwd, env,
    stdio: ['ignore', 'pipe', 'pipe'] });
  let log = '', spawnError;
  child.on('error', error => { spawnError = error; });
  child.stdout.on('data', chunk => { log = (log + chunk).slice(-32000); });
  child.stderr.on('data', chunk => { log = (log + chunk).slice(-32000); });
  const call = async (method, path, body) => {
    const response = await fetch(`http://127.0.0.1:${port}${path}`, {
      method, signal: AbortSignal.timeout(10000),
      ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body) }),
    });
    return { status: response.status, body: await response.json() };
  };
  const stop = async () => {
    if (child.exitCode === null && child.signalCode === null) {
      const done = once(child, 'exit');
      child.kill('SIGTERM');
      await Promise.race([done, delay(3000).then(() => {
        if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
      })]);
      if (child.exitCode === null && child.signalCode === null) await done;
    }
  };
  try {
    for (let i = 0; ; i++) {
      if (spawnError) throw spawnError;
      if (child.exitCode !== null || child.signalCode !== null) throw new Error(log);
      try { if ((await call('GET', '/v1/health')).status === 200) break; }
      catch (error) { if (i >= 200) throw error; }
      if (i >= 200) throw new Error('API health did not become ready');
      await delay(20);
    }
    return { child, call, stop, get log() { return log; } };
  } catch (error) { await stop(); throw error; }
}

async function caseFixture(name, check) {
  await mkdir(output, { recursive: true });
  const cwd = await mkdtemp(join(output, name + '-'));
  await chmod(cwd, 0o700);
  const dir = join(cwd, '.uma-sim', 'library');
  const snapshot = join(dir, 'keep.snapshot.json');
  const metadata = join(dir, 'keep.meta.json');
  let receiver;
  const record = { name, binarySha256: hash(await readFile(binary)), identity: { uid, gid } };
  try {
    const seed = await startServer(cwd);
    try {
      assert.equal((await seed.call('POST', '/v1/run/start', { session: 'original', seed: 42,
        raceModel: 'stub' })).status, 200);
      assert.equal((await seed.call('POST', '/v1/library/save', { session: 'original', name: 'keep',
        label: 'Original checkpoint', note: 'Keep the authored prior note' })).status, 200);
    } finally { await seed.stop(); }
    const previous = { snapshot: await readFile(snapshot), metadata: await readFile(metadata) };
    record.previousSha256 = { snapshot: hash(previous.snapshot), metadata: hash(previous.metadata) };
    const fixture = { cwd, dir, snapshot, metadata, previous, record,
      async receiver(options) {
        receiver = await startServer(cwd, options);
        assert.equal((await receiver.call('POST', '/v1/run/start', { session: 'replacement', seed: 91,
          raceModel: 'stub' })).status, 200);
        return receiver;
      } };
    await check(fixture);
    record.passed = true;
  } catch (error) {
    record.passed = false;
    record.error = String(error?.stack || error);
    throw error;
  } finally {
    if (receiver) { await receiver.stop(); record.serverLog = receiver.log; }
    receipts.push(record);
    await chmod(dir, 0o700).catch(() => {});
    await rm(cwd, { recursive: true, force: true });
  }
}

test('an existing recovery reservation is preserved and refuses replacement before any saved byte changes',
  { skip }, async () => caseFixture('reservation-collision', async f => {
    const api = await f.receiver();
    const priorRecovery = join(f.dir, `.rollback-${api.child.pid}-0-keep.snapshot.json`);
    const sentinel = Buffer.from('AUTHORED RETAINED RECOVERY COPY');
    await writeFile(priorRecovery, sentinel);
    const failed = await api.call('POST', '/v1/library/save', {
      session: 'replacement', name: 'keep', overwrite: true,
    });
    f.record.failure = failed;
    assert.equal(failed.status, 500);
    assert.deepEqual(await readFile(f.snapshot), f.previous.snapshot);
    assert.deepEqual(await readFile(f.metadata), f.previous.metadata);
    assert.deepEqual(await readFile(priorRecovery), sentinel);
    const retried = await api.call('POST', '/v1/library/save', {
      session: 'replacement', name: 'keep', overwrite: true,
    });
    assert.equal(retried.status, 200);
    assert.equal(retried.body.entry.seed, 91);
    assert.deepEqual(await readFile(priorRecovery), sentinel);
    assert.deepEqual((await readdir(f.dir)).filter(name => name.startsWith('.rollback-')),
      [priorRecovery.split('/').at(-1)]);
    f.record.retry = { status: retried.status, seed: retried.body.entry.seed };
  }));

test('a real partial backup write failure leaves the original pair intact and removes its incomplete recovery file',
  { skip }, async () => caseFixture('partial-backup-write', async f => {
    assert.ok(f.previous.snapshot.length > 1024, 'the fixture exceeds the child file-size ceiling');
    const api = await f.receiver({ maxFileBytes: 1024 });
    const failed = await api.call('POST', '/v1/library/save', {
      session: 'replacement', name: 'keep', overwrite: true,
    });
    f.record.failure = failed;
    assert.equal(failed.status, 500);
    assert.deepEqual(await readFile(f.snapshot), f.previous.snapshot);
    assert.deepEqual(await readFile(f.metadata), f.previous.metadata);
    assert.deepEqual((await readdir(f.dir)).filter(name => name.startsWith('.rollback-')), []);
    const loaded = await api.call('POST', '/v1/library/load', { name: 'keep', session: 'verified' });
    assert.equal(loaded.status, 200);
    f.record.restoredStatus = loaded.status;
  }));

test('a refused real rollback reports complete recoverable bytes that survive process exit and restore through the API',
  { skip }, async () => caseFixture('refused-rollback', async f => {
    const api = await f.receiver();
    const fifo = join(f.dir, `.tmp-${api.child.pid}-keep.meta.json`);
    await run('mkfifo', ['-m', '666', fifo]);
    const replacing = api.call('POST', '/v1/library/save', {
      session: 'replacement', name: 'keep', overwrite: true,
    });
    // Attach rejection handling immediately while the authored FIFO holds the
    // server after snapshot publication and before metadata completion.
    replacing.catch(() => {});
    try {
      for (let n = 0; ; n++) {
        if (!(await readFile(f.snapshot)).equals(f.previous.snapshot)) break;
        if (n === 250) throw new Error('replacement snapshot did not reach the FIFO boundary');
        await delay(10);
      }
      await chmod(f.dir, 0o555);
      await readFile(fifo); // Release the real metadata writer; rename now receives EACCES.
      const failed = await replacing;
      f.record.failure = failed;
      assert.equal(failed.status, 500);
      const backups = (await readdir(f.dir)).filter(name => name.startsWith('.rollback-'));
      f.record.afterFailure = {
        snapshotMatchesPrior: (await readFile(f.snapshot)).equals(f.previous.snapshot),
        metadataMatchesPrior: (await readFile(f.metadata)).equals(f.previous.metadata),
        recoveryFiles: backups,
      };
      assert.deepEqual(await readFile(f.metadata), f.previous.metadata);
      assert.equal(backups.length, 1, 'the prior saved snapshot must remain recoverable');
      assert.match(failed.body.error, /snapshot restoration failed/);
      assert.match(failed.body.error, /prior snapshot retained at/);
      const backup = join(f.dir, backups[0]);
      assert.ok(failed.body.error.includes(backup), 'the error gives the concrete recovery path');
      assert.deepEqual(await readFile(backup), f.previous.snapshot);
      await api.stop();
      assert.deepEqual(await readFile(backup), f.previous.snapshot, 'recovery survives process exit');
      await chmod(f.dir, 0o700);
      await rename(backup, f.snapshot);
      const recovered = await startServer(f.cwd);
      try {
        const loaded = await recovered.call('POST', '/v1/library/load', { name: 'keep', session: 'restored' });
        assert.equal(loaded.status, 200);
        assert.deepEqual(await readFile(f.snapshot), f.previous.snapshot);
        assert.deepEqual(await readFile(f.metadata), f.previous.metadata);
        f.record.manualRecovery = { status: loaded.status,
          snapshotSha256: hash(await readFile(f.snapshot)), metadataSha256: hash(await readFile(f.metadata)) };
      } finally { await recovered.stop(); }
    } finally {
      await chmod(f.dir, 0o700);
      // If a prior assertion failed, stopping the disposable process also
      // releases its FIFO writer; no real application process is signalled.
      await api.stop();
    }
  }));

after(async () => {
  if (skip) return;
  await mkdir(output, { recursive: true });
  await writeFile(join(output, 'case-receipts.json'), JSON.stringify({
    node: process.version, platform: process.platform, architecture: process.arch,
    receiverIdentity: process.env.UMA_SIM_TEST_RECEIVER_ID || 'unspecified', receipts,
  }, null, 2) + '\n');
});
