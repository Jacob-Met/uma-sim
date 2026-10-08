import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Run the unchanged receiver against a built baseline or candidate:
// node uma-lab-receiver.mjs DIST OUTPUT [CHROMIUM_EXECUTABLE]
const [distArg, outArg, chromiumExecutable] = process.argv.slice(2);
assert(distArg && outArg, 'Usage: node uma-lab-receiver.mjs DIST OUTPUT [CHROMIUM_EXECUTABLE]');
const dist = path.resolve(distArg);
const output = path.resolve(outArg);
await fs.mkdir(output, { recursive: true });
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright', {
  paths: [process.cwd(), process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES].filter(Boolean),
}));

const names = { alpha: 'Alpha', beta: 'Beta', gamma: 'Gamma', delta: 'Delta', 'path/space &é': 'Encoded' };
const outcome = {
  steps: 1, finalTurn: 9, careerComplete: false, completedRaces: [],
  stats: { speed: 100, stamina: 90, power: 80, guts: 70, wit: 60 },
  energy: 50, mood: 'NORMAL', fans: 100, skillPoints: 30,
  learnedSkills: [], sparks: [], scenarioResources: {}, totalRngCalls: 12, telemetryRecords: 1,
};
function comparison(a, b) {
  return {
    aId: a, aName: names[a], bId: b, bName: names[b], checkpointName: 'checkpoint',
    checkpointTurn: 8, seed: 412, sameCheckpoint: true, comparedAt: '2026-10-08T00:00:00Z',
    firstDivergence: null, aligned: [], outcomeA: outcome, outcomeB: outcome,
    caveats: ['Independent HTTP receiver fixture; not a simulation result.'],
  };
}
function newFixture() {
  return {
    branches: Object.entries(names).map(([id, name]) => ({
      id, name, checkpointName: 'checkpoint', seed: 412, scenarioId: 'ura',
      traineeName: 'Receiver', policy: 'bot', steps: 1, careerComplete: false,
      finalTurn: 9, fans: 100, startedAt: '2026-10-08T00:00:00Z',
    })),
    calls: [], failures: [], compareGate: null, failComparison: false, failDelete: false,
    failListAfterDelete: false, deletionConfirmed: false,
  };
}
let fixture = newFixture();
function json(res, body, status = 200) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}
const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://127.0.0.1');
    if (url.pathname.startsWith('/v1/')) {
      const current = fixture;
      let raw = '';
      for await (const chunk of req) raw += chunk;
      const body = raw ? JSON.parse(raw) : undefined;
      current.calls.push({ method: req.method, path: url.pathname, query: Object.fromEntries(url.searchParams), body });
      if (url.pathname === '/v1/health') return json(res, { ok: true, version: 'receiver-fixture', repoRoot: true });
      if (url.pathname.startsWith('/v1/catalog/')) return json(res, { items: [] });
      if (url.pathname === '/v1/sessions') return json(res, { sessions: [], active: '' });
      if (url.pathname === '/v1/library') return json(res, { entries: [{
        name: 'checkpoint', label: 'Checkpoint', note: '', seed: 412, scenarioId: 'ura',
        traineeName: 'Receiver', turn: 8, phase: 'training', dateLabel: 'Junior January',
        rngCalls: 11, savedAt: '2026-10-08T00:00:00Z', savedAtUnix: 0,
        fingerprint: { coreVersion: 'fixture', snapshotSchema: 1, eventCatalogCount: 0,
          contentPackEvents: 0, repoRootDetected: true },
      }] });
      if (url.pathname === '/v1/lab/branches') {
        if (current.failListAfterDelete && current.deletionConfirmed)
          return json(res, { error: 'Receiver fixture branch list refresh failure' }, 503);
        return json(res, { branches: current.branches });
      }
      if (url.pathname === '/v1/lab/branch/delete') {
        if (current.failDelete) return json(res, { error: 'Receiver fixture deletion failure' }, 503);
        current.branches = current.branches.filter((x) => x.id !== body.id);
        current.deletionConfirmed = true;
        return json(res, { deleted: body.id });
      }
      if (url.pathname === '/v1/lab/compare') {
        if (current.compareGate) await current.compareGate;
        if (current.failComparison) return json(res, { error: 'Receiver fixture comparison failure' }, 503);
        return json(res, comparison(body.a, body.b));
      }
      if (url.pathname === '/v1/lab/branch') return json(res, { branch: current.branches[0], outcome });
      if (url.pathname === '/v1/lab/report') {
        const c = comparison(url.searchParams.get('a'), url.searchParams.get('b'));
        const format = url.searchParams.get('format');
        const text = format === 'json' ? JSON.stringify(c, null, 2)
          : `# ${c.aName} vs ${c.bName}\n\nBranch A: ${c.aId}\nBranch B: ${c.bId}\n`;
        res.writeHead(200, {
          'Content-Type': format === 'json' ? 'application/json' : 'text/markdown',
          'Content-Disposition': `attachment; filename="comparison.${format === 'json' ? 'json' : 'md'}"`,
          'Content-Length': Buffer.byteLength(text),
        });
        return res.end(text);
      }
      current.failures.push(`Unexpected ${req.method} ${url.pathname}`);
      return json(res, { error: 'Unexpected fixture route' }, 404);
    }
    const relative = url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname.slice(1));
    const target = path.resolve(dist, relative);
    if (target !== dist && !target.startsWith(dist + path.sep)) return json(res, { error: 'Outside fixture root' }, 400);
    const bytes = await fs.readFile(target);
    const type = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' }[path.extname(target)] || 'application/octet-stream';
    res.writeHead(200, { 'Content-Type': type });
    res.end(bytes);
  } catch (error) {
    json(res, { error: String(error) }, 500);
  }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({
  headless: true, executablePath: chromiumExecutable,
  args: ['--no-sandbox', '--disable-dev-shm-usage'],
});
const results = [];
const browserVersion = browser.version();
async function ready(page) {
  await page.goto(origin);
  await page.getByRole('button', { name: 'Library & lab', exact: true }).click();
  await page.getByLabel('Compare A: Alpha', { exact: true }).waitFor();
  await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
}
async function select(page, a, b) {
  await page.getByLabel(`Compare A: ${names[a]}`, { exact: true }).check();
  await page.getByLabel(`Compare B: ${names[b]}`, { exact: true }).check();
}
async function compare(page, a = 'alpha', b = 'beta') {
  await select(page, a, b);
  await page.getByRole('button', { name: 'Compare selected branches', exact: true }).click();
  await page.getByRole('heading', { name: `${names[a]} vs ${names[b]}`, exact: true }).waitFor();
  await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
}
async function download(page, label) {
  const pending = page.waitForEvent('download');
  await page.getByRole('link', { name: label, exact: true }).click();
  const downloaded = await pending;
  const failure = await downloaded.failure();
  assert.equal(failure, null, `Actual browser download failed: ${failure}`);
  const filename = await downloaded.path();
  assert(filename, 'Browser did not materialize the downloaded report');
  return fs.readFile(filename, 'utf8');
}
async function reportsMatch(page, a, b, allowClear = false) {
  const heading = page.getByRole('heading', { name: `${names[a]} vs ${names[b]}`, exact: true });
  if (allowClear && await heading.count() === 0) {
    assert.equal(await page.getByRole('link', { name: /Download .* report/ }).count(), 0);
    return { result: 'cleared-together' };
  }
  assert.equal(await heading.count(), 1, `Expected the successful ${a}/${b} comparison to remain visible`);
  const md = await download(page, 'Download markdown report');
  const jr = JSON.parse(await download(page, 'Download JSON report'));
  const actual = { json: [jr.aId, jr.bId], markdown: md.trim().split('\n'), display: await heading.textContent() };
  assert.deepEqual([jr.aId, jr.bId], [a, b], `Downloaded JSON identity differs from the displayed result: ${JSON.stringify(actual)}`);
  assert(md.includes(`Branch A: ${a}\n`) && (md.includes(`Branch B: ${b}\n`) || md.endsWith(`Branch B: ${b}`)), `Markdown identity differs from display: ${JSON.stringify(actual)}`);
  return actual;
}
async function deleteBranch(page, id) {
  const row = page.getByRole('row').filter({ has: page.getByLabel(`Compare A: ${names[id]}`, { exact: true }) });
  page.once('dialog', (dialog) => dialog.accept());
  await row.getByRole('button', { name: 'Delete', exact: true }).click();
  await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
}
async function runCase(name, fn) {
  fixture = newFixture();
  const context = await browser.newContext({ acceptDownloads: true, viewport: { width: 1280, height: 1000 } });
  const page = await context.newPage();
  page.setDefaultTimeout(3500);
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  let detail;
  try {
    await ready(page);
    detail = await fn(page, fixture);
    assert.deepEqual(errors, [], 'Uncaught browser errors');
    assert.deepEqual(fixture.failures, [], 'Unexpected receiving API calls');
    results.push({ name, passed: true, detail, requests: fixture.calls.filter((x) => x.method !== 'GET' || x.path === '/v1/lab/report') });
  } catch (error) {
    const entry = { name, passed: false, error: error.message, errors,
      requests: fixture.calls.filter((x) => x.method !== 'GET' || x.path === '/v1/lab/report') };
    if (results.filter((x) => !x.passed).length < 3) {
      const filename = `failure-${name.replace(/[^a-z0-9]+/gi, '-')}.png`;
      await page.screenshot({ path: path.join(output, filename), fullPage: true }).catch(() => {});
      entry.screenshot = filename;
    }
    results.push(entry);
  } finally {
    await context.close();
  }
  console.log(`${results.at(-1).passed ? 'PASS' : 'FAIL'} ${name}${results.at(-1).passed ? '' : ': ' + results.at(-1).error}`);
}

