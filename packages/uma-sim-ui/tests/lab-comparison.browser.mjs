import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createServer, request as httpRequest} from 'node:http';
import {readFile, readdir, mkdir, mkdtemp, writeFile} from 'node:fs/promises';
import {createWriteStream} from 'node:fs';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const repo = process.env.UMA_LAB_REPO || path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const outputRoot = process.env.UMA_LAB_OUTPUT_ROOT || path.join(repo, '.lab-receiving');
const stage = process.env.LAB_STAGE || 'candidate';
assert(/^[a-z0-9-]+$/.test(stage), 'Use a plain stage name');
const evidence = path.join(outputRoot, stage);
await mkdir(evidence, {recursive: true});
const {chromium} = await (process.env.UMA_LAB_PLAYWRIGHT_MODULE
  ? import(pathToFileURL(process.env.UMA_LAB_PLAYWRIGHT_MODULE))
  : import('playwright'));
const runtime = await mkdtemp(path.join(evidence, 'runtime-'));
const unusedPort = async () => {
  const server = createServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
};
const apiPort = await unusedPort();
const apiLog = createWriteStream(path.join(evidence, 'api.log'));
const apiBinary = process.env.UMA_LAB_API_BINARY || path.join(repo, 'target/debug/uma-sim-api');
const api = spawn(apiBinary, [String(apiPort)], {cwd: runtime, stdio: ['ignore', 'pipe', 'pipe']});
api.stdout.pipe(apiLog); api.stderr.pipe(apiLog);
const apiBase = 'http://127.0.0.1:' + apiPort;
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let apiReady = false;
for (let i = 0; i < 100; i++) {
  try { const r = await fetch(apiBase + '/v1/health'); if (r.ok) { apiReady = true; break; } } catch {}
  await sleep(50);
}
if (!apiReady) { api.kill(); throw new Error('Isolated API did not become ready'); }
const mime = {'.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.json': 'application/json'};
const dist = path.join(repo, 'packages/uma-sim-ui/dist');
const proxy = createServer(async (req, res) => {
  if (req.url.startsWith('/v1/')) {
    const upstream = httpRequest(apiBase + req.url, {method: req.method, headers: req.headers}, incoming => {
      res.writeHead(incoming.statusCode, incoming.headers); incoming.pipe(res);
    });
    upstream.on('error', e => {res.writeHead(502); res.end(String(e));});
    req.pipe(upstream); return;
  }
  try {
    const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    const file = path.resolve(dist, '.' + (pathname === '/' ? '/index.html' : pathname));
    if (!file.startsWith(dist + path.sep)) throw new Error('outside static root');
    const bytes = await readFile(file);
    res.writeHead(200, {'Content-Type': mime[path.extname(file)] || 'application/octet-stream'}); res.end(bytes);
  } catch {res.writeHead(404); res.end('Not found');}
});
await new Promise(resolve => proxy.listen(0, '127.0.0.1', resolve));
const uiBase = 'http://127.0.0.1:' + proxy.address().port;
const suffix = Date.now().toString(36);
const checkpoint = 'receiving-' + suffix;
const names = {a: 'bot-' + suffix, b: 'default-' + suffix, c: 'control-' + suffix};
const hash = async file => createHash('sha256').update(await readFile(file)).digest('hex');
const sourceFiles = {};
for (const file of ['packages/uma-sim-ui/src/components/ComparePanel.tsx', 'packages/uma-sim-ui/src/state/labStore.ts', 'uma-sim-core/src/api.rs', 'uma-sim-core/src/career_lab.rs']) sourceFiles[file] = {sha256: await hash(path.join(repo, file))};
const builtAssets = {};
for (const file of await readdir(path.join(dist, 'assets'))) builtAssets[file] = await hash(path.join(dist, 'assets', file));
const receipt = {stage, startedAt: new Date().toISOString(), source: execFileSync('git', ['-C', repo, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim(), sourceFiles, builtAssets, apiBinary: {sha256: await hash(apiBinary)}, apiBase, uiBase, checkpoint, names, cases: [], pageErrors: [], requests: []};
let browser, page;
const getJson = async pathname => { const r = await fetch(apiBase + pathname); assert.equal(r.status, 200); return r.json(); };
try {
  browser = await chromium.launch({headless: true, ...(process.env.UMA_LAB_CHROMIUM_EXECUTABLE ? {executablePath: process.env.UMA_LAB_CHROMIUM_EXECUTABLE} : {})});
  receipt.browser = browser.version();
  const context = await browser.newContext({viewport: {width: 1440, height: 1100}, acceptDownloads: true});
  await context.route('**/*', route => {
    const u = new URL(route.request().url());
    return ['127.0.0.1', 'localhost'].includes(u.hostname) ? route.continue() : route.abort();
  });
  page = await context.newPage();
  page.on('pageerror', e => receipt.pageErrors.push(String(e)));
  page.on('request', req => {
    if (new URL(req.url()).pathname.startsWith('/v1/')) receipt.requests.push({method: req.method(), path: new URL(req.url()).pathname, body: req.postData()});
  });
  page.on('dialog', dialog => dialog.accept());
  await page.goto(uiBase);
  await page.getByRole('button', {name: 'Start run', exact: true}).waitFor();
  await page.locator('.field').filter({has: page.locator('label', {hasText: /^Race model$/})}).locator('select').selectOption('stub');
  await page.getByRole('button', {name: 'Start run', exact: true}).click();
  await page.getByRole('button', {name: 'Auto step', exact: true}).waitFor();
  for (let i = 0; i < 10; i++) {
    await page.getByRole('button', {name: 'Auto step', exact: true}).click();
    await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  }
  const before = await getJson('/v1/run/state');
  await page.getByRole('button', {name: 'Library & lab', exact: true}).click();
  await page.getByRole('heading', {name: 'Career library', exact: true}).waitFor();
  await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  await page.getByPlaceholder('e.g. debut-fork (letters, digits, -_.  up to 64)').fill(checkpoint);
  await page.getByRole('button', {name: 'Save checkpoint', exact: true}).click();
  const savedRow = page.locator('tr').filter({has: page.getByText(checkpoint, {exact: true})}).filter({has: page.getByRole('button', {name: 'Resume', exact: true})});
  await savedRow.waitFor();
  const library = await getJson('/v1/library');
  const entry = library.entries.find(x => x.name === checkpoint);
  assert(entry); assert.equal(entry.seed, 42);
  receipt.entry = entry;
  receipt.cases.push({name: 'save checkpoint through actual React UI', pass: true, turn: entry.turn});
  const exported = await getJson('/v1/library/export?name=' + encodeURIComponent(checkpoint));
  await savedRow.getByRole('button', {name: 'Fork open', exact: true}).click();
  await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  const sessions = await getJson('/v1/sessions');
  assert(sessions.active);
  assert.equal(sessions.sessions.length, 2);
  receipt.sessionsAfterFork = sessions;
  const forkState = await getJson('/v1/run/state');
  assert.equal(forkState.state.turn, before.state.turn);
  assert.deepEqual(await getJson('/v1/library/export?name=' + encodeURIComponent(checkpoint)), exported);
  receipt.cases.push({name: 'fork open preserves checkpoint and main session', pass: true});
  const forkRow = page.locator('tr').filter({has: page.getByRole('button', {name: 'Close', exact: true})});
  await forkRow.getByRole('button', {name: 'Close', exact: true}).click();
  await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  await savedRow.getByRole('button', {name: 'Resume', exact: true}).click();
  await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  assert.equal((await getJson('/v1/run/state')).state.turn, entry.turn);
  receipt.cases.push({name: 'close fork and resume saved checkpoint through UI', pass: true});
  await page.getByLabel(/^Checkpoint/).selectOption(checkpoint);
  const runBranch = async (name, policy) => {
    await page.getByLabel('Branch name (optional)', {exact: true}).fill(name);
    await page.getByLabel(/^Policy/).selectOption(policy);
    await page.getByRole('button', {name: 'Run branch', exact: true}).click();
    await page.locator('.busy-overlay').waitFor({state: 'hidden'});
    await page.getByRole('radio', {name: 'Compare A: ' + name, exact: true}).waitFor();
  };
  await runBranch(names.a, 'bot');
  await runBranch(names.b, 'default');
  await runBranch(names.c, 'bot');
  const branches = (await getJson('/v1/lab/branches')).branches;
  const a = branches.find(x => x.name === names.a), b = branches.find(x => x.name === names.b), c = branches.find(x => x.name === names.c);
  assert(a && b && c);
  receipt.branches = [a,b,c];
  await page.getByRole('radio', {name: 'Compare A: ' + names.a, exact: true}).check();
  await page.getByRole('radio', {name: 'Compare B: ' + names.b, exact: true}).check();
  await page.getByRole('button', {name: 'Compare selected branches', exact: true}).click();
  await page.getByRole('heading', {name: names.a + ' vs ' + names.b, exact: true}).waitFor();
  const download = await Promise.all([page.waitForEvent('download'), page.getByRole('link', {name: 'Download JSON report', exact: true}).click()]);
  const firstPath = path.join(evidence, 'initial-comparison.json');
  await download[0].saveAs(firstPath);
  const initialReport = JSON.parse(await readFile(firstPath, 'utf8'));
  assert.equal(initialReport.aId, a.id); assert.equal(initialReport.bId, b.id);
  receipt.cases.push({name: 'run branches, compare and download match visible pair', pass: true});
  await page.getByRole('radio', {name: 'Compare B: ' + names.c, exact: true}).check();
  const headingStillAB = await page.getByRole('heading', {name: names.a + ' vs ' + names.b, exact: true}).isVisible();
  const reportLink = await page.getByRole('link', {name: 'Download JSON report', exact: true}).getAttribute('href');
  const changedDownload = await Promise.all([page.waitForEvent('download'), page.getByRole('link', {name: 'Download JSON report', exact: true}).click()]);
  const changedPath = path.join(evidence, 'after-selection-report.json');
  await changedDownload[0].saveAs(changedPath);
  const changedReport = JSON.parse(await readFile(changedPath, 'utf8'));
  const agrees = !headingStillAB || (changedReport.aId === a.id && changedReport.bId === b.id);
  const markdownDownload = await Promise.all([page.waitForEvent('download'), page.getByRole('link', {name: 'Download markdown report', exact: true}).click()]);
  const markdownPath = path.join(evidence, 'after-selection-report.md');
  await markdownDownload[0].saveAs(markdownPath);
  const markdown = await readFile(markdownPath, 'utf8');
  const markdownAgrees = markdown.includes(names.a) && markdown.includes(names.b) && !markdown.includes(names.c);
  receipt.cases.push({name: 'markdown download remains bound to displayed comparison', pass: markdownAgrees});

  receipt.cases.push({name: 'download remains bound to displayed comparison after draft selection', pass: agrees, headingStillAB, reportLink, displayedIds: [a.id,b.id], downloadedIds: [changedReport.aId, changedReport.bId]});
  await page.screenshot({path: path.join(evidence, 'comparison-selection.png'), fullPage: true});
  receipt.counterexampleReproduced = !agrees;
  // Hold the real comparison response while changing the next selection via keyboard.
  await page.getByRole('radio', {name: 'Compare B: ' + names.b, exact: true}).check();
  let releaseComparison;
  let seenComparison;
  const receivedComparison = new Promise(resolve => {seenComparison = resolve;});
  const heldComparison = new Promise(resolve => {releaseComparison = resolve;});
  await page.route('**/v1/lab/compare', async route => {
    const response = await route.fetch();
    seenComparison();
    await heldComparison;
    await route.fulfill({response});
  });
  await page.getByRole('button', {name: 'Compare selected branches', exact: true}).click();
  await receivedComparison;
  await page.getByRole('radio', {name: 'Compare B: ' + names.c, exact: true}).focus();
  await page.keyboard.press('Space');
  assert(await page.getByRole('radio', {name: 'Compare B: ' + names.c, exact: true}).isChecked());
  releaseComparison();
  await page.locator('.busy-overlay').waitFor({state: 'hidden'});
  const draftPreserved = await page.getByRole('radio', {name: 'Compare B: ' + names.c, exact: true}).isChecked();
  receipt.cases.push({name: 'delayed comparison preserves newer draft selection', pass: draftPreserved});
  await page.unroute('**/v1/lab/compare');
  // Delete only the selected draft branch; the displayed A/B result remains valid.
  await page.getByRole('radio', {name: 'Compare B: ' + names.c, exact: true}).check();
  const rowFor = name => page.locator('tr').filter({has: page.getByRole('radio', {name: 'Compare A: ' + name, exact: true})});
  await rowFor(names.c).getByRole('button', {name: 'Delete', exact: true}).click();
  await page.getByRole('radio', {name: 'Compare A: ' + names.c, exact: true}).waitFor({state: 'detached'});
  const retainedAfterUnrelatedDelete = await page.getByRole('heading', {name: names.a + ' vs ' + names.b, exact: true}).isVisible();
  receipt.cases.push({name: 'deleting draft-only branch preserves displayed comparison', pass: retainedAfterUnrelatedDelete});
  // Delete only a displayed branch while a different next pair is selected.
  await page.getByRole('radio', {name: 'Compare B: ' + names.b, exact: true}).check();
  await page.getByRole('button', {name: 'Compare selected branches', exact: true}).click();
  await page.getByRole('heading', {name: names.a + ' vs ' + names.b, exact: true}).waitFor();
  names.d = 'later-' + suffix;
  await runBranch(names.d, 'bot');
  await page.getByRole('radio', {name: 'Compare B: ' + names.d, exact: true}).check();
  await rowFor(names.b).getByRole('button', {name: 'Delete', exact: true}).click();
  await page.getByRole('radio', {name: 'Compare A: ' + names.b, exact: true}).waitFor({state: 'detached'});
  const clearedAfterDisplayedDelete = !(await page.getByRole('heading', {name: names.a + ' vs ' + names.b, exact: true}).isVisible())
    && await page.getByRole('link', {name: 'Download JSON report', exact: true}).count() === 0;
  receipt.cases.push({name: 'deleting displayed-only branch clears result and downloads', pass: clearedAfterDisplayedDelete});
  assert(await page.getByRole('radio', {name: 'Compare A: ' + names.a, exact: true}).isChecked());
  assert(await page.getByRole('radio', {name: 'Compare B: ' + names.d, exact: true}).isChecked());
  assert.equal(receipt.pageErrors.length, 0);
  if (stage === 'baseline') assert(!agrees, 'Expected baseline contradiction was not reproduced');
  else {
    assert(agrees, 'Downloaded comparison contradicts displayed result');
    assert(markdownAgrees, 'Markdown comparison contradicts displayed result');
    assert(draftPreserved, 'Delayed result overwrote newer draft selection');
    assert(retainedAfterUnrelatedDelete, 'Unrelated draft deletion cleared valid result');
    assert(clearedAfterDisplayedDelete, 'Deleted displayed branch retained an exportable result');
  }
  receipt.completed = true;
} catch (e) {
  if (page) await page.screenshot({path: path.join(evidence, 'failure.png'), fullPage: true}).catch(() => {});
  receipt.failure = String(e);
  receipt.stack = e.stack;
  process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  await new Promise(resolve => proxy.close(resolve));
  api.kill('SIGTERM');
  apiLog.end();
  await writeFile(path.join(evidence, 'receipt.json'), JSON.stringify(receipt,null,2)+'\n');
  console.log(JSON.stringify({stage, completed: receipt.completed ?? false, counterexampleReproduced: receipt.counterexampleReproduced, cases: receipt.cases, pageErrors: receipt.pageErrors, failure: receipt.failure, evidence},null,2));
}
