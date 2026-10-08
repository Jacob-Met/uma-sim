// SPDX-License-Identifier: GPL-3.0-only
// Full built App receiving against its exact native API, with isolated state.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';

const [root, binary, out] = process.argv.slice(2).map(value => path.resolve(value));
const require = createRequire(path.join(root, 'packages/uma-sim-ui/package.json'));
const { chromium } = require('playwright');
const dist = path.join(root, 'packages/uma-sim-ui/dist');
await fs.mkdir(out, { recursive: true });
const stateDir = await fs.mkdtemp(path.join(process.env.TMPDIR, 'uma-race-receiving-'));
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const paths = ['packages/uma-sim-ui/src/App.tsx', 'packages/uma-sim-ui/src/components/RacePanel.tsx',
  'packages/uma-sim-ui/src/components/raceResults.ts', 'packages/uma-sim-ui/src/components/race-results.css',
  'packages/uma-sim-ui/src/state/runStore.ts', 'packages/uma-sim-ui/src/api/client.ts',
  'uma-sim-core/src/api.rs', 'uma-sim-core/src/engine.rs', 'uma-sim-core/src/race.rs'];
async function sourcePins() {
  return Object.fromEntries(await Promise.all(paths.map(async name => [name, digest(await fs.readFile(path.join(root, name)))])));
}
const beforePins = await sourcePins();
let checkoutIdentity;
try {
  checkoutIdentity = { head: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    workingTreeStatus: execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim() };
} catch { checkoutIdentity = { head: null, note: 'Git identity unavailable; exact tested source hashes are recorded below.' }; }
const report = { status: 'running', checkoutIdentity,
  qualifiedProductionFreeze: 'ab287fed5e5f30b2b999c31c054b234ca1211461',
  qualifiedNativeBaseline: '4dc00f155d29fb37887524455f0627399191c915', binarySha256: digest(await fs.readFile(binary)),
  sourceBefore: beforePins, checks: [], uiRequests: [], driverRequests: [], blockedExternalUrls: [], browserExceptions: [] };
