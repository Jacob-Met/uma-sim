import test from 'node:test';
import assert from 'node:assert/strict';
import { comparableComparison, comparableReport } from './helpers/mcp-client.mjs';

const raw = '2026-10-08T13:30:30Z';
const renderStamp = stamp => stamp.replace(/[-:]/g, ch => '\\' + ch);
const escaped = renderStamp(raw);
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

test('preserve the current fully escaped whole-second timestamp format', () => {
  const expected = report(escaped).replace('- Compared at: ' + escaped, marker);
  assert.equal(comparableReport(report(escaped)), expected);
  assert.equal(comparableReport(report(renderStamp('2027-01-02T03:04:05Z'))), expected);
  assert.throws(() => comparableReport(report(raw)), 'current source requires escaped punctuation');
  assert.throws(() => comparableReport(report(renderStamp('2026-10-08T13:30:30.123456Z'))));
});

test('normalize only the whole generated line and retain identical text inside data', () => {
  const literal = 'Literal code: ' + tick + '- Compared at: ' + escaped + tick;
  const input = literal + '\n' + report(escaped);
  const expected = literal + '\n' + report(escaped).replace('- Compared at: ' + escaped, marker);
  assert.equal(comparableReport(input), expected);
});

test('wrong branch identity, counts, retained dates and escape bytes remain distinguishable', () => {
  const input = report(escaped);
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

test('invalid timestamp syntax and unknown escape sequences refuse', () => {
  for (const stamp of [
    '', 'not a timestamp', escaped + ' trailing',
    '2026\\-10\\-08T13:30:30Z',
    '\\2026\\-10\\-08T13\\:30\\:30Z',
    '2026\\-10\\-08\\T13\\:30\\:30Z',
    '2026\\\\-10\\-08T13\\:30\\:30Z',
  ]) assert.throws(() => comparableReport(report(stamp)), undefined, stamp);
});

test('the escaped shape cannot admit unparseable month, day or hour values', () => {
  for (const stamp of ['2026-13-08T13:30:30Z', '2026-10-32T13:30:30Z', '2026-10-08T25:30:30Z']) {
    assert.throws(() => comparableReport(report(renderStamp(stamp))), undefined, stamp);
  }
});

test('missing or duplicate generated lines cannot disappear as timestamp variance', () => {
  assert.throws(() => comparableReport(report(escaped).replace('- Compared at: ' + escaped + '\n', '')));
  assert.throws(() => comparableReport(report(escaped) + '- Compared at: ' + escaped + '\n'));
  assert.throws(() => comparableReport(report(escaped) + '- Compared at: ' + renderStamp('2027-01-02T03:04:05Z') + '\n'));
});

test('the JSON helper keeps its existing raw UTC and experiment identity contract', () => {
  const original = { comparedAt: raw, aId: 'branch-a', bId: 'branch-b', outcomeA: { steps: 2 }, note: 'literal \\- note' };
  const snapshot = structuredClone(original);
  assert.deepEqual(comparableComparison(original), {
    aId: 'branch-a', bId: 'branch-b', outcomeA: { steps: 2 }, note: 'literal \\- note',
  });
  assert.deepEqual(original, snapshot);
  assert.throws(() => comparableComparison({ ...original, comparedAt: escaped }));
  assert.notDeepEqual(comparableComparison({ ...original, aId: 'wrong-branch' }), comparableComparison(original));
});

test('valid Gregorian leap days remain accepted in the current escaped format', () => {
  const expected = report(escaped).replace('- Compared at: ' + escaped, marker);
  for (const stamp of ['2024-02-29T23:59:59Z', '2000-02-29T00:00:00Z']) {
    assert.equal(comparableReport(report(renderStamp(stamp))), expected);
  }
});

test('finite Date.parse normalization cannot hide day or midnight rollover', () => {
  for (const stamp of [
    '2026-02-30T12:34:29Z', '2026-02-29T12:34:29Z',
    '2026-04-31T12:34:29Z', '1900-02-29T12:34:29Z',
    '2026-10-08T24:00:00Z',
  ]) {
    assert.ok(Number.isFinite(Date.parse(stamp)), 'counterexample reaches finite Date.parse');
    assert.throws(() => comparableReport(report(renderStamp(stamp))), undefined, stamp);
  }
});
