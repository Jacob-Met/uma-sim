import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import fsSync from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const [binaryArg, dataArg, outputArg, sourceHead] = process.argv.slice(2);
if (!binaryArg || !dataArg || !outputArg || !/^[a-f0-9]{40}$/.test(sourceHead || '')) {
  throw new Error('Usage: node receiving.mjs API_BINARY DATA_ROOT NEW_OUTPUT SOURCE_HEAD');
}
const binary = path.resolve(binaryArg);
const dataRoot = path.resolve(dataArg);
const output = path.resolve(outputArg);
const own = path.dirname(fileURLToPath(import.meta.url));
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const json = v => JSON.stringify(v, null, 2) + '\n';
const stable = v => Array.isArray(v) ? v.map(stable) : v && typeof v === 'object'
  ? Object.fromEntries(Object.keys(v).sort().map(k => [k, stable(v[k])])) : v;
const digest = v => hash(JSON.stringify(stable(v)));
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const binaryBefore = hash(await fs.readFile(binary));
await fs.mkdir(output);
const results = [];
let fatal = null;

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
}
async function filesAt(root) {
  const rows = [];
  async function walk(dir) {
    let entries;
    try { entries = await fs.readdir(dir, { withFileTypes: true }); }
    catch (e) { if (e.code === 'ENOENT') return; throw e; }
    for (const e of entries.sort((a,b) => a.name.localeCompare(b.name))) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) await walk(p);
      else if (e.isFile()) {
        const b = await fs.readFile(p);
        rows.push({ path: path.relative(root, p), bytes: b.length, sha256: hash(b) });
      } else throw new Error('Unexpected non-file in receiver-owned storage: ' + p);
    }
  }
  await walk(root);
  return rows;
}
async function launch(name) {
  const dir = path.join(output, name);
  const cwd = path.join(dir, 'state');
  await fs.mkdir(cwd, { recursive: true });
  const port = await freePort();
  const log = fsSync.openSync(path.join(dir, 'api.log'), 'wx');
  const child = spawn(binary, [String(port)], {
    cwd, stdio: ['ignore', log, log],
    env: { ...process.env, UMA_REPO_ROOT: dataRoot, UMA_RACE_MODEL: 'stub', UMA_POLICY_CMD: '' }
  });
  fsSync.closeSync(log);
  let spawnError;
  child.once('error', e => { spawnError = e; });
  const exited = new Promise(resolve => child.once('exit', (code, signal) => resolve({ code, signal })));
  const base = 'http://127.0.0.1:' + port;
  async function request(method, route, body) {
    const response = await fetch(base + route, {
      method,
      headers: body === undefined ? {} : { 'content-type': 'application/json' },
      ...(body === undefined ? {} : { body: typeof body === 'string' ? body : JSON.stringify(body) }),
      signal: AbortSignal.timeout(15000)
    });
    const text = await response.text();
    let value;
    try { value = JSON.parse(text); } catch { value = null; }
    return { status: response.status, headers: Object.fromEntries(response.headers), text, value };
  }
  async function expect(method, route, body, status = 200) {
    const r = await request(method, route, body);
    assert.equal(r.status, status, route + ': ' + r.text.slice(0, 500));
    return r;
  }
  async function stop() {
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
    const ended = await Promise.race([exited, pause(1500).then(() => null)]);
    if (!ended && child.exitCode === null && child.signalCode === null) {
      child.kill('SIGKILL');
      await Promise.race([exited, pause(2000)]);
    }
  }
  try {
    let ready = false;
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      if (spawnError) throw spawnError;
      if (child.exitCode !== null) throw new Error('API exited during startup: ' + child.exitCode);
      try {
        if ((await request('GET', '/v1/health')).status === 200) { ready = true; break; }
      } catch {}
      await pause(80);
    }
    assert.ok(ready, 'API health must become ready');
    await expect('POST', '/v1/run/start', {
      seed: '137', scenario: 'ura', trainee: 'Special Week', raceModel: 'stub',
      policy: 'default', speed: '17', dialogue: 'off',
      session: 'receiver-alpha', label: 'Receiver alpha'
    });
    await expect('POST', '/v1/session/fork', {
      session: 'receiver-alpha', id: 'receiver-beta', label: 'Receiver beta'
    });
    await expect('POST', '/v1/library/save', {
      session: 'receiver-alpha', name: 'receiver-kept', label: 'Kept before admission test'
    });
    await expect('POST', '/v1/lab/branch', {
      checkpoint: 'session:receiver-alpha', name: 'receiver-kept-branch',
      maxActions: 1, policy: 'default'
    });
    async function state() {
      const sessions = (await expect('GET', '/v1/sessions')).value;
      const snapshots = {};
      for (const entry of sessions.sessions) {
        // The existing explicit-empty resolver is separately owned by #63.
        // Observe nonempty IDs and the actual active legacy session only.
        if (entry.id === '' && sessions.active !== '') continue;
        const route = entry.id === '' ? '/v1/run/state'
          : '/v1/run/state?session=' + encodeURIComponent(entry.id);
        snapshots[entry.id] = (await expect('GET', route)).value;
      }
      return {
        sessions, snapshots,
        activeText: (await expect('GET', '/v1/run/text')).value,
        telemetry: (await expect('GET', '/v1/run/telemetry')).value,
        library: (await expect('GET', '/v1/library')).value,
        branches: (await expect('GET', '/v1/lab/branches')).value,
        storedFiles: await filesAt(path.join(cwd, '.uma-sim'))
      };
    }
    const initial = await state();
    assert.deepEqual(initial.snapshots['receiver-alpha'], initial.snapshots['receiver-beta'],
      'Actual session fork must provide an identical independent initial snapshot');
    return { name, dir, cwd, port, base, child, request, expect, state, stop };
  } catch (e) {
    await stop();
    throw e;
  }
}
function rawRequest(port, route, bytes, declaredLength = bytes.length) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0, ended = false;
    const socket = net.createConnection({ host: '127.0.0.1', port, allowHalfOpen: true });
    const timer = setTimeout(() => { socket.destroy(); reject(new Error('Raw request timed out')); }, 7000);
    const finish = () => {
      if (ended) return;
      ended = true; clearTimeout(timer); socket.destroy();
      const raw = Buffer.concat(chunks);
      const text = raw.toString('utf8');
      const boundary = text.indexOf('\r\n\r\n');
      const status = Number((/^HTTP\/1\.[01] (\d{3})/.exec(text) || [])[1]) || null;
      const headers = {};
      if (boundary >= 0) {
        for (const line of text.slice(0, boundary).split('\r\n').slice(1)) {
          const colon = line.indexOf(':');
          if (colon >= 0) headers[line.slice(0,colon).toLowerCase()] = line.slice(colon+1).trim();
        }
      }
      const body = boundary >= 0 ? text.slice(boundary + 4) : '';
      let value; try { value = JSON.parse(body); } catch { value = null; }
      resolve({ status, headers, text: body, value, rawBytes: raw.length,
        requestBodyHex: bytes.toString('hex'), declaredLength });
    };
    socket.once('connect', () => {
      const head = 'POST ' + route + ' HTTP/1.1\r\nHost: 127.0.0.1\r\n'
        + 'Content-Type: application/json\r\nContent-Length: ' + declaredLength
        + '\r\nConnection: close\r\n\r\n';
      socket.end(Buffer.concat([Buffer.from(head), bytes]));
    });
    socket.on('data', b => {
      total += b.length;
      if (total > 4 * 1024 * 1024) { socket.destroy(); reject(new Error('Response exceeded bounded receiver limit')); }
      else chunks.push(b);
    });
    socket.once('end', finish);
    socket.once('close', finish);
    socket.once('error', e => {
      if (e.code === 'ECONNRESET') finish();
      else { clearTimeout(timer); reject(e); }
    });
  });
}
async function runCase(name, action) {
  let api;
  const record = { name, passed: false, failures: [], setupError: null };
  try {
    api = await launch(name);
    record.pid = api.child.pid;
    record.port = api.port;
    await action(api, record);
    record.passed = record.failures.length === 0;
  } catch (e) {
    if (!api) record.setupError = e.stack || String(e);
    else record.failures.push({ check: 'receiver execution', error: e.stack || String(e) });
  } finally {
    if (api) await api.stop();
    results.push(record);
    await fs.writeFile(path.join(output, name + '.json'), json(record));
    process.stdout.write(JSON.stringify({ name, passed: record.passed,
      failures: record.failures.map(x => x.check), setupError: !!record.setupError }) + '\n');
  }
}
function verify(record, name, fn) {
  try { fn(); } catch (e) { record.failures.push({ check: name, error: e.message }); }
}
async function refusalCase(name, route, rawBody, options = {}) {
  await runCase(name, async (api, record) => {
    const before = await api.state();
    const response = options.raw
      ? await rawRequest(api.port, route, Buffer.isBuffer(rawBody) ? rawBody : Buffer.from(rawBody),
          options.declaredLength)
      : await api.request('POST', route, rawBody);
    const after = await api.state();
    record.response = response;
    record.beforeDigest = digest(before);
    record.afterDigest = digest(after);
    await fs.writeFile(path.join(api.dir, 'before.json'), json(before));
    await fs.writeFile(path.join(api.dir, 'after.json'), json(after));
    verify(record, 'request refused', () => {
      if (options.transportMayClose) assert.ok(response.status === null || response.status === 400,
        'Malformed transport should be refused, got ' + response.status);
      else assert.equal(response.status, 400);
      if (response.status !== null) {
        assert.match(response.headers['content-type'] || '', /application\/json/);
        assert.equal(response.headers['access-control-allow-origin'], '*');
        assert.ok(typeof response.value?.error === 'string' && response.value.error.length > 0,
          'Readable JSON error must explain refusal');
      }
    });
    verify(record, 'complete observed state and stored bytes unchanged', () => assert.deepEqual(after, before));
    await api.expect('POST', '/v1/run/auto', { session: 'receiver-alpha', policy: 'default' });
    await api.expect('POST', '/v1/run/auto', { session: 'receiver-beta', policy: 'default' });
    const next = await api.state();
    record.nextAlphaDigest = digest(next.snapshots['receiver-alpha']);
    record.nextBetaDigest = digest(next.snapshots['receiver-beta']);
    await fs.writeFile(path.join(api.dir, 'next.json'), json(next));
    verify(record, 'next valid action preserves deterministic continuation', () =>
      assert.deepEqual(next.snapshots['receiver-alpha'], next.snapshots['receiver-beta']));
    await api.expect('GET', '/v1/health');
  });
}
try {
  const malformed = [
    ['malformed_start', '/v1/run/start', '{"seed":"999",'],
    ['trailing_auto', '/v1/run/auto', '{} {"unexpected":1}'],
    ['malformed_action', '/v1/run/action', '{"action":}'],
    ['malformed_fast', '/v1/run/fast', '{"multiplier":"31",'],
    ['malformed_fork', '/v1/session/fork', '{'],
    ['malformed_checkpoint_save', '/v1/library/save', '{'],
    ['malformed_branch', '/v1/lab/branch', '{']
  ];
  for (const args of malformed) await refusalCase(...args);
  for (const [name, body] of [['null', 'null'], ['boolean', 'false'], ['array', '[{}]'], ['string', '"{}"']]) {
    await refusalCase('nonobject_' + name, '/v1/run/auto', body);
  }
  await refusalCase('invalid_utf8_start', '/v1/run/start',
    Buffer.concat([Buffer.from('{"session":"receiver-alpha"}'), Buffer.from([255])]), { raw: true });
  await refusalCase('truncated_json_start', '/v1/run/start', '{"seed":',
    { raw: true, declaredLength: 24, transportMayClose: true });
  await refusalCase('short_declared_object_start', '/v1/run/start', '{}',
    { raw: true, declaredLength: 14, transportMayClose: true });

  for (const [name, body] of [['empty', ''], ['whitespace', ' \r\n\t '], ['object', '{}']]) {
    await runCase('valid_' + name + '_auto', async (api, record) => {
      const r = await api.expect('POST', '/v1/run/auto', body);
      record.response = r;
      await api.expect('POST', '/v1/run/auto', { session: 'receiver-alpha', policy: 'default' });
      const after = await api.state();
      await fs.writeFile(path.join(api.dir, 'after.json'), json(after));
      verify(record, 'legacy empty-object operation advances once', () =>
        assert.deepEqual(after.snapshots['receiver-alpha'], after.snapshots['receiver-beta']));
    });
  }
  await runCase('valid_utf8_object_start', async (api, record) => {
    const label = '検証 Δ café';
    const response = await rawRequest(api.port, '/v1/run/start', Buffer.from(JSON.stringify({
      seed: '203', scenario: 'ura', raceModel: 'stub', session: 'receiver-unicode', label
    })));
    record.response = response;
    const after = await api.state();
    await fs.writeFile(path.join(api.dir, 'after.json'), json(after));
    verify(record, 'valid UTF-8 object is admitted intact', () => {
      assert.equal(response.status, 200);
      assert.equal(after.sessions.active, 'receiver-unicode');
      assert.equal(after.sessions.sessions.find(x => x.id === 'receiver-unicode').label, label);
      assert.ok(after.snapshots['receiver-unicode']);
    });
  });
  await runCase('valid_routing_refusals', async (api, record) => {
    const before = await api.state();
    record.responses = [
      await api.expect('POST', '/v1/receiver-unknown', {}, 404),
      await api.expect('PUT', '/v1/run/start', {}, 405),
      await api.expect('POST', '/v1/run/auto', { session: 'receiver-missing' }, 404)
    ];
    const after = await api.state();
    verify(record, 'routing refusals preserve actual state', () => assert.deepEqual(after, before));
  });
} catch (e) { fatal = e.stack || String(e); }

