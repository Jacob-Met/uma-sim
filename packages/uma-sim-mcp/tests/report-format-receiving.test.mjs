import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, toolJson, toolText, executionError } from './helpers/mcp-client.mjs';

test('report receiving: HTTP errors retain raw bodies before format decoding and healthy text remains text', { timeout: 15000 }, async t => {
  // These are authored HTTP contract fixtures, not native report contents.
  // This targets the API format adapter used when composing the career-lab
  // text endpoint with the independently owned HTTP error handler.
  const markdown = '{"looks":"like JSON"}\n# Actual report text\n';
  const report = { a: 'left', b: 'right', firstDivergence: { kind: 'decision' } };
  const cases = [
    { status: 503, body: 'upstream refused: 雪\nretained trailing spaces  ', format: 'markdown' },
    { status: 409, body: '["conflict",{"branch":"left"}]', format: 'json' },
    { status: 404, body: '', format: 'markdown' },
    { status: 200, body: markdown, format: 'markdown' },
    { status: 200, body: JSON.stringify(report), format: 'json' },
  ];
  let index = 0;
  const h = await fixture(t, (_request, _req, res) => {
    const item = cases[index++];
    assert.ok(item, 'bridge retried or issued an unexpected report request');
    res.writeHead(item.status, { 'Content-Type': item.format === 'json' ? 'application/json' : 'text/markdown; charset=utf-8' });
    res.end(item.body);
  });
  for (const [position, item] of cases.entries()) {
    const args = { a: 'left:雪&injected=wrong', b: 'right', ...(position === 0 ? {} : { format: item.format }) };
    const response = await h.client.call('sim_lab_report', args);
    if (item.status !== 200) {
      executionError(response, `HTTP ${item.status}`);
      assert.equal(toolText(response), `uma-sim API returned HTTP ${item.status}${item.body ? `: ${item.body}` : ''}`);
    } else if (item.format === 'markdown') {
      assert.notEqual(response.result?.isError, true);
      assert.equal(toolText(response), markdown, 'text report was parsed, quoted, trimmed, or escaped');
    } else {
      assert.deepEqual(toolJson(response), report);
    }
  }
  assert.equal(index, cases.length, 'HTTP errors must not trigger automatic retries');
  assert.equal(h.requests.length, cases.length);
  for (const [position, request] of h.requests.entries()) {
    assert.equal(request.method, 'GET');
    assert.equal(request.path, '/v1/lab/report');
    assert.equal(request.body, undefined);
    assert.deepEqual(Object.fromEntries(request.query), { a: 'left:雪&injected=wrong', b: 'right', format: cases[position].format });
  }
  assert.deepEqual(h.client.unclaimed, []);
  t.diagnostic(JSON.stringify({ case: 'report-format-composition', statuses: cases.map(item => item.status), requests: h.requests.length, healthyMarkdownBytes: Buffer.byteLength(markdown), jsonReport: report }));
});
