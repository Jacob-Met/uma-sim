#!/usr/bin/env node
// Exercise the real builder in a fresh input copy. No npm scripts or installs run.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { readFile, writeFile, mkdir, mkdtemp, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const options = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i];
  if (!['--source', '--toolchain', '--output'].includes(key) || !process.argv[i + 1] || Object.hasOwn(options, key)) {
    throw new Error('Usage: node tests/receive-build.mjs [--source REPOSITORY] [--toolchain EXISTING_UI_PACKAGE] [--output NEW_DIRECTORY]');
  }
  options[key] = resolve(process.argv[i + 1]);
}
const source = options['--source'] ?? resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const toolchain = options['--toolchain'] ?? join(source, 'packages/uma-sim-ui');
const output = options['--output'] ?? await mkdtemp(join(tmpdir(), 'uma-training-build-receiving-'));
if (options['--output']) await mkdir(output, { mode: 0o700 });
const trial = join(output, 'input-copy');
const demo = join(trial, 'packages/training-demo');
const ui = join(trial, 'packages/uma-sim-ui');
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const gitBlob = bytes => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
const pageExpected = { bytes: 348711, sha256: '56f75bd888cc20b5c71ea007a76430736c0668eb89805dc867b81fb053056b85' };
const fieldsSha256 = '4956fe167824c184fd52951070ea1f3d9dede8cf471d893f1a22d99e32e910ae';
const paths = [
  'packages/training-demo/build.mjs',
  'packages/training-demo/index.html',
  'packages/training-demo/fixture.json',
  'packages/training-demo/src/main.tsx',
  'packages/training-demo/src/demo.css',
  'packages/uma-sim-ui/index.html',
  'packages/uma-sim-ui/package.json',
  'packages/uma-sim-ui/package-lock.json',
  'packages/uma-sim-ui/tsconfig.json',
  'packages/uma-sim-ui/src/components/StatsPanel.tsx',
  'packages/uma-sim-ui/src/components/ChoicePanel.tsx',
  'packages/uma-sim-ui/src/api/types.ts',
  'packages/uma-sim-ui/src/styles/app.css',
];
// The UI types can import other UI modules. Copy its source tree to preserve
// that real typecheck context instead of substituting declarations or stubs.
async function addUiSources(directory) {
  const entries = await readdir(join(source, directory), { withFileTypes: true });
  entries.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  for (const entry of entries) {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) await addUiSources(path);
    else if (entry.isFile()) { if (!paths.includes(path)) paths.push(path); }
    else throw new Error(`Unexpected non-file UI source: ${path}`);
  }
}
await addUiSources('packages/uma-sim-ui/src');
const pinnedUiPaths = [
  'packages/uma-sim-ui/index.html', 'packages/uma-sim-ui/package.json',
  'packages/uma-sim-ui/package-lock.json', 'packages/uma-sim-ui/tsconfig.json',
  'packages/uma-sim-ui/src/components/StatsPanel.tsx',
  'packages/uma-sim-ui/src/components/ChoicePanel.tsx',
  'packages/uma-sim-ui/src/api/types.ts', 'packages/uma-sim-ui/src/styles/app.css',
];
const inputs = [];
for (const path of paths) {
  const bytes = await readFile(join(source, path));
  await mkdir(dirname(join(trial, path)), { recursive: true });
  await writeFile(join(trial, path), bytes, { flag: 'wx' });
  inputs.push({ path, bytes: bytes.length, sha256: sha256(bytes), gitBlob: gitBlob(bytes) });
}
await writeFile(join(output, 'input-manifest.json'), JSON.stringify({ source, inputs }, null, 2) + '\n', { flag: 'wx' });
const packagePath = join(ui, 'package.json');
const packageBytes = await readFile(packagePath);
const originalPackage = JSON.parse(packageBytes);
const builderSha256 = inputs.find(x => x.path === 'packages/training-demo/build.mjs').sha256;
const pagePath = join(demo, 'out/index.html');
const receiptPath = join(demo, 'out/build-receipt.json');
const cases = [];
const startedAt = new Date().toISOString();

