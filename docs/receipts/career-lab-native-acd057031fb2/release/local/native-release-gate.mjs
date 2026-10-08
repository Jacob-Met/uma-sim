import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';

const repo = '/srv/hamon-estate/worktrees/chatgpt-acd057031fb2-uma-native';
const namespace = '/srv/hamon-estate/artifacts/chatgpt-acd057031fb2';
const output = path.join(namespace, 'release-receiving-fe0ff870');
const sourceHead = 'fe0ff870ee10f21e1a7cfa9264ef3196bc417854';
const uiSourceHead = 'c19fcd5f2dac1f193f167c407cbc3215522b4eb5';
const expectedTree = '7842ede60ef41699c0930dc2c5f217aa4fdcec10';
const archive = path.join(namespace, 'uma-ui-c19fcd5-dist.tar.gz');
const archiveSha = '46c77e9e336333be5dfe0b3fdb27605b4e308062b8ef80c19e891a72092f7d3d';
const assets = [
  { path: 'index.html', bytes: 394, sha256: '4dbea013d8024e8236d0b0c884fc70efab5ebf5e92dec3dceaa02acf6a9dc841' },
  { path: 'assets/index-C8rte7ax.css', bytes: 8250, sha256: 'beda9c92acf18ac0a0ca344bea9c6be08715624da91d007032ffdcb5dad195ac' },
  { path: 'assets/index-uVUO5YHC.js', bytes: 192349, sha256: '3614049c80538516288a75776571c079ea2703e639aa4dc44bee32adced83d9c' },
];
const inputPaths = ['Cargo.toml', 'Cargo.lock', 'uma-sim-core', 'uma-race-core', 'packages/uma-sim-ui', 'research', 'content_packs', 'knowledge/canonical/by_kind', '.github/workflows/ci.yml', 'scripts/smoke_release_layout.sh', 'scripts/calibrate_grand_live.py'];
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fileHash = filename => hash(fs.readFileSync(filename));
const direct = (program, args, options = {}) => {
  const r = spawnSync(program, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...options });
  assert.equal(r.status, 0, [program, ...args].join(' ') + '\n' + r.stderr);
  return r.stdout.trim();
};
const git = args => direct('/usr/bin/git', ['-C', repo, ...args]);
assert.equal(git(['rev-parse', 'HEAD']), sourceHead);
assert.equal(git(['rev-parse', 'HEAD^{tree}']), expectedTree);
assert.equal(git(['status', '--porcelain']), '');
assert.equal(git(['diff', '--name-only', uiSourceHead, sourceHead, '--', ...inputPaths]), '');
assert.equal(fileHash(archive), archiveSha);
assert.ok(!fs.existsSync(output), 'The receiving directory already exists. Inspect the previous run before continuing.');
fs.mkdirSync(output, { recursive: true });
const save = (filename, data) => fs.writeFileSync(path.join(output, filename), JSON.stringify(data, null, 2) + '\n');
const disk = () => {
  const s = fs.statfsSync(namespace);
  return { blockSize: s.bsize, availableBytes: s.bavail * s.bsize, freeBytes: s.bfree * s.bsize };
};
const sourcePins = () => Object.fromEntries(git(['ls-tree', '-r', 'HEAD', '--', ...inputPaths]).split('\n').filter(Boolean).map(line => {
  const [metadata, filename] = line.split('\t');
  const [, type, blob] = metadata.split(' ');
  assert.equal(type, 'blob');
  const bytes = fs.readFileSync(path.join(repo, filename));
  assert.equal(createHash('sha1').update('blob ' + bytes.length + '\0').update(bytes).digest('hex'), blob, filename);
  return [filename, hash(bytes)];
}));
const before = sourcePins();
const target = path.join(namespace, 'release-target');
const cargoHome = path.join(namespace, 'release-cargo-home');
const temp = path.join(output, 'tmp');
for (const directory of [target, cargoHome, temp]) fs.mkdirSync(directory, { recursive: true });
const overrides = {
  CARGO_HOME: cargoHome, CARGO_TARGET_DIR: target, RUSTC: '/usr/bin/rustc',
  CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_PROFILE_RELEASE_DEBUG: '0',
  CARGO_PROFILE_RELEASE_STRIP: 'symbols', TMPDIR: temp,
};
const env = { ...process.env, ...overrides };
const childOnlyUnset = ['CARGO_NET_OFFLINE', 'UMA_REPO_ROOT', 'UMA_POLICY_CMD'];
for (const name of childOnlyUnset) delete env[name];
const receipt = {
  schema: 'uma-sim-native-release-receiving.v1', startedAt: new Date().toISOString(),
  source: { repo, commit: sourceHead, tree: expectedTree, uiSourceCommit: uiSourceHead, releaseInputDiff: [], before },
  environment: {
    node: process.version, rustc: direct('/usr/bin/rustc', ['--version']), cargo: direct('/usr/bin/cargo', ['--version']),
    overrides, childOnlyUnset, note: 'Only the receiving child environment changes; no user/common environment, Git ref or tracked source is edited.',
  },
  profile: 'Standard release optimization (opt-level3), two jobs, debug0, incremental0, symbol stripping to limit disk use.',
  uiArchive: { path: archive, sha256: archiveSha, assets }, diskBefore: disk(), steps: [],
  separation: 'Actual c19 UI build is transferred by verified bytes. Native source is fe0 (docs-only successor). This receipt qualifies release embedding and staged native runtime, not exact GitHub CI, browser interaction, or the prior354-test workspace gate.',
};
const run = (name, program, args, cwd = repo) => {
  console.log(JSON.stringify({ phase: name, state: 'started', at: new Date().toISOString() }));
  const startedAt = new Date().toISOString();
  const r = spawnSync(program, args, { cwd, env, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  fs.writeFileSync(path.join(output, name + '.stdout'), r.stdout || '');
  fs.writeFileSync(path.join(output, name + '.stderr'), r.stderr || '');
  const step = { name, program, args, cwd, startedAt, finishedAt: new Date().toISOString(), exitCode: r.status, error: r.error?.message ?? null, stdoutSha256: hash(r.stdout || ''), stderrSha256: hash(r.stderr || '') };
  receipt.steps.push(step);
  save('receipt.json', receipt);
  console.log(JSON.stringify({ phase: name, exitCode: r.status, error: step.error, disk: disk() }));
  assert.equal(r.status, 0, name + '\n' + r.stderr);
  return r.stdout;
};
let api, apiDone, apiLog, hiddenDist;
try {
  assert.ok(receipt.diskBefore.availableBytes >= 700 * 1024 * 1024, 'Less than700MiB available; preserve the gate before starting another build.');
  const archiveMembers = direct('/usr/bin/tar', ['-tzf', archive]).split('\n').filter(name => !name.endsWith('/'));
  assert.deepEqual(archiveMembers.sort(), assets.map(asset => 'packages/uma-sim-ui/dist/' + asset.path).sort());
  direct('/usr/bin/tar', ['-xzf', archive, '-C', repo, ...assets.map(asset => 'packages/uma-sim-ui/dist/' + asset.path)]);
  const dist = path.join(repo, 'packages/uma-sim-ui/dist');
  for (const asset of assets) {
    const bytes = fs.readFileSync(path.join(dist, asset.path));
    assert.equal(bytes.length, asset.bytes); assert.equal(hash(bytes), asset.sha256);
  }
  receipt.uiAssetReadback = '3/3 exact; the real built index exists before cargo build, so the build-script placeholder path is not used.';
  const metadata = JSON.parse(run('locked-release-metadata', '/usr/bin/cargo', ['metadata', '--locked', '--format-version', '1', '--features', 'embed-ui', '--filter-platform', 'x86_64-unknown-linux-gnu']));
  const versions = metadata.packages.filter(pkg => pkg.rust_version).map(pkg => ({ name: pkg.name, version: pkg.version, rustVersion: pkg.rust_version }));
  const incompatible = versions.filter(pkg => {
    const [major, minor] = pkg.rustVersion.split('.').map(Number);
    return major > 1 || major === 1 && minor > 93;
  });
  receipt.declaredMsrv = { compiler: '1.93.1', packages: versions, incompatible };
  save('declared-msrv.json', receipt.declaredMsrv);
  assert.deepEqual(incompatible, [], 'A declared dependency MSRV exceeds the native toolchain.');
  run('release-build', '/usr/bin/cargo', ['build', '--locked', '--release', '--features', 'embed-ui', '-p', 'uma-sim-core']);
  const stage = path.join(output, 'stage');
  fs.mkdirSync(path.join(stage, 'knowledge/canonical/by_kind'), { recursive: true });
  receipt.binaries = [];
  for (const name of ['uma-sim', 'uma-sim-api']) {
    const compiled = path.join(target, 'release', name);
    const staged = path.join(stage, name);
    fs.copyFileSync(compiled, staged);
    fs.chmodSync(staged, 0o755);
    assert.equal(fileHash(staged), fileHash(compiled));
    receipt.binaries.push({ name, compiled, staged, bytes: fs.statSync(staged).size, sha256: fileHash(staged) });
  }
  for (const folder of ['research', 'content_packs']) fs.cpSync(path.join(repo, folder), path.join(stage, folder), { recursive: true });
  for (const filename of fs.readdirSync(path.join(repo, 'knowledge/canonical/by_kind')).filter(filename => filename.endsWith('.json')))
    fs.copyFileSync(path.join(repo, 'knowledge/canonical/by_kind', filename), path.join(stage, 'knowledge/canonical/by_kind', filename));
  run('staged-release-layout', '/bin/bash', ['scripts/smoke_release_layout.sh', stage, 'uma-sim']);
  assert.deepEqual(sourcePins(), before);
  const runtime = path.join(output, 'embedded-runtime');
  fs.mkdirSync(runtime);
  const reserver = net.createServer();
  reserver.listen(0, '127.0.0.1');
  await once(reserver, 'listening');
  const port = reserver.address().port;
  await new Promise(resolve => reserver.close(resolve));
  hiddenDist = dist + '.release-receiving-hidden';
  assert.ok(!fs.existsSync(hiddenDist));
  fs.renameSync(dist, hiddenDist);
  receipt.embeddedRuntime = { origin: 'http://127.0.0.1:' + port, cwd: runtime, sourceDistHidden: true, checks: [] };
  apiLog = fs.createWriteStream(path.join(output, 'embedded-api.log'));
  api = spawn(path.join(stage, 'uma-sim-api'), [String(port)], { cwd: runtime, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let apiExited = false;
  apiDone = once(api, 'exit').then(([code, signal]) => { apiExited = true; receipt.embeddedRuntime.exit = { code, signal }; });
  api.stdout.pipe(apiLog, { end: false }); api.stderr.pipe(apiLog, { end: false });
  for (let attempt = 0; ; attempt++) {
    assert.ok(!apiExited, 'Staged API exited before health was ready.');
    try { const response = await fetch(receipt.embeddedRuntime.origin + '/v1/health', { signal: AbortSignal.timeout(1000) }); if (response.ok) break; } catch {}
    assert.ok(attempt < 99, 'Staged API readiness timeout.');
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  for (const asset of assets) {
    const route = asset.path === 'index.html' ? '/' : '/' + asset.path;
    const response = await fetch(receipt.embeddedRuntime.origin + route, { signal: AbortSignal.timeout(5000) });
    const bytes = Buffer.from(await response.arrayBuffer());
    const result = { route, status: response.status, bytes: bytes.length, sha256: hash(bytes), contentType: response.headers.get('content-type'), expectedSha256: asset.sha256 };
    receipt.embeddedRuntime.checks.push(result);
    fs.mkdirSync(path.dirname(path.join(runtime, asset.path)), { recursive: true });
    fs.writeFileSync(path.join(runtime, asset.path), bytes);
    assert.equal(response.status, 200); assert.equal(bytes.length, asset.bytes); assert.equal(hash(bytes), asset.sha256);
  }
  const missing = await fetch(receipt.embeddedRuntime.origin + '/assets/receiving-definitely-absent.js', { signal: AbortSignal.timeout(5000) });
  receipt.embeddedRuntime.missingAsset = { route: '/assets/receiving-definitely-absent.js', status: missing.status };
  assert.equal(missing.status, 404);
  receipt.embeddedRuntime.passed = true;
  receipt.passed = true;
} catch (error) {
  receipt.passed = false;
  receipt.failure = error.stack;
  console.log(JSON.stringify({ phase: 'failure', error: error.message }));
} finally {
  if (api) { api.kill('SIGTERM'); await apiDone; apiLog.end(); }
  if (hiddenDist && fs.existsSync(hiddenDist)) fs.renameSync(hiddenDist, path.join(repo, 'packages/uma-sim-ui/dist'));
  try {
    receipt.source.after = sourcePins();
    assert.deepEqual(receipt.source.after, before);
    assert.equal(git(['rev-parse', 'HEAD']), sourceHead);
    assert.equal(git(['status', '--porcelain']), '');
    receipt.sourceUnchanged = true;
  } catch (error) { receipt.passed = false; receipt.sourceVerificationFailure = error.stack; }
  receipt.diskAfter = disk();
  receipt.finishedAt = new Date().toISOString();
  save('receipt.json', receipt);
  console.log(JSON.stringify({ output, passed: receipt.passed, sourceUnchanged: receipt.sourceUnchanged, steps: receipt.steps.map(({ name, exitCode }) => ({ name, exitCode })), binarySha256: receipt.binaries?.map(({ name, sha256 }) => ({ name, sha256 })), embeddedAssets: receipt.embeddedRuntime?.checks, failure: receipt.failure ?? receipt.sourceVerificationFailure ?? null }));
}
if (!receipt.passed) process.exitCode = 1;
