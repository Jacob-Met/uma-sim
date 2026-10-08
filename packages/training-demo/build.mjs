#!/usr/bin/env node
// Use the existing UI's locked toolchain; never modify that package or its cache.
import { readFile, writeFile, mkdir, mkdtemp, rm, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { dirname, join, resolve, relative, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const repository = resolve(root, '../..');
const ui = resolve(root, '../uma-sim-ui');
const args = process.argv.slice(2);
if (args.length && (args.length !== 2 || args[0] !== '--toolchain')) throw new Error('Usage: node build.mjs [--toolchain EXISTING_UI_PACKAGE]');
const toolchain = args.length ? resolve(args[1]) : ui;
const dependencies = join(toolchain, 'node_modules');
const require = createRequire(join(toolchain, 'package.json'));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const gitBlob = bytes => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
const sourcePins = {
  'packages/uma-sim-ui/index.html': '1711cea1cf433f0deb7d72b647e97c3906fe5353',
  'packages/uma-sim-ui/src/components/StatsPanel.tsx': '4602576f5358f2daefc811cc773eec637f55f430',
  'packages/uma-sim-ui/src/components/ChoicePanel.tsx': '21463054689cfc3c2c19258c4a98eb1e57b33c0e',
  'packages/uma-sim-ui/src/api/types.ts': '18ea43a689f67043b9cdc872c8178e16728f4de5',
  'packages/uma-sim-ui/src/styles/app.css': 'f28cbea6a9f577f6a5269341a7e5ad2a0eed1aa6',
  'packages/uma-sim-ui/package.json': 'db6b16c96113566f0b9e9ac1b415c57a38c705af',
  'packages/uma-sim-ui/package-lock.json': '10a4968db9d4507b9448dd00dba265eed483f419',
};
for (const [path, expected] of Object.entries(sourcePins)) {
  if (gitBlob(await readFile(join(repository, path))) !== expected) throw new Error(`Pinned UI source changed: ${path}`);
}
const fixtureBytes = await readFile(join(root, 'fixture.json'));
const fixtureSha256 = '906e6876f2485b012677ffda82295858ad5afda818c33b57a39fece9f992d9de';
if (sha256(fixtureBytes) !== fixtureSha256) throw new Error('The qualified native fixture changed.');
const fixture = JSON.parse(fixtureBytes);
if (fixture.schema !== 'uma.training-demo.v1' || fixture.source.commit !== '0bc58cb83e89f07c6af3056dd7c37436906d994f' || fixture.checkpoints.length !== 9 || fixture.checkpoints.some(checkpoint => checkpoint.outcomes.length !== 7)) throw new Error('Unexpected qualified fixture contract.');
const lock = JSON.parse(await readFile(join(ui, 'package-lock.json'), 'utf8'));
const versions = {};
for (const name of ['react', 'react-dom', 'vite', 'typescript', '@types/react', '@types/react-dom']) {
  const actual = JSON.parse(await readFile(join(dependencies, name, 'package.json'), 'utf8')).version;
  const expected = lock.packages[`node_modules/${name}`].version;
  if (actual !== expected) throw new Error(`Toolchain ${name} is ${actual}; the current UI lock requires ${expected}. Run npm ci in packages/uma-sim-ui.`);
  versions[name] = actual;
}

const ts = require('typescript');
const configuration = ts.parseJsonConfigFileContent(JSON.parse(await readFile(join(ui, 'tsconfig.json'), 'utf8')), ts.sys, ui);
const program = ts.createProgram([join(root, 'src/main.tsx')], {
  ...configuration.options, incremental: false, tsBuildInfoFile: undefined, noEmit: true,
  baseUrl: toolchain, typeRoots: [join(dependencies, '@types')],
  paths: {
    react: [join(dependencies, '@types/react/index.d.ts')],
    'react/*': [join(dependencies, '@types/react/*')],
    'react-dom/*': [join(dependencies, '@types/react-dom/*')],
  },
});
const diagnostics = ts.getPreEmitDiagnostics(program);
if (diagnostics.length) {
  throw new Error(ts.formatDiagnosticsWithColorAndContext(diagnostics, { getCanonicalFileName: name => name, getCurrentDirectory: () => root, getNewLine: () => '\n' }));
}

const temporary = await mkdtemp(join(root, '.build-'));
try {
  const vitePath = join(dirname(require.resolve('vite/package.json')), 'dist/node/index.js');
  const { build } = await import(pathToFileURL(vitePath).href);
  await build({
    root, configFile: false, base: './', logLevel: 'warn', cacheDir: join(temporary, 'cache'),
    resolve: { alias: [
      { find: 'react-dom', replacement: join(dependencies, 'react-dom') },
      { find: 'react', replacement: join(dependencies, 'react') },
    ] },
    esbuild: { jsx: 'automatic', jsxImportSource: 'react', legalComments: 'inline' },
    build: {
      outDir: join(temporary, 'bundle'), emptyOutDir: true, cssCodeSplit: false,
      assetsInlineLimit: Number.MAX_SAFE_INTEGER, reportCompressedSize: false,
      rollupOptions: { output: { inlineDynamicImports: true } },
    },
  });
  const bundle = join(temporary, 'bundle');
  let html = await readFile(join(bundle, 'index.html'), 'utf8');
  const scriptTags = [...html.matchAll(/<script\b[^>]*\bsrc="([^"]+)"[^>]*><\/script>/g)];
  const styleTags = [...html.matchAll(/<link\b[^>]*\bhref="([^"]+)"[^>]*>/g)];
  if (scriptTags.length !== 1 || styleTags.length !== 1 || !styleTags[0][0].includes('rel="stylesheet"')) throw new Error('Expected one compiled script and one stylesheet.');
  function assetPath(path) {
    const absolute = resolve(bundle, path);
    const rel = relative(bundle, absolute);
    if (!path.startsWith('./assets/') || rel.startsWith(`..${sep}`) || rel === '..') throw new Error('Compiled asset left the isolated bundle.');
    return absolute;
  }
  const script = await readFile(assetPath(scriptTags[0][1]), 'utf8');
  const style = await readFile(assetPath(styleTags[0][1]), 'utf8');
  if (/<\/style/i.test(style) || /@import\b|url\s*\(/i.test(style)) throw new Error('The standalone stylesheet contains an external resource or closing tag.');
  html = html.replace(scriptTags[0][0], () => `<script type="module">${script.replace(/<\/script/gi, '<\\/script')}</script>`);
  html = html.replace(styleTags[0][0], () => `<style>${style}</style>`);
  if (/<script\b[^>]*\bsrc=|<link\b[^>]*\bhref=/i.test(html) || !html.includes('<!-- hamon-demo:uma-training -->')) throw new Error('Standalone page has a missing marker or unresolved asset.');
  const files = await readdir(join(bundle, 'assets'));
  if (files.length !== 2) throw new Error('Unexpected additional bundle artifacts.');
  const output = join(root, 'out');
  await mkdir(output, { recursive: true });
  await writeFile(join(output, 'index.html'), html);
  const receipt = {
    schema: 'uma.training-demo.build.v1',
    sourceCommit: 'fe2e6fdcdf7625cb5beac0e0062ace0360288a02',
    sourceTree: '6ee572f7ef9a3623a8f2bd192235787fb9fbecb9',
    nativeFixtureSource: { commit: fixture.source.commit, tree: fixture.source.tree },
    inputUiBlobs: sourcePins, fixtureSha256, toolchain: versions,
    builderSha256: sha256(await readFile(fileURLToPath(import.meta.url))),
    page: { path: 'out/index.html', bytes: Buffer.byteLength(html), sha256: sha256(html) },
    selfContained: true, typecheck: 'passed',
  };
  await writeFile(join(output, 'build-receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`);
  console.log(JSON.stringify(receipt, null, 2));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
