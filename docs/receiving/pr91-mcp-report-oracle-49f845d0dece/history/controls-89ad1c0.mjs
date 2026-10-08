import test from 'node:test';
import assert from 'node:assert/strict';
import { comparableComparison, comparableReport } from './helpers/mcp-client.mjs';

const plain = '2026-10-08T13:30:30Z';
const escaped = '2026\\-10\\-08T13\\:30\\:30Z';
const tick = String.fromCharCode(96);
const marker = '- Compared at: <generated per request>';
const report = stamp => [
  '# Branch comparison: literal A vs literal B',
  '- Checkpoint: ' + tick + 'saved-checkpoint' + tick + ' (seed 7, turn 4)',
  '- Same checkpoint: true',
  '- Compared at: ' + stamp,
  '',
  '| Branch | literal A | literal B |',
  '| Steps | 2 | 3 |',
  'Literal retained date: 2026-10-08T00:00:00Z',
  '',
].join('\n');

test('report comparison admits plain and Markdown-escaped generated UTC timestamps', () => {
  const expected = report(plain).replace('- Compared at: ' + plain, marker);
  assert.equal(comparableReport(report(plain)), expected);
  assert.equal(comparableReport(report(escaped)), expected);
  assert.equal(comparableReport(report('2027\\-01\\-02T03\\:04\\:05Z')), expected);
});

test('normalization changes only the complete generated line, not identical text in data', () => {
  const literal = 'Literal code: ' + tick + '- Compared at: ' + plain + tick;
  const input = literal + '\n' + report(plain);
  const expected = literal + '\n' + report(plain).replace('- Compared at: ' + plain, marker);
  assert.equal(comparableReport(input), expected);
});

test('wrong branch identity, counts, retained dates and escape bytes stay distinguishable', () => {
  const input = report(plain);
  const normal = comparableReport(input);
  for (const [before, after] of [
    ['literal A', 'different A'],
    ['| Steps | 2 | 3 |', '| Steps | 9 | 3 |'],
    ['2026-10-08T00:00:00Z', '2026-10-09T00:00:00Z'],
    ['saved-checkpoint', 'saved\\-checkpoint'],
  ]) {
    const changed = input.replace(before, after);
    assert.notEqual(comparableReport(changed), normal, before);
    assert.ok(comparableReport(changed).includes(after), before);
  }
});

test('timestamp decoding refuses invalid syntax and escapes outside punctuation', () => {
  for (const stamp of [
    '', 'not a timestamp', plain + ' trailing',
    '2026-10-08T13:30Z', '2026-10-08T13:30:30+00:00',
    '2026-10-08T13:30:30.000Z',
    '\\2026\\-10\\-08T13\\:30\\:30Z',
    '2026\\-10\\-08\\T13\\:30\\:30Z',
    '2026\\\\-10\\-08T13\\:30\\:30Z',
  ]) assert.throws(() => comparableReport(report(stamp)), undefined, stamp);
});

test('timestamp decoding refuses invalid dates accepted by the old shape-only oracle', () => {
  for (const stamp of [
    '2026-13-08T13:30:30Z',
    '2026-10-32T13:30:30Z',
    '2026-10-08T25:30:30Z',
  ]) assert.throws(() => comparableReport(report(stamp)), undefined, stamp);
});

test('missing or duplicate generated lines are never discarded as timestamp variance', () => {
  assert.throws(() => comparableReport(report(plain).replace('- Compared at: ' + plain + '\n', '')));
  assert.throws(() => comparableReport(report(plain) + '- Compared at: ' + plain + '\n'));
  assert.throws(() => comparableReport(report(plain) + '- Compared at: ' + escaped + '\n'));
});

test('JSON comparison remains raw UTC and preserves all experiment data', () => {
  const original = { comparedAt: plain, aId: 'branch-a', bId: 'branch-b', outcomeA: { steps: 2 }, note: 'literal \\- note' };
  const snapshot = structuredClone(original);
  assert.deepEqual(comparableComparison(original), {
    aId: 'branch-a', bId: 'branch-b', outcomeA: { steps: 2 }, note: 'literal \\- note',
  });
  assert.deepEqual(original, snapshot);
  assert.throws(() => comparableComparison({ ...original, comparedAt: escaped }));
  assert.throws(() => comparableComparison({ ...original, comparedAt: '2026-13-08T13:30:30Z' }));
  assert.notDeepEqual(comparableComparison({ ...original, aId: 'wrong-branch' }), comparableComparison(original));
});
