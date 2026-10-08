import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Actual application receiver. API fixtures are observations, not simulator claims.
// node uma-trace-receiver.mjs DIST OUTPUT [CHROMIUM_EXECUTABLE]
const [distArg, outputArg, chromiumExecutable] = process.argv.slice(2);
assert(distArg && outputArg, 'Usage: node uma-trace-receiver.mjs DIST OUTPUT [CHROMIUM_EXECUTABLE]');
const dist = path.resolve(distArg);
const output = path.resolve(outputArg);
await fs.mkdir(output, { recursive: true });
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright', {
  paths: [path.dirname(fileURLToPath(import.meta.url)), process.cwd(), process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES].filter(Boolean),
}));
const digest = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const names = { alpha: 'Alpha', beta: 'Beta', gamma: 'Gamma', delta: 'Delta', 'path/space &é': 'Encoded' };
const outcome = {
  steps: 4, finalTurn: 11, careerComplete: false, completedRaces: [],
  stats: { speed: 100, stamina: 90, power: 80, guts: 70, wit: 60 },
  energy: 50, mood: 'NORMAL', fans: 100, skillPoints: 30,
  learnedSkills: [], sparks: [], scenarioResources: {}, totalRngCalls: 18, telemetryRecords: 4,
};
function step(stepIndex, turn, phase, actionId, actionLabel, overrideApplied, overrideRejected) {
  return {
    stepIndex, turn, dateLabel: `Date for turn ${turn}`, phase, actionId, actionLabel,
    overrideApplied, overrideRejected, rngCallsBefore: 10 + stepIndex * 2,
    rngCallsAfter: 12 + stepIndex * 2, energy: 50, mood: 'NORMAL', fans: 100,
    skillPoints: 30, stats: outcome.stats, newRaces: [], totalRaces: 0, totalSkills: 0,
  };
}
function branch(id) {
  return {
    id, name: names[id], checkpointName: 'checkpoint', checkpointTurn: 8, seed: 412,
    scenarioId: 'ura', traineeName: 'Receiver', startedAt: '2026-10-08T00:00:00Z',
    config: { policy: 'bot', maxActions: 4, overrides: [
      { turn: 8, actionId: 'rest' }, { turn: 10, actionId: 'train_stamina' },
      { turn: 99, actionId: 'future:action' },
    ] },
    timeline: [
      step(0, 8, 'EVENT_CHOICE', 'event_option_1', 'Accept event', false, true),
      step(1, 8, 'TRAINING', 'rest', 'Rest', true, false),
      step(2, 10, 'TRAINING', 'train_stamina', 'Train stamina', true, false),
      step(3, 11, 'TRAINING', 'train_speed', 'Train speed', false, false),
    ],
    outcome,
    receiverEvidence: { kind: 'independent HTTP fixture', preserveUnknownPayload: ['exact', id] },
  };
}
const traces = Object.fromEntries(Object.keys(names).map((id) => [id, branch(id)]));
function freshFixture() {
  const fixture = {
    branches: Object.values(traces).map((b) => ({
      id: b.id, name: b.name, checkpointName: b.checkpointName, seed: b.seed,
      scenarioId: b.scenarioId, traineeName: b.traineeName, policy: b.config.policy,
      steps: b.timeline.length, careerComplete: false, finalTurn: b.outcome.finalTurn,
      fans: b.outcome.fans, startedAt: b.startedAt,
    })),
    calls: [], errors: [], plans: new Map(), releases: [], failDeletion: false,
    plan(id, response) {
      this.plans.set(id, [...(this.plans.get(id) ?? []), response]);
    },
    hold(id, response = {}) {
      let release;
      const gate = new Promise((resolve) => { release = resolve; });
      this.plan(id, { ...response, gate });
      this.releases.push(release);
      return release;
    },
  };
  return fixture;
}
let fixture = freshFixture();
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
      const call = { method: req.method, path: url.pathname, query: Object.fromEntries(url.searchParams), body };
      current.calls.push(call);
      if (url.pathname === '/v1/health') return json(res, { ok: true, version: 'trace-receiver-fixture', repoRoot: true });
      if (url.pathname.startsWith('/v1/catalog/')) return json(res, { items: [] });
      if (url.pathname === '/v1/sessions') return json(res, { sessions: [], active: '' });
      if (url.pathname === '/v1/library') return json(res, { entries: [] });
      if (url.pathname === '/v1/lab/branches') return json(res, { branches: current.branches });
      if (url.pathname === '/v1/lab/branch' && req.method === 'GET') {
        const id = url.searchParams.get('id');
        const plan = current.plans.get(id)?.shift() ?? {};
        if (plan.gate) await plan.gate;
        if (plan.status) {
          call.responseStatus = plan.status;
          return json(res, { error: `Independent trace read failure for ${id}` }, plan.status);
        }
        const payload = plan.payload ?? traces[id];
        call.responseStatus = 200;
        call.payloadSha256 = digest(JSON.stringify(payload));
        return json(res, { branch: payload });
      }
      if (url.pathname === '/v1/lab/branch/delete') {
        if (current.failDeletion) return json(res, { error: 'Independent deletion failure' }, 503);
        current.branches = current.branches.filter((b) => b.id !== body.id);
        return json(res, { deleted: body.id });
      }
      current.errors.push(`Unexpected ${req.method} ${url.pathname}`);
      return json(res, { error: 'Unexpected receiver API route' }, 404);
    }
    const relative = url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname.slice(1));
    const target = path.resolve(dist, relative);
    if (!target.startsWith(dist + path.sep)) return json(res, { error: 'Outside fixture root' }, 400);
    const bytes = await fs.readFile(target);
    res.writeHead(200, { 'Content-Type': { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' }[path.extname(target)] ?? 'application/octet-stream' });
    res.end(bytes);
  } catch (error) { json(res, { error: String(error) }, 500); }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({
  headless: true, executablePath: chromiumExecutable, args: ['--no-sandbox', '--disable-dev-shm-usage'],
});
const browserVersion = browser.version();
const results = [];
const panel = (page) => page.locator('section.branch-trace');
const exportButton = (page) => panel(page).getByRole('button', { name: /^(Download full trace JSON|Download displayed trace)$/ });
const row = (page, id) => page.getByRole('row').filter({ has: page.getByLabel(`Compare A: ${names[id]}`, { exact: true }) });
async function ready(page) {
  await page.goto(origin);
  await page.getByRole('button', { name: 'Library & lab', exact: true }).click();
  await page.getByLabel('Compare A: Alpha', { exact: true }).waitFor();
  await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
}
async function inspect(page, id, wait = true) {
  await row(page, id).getByRole('button', { name: 'Inspect trace', exact: true }).click();
  if (wait) await panel(page).getByRole('heading', { name: `Trace: ${names[id]}`, exact: true }).waitFor();
}
async function loading(page, id) {
  const status = panel(page).getByRole('status');
  await status.filter({ hasText: `Loading trace for ${id}` }).waitFor();
  assert((await status.textContent()).includes(id));
}
async function assertReady(page, id) {
  assert.equal(await panel(page).getByRole('heading', { name: `Trace: ${names[id]}`, exact: true }).count(), 1);
  const branchId = panel(page).locator('dd').filter({ has: page.locator('code').filter({ hasText: id }) });
  assert(await branchId.count() >= 1, `Displayed trace does not identify ${id}`);
  assert.equal(await panel(page).getByRole('alert').count(), 0, 'A stale error appeared on a successful trace');
}
async function assertIdle(page) {
  assert.equal(await exportButton(page).count(), 0);
  assert.equal(await panel(page).getByRole('button', { name: 'Close trace', exact: true }).count(), 0);
  assert.equal(await panel(page).getByRole('alert').count(), 0);
  assert.equal(await panel(page).getByRole('status').count(), 0);
}
async function exportEquals(page, id) {
  const pending = page.waitForEvent('download');
  await exportButton(page).click();
  const file = await pending;
  assert.equal(await file.failure(), null, 'Actual browser trace download failed');
  const local = await file.path();
  assert(local, 'Browser did not materialize trace download');
  const payload = JSON.parse(await fs.readFile(local, 'utf8'));
  assert.deepEqual(payload, traces[id], 'Downloaded JSON does not exactly preserve the displayed HTTP branch object');
  return { id: payload.id, filename: file.suggestedFilename(), payloadSha256: digest(JSON.stringify(payload)) };
}
async function deleted(page, id) {
  page.once('dialog', (dialog) => dialog.accept());
  await row(page, id).getByRole('button', { name: 'Delete', exact: true }).click();
  await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
}
async function releaseAndSettle(page, id, release) {
  const delivered = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return url.pathname === '/v1/lab/branch' && url.searchParams.get('id') === id;
  });
  release();
  const response = await delivered;
  await response.finished();
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}
async function runCase(name, fn, allowDeletion = false) {
  fixture = freshFixture();
  const context = await browser.newContext({ acceptDownloads: true, viewport: { width: 1280, height: 1000 } });
  const page = await context.newPage();
  page.setDefaultTimeout(4000);
  const browserErrors = [];
  page.on('pageerror', (error) => browserErrors.push(error.message));
  try {
    await ready(page);
    const detail = await fn(page, fixture);
    assert.deepEqual(browserErrors, [], 'Uncaught browser errors');
    assert.deepEqual(fixture.errors, [], 'Unexpected receiving API calls');
    const writes = fixture.calls.filter((x) => x.method !== 'GET');
    assert(writes.every((x) => allowDeletion && x.path === '/v1/lab/branch/delete'), `Read-only inspection issued an API mutation: ${JSON.stringify(writes)}`);
    results.push({ name, passed: true, detail, requests: fixture.calls.filter((x) => !['/v1/health', '/v1/sessions', '/v1/library'].includes(x.path) && !x.path.startsWith('/v1/catalog/')) });
  } catch (error) {
    const result = { name, passed: false, error: error.message, browserErrors, requests: fixture.calls };
    if (results.filter((x) => !x.passed).length < 3) {
      const filename = `failure-${name.replace(/[^a-z0-9]+/gi, '-')}.png`;
      await page.screenshot({ path: path.join(output, filename), fullPage: true }).catch(() => {});
      result.screenshot = filename;
    }
    results.push(result);
  } finally {
    fixture.releases.forEach((release) => release());
    await context.close();
  }
  console.log(`${results.at(-1).passed ? 'PASS' : 'FAIL'} ${name}${results.at(-1).passed ? '' : ': ' + results.at(-1).error}`);
  return results.at(-1).passed;
}