let apiProcess, server, browser, page;
let apiOutput = '', releaseAction, holdNextAction = false;
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
async function api(route) {
  const response = await fetch(apiUrl + route, { signal: AbortSignal.timeout(20000) });
  const value = await response.json();
  report.driverRequests.push({ route, status: response.status });
  assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`);
  return value;
}
const raceLines = snapshot => snapshot.state.log.filter(line => /^Race(?:\s|$)/u.test(line));
async function settle() {
  await page.waitForFunction(() => !document.querySelector('.busy-overlay') && !document.querySelector('.banner.error'));
  await page.waitForTimeout(35);
}
async function ordinaryStep() {
  await settle();
  if (await page.locator('.modal button').count()) await page.locator('.modal button').first().click();
  else await page.getByRole('button', { name: 'Auto step', exact: true }).click();
  await settle();
}
async function assertLatest(line) {
  const latest = page.locator('.race-latest');
  assert.equal(await latest.locator('.race-record pre').textContent(), line);
  const match = /^Race (\S+) (\d+\w+) \+(\d+) fans \[physics t=([\d.]+)s course=\d+ seed=\d+ field=(\d+) /.exec(line);
  assert.ok(match, 'This helper compares actual physics fields independently of the product parser');
  assert.equal(await latest.locator('h3').textContent(), match[1]);
  assert.deepEqual(await latest.locator('dd').allTextContents(), [match[2], match[5], match[4] + ' s', '+' + match[3]]);
}
async function startRun(model = 'physics', seed = '29') {
  const start = page.getByRole('button', { name: 'Start run', exact: true });
  await start.waitFor();
  await page.locator('.field').filter({ has: page.locator('label', { hasText: /^Seed$/ }) }).locator('input').fill(seed);
  await page.locator('.field').filter({ has: page.locator('label', { hasText: /^Race model$/ }) }).locator('select').selectOption(model);
  await start.click(); await page.locator('.turn-layout').waitFor(); await settle();
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
      const input = []; for await (const chunk of request) input.push(chunk);
      const body = Buffer.concat(input);
      const entry = { path: request.url, method: request.method, ...(body.length ? { body: JSON.parse(body) } : {}) };
      report.uiRequests.push(entry);
      const hold = holdNextAction && request.method === 'POST' && request.url === '/v1/run/action';
      if (hold) holdNextAction = false;
      const upstream = http.request(apiUrl + request.url, { method: request.method, headers: request.headers }, async res => {
        entry.status = res.statusCode;
        const chunks = []; for await (const chunk of res) chunks.push(chunk);
        if (hold) await new Promise(resolve => { releaseAction = resolve; });
        response.writeHead(res.statusCode, res.headers); response.end(Buffer.concat(chunks));
      });
      upstream.on('error', error => { entry.error = String(error); response.writeHead(502); response.end(); });
      upstream.end(body); return;
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
    report.blockedExternalUrls.push(route.request().url()); return route.abort();
  });
  page = await context.newPage();
  page.on('pageerror', error => report.browserExceptions.push(String(error)));
  await page.goto(origin); await startRun();
  let snapshot = await api('/v1/run/state');
  assert.equal(snapshot.state.phase, 'MANDATORY_RACE');
  assert.equal(await page.locator('.race-panel .race-note').textContent(), 'No race result recorded yet.');
  const enter = page.getByRole('button', { name: 'Enter race', exact: true });
  holdNextAction = true;
  await enter.click();
  await page.waitForFunction(() => document.querySelector('.race-pending button')?.disabled === true);
  for (let attempt = 0; attempt < 100 && !releaseAction; attempt++) await pause(20);
  assert.ok(releaseAction, 'Held only the real native action response for the busy check');
  assert.equal(await enter.isDisabled(), true);
  assert.equal(report.uiRequests.filter(request => request.path === '/v1/run/action').length, 1);
  releaseAction(); releaseAction = undefined; await settle();
  snapshot = await api('/v1/run/state');
  await assertLatest(raceLines(snapshot).at(-1));
  report.firstPhysics = snapshot;
  report.checks.push('Real mandatory race action is busy-disabled and displays exact native physics fields and original text');

  const transitions = [];
  for (let count = 0; count < 100 && raceLines(snapshot).length < 3; count++) {
    const choices = (await api('/v1/run/choices')).choices;
    const raceIndex = choices.findIndex(choice => choice.id === 'race');
    if (await page.locator('.modal button').count()) {
      transitions.push({ turn: snapshot.state.turn, action: 'first displayed event option' });
      await page.locator('.modal button').first().click();
    } else if (raceIndex >= 0) {
      transitions.push({ turn: snapshot.state.turn, action: 'offered race', id: choices[raceIndex].id });
      await page.locator('.choice-list button').nth(raceIndex).click();
    } else {
      transitions.push({ turn: snapshot.state.turn, action: 'Auto step' });
      await page.getByRole('button', { name: 'Auto step', exact: true }).click();
    }
    await settle(); snapshot = await api('/v1/run/state');
  }
  const nativeLines = raceLines(snapshot);
  assert.equal(nativeLines.length, 3);
  assert.ok(new Set(nativeLines.map(line => line.split(' ')[1])).size < nativeLines.length, 'Real repeated optional race occurrences');
  await assertLatest(nativeLines.at(-1));
  assert.equal(await page.locator('.race-history .race-result').count(), 2);
  assert.deepEqual(await page.locator('.race-history pre').allTextContents(), nativeLines.slice(0, -1).reverse());
  report.threeRaceSnapshot = snapshot; report.transitions = transitions;
  report.checks.push('Three real race occurrences, including repeated optional race IDs, retain exact ordered history');

  for (let count = 0; count < 10 && await page.locator('.modal button').count(); count++) await ordinaryStep();
  const offered = (await api('/v1/run/choices')).choices;
  const restIndex = offered.findIndex(choice => choice.id === 'rest');
  assert.ok(restIndex >= 0, 'A real ordinary rest must be offered');
  const beforeRest = await api('/v1/run/state');
  await page.locator('.choice-list button').nth(restIndex).click(); await settle();
  const afterRest = await api('/v1/run/state');
  assert.notDeepEqual(afterRest.state, beforeRest.state);
  assert.deepEqual(raceLines(afterRest), nativeLines);
  await assertLatest(nativeLines.at(-1));
  report.afterRest = afterRest;
  report.checks.push('An offered native rest changes the career while all recorded race results remain visible');

  const beforeDisclosure = await api('/v1/run/state');
  const writesBefore = report.uiRequests.filter(request => request.method !== 'GET').length;
  const original = page.locator('.race-latest > .race-result > .race-record > summary');
  await original.focus(); await page.keyboard.press('Enter');
  assert.equal(await page.locator('.race-latest .race-record').getAttribute('open'), '');
  const history = page.locator('.race-history > summary');
  await history.focus(); await page.keyboard.press('Space');
  assert.equal(await page.locator('.race-history').getAttribute('open'), '');
  const olderOriginal = page.locator('.race-history .race-record > summary').first();
  await olderOriginal.focus(); await page.keyboard.press('Enter');
  assert.equal(await page.locator('.race-history .race-record').first().getAttribute('open'), '');
  assert.deepEqual(await api('/v1/run/state'), beforeDisclosure);
  assert.equal(report.uiRequests.filter(request => request.method !== 'GET').length, writesBefore);
  report.checks.push('Keyboard Enter/Space disclosures expose retained text without any action POST or native state mutation');
  await page.locator('.race-panel').screenshot({ path: path.join(out, 'race-results-desktop.png') });

  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('.race-panel').scrollIntoViewIfNeeded();
  const geometry = await page.locator('.race-panel').evaluate(panel => {
    const rect = panel.getBoundingClientRect();
    return { left: rect.left, right: rect.right, viewport: innerWidth, width: panel.clientWidth, scrollWidth: panel.scrollWidth,
      overflowingChildren: [...panel.querySelectorAll('*')].filter(element => element.clientWidth && element.scrollWidth > element.clientWidth + 1).map(element => element.tagName + '.' + element.className) };
  });
  assert.ok(geometry.left >= 0 && geometry.right <= geometry.viewport + 1);
  assert.ok(geometry.scrollWidth <= geometry.width + 1);
  assert.deepEqual(geometry.overflowingChildren, []);
  await assertLatest(nativeLines.at(-1));
  await page.locator('.race-panel').screenshot({ path: path.join(out, 'race-results-phone.png') });
  report.phoneGeometry = geometry;
  report.checks.push('At 390px the result, expanded history and exact original records fit without horizontal overflow');

  await page.getByRole('button', { name: 'New run', exact: true }).click();
  assert.equal(await page.locator('.race-panel').count(), 0);
  await startRun('stub', '29');
  assert.equal(await page.locator('.race-result').count(), 0);
  await page.getByRole('button', { name: 'Enter race', exact: true }).click(); await settle();
  const stub = await api('/v1/run/state'); const stubLine = raceLines(stub).at(-1);
  assert.match(stubLine, /^Race \S+ \+\d+ fans$/);
  assert.equal(await page.locator('.race-latest .race-result').getAttribute('data-race-kind'), 'fan-only');
  assert.equal(await page.locator('.race-latest pre').textContent(), stubLine);
  assert.deepEqual(await page.locator('.race-latest dt').allTextContents(), ['Fan gain']);
  assert.equal(await page.locator('.race-latest .race-note').textContent(), 'Place, finish time and field size were not recorded.');
  report.stubSnapshot = stub;
  report.checks.push('A real new stub career replaces physics history and shows only the fan gain actually recorded');
  await page.getByRole('button', { name: 'New run', exact: true }).click();
  await startRun('physics', '133');
  assert.equal(await page.locator('.race-result').count(), 0);
  assert.equal(await page.locator('.race-history').count(), 0);
  assert.equal(await page.locator('.race-panel .race-note').textContent(), 'No race result recorded yet.');
  report.checks.push('A subsequent empty career replaces all supplied results without retaining the prior run');
  assert.deepEqual(report.browserExceptions, []);
  report.sourceAfter = await sourcePins(); assert.deepEqual(report.sourceAfter, beforePins);
  report.status = 'passed';
} catch (error) {
  report.status = 'failed'; report.error = String(error.stack ?? error); process.exitCode = 1;
  if (page) await page.screenshot({ path: path.join(out, 'failure.png') }).catch(() => {});
} finally {
  if (releaseAction) releaseAction();
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
  if (apiProcess) { apiProcess.kill(); await new Promise(resolve => apiProcess.once('exit', resolve)); }
  await fs.writeFile(path.join(out, 'api.log'), apiOutput);
  await fs.writeFile(path.join(out, 'browser-receipt.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ status: report.status, checks: report.checks, error: report.error,
    uiRequests: report.uiRequests.length, blockedExternalRequests: report.blockedExternalUrls.length }));
}
