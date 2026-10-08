import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, toolJson, toolText } from './helpers/mcp-client.mjs';

test('success payload receiving: legacy JSON strings stay encoded while default Markdown reports stay literal', { timeout: 15000 }, async t => {
  // Authored HTTP payloads exercise the formatter without inventing a native
  // career or report result. The report's JSON mode follows legacy encoding.
  const payload = 'line one\nline two: "quoted" and 雪';
  const cases = [
    { tool: 'sim_text', args: { session: 'literal-reader' }, path: '/v1/run/text', query: { session: 'literal-reader' }, json: true },
    { tool: 'sim_lab_report', args: { a: 'left', b: 'right' }, path: '/v1/lab/report', query: { a: 'left', b: 'right', format: 'markdown' }, json: false },
    { tool: 'sim_lab_report', args: { a: 'left', b: 'right', format: 'json' }, path: '/v1/lab/report', query: { a: 'left', b: 'right', format: 'json' }, json: true },
  ];
  let index = 0;
  const h = await fixture(t, (_entry, _req, res) => {
    const item = cases[index++];
    assert.ok(item, 'unexpected success-payload request');
    res.writeHead(200, { 'Content-Type': item.json ? 'application/json' : 'text/markdown; charset=utf-8' });
    res.end(item.json ? JSON.stringify(payload) : payload);
  });
  for (const item of cases) {
    const response = await h.client.call(item.tool, item.args);
    assert.notEqual(response.result?.isError, true);
    assert.equal(toolText(response), item.json ? JSON.stringify(payload, null, 2) : payload);
    if (item.json) assert.equal(toolJson(response), payload, 'legacy JSON-decoding callers lost the original string');
  }
  assert.equal(index, cases.length);
  assert.equal(h.requests.length, cases.length);
  for (const [position, entry] of h.requests.entries()) {
    assert.equal(entry.method, 'GET');
    assert.equal(entry.path, cases[position].path);
    assert.equal(entry.body, undefined);
    assert.deepEqual(Object.fromEntries(entry.query), cases[position].query);
  }
  assert.deepEqual(h.client.unclaimed, []);
  t.diagnostic(JSON.stringify({ case: 'success-payload-owner-contract', requests: h.requests.length, modes: ['legacy JSON string', 'default raw Markdown', 'JSON report string'] }));
});