async function invoke(name, expectation, expectedError) {
  const before = expectation === 'reject'
    ? { page: sha256(await readFile(pagePath)), receipt: sha256(await readFile(receiptPath)) }
    : null;
  const result = spawnSync(process.execPath, [join(demo, 'build.mjs'), '--toolchain', toolchain], {
    cwd: trial, encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024,
  });
  const actualPackage = await readFile(packagePath);
  await writeFile(join(output, `${name}.stdout.log`), result.stdout ?? '', { flag: 'wx' });
  await writeFile(join(output, `${name}.stderr.log`), result.stderr ?? '', { flag: 'wx' });
  await writeFile(join(output, `${name}.package.json`), actualPackage, { flag: 'wx' });
  const record = { name, expectation, exit: result.status, signal: result.signal, packageBlob: gitBlob(actualPackage) };
  assert.ifError(result.error);
  assert.equal(result.signal, null, `${name}: builder was interrupted`);
  if (expectation === 'accept') {
    assert.equal(result.status, 0, `${name}: ${result.stderr}`);
    const page = await readFile(pagePath);
    const rawReceipt = await readFile(receiptPath);
    const receipt = JSON.parse(rawReceipt);
    assert.equal(page.length, pageExpected.bytes, `${name}: page length`);
    assert.equal(sha256(page), pageExpected.sha256, `${name}: qualified page bytes`);
    assert.equal(receipt.page.bytes, page.length);
    assert.equal(receipt.page.sha256, sha256(page));
    assert.equal(receipt.inputUiBlobs['packages/uma-sim-ui/package.json'], gitBlob(actualPackage), `${name}: raw package provenance`);
    assert.equal(receipt.nonScriptPackageSHA, fieldsSha256, `${name}: qualified manifest fields`);
    assert.deepEqual(receipt.ignoredPackageFields, ['scripts']);
    assert.equal(receipt.builderSha256, builderSha256);
    assert.equal(receipt.typecheck, 'passed');
    assert.equal(receipt.selfContained, true);
    const pinnedInputs = inputs.filter(x => pinnedUiPaths.includes(x.path));
    const expectedUiBlobs = Object.fromEntries(pinnedInputs.map(x => [x.path, x.gitBlob]));
    expectedUiBlobs['packages/uma-sim-ui/package.json'] = gitBlob(actualPackage);
    assert.deepEqual(receipt.inputUiBlobs, expectedUiBlobs);
    await writeFile(join(output, `${name}.page.html`), page, { flag: 'wx' });
    await writeFile(join(output, `${name}.build-receipt.json`), rawReceipt, { flag: 'wx' });
    record.pageSha256 = sha256(page);
    record.receiptSha256 = sha256(rawReceipt);
  } else {
    assert.notEqual(result.status, 0, `${name}: changed input was accepted`);
    assert.match(result.stderr, expectedError, `${name}: failure must identify the intended boundary`);
    assert.deepEqual({ page: sha256(await readFile(pagePath)), receipt: sha256(await readFile(receiptPath)) }, before, `${name}: rejection must retain prior output bytes`);
    record.outputUnchanged = true;
    record.rejection = expectedError.source;
  }
  cases.push(record);
}

try {
  await invoke('current-package', 'accept');
  const scriptOnly = structuredClone(originalPackage);
  scriptOnly.scripts = {
    test: 'node -e "process.exit(73)"', build: 'node -e "process.exit(73)"',
    typecheck: 'node -e "process.exit(73)"', prepare: 'node -e "process.exit(73)"',
    'receiving-added-script': 'node -e "process.exit(73)"',
  };
  await writeFile(packagePath, JSON.stringify(scriptOnly, null, 2) + '\n');
  assert.notEqual(gitBlob(await readFile(packagePath)), gitBlob(packageBytes));
  await invoke('script-only', 'accept');
  const packageCases = [
    ['dependency-version', p => { p.dependencies.react = '^19.0.0'; }],
    ['dependency-value-type', p => { p.dependencies.react = 18; }],
    ['dependency-container-type', p => { p.dependencies = []; }],
    ['added-non-script-field', p => { p.engines = { node: '>=999.0.0' }; }],
    ['deleted-non-script-field', p => { delete p.license; }],
    ['changed-non-script-type', p => { p.private = 'true'; }],
  ];
  for (const [name, change] of packageCases) {
    const altered = structuredClone(originalPackage);
    change(altered);
    await writeFile(packagePath, JSON.stringify(altered, null, 2) + '\n');
    await invoke(name, 'reject', /Pinned UI package fields changed \(only scripts may differ\)/);
  }
  await writeFile(packagePath, packageBytes);
  for (const [name, path] of [
    ['exact-lock-pin', join(ui, 'package-lock.json')],
    ['consumed-source-pin', join(ui, 'src/components/StatsPanel.tsx')],
    ['compiler-options-pin', join(ui, 'tsconfig.json')],
  ]) {
    const original = await readFile(path);
    try {
      await writeFile(path, Buffer.concat([original, Buffer.from('\n')]));
      await invoke(name, 'reject', /Pinned UI source changed:/);
    } finally {
      await writeFile(path, original);
    }
  }
  for (const input of inputs) {
    assert.equal(sha256(await readFile(join(source, input.path))), input.sha256, `source input changed: ${input.path}`);
    assert.equal(sha256(await readFile(join(trial, input.path))), input.sha256, `trial input not restored: ${input.path}`);
  }
  const receipt = {
    schema: 'uma.training-demo.build-boundary-receiving.v1', startedAt, completedAt: new Date().toISOString(),
    source, toolchain, output, node: process.version, platform: process.platform, arch: process.arch,
    builderSha256, page: pageExpected, fieldsSha256, inputs, cases,
    groups: cases.length, passed: cases.length, sourceInputsUnchanged: true, trialInputsRestored: true,
    runtimeOrBrowserRetested: false,
  };
  await writeFile(join(output, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ result: 'PASS', groups: cases.length, output, builderSha256, page: pageExpected }, null, 2));
} catch (error) {
  await writeFile(join(output, 'failure.json'), JSON.stringify({ startedAt, failedAt: new Date().toISOString(), source, toolchain, builderSha256, completedCases: cases, error: { name: error.name, message: error.message, stack: error.stack } }, null, 2) + '\n', { flag: 'wx' });
  throw error;
} finally {
  await writeFile(packagePath, packageBytes);
}