const binaryAfter = hash(await fs.readFile(binary));
const report = {
  schema: 'hamon.uma-api-independent-admission.v1',
  sourceHead,
  reviewedBaselineHead: '0ac14602addd1a4610aa8359896915a34cce5659',
  reviewedBaselineApiBlob: '4ff7034bc5701d793cd93d645d89d40d73fc3273',
  binary, binaryBefore, binaryAfter, dataRoot,
  receiverSha256: hash(await fs.readFile(fileURLToPath(import.meta.url))),
  frozenContractSha256: hash(await fs.readFile(path.join(own, 'frozen-contract.md'))),
  node: process.version, platform: process.platform, arch: process.arch,
  passed: results.filter(x => x.passed).length,
  failed: results.filter(x => !x.passed).map(x => x.name),
  results, fatal,
  scope: 'Actual loopback HTTP against exact supplied Rust API; fresh per-case cwd and synthetic seeded careers. No live API, existing store, provider or owner source mutation. Full snapshot/disk/next-action acceptance is separate from repository CI and deployment.'
};
await fs.writeFile(path.join(output, 'receipt.json'), json(report));
process.stdout.write(JSON.stringify({ passed: report.passed, failed: report.failed,
  fatal, binaryUnchanged: binaryBefore === binaryAfter }) + '\n');
process.exitCode = fatal || report.failed.length || binaryBefore !== binaryAfter ? 1 : 0;
