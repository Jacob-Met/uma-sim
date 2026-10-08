import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { resolveObjectURL } from 'node:buffer';
import { createHash } from 'node:crypto';

// Independent native receiving scenario. React callbacks, Node Blob, object-URL
// registry and timers are real. The document and anchor dispatch are authored
// simulations. No browser, native DOM, download/save completion or network claim.
const [uiArg, rendererArg, outputArg] = process.argv.slice(2);
assert(uiArg && rendererArg && outputArg,
  'Usage: node review-dispatch-refusal.mjs UI_ROOT MATCHING_RENDERER_ROOT NEW_OUTPUT');
const ui = path.resolve(uiArg);
const output = path.resolve(outputArg);
await fs.mkdir(output); // Refuse an existing output; never overwrite evidence.
const require = createRequire(path.join(ui, 'package.json'));
const rendererRequire = createRequire(path.join(path.resolve(rendererArg), 'package.json'));
assert.equal(require.resolve('react'), rendererRequire.resolve('react'));
const React = require('react');
const renderer = rendererRequire('./index.js');
const ts = require('typescript');
assert.equal(React.version, rendererRequire('./package.json').version);
assert.notEqual(process.env.NODE_ENV, 'production');
const { act } = renderer;
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sourcePins = [];
const pinned = {
  'logExport.ts': '7ea3a0b6066e926af6825397feb5115cebd82bbbba2923dbd900d41c09e7937c',
  'LogDownload.tsx': 'cf92740a422128141f242c449f120d6e0cae02b627792b8ff388ac270f12b1d6',
  'logView.ts': '7dcd6bef0674cfbf7ea53b896faae192c9f0ac3e093487958f37494f2b5067d2',
  'LogPanel.tsx': '2d23c9ad7932da56d434e08c8f20af490cd9f628d71dfaa92858f0269fa557a2',
};
async function load(name, locals = {}) {
  const source = await fs.readFile(path.join(ui, 'src/components', name), 'utf8');
  assert.equal(sha(source), pinned[name], `frozen input ${name}`);
  sourcePins.push({ path: `src/components/${name}`, sha256: sha(source) });
  let code = ts.transpileModule(source, { compilerOptions: {
    module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020,
    jsx: ts.JsxEmit.ReactJSX,
  } }).outputText;
  for (const [name, url] of Object.entries({
    react: pathToFileURL(require.resolve('react')).href,
    'react/jsx-runtime': pathToFileURL(require.resolve('react/jsx-runtime')).href,
    ...locals,
  })) code = code.replaceAll(JSON.stringify(name), JSON.stringify(url));
  code = code.replace(/^import ["'][^"']+\.css["'];\s*/gm, '');
  return `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`;
}
const exportUrl = await load('logExport.ts');
const downloadUrl = await load('LogDownload.tsx', { './logExport': exportUrl });
const viewUrl = await load('logView.ts');
const panelUrl = await load('LogPanel.tsx', { './logView': viewUrl, './LogDownload': downloadUrl });
const { LogPanel } = await import(panelUrl);

const fixture = Object.freeze([
  'needle — ウマ娘 🏇', '', 'duplicate', 'duplicate',
  Array.from({ length: 32 }, (_, n) => String.fromCharCode(n)).join(''),
  '\u007f\ufeff\u2028\u2029', 'lone high \ud800', 'lone low \udfff',
  'e\u0301 é <script>literal</script> " \\ \\u0000',
]);
const nextFixture = Object.freeze(['replacement retained history', '', 'replacement retained history']);
const original = {
  document: Object.getOwnPropertyDescriptor(globalThis, 'document'),
  fetch: globalThis.fetch, create: URL.createObjectURL, revoke: URL.revokeObjectURL,
};
const created = [], revoked = [], attempts = [], attached = new Set(), checks = [];
let component, failDispatch = true, fetches = 0;
const check = (name, assertion) => { assertion(); checks.push({ name, passed: true }); };
const text = (node) => node == null ? '' : typeof node === 'string' || typeof node === 'number'
  ? String(node) : (node.children ?? []).map(text).join('');