let capabilityPresent = false;
try {
  capabilityPresent = await runCase('saved branch rows expose trace inspection in the actual application', async (page) => {
    assert.equal(await row(page, 'alpha').getByRole('button', { name: 'Inspect trace', exact: true }).count(), 1,
      'The actual application has no branch-trace entry point; dependent capability tests are not run');
    assert.equal(await panel(page).count(), 1, 'The actual application has no trace inspector');
  });
  if (capabilityPresent) {
    await runCase('trace preserves authored overrides per-step mixed flags and exact full JSON', async (page) => {
      await page.getByLabel('Compare A: Gamma', { exact: true }).check();
      await page.getByLabel('Compare B: Delta', { exact: true }).check();
      await inspect(page, 'alpha');
      await assertReady(page, 'alpha');
      const authored = panel(page).getByRole('region', { name: 'Authored overrides', exact: true });
      const turn8 = authored.getByRole('row').filter({ has: page.getByRole('cell', { name: '8', exact: true }) });
      const observation = await turn8.textContent();
      assert(observation.includes('1 applied') && observation.includes('1 rejected'), `Mixed observations were collapsed: ${observation}`);
      const future = authored.getByRole('row').filter({ has: page.getByRole('cell', { name: '99', exact: true }) });
      assert.match(await future.textContent(), /No step recorded|[Uu]nobserved/);
      assert(!/rejected/i.test(await future.textContent()), 'An unobserved override was classified as rejected');
      const actions = panel(page).getByRole('region', { name: 'Recorded branch steps', exact: true });
      const rows = actions.locator('tbody tr');
      assert.equal(await rows.count(), 3);
      const rejected = rows.filter({ hasText: 'EVENT_CHOICE' });
      assert.match(await rejected.textContent(), /Accept event/);
      assert.match(await rejected.textContent(), /Rejected/);
      assert(!/Applied/.test(await rejected.textContent()));
      const applied = rows.filter({ has: page.locator('code').filter({ hasText: /^rest$/ }) });
      assert.equal(await applied.count(), 1, 'The later applied rest at the same turn is missing');
      assert.match(await applied.textContent(), /Applied/);
      assert(!/Rejected/.test(await applied.textContent()));
      await panel(page).getByRole('checkbox', { name: 'Only steps with override flags', exact: true }).uncheck();
      assert.equal(await rows.count(), 4, 'Unfiltered trace dropped the ordinary step');
      assert.match(await rows.last().textContent(), /Train speed/);
      assert(await page.getByLabel('Compare A: Gamma', { exact: true }).isChecked());
      assert(await page.getByLabel('Compare B: Delta', { exact: true }).isChecked());
      return exportEquals(page, 'alpha');
    });
    await runCase('loading B never relabels or exports the old A trace as B', async (page, f) => {
      await inspect(page, 'alpha');
      const release = f.hold('beta');
      await inspect(page, 'beta', false);
      await loading(page, 'beta');
      if (await exportButton(page).count()) {
        await assertReady(page, 'alpha');
        await exportEquals(page, 'alpha');
      }
      await releaseAndSettle(page, 'beta', release);
      await panel(page).getByRole('heading', { name: 'Trace: Beta', exact: true }).waitFor();
      await assertReady(page, 'beta');
      return exportEquals(page, 'beta');
    });
    await runCase('late A response cannot replace the newer B trace', async (page, f) => {
      const release = f.hold('alpha');
      await inspect(page, 'alpha', false);
      await loading(page, 'alpha');
      await inspect(page, 'beta');
      await releaseAndSettle(page, 'alpha', release);
      await assertReady(page, 'beta');
      return exportEquals(page, 'beta');
    });
    await runCase('late A failure cannot overwrite the newer B trace with an obsolete error', async (page, f) => {
      const release = f.hold('alpha', { status: 503 });
      await inspect(page, 'alpha', false);
      await loading(page, 'alpha');
      await inspect(page, 'beta');
      await releaseAndSettle(page, 'alpha', release);
      await assertReady(page, 'beta');
      return exportEquals(page, 'beta');
    });
    for (const response of [{}, { status: 503 }]) {
      await runCase(`closing inspector prevents a late ${response.status ? 'error' : 'response'} from reopening it`, async (page, f) => {
        const release = f.hold('alpha', response);
        await inspect(page, 'alpha', false);
        await loading(page, 'alpha');
        await panel(page).getByRole('button', { name: 'Close trace', exact: true }).click();
        await releaseAndSettle(page, 'alpha', release);
        await assertIdle(page);
      });
    }
    await runCase('unmounted trace read cannot affect a newly opened lab view', async (page, f) => {
      const release = f.hold('alpha', { status: 503 });
      await inspect(page, 'alpha', false);
      await loading(page, 'alpha');
      await page.getByRole('button', { name: 'Run', exact: true }).click();
      await page.getByRole('button', { name: 'Library & lab', exact: true }).click();
      await page.getByLabel('Compare A: Beta', { exact: true }).waitFor();
      await page.locator('.busy-overlay').waitFor({ state: 'hidden' });
      await inspect(page, 'beta');
      await releaseAndSettle(page, 'alpha', release);
      await assertReady(page, 'beta');
    });
    await runCase('retry requests the failed branch and keeps its error identity explicit', async (page, f) => {
      f.plan('beta', { status: 503 });
      await inspect(page, 'beta', false);
      await panel(page).getByRole('alert').waitFor();
      assert.match(await panel(page).getByRole('alert').textContent(), /beta/);
      assert.equal(await exportButton(page).count(), 0);
      await page.getByLabel('Compare A: Gamma', { exact: true }).check();
      await panel(page).getByRole('button', { name: 'Retry trace', exact: true }).click();
      await panel(page).getByRole('heading', { name: 'Trace: Beta', exact: true }).waitFor();
      const reads = f.calls.filter((x) => x.path === '/v1/lab/branch').map((x) => x.query.id);
      assert.deepEqual(reads, ['beta', 'beta']);
      return exportEquals(page, 'beta');
    });
    await runCase('wrong branch identity response is rejected without a wrong trace export', async (page, f) => {
      f.plan('alpha', { payload: traces.beta });
      await inspect(page, 'alpha', false);
      await panel(page).getByRole('alert').waitFor();
      assert.match(await panel(page).getByRole('alert').textContent(), /alpha/);
      assert.equal(await exportButton(page).count(), 0);
      assert.equal(await panel(page).getByRole('heading', { name: 'Trace: Beta', exact: true }).count(), 0);
    });
    await runCase('missing trace configuration fails visibly without crashing the application', async (page, f) => {
      const malformed = { ...traces.alpha };
      delete malformed.config;
      f.plan('alpha', { payload: malformed });
      await inspect(page, 'alpha', false);
      await panel(page).getByRole('alert').waitFor();
      assert.match(await panel(page).getByRole('alert').textContent(), /alpha/);
      assert.equal(await exportButton(page).count(), 0);
    });
    await runCase('reserved branch ID is retrieved unchanged and exported intact', async (page, f) => {
      const id = 'path/space &é';
      await inspect(page, id);
      await assertReady(page, id);
      assert.deepEqual(f.calls.filter((x) => x.path === '/v1/lab/branch').map((x) => x.query.id), [id]);
      return exportEquals(page, id);
    });
    await runCase('confirmed deletion invalidates the currently loaded trace', async (page) => {
      await inspect(page, 'alpha');
      await deleted(page, 'alpha');
      await page.getByLabel('Compare A: Alpha', { exact: true }).waitFor({ state: 'hidden' });
      await assertIdle(page);
    }, true);
    await runCase('confirmed deletion prevents a pending trace response from reopening the inspector', async (page, f) => {
      const release = f.hold('alpha');
      await inspect(page, 'alpha', false);
      await loading(page, 'alpha');
      await deleted(page, 'alpha');
      await page.getByLabel('Compare A: Alpha', { exact: true }).waitFor({ state: 'hidden' });
      await releaseAndSettle(page, 'alpha', release);
      await assertIdle(page);
    }, true);
    await runCase('failed deletion preserves the valid loaded trace', async (page, f) => {
      await inspect(page, 'alpha');
      f.failDeletion = true;
      await deleted(page, 'alpha');
      await page.locator('.banner.error').waitFor();
      await assertReady(page, 'alpha');
      return exportEquals(page, 'alpha');
    }, true);
  }
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
  const report = {
    baselineCommit: 'f5f9b29393731d18aee2d66a31d89c315aa79c60', evaluatedDist: dist,
    receiverSha256: digest(await fs.readFile(fileURLToPath(import.meta.url))),
    distIndexSha256: digest(await fs.readFile(path.join(dist, 'index.html'))),
    browserVersion, runtime: process.version, capabilityPresent,
    scope: 'Actual built application and downloads against independent HTTP branch fixtures; no native simulation or deployment qualification.',
    passed: results.filter((x) => x.passed).length, failed: results.filter((x) => !x.passed).length,
    dependentCasesNotRun: capabilityPresent ? 0 : 14, results,
  };
  await fs.writeFile(path.join(output, 'trace-receiver-results.json'), JSON.stringify(report, null, 2) + '\n');
  await fs.writeFile(path.join(output, 'trace-response-fixtures.json'), JSON.stringify(traces, null, 2) + '\n');
  console.log(JSON.stringify({ passed: report.passed, failed: report.failed, dependentCasesNotRun: report.dependentCasesNotRun, receipt: path.join(output, 'trace-receiver-results.json') }));
  if (report.failed) process.exitCode = 1;
}