try {
  await runCase('successful comparison actual markdown and JSON downloads', async (page) => {
    await compare(page);
    return reportsMatch(page, 'alpha', 'beta');
  });
  await runCase('changed selection cannot retarget displayed comparison downloads', async (page) => {
    await compare(page);
    await select(page, 'gamma', 'delta');
    return reportsMatch(page, 'alpha', 'beta', true);
  });
  await runCase('failed new comparison keeps prior result and downloads coherent', async (page, f) => {
    await compare(page);
    await select(page, 'gamma', 'delta');
    f.failComparison = true;
    await page.getByRole('button', { name: 'Compare selected branches', exact: true }).click();
    await page.locator('.banner.error').waitFor();
    return reportsMatch(page, 'alpha', 'beta', true);
  });
  await runCase('delete displayed branch after selection changes invalidates result', async (page) => {
    await compare(page);
    await select(page, 'gamma', 'delta');
    await deleteBranch(page, 'alpha');
    assert.equal(await page.getByRole('heading', { name: 'Alpha vs Beta', exact: true }).count(), 0);
    assert.equal(await page.getByRole('link', { name: /Download .* report/ }).count(), 0);
    assert(await page.getByLabel('Compare A: Gamma', { exact: true }).isChecked());
    assert(await page.getByLabel('Compare B: Delta', { exact: true }).isChecked());
  });
  await runCase('delete unrelated pending branch preserves successful comparison', async (page) => {
    await compare(page);
    await select(page, 'gamma', 'delta');
    await deleteBranch(page, 'gamma');
    assert(await page.getByRole('button', { name: 'Compare selected branches', exact: true }).isDisabled(), 'Deleted pending A selection must be cleared');
    return reportsMatch(page, 'alpha', 'beta');
  });
  await runCase('deleting selected compared branch requires replacement selection', async (page) => {
    await compare(page);
    await deleteBranch(page, 'alpha');
    assert(await page.getByRole('button', { name: 'Compare selected branches', exact: true }).isDisabled(), 'Deleted A selection still enables compare');
    assert.equal(await page.getByRole('link', { name: /Download .* report/ }).count(), 0);
  });
  await runCase('failed branch deletion preserves valid comparison and selected row', async (page, f) => {
    await compare(page);
    f.failDelete = true;
    await deleteBranch(page, 'alpha');
    await page.locator('.banner.error').waitFor();
    assert(await page.getByLabel('Compare A: Alpha', { exact: true }).isChecked());
    return reportsMatch(page, 'alpha', 'beta');
  });
  await runCase('confirmed deletion invalidates result when branch list refresh fails', async (page, f) => {
    await compare(page);
    f.failListAfterDelete = true;
    await deleteBranch(page, 'alpha');
    assert(f.deletionConfirmed, 'Fixture did not confirm the deletion');
    assert.equal(await page.getByRole('heading', { name: 'Alpha vs Beta', exact: true }).count(), 0,
      'A confirmed deletion must clear its comparison before a fallible refresh');
    assert.equal(await page.getByRole('link', { name: /Download .* report/ }).count(), 0);
    assert(await page.getByRole('button', { name: 'Compare selected branches', exact: true }).isDisabled());
    await page.locator('.banner.error').waitFor();
    return { deletionConfirmed: true, error: await page.locator('.banner.error').textContent() };
  });
  await runCase('in flight comparison cannot overwrite newer keyboard selections', async (page, f) => {
    let release;
    f.compareGate = new Promise((resolve) => { release = resolve; });
    await select(page, 'alpha', 'beta');
    const requested = page.waitForRequest((req) => req.url().endsWith('/v1/lab/compare'));
    await page.getByRole('button', { name: 'Compare selected branches', exact: true }).click();
    await requested;
    try {
      await page.getByLabel('Compare A: Gamma', { exact: true }).focus();
      await page.keyboard.press('Space');
      await page.getByLabel('Compare B: Delta', { exact: true }).focus();
      await page.keyboard.press('Space');
      assert(await page.getByLabel('Compare A: Gamma', { exact: true }).isChecked());
      assert(await page.getByLabel('Compare B: Delta', { exact: true }).isChecked());
    } finally { release(); }
    await page.getByRole('heading', { name: 'Alpha vs Beta', exact: true }).waitFor();
    assert(await page.getByLabel('Compare A: Gamma', { exact: true }).isChecked(), 'Delayed response overwrote newer A selection');
    assert(await page.getByLabel('Compare B: Delta', { exact: true }).isChecked(), 'Delayed response overwrote newer B selection');
    return reportsMatch(page, 'alpha', 'beta');
  });
  await runCase('report query encoding preserves branch identity', async (page) => {
    await compare(page, 'path/space &é', 'beta');
    return reportsMatch(page, 'path/space &é', 'beta');
  });

  const invalidOverrides = [
    ['malformed nonblank line', '8:rest\nnot-an-override'],
    ['missing turn', ':rest'],
    ['fractional turn', '10.5:train_stamina'],
    ['negative turn', '-1:rest'],
    ['unsafe integer turn', '9007199254740992:rest'],
    ['missing action', '8:  '],
    ['conflicting duplicate turn', '8:rest\n8:train_speed'],
  ];
  for (const [label, text] of invalidOverrides) {
    await runCase(`invalid override ${label} blocks branch mutation`, async (page, f) => {
      await page.getByLabel(/^Checkpoint/).selectOption('checkpoint');
      await page.locator('textarea').fill(text);
      const submit = page.getByRole('button', { name: 'Run branch', exact: true });
      const submissionDisabled = await submit.isDisabled();
      if (!submissionDisabled) await submit.click();
      await page.waitForTimeout(120);
      const mutations = f.calls.filter((x) => x.method === 'POST' && x.path === '/v1/lab/branch');
      assert.equal(mutations.length, 0, `Invalid user experiment reached branch API: ${JSON.stringify(mutations)}`);
      const errors = await page.locator('.banner.error, [role="alert"]').allTextContents();
      assert(errors.some((x) => x.trim()), 'Invalid experiment had no visible actionable error');
      return { entered: text, submissionDisabled, errors };
    });
  }
  for (const value of ['', '0', '501', '1.5']) {
    await runCase(`invalid max actions ${value || 'empty'} blocks branch mutation`, async (page, f) => {
      await page.getByLabel(/^Checkpoint/).selectOption('checkpoint');
      await page.getByRole('spinbutton', { name: 'Max actions', exact: true }).fill(value);
      const submit = page.getByRole('button', { name: 'Run branch', exact: true });
      const submissionDisabled = await submit.isDisabled();
      if (!submissionDisabled) await submit.click();
      await page.waitForTimeout(120);
      const mutations = f.calls.filter((x) => x.method === 'POST' && x.path === '/v1/lab/branch');
      assert.equal(mutations.length, 0, `Invalid budget reached branch API: ${JSON.stringify(mutations)}`);
      const errors = await page.locator('.banner.error, [role="alert"]').allTextContents();
      assert(errors.some((x) => x.trim()), 'Invalid budget had no visible actionable error');
      return { entered: value, submissionDisabled, errors };
    });
  }
  await runCase('valid turn zero whitespace blank lines and colon action are transmitted exactly', async (page, f) => {
    await page.getByLabel(/^Checkpoint/).selectOption('checkpoint');
    await page.getByRole('spinbutton', { name: 'Max actions', exact: true }).fill('1');
    await page.locator('textarea').fill('  0:rest  \r\n\r\n  8 : custom:action  \n');
    const sent = page.waitForRequest((req) => req.url().endsWith('/v1/lab/branch') && req.method() === 'POST');
    await page.getByRole('button', { name: 'Run branch', exact: true }).click();
    await sent;
    const calls = f.calls.filter((x) => x.path === '/v1/lab/branch');
    assert.equal(calls.length, 1);
    assert.deepEqual(calls[0].body.overrides, [{ turn: 0, actionId: 'rest' }, { turn: 8, actionId: 'custom:action' }]);
    assert.equal(calls[0].body.maxActions, 1);
    return calls[0].body;
  });
  await runCase('valid upper action budget and empty overrides are transmitted exactly', async (page, f) => {
    await page.getByLabel(/^Checkpoint/).selectOption('checkpoint');
    await page.getByRole('spinbutton', { name: 'Max actions', exact: true }).fill('500');
    const sent = page.waitForRequest((req) => req.url().endsWith('/v1/lab/branch') && req.method() === 'POST');
    await page.getByRole('button', { name: 'Run branch', exact: true }).click();
    await sent;
    const calls = f.calls.filter((x) => x.path === '/v1/lab/branch');
    assert.equal(calls.length, 1);
    assert.deepEqual(calls[0].body.overrides, []);
    assert.equal(calls[0].body.maxActions, 500);
    return calls[0].body;
  });
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
  const scriptBytes = await fs.readFile(fileURLToPath(import.meta.url));
  const indexBytes = await fs.readFile(path.join(dist, 'index.html'));
  const report = {
    baselineCommit: 'f5f9b29393731d18aee2d66a31d89c315aa79c60',
    evaluatedDist: dist, browserVersion, runtime: process.version,
    receiverSha256: crypto.createHash('sha256').update(scriptBytes).digest('hex'),
    distIndexSha256: crypto.createHash('sha256').update(indexBytes).digest('hex'),
    scope: 'Actual React browser behavior against independent HTTP API fixtures; not native backend simulation or deployment qualification.',
    passed: results.filter((x) => x.passed).length, failed: results.filter((x) => !x.passed).length, results,
  };
  await fs.writeFile(path.join(output, 'receiver-results.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ passed: report.passed, failed: report.failed, receipt: path.join(output, 'receiver-results.json') }));
  if (report.failed) process.exitCode = 1;
}
