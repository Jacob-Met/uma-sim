// SPDX-License-Identifier: GPL-3.0-only
// Exact-baseline witness using the native API and production browser bundle.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const [root, binary, out] = process.argv.slice(2).map(value => path.resolve(value));
const require = createRequire(path.join(root, 'packages/uma-sim-ui/package.json'));
const { chromium } = require('playwright');
const ts = require('typescript');
const dist = path.join(root, 'packages/uma-sim-ui/dist');
await fs.mkdir(out, { recursive: true });
const stateDir = await fs.mkdtemp(path.join(process.env.TMPDIR, 'uma-race-baseline-'));
const calls = [], blocked = [], exceptions = [];
let apiProcess, server, browser;
let apiOutput = '';
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const paths = ['packages/uma-sim-ui/src/components/RacePanel.tsx', 'packages/uma-sim-ui/src/App.tsx',
  'uma-sim-core/src/api.rs', 'uma-sim-core/src/engine.rs', 'uma-sim-core/src/race.rs'];
const pins = Object.fromEntries(await Promise.all(paths.map(async name => [name, digest(await fs.readFile(path.join(root, name)))])));
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function freePort() {
  const reservation = http.createServer();
  await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise(resolve => reservation.close(resolve));
  return port;
}
const apiPort = await freePort();
const apiUrl = `http://127.0.0.1:${apiPort}`;
async function api(route, body) {
  const response = await fetch(apiUrl + route, { method: body ? 'POST' : 'GET',
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined, signal: AbortSignal.timeout(20000) });
  const value = await response.json();
  calls.push({ route, method: body ? 'POST' : 'GET', ...(body ? { body } : {}), status: response.status });
  assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`);
  return value;
}
try {
  apiProcess = spawn(binary, [String(apiPort)], { cwd: stateDir,
    env: { PATH: process.env.PATH, TMPDIR: stateDir, UMA_REPO_ROOT: root }, stdio: ['ignore', 'pipe', 'pipe'] });
  apiProcess.stdout.on('data', bytes => { apiOutput += bytes; });
  apiProcess.stderr.on('data', bytes => { apiOutput += bytes; });
  for (let attempt = 0; attempt < 100; attempt++) {
    try { await api('/v1/health'); break; }
    catch (error) { if (apiProcess.exitCode !== null || attempt === 99) throw error; await pause(100); }
  }
  server = http.createServer(async (request, response) => {
    if (request.url.startsWith('/v1/')) {
      const upstream = http.request(apiUrl + request.url, { method: request.method, headers: request.headers }, res => {
        response.writeHead(res.statusCode, res.headers); res.pipe(response);
      });
      upstream.on('error', () => { response.writeHead(502); response.end(); });
      request.pipe(upstream); return;
    }
    try {
      const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      const file = path.resolve(dist, '.' + (pathname === '/' ? '/index.html' : pathname));
      assert.ok(file.startsWith(dist + path.sep));
      const bytes = await fs.readFile(file);
      response.writeHead(200, { 'content-type': ({ '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' })[path.extname(file)] ?? 'application/octet-stream' });
      response.end(bytes);
    } catch { response.writeHead(404); response.end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ executablePath: process.env.UMA_RACE_CHROMIUM,
    headless: true, args: ['--no-sandbox', '--disable-background-networking', '--disable-component-update', '--disable-sync'] });
  const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  await context.route('**/*', route => {
    if (route.request().url().startsWith(origin + '/')) return route.continue();
    blocked.push(route.request().url()); return route.abort();
  });
  const page = await context.newPage();
  page.on('pageerror', error => exceptions.push(String(error)));
  await page.goto(origin);
  const start = page.getByRole('button', { name: 'Start run', exact: true });
  await start.waitFor();
  await page.locator('.field').filter({ has: page.locator('label', { hasText: /^Seed$/ }) }).locator('input').fill('29');
  await start.click();
  await page.locator('.turn-layout').waitFor();
  async function settle() {
    await page.waitForFunction(() => !document.querySelector('.busy-overlay') && !document.querySelector('.banner.error'));
    await page.waitForTimeout(30);
  }
  let snapshot;
  const transitions = [];
  for (let count = 0; count < 160; count++) {
    await settle();
    snapshot = await api('/v1/run/state');
    if (snapshot.state.log.some(line => line.startsWith('Race '))) break;
    let action;
    if (await page.locator('.modal button').count()) {
      action = 'first displayed event option'; await page.locator('.modal button').first().click();
    } else if (await page.getByRole('button', { name: 'Enter race', exact: true }).count()) {
      action = 'mandatory Enter race'; await page.getByRole('button', { name: 'Enter race', exact: true }).click();
    } else {
      action = 'Auto step'; await page.getByRole('button', { name: 'Auto step', exact: true }).click();
    }
    transitions.push({ turn: snapshot.state.turn, phase: snapshot.state.phase, action });
  }
  await settle();
  const physicsLines = snapshot.state.log.filter(line => line.startsWith('Race '));
  assert.ok(physicsLines.some(line => line.includes('[physics ')), 'The real browser career must complete a physics race');
  const text = (await api('/v1/run/text')).text;
  assert.ok(physicsLines.every(line => !text.includes(line)), 'Current-state rendering does not contain retained outcomes');
  const visibleRaceCards = await page.getByRole('heading', { name: 'Race', exact: true }).count();
  assert.equal(visibleRaceCards, 0, 'Baseline loses the race panel after the actual recorded race');
  await page.screenshot({ path: path.join(out, 'baseline-after-race.png') });
  const source = await fs.readFile(path.join(root, paths[0]), 'utf8');
  const transpiled = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }).outputText;
  const moduleDir = path.join(stateDir, 'parser'); await fs.mkdir(moduleDir);
  await fs.symlink(path.join(root, 'packages/uma-sim-ui/node_modules'), path.join(moduleDir, 'node_modules'), 'dir');
  await fs.writeFile(path.join(moduleDir, 'RacePanel.cjs'), transpiled);
  const { parseRaceResult } = require(path.join(moduleDir, 'RacePanel.cjs'));
  assert.equal(parseRaceResult(physicsLines), null, 'Even correct native physics input is not parsed by the baseline');
  let stub = await api('/v1/run/start', { seed: 29, scenario: 'ura', raceModel: 'stub', dialogue: 'off' });
  for (let count = 0; count < 160 && !stub.state.log.some(line => line.startsWith('Race ')); count++) {
    const result = await api('/v1/run/auto', { policy: 'bot' }); stub = result.state;
  }
  const stubLines = stub.state.log.filter(line => line.startsWith('Race '));
  assert.ok(stubLines.length > 0);
  const parsedStub = parseRaceResult(stubLines);
  assert.equal(parsedStub.place, '1', 'Baseline infers an unrecorded place from a stub line');
  assert.deepEqual(exceptions, []);
  const afterPins = Object.fromEntries(await Promise.all(paths.map(async name => [name, digest(await fs.readFile(path.join(root, name)))])));
  assert.deepEqual(afterPins, pins);
  await fs.writeFile(path.join(out, 'baseline-native-snapshot.json'), JSON.stringify(snapshot, null, 2) + '\n');
  await fs.writeFile(path.join(out, 'baseline-stub-snapshot.json'), JSON.stringify(stub, null, 2) + '\n');
  await fs.writeFile(path.join(out, 'baseline-witness.json'), JSON.stringify({ status: 'baseline-defect-reproduced',
    source: '4dc00f155d29fb37887524455f0627399191c915', sourceSha256: pins, binarySha256: digest(await fs.readFile(binary)),
    seed: 29, physicsLines, stubLines, visibleRaceCards, physicsParse: null, parsedStub, currentStateText: text,
    transitions, calls, blockedCatalogImages: blocked, exceptions,
    limits: 'Disposable loopback Rust API and fresh browser; no external requests allowed. Missing-place behavior describes the retained record, not the internal stub outcome.' }, null, 2) + '\n');
  console.log(JSON.stringify({ status: 'baseline-defect-reproduced', physicsLines, stubLines, visibleRaceCards, nativePhysicsParse: null, stubInferredPlace: parsedStub.place, transitions: transitions.length }));
} finally {
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
  if (apiProcess) { apiProcess.kill(); await new Promise(resolve => apiProcess.once('exit', resolve)); }
  await fs.writeFile(path.join(out, 'api.log'), apiOutput);
}