const controls = () => component.root.findAllByType('button')
  .filter((node) => text(node).startsWith('Download all '));
const reader = () => ({
  query: component.root.findByProps({ type: 'search' }).props.value,
  status: text(component.root.findByProps({ role: 'status' })),
  rows: component.root.findAllByType('li').map(text),
  summary: text(component.root.findByType('pre')),
});
const alerts = () => component.root.findAllByProps({ role: 'alert' }).map(text);
URL.createObjectURL = (blob) => { const url = original.create(blob); created.push(url); return url; };
URL.revokeObjectURL = (url) => { revoked.push(url); original.revoke(url); };
globalThis.fetch = () => { fetches += 1; throw new Error('no network allowed in this scenario'); };
Object.defineProperty(globalThis, 'document', { configurable: true, value: {
  createElement(tag) {
    assert.equal(tag, 'a');
    return { href: '', download: '',
      click() {
        assert(attached.has(this));
        const blob = resolveObjectURL(this.href);
        assert(blob instanceof Blob);
        attempts.push({ filename: this.download, blob, url: this.href, refused: failDispatch });
        if (failDispatch) throw new Error('authored synchronous anchor dispatch refusal after allocation');
      },
      remove() { attached.delete(this); },
    };
  },
  body: { appendChild(anchor) { attached.add(anchor); } },
} });
const receipt = {
  scenario: 'post-allocation synchronous dispatch refusal, cleanup, and explicit retry',
  boundary: 'Actual native React callbacks and Node Blob/URL/timer behavior; simulated document/anchor with authored synchronous refusal; no browser or download acceptance.',
  result: 'running', checks, source_files: sourcePins,
  runtime: { node: process.version, react: React.version, renderer: rendererRequire('./package.json').version,
    typescript: ts.version, renderer_root: path.resolve(rendererArg) },
};
try {
  act(() => { component = renderer.create(React.createElement(LogPanel, { history: fixture, lines: ['summary must stay separate'] })); });
  act(() => component.root.findByProps({ type: 'search' }).props.onChange({ target: { value: 'needle' } }));
  const select = component.root.findByType('select');
  act(() => select.props.onChange({ target: { value: 'json' } }));
  const before = reader();
  check('implicit format label, full count, enabled native button and preserved live reader status', () => {
    assert.equal(select.parent.type, 'label');
    assert(text(select.parent).includes('Log file format'));
    assert.equal(text(controls()[0]), `Download all ${fixture.length} entries`);
    assert.equal(controls()[0].props.disabled, false);
    assert.equal(before.rows.length, 1);
    assert(before.status.includes('Following paused'));
    assert.equal(component.root.findByProps({ role: 'status' }).props['aria-atomic'], 'true');
  });
  act(() => controls()[0].props.onClick());
  check('dispatch refusal is caught synchronously, surfaced as alert, and leaves reader intact', () => {
    assert.deepEqual(alerts(), ['Could not start the log download. Please try again.']);
    assert.deepEqual(reader(), before);
    assert.equal(attempts.length, 1);
    assert.equal(attempts[0].refused, true);
    assert.equal(attached.size, 0);
    assert.deepEqual(revoked, []);
    assert(resolveObjectURL(created[0]) instanceof Blob);
  });
  const firstBytes = Buffer.from(await attempts[0].blob.arrayBuffer());
  await fs.writeFile(path.join(output, 'refused-dispatch.json'), firstBytes, { flag: 'wx' });
  check('actual captured JSON Blob preserves all retained UTF-16 strings, duplicates and empty entries', () => {
    assert.equal(attempts[0].filename, 'uma-sim-retained-log.json');
    assert.equal(attempts[0].blob.type, 'application/json;charset=utf-8');
    assert.deepEqual(JSON.parse(firstBytes.toString('utf8')), {
      format: 'uma-sim-retained-log/1', scope: 'displayed-retained-history', entryCount: fixture.length, entries: fixture,
    });
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  check('failed attempt releases its real object URL with no timer-driven retry', () => {
    assert.deepEqual(revoked, created);
    assert.equal(resolveObjectURL(created[0]), undefined);
    assert.equal(attempts.length, 1);
    assert.equal(attached.size, 0);
  });
  act(() => component.update(React.createElement(LogPanel, { history: nextFixture, lines: ['summary must stay separate'] })));
  const nextReader = reader();
  failDispatch = false;
  act(() => controls()[0].props.onClick());
  check('explicit retry alone clears failure and captures only the current replacement history', () => {
    assert.deepEqual(alerts(), []);
    assert.deepEqual(reader(), nextReader);
    assert.equal(nextReader.query, 'needle');
    assert.equal(nextReader.rows.length, 0);
    assert.equal(attempts.length, 2);
    assert.equal(attempts[1].refused, false);
    assert.notEqual(attempts[1].url, attempts[0].url);
    assert.equal(attached.size, 0);
  });
  const retryBytes = Buffer.from(await attempts[1].blob.arrayBuffer());
  const firstBytesAfterRetry = Buffer.from(await attempts[0].blob.arrayBuffer());
  await fs.writeFile(path.join(output, 'explicit-retry.json'), retryBytes, { flag: 'wx' });
  check('retry uses current exact entries and cannot rewrite the already captured Blob', () => {
    assert.deepEqual(JSON.parse(retryBytes.toString('utf8')).entries, nextFixture);
    assert.equal(JSON.parse(retryBytes.toString('utf8')).entryCount, nextFixture.length);
    assert.equal(Buffer.compare(firstBytes, firstBytesAfterRetry), 0);
    assert.equal(firstBytes.includes(Buffer.from('summary must stay separate')), false);
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  check('both independent URLs are revoked exactly once; no anchors, fetch, or extra dispatch', () => {
    assert.equal(created.length, 2);
    assert.deepEqual(revoked, created);
    assert.equal(new Set(revoked).size, 2);
    assert(created.every((url) => resolveObjectURL(url) === undefined));
    assert.equal(attached.size, 0);
    assert.equal(fetches, 0);
    assert.equal(attempts.length, 2);
  });
  for (const pin of sourcePins) assert.equal(sha(await fs.readFile(path.join(ui, pin.path))), pin.sha256);
  receipt.result = 'pass';
  receipt.files = [
    { file: 'refused-dispatch.json', bytes: firstBytes.length, sha256: sha(firstBytes) },
    { file: 'explicit-retry.json', bytes: retryBytes.length, sha256: sha(retryBytes) },
  ];
  receipt.object_urls_created = created.length;
  receipt.object_urls_revoked = revoked.length;
  receipt.dispatch_attempts = attempts.length;
  receipt.authored_dispatch_refusals = 1;
  receipt.browser_acceptance = 'unestablished';
  receipt.source_unchanged = true;
} catch (error) {
  receipt.result = 'fail';
  receipt.error = { name: error.name, message: error.message, stack: error.stack };
  process.exitCode = 1;
} finally {
  if (component) act(() => component.unmount());
  await new Promise((resolve) => setTimeout(resolve, 0));
  for (const url of created) original.revoke(url);
  URL.createObjectURL = original.create;
  URL.revokeObjectURL = original.revoke;
  globalThis.fetch = original.fetch;
  if (original.document) Object.defineProperty(globalThis, 'document', original.document);
  else delete globalThis.document;
  receipt.harness_sha256 = sha(await fs.readFile(new URL(import.meta.url)));
  await fs.writeFile(path.join(output, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ result: receipt.result, checks: checks.length, scenario: receipt.scenario, browser_acceptance: 'unestablished' }));
}
