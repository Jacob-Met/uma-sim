// Independent focused receiving of the accepted dangling-symlink target guard.
// Reuses the already qualified reviewer collision shim and exact native binary.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const [candidate, shim, output, source] = process.argv.slice(2);
assert.equal(process.argv.length, 6);
assert.match(output, /^\/dev\/shm\/integration-69570d292200-cli-target-guard-/);
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const expectedBinary = 'ab4bbcaf06e55234abe688ec50cce7e8307e155fdc2a1dacbde769c28f61ff03';
const expectedSession = '95653ec2bc94c22598443c9e2e1c9d0301f2ec0183f0856d8c5baaa88098d637';
const binaryBefore = sha(fs.readFileSync(candidate));
assert.equal(binaryBefore, expectedBinary);
const sessionPath = path.join(source, 'uma-sim-core/src/session.rs');
assert.equal(sha(fs.readFileSync(sessionPath)), expectedSession);
assert.ok(fs.statSync(shim).isFile());
fs.mkdirSync(output);
const sessionDirectory = path.join(output, '.uma-sim');
fs.mkdirSync(sessionDirectory);
fs.writeFileSync(path.join(sessionDirectory, 'unrelated.txt'), 'independent target-guard sentinel\n');
const childEnvironment = { ...process.env };
for (const key of ['LD_PRELOAD', 'HAMON_REVIEW_PHASE', 'HAMON_REVIEW_DIR', 'HAMON_REVIEW_TARGET', 'UMA_POLICY_CMD', 'LLVM_PROFILE_FILE']) delete childEnvironment[key];
childEnvironment.REVIEWER_SHIM_PATH = shim;
childEnvironment.REVIEWER_SESSION_DIRECTORY = sessionDirectory;
const script = [
  'set -e',
  'ln -s ".session-$$-0.tmp" .uma-sim/session.json',
  'export LD_PRELOAD="$REVIEWER_SHIM_PATH"',
  'export HAMON_REVIEW_PHASE=collision',
  'export HAMON_REVIEW_DIR="$REVIEWER_SESSION_DIRECTORY"',
  'export HAMON_REVIEW_TARGET="$REVIEWER_SESSION_DIRECTORY/session.json"',
  'exec "$@"'
].join('\n');
const result = spawnSync('/bin/bash', ['-c', script, 'independent-target-guard', candidate,
  'start', '--seed=42', '--scenario=ura', '--dialogue=off', '--race-model=stub', '--deck=10001'],
  { cwd: output, env: childEnvironment, encoding: 'utf8', timeout: 30000, maxBuffer: 8*1024*1024 });
fs.writeFileSync(path.join(output, 'start.stdout'), result.stdout ?? '');
fs.writeFileSync(path.join(output, 'start.stderr'), result.stderr ?? '');
assert.ifError(result.error);
assert.equal(result.signal, null);
assert.equal(result.status, 0, result.stderr);
const link = path.join(sessionDirectory, 'session.json');
assert.ok(fs.lstatSync(link).isSymbolicLink());
const targetName = '.session-' + result.pid + '-0.tmp';
assert.equal(fs.readlinkSync(link), targetName);
const collision = result.stderr.match(/^HAMON_INDEPENDENT_COLLISION (.+)$/m);
assert.ok(collision, 'native exclusive allocation actually intercepted');
const expectedCollision = path.join(sessionDirectory, '.session-' + result.pid + '-1.tmp');
assert.equal(collision[1], expectedCollision, 'first allocation skipped destination counter zero');
assert.equal(fs.readFileSync(expectedCollision, 'utf8'), 'independent temporary-name collision sentinel\n');
const target = path.join(sessionDirectory, targetName);
const bytes = fs.readFileSync(target);
assert.ok(bytes.length > 1024, 'complete new native snapshot');
JSON.parse(bytes);
assert.deepEqual(fs.readFileSync(link), bytes);
assert.equal(fs.readFileSync(path.join(sessionDirectory, 'unrelated.txt'), 'utf8'), 'independent target-guard sentinel\n');
assert.deepEqual(fs.readdirSync(sessionDirectory).sort(), ['session.json', 'unrelated.txt', targetName, path.basename(expectedCollision)].sort());
const reload = spawnSync(candidate, ['state'], { cwd: output, env: childEnvironment, encoding: 'utf8', timeout: 30000 });
fs.writeFileSync(path.join(output, 'state.stdout'), reload.stdout ?? '');
fs.writeFileSync(path.join(output, 'state.stderr'), reload.stderr ?? '');
assert.ifError(reload.error);
assert.equal(reload.signal, null);
assert.equal(reload.status, 0, reload.stderr);
assert.equal(reload.stderr, '');
assert.ok(reload.stdout.length && !reload.stdout.includes('No session.'), 'new configured target reloads');
assert.deepEqual(fs.readFileSync(target), bytes);
assert.equal(sha(fs.readFileSync(candidate)), expectedBinary);
assert.equal(sha(fs.readFileSync(sessionPath)), expectedSession);
const receipt = {
  result: 'PASS', head: 'acbe58aeface82d7e3cac1134c6d7126a910bb15',
  candidateSha256: expectedBinary, sessionSha256: expectedSession,
  reviewerProgramSha256: sha(fs.readFileSync(fileURLToPath(import.meta.url))),
  shimSha256: sha(fs.readFileSync(shim)), childPid: result.pid,
  sessionLink: fs.readlinkSync(link), firstExclusiveCollision: collision[1],
  targetBytes: bytes.length, targetSha256: sha(bytes), nativeStateReload: true,
  unrelatedBytesPreserved: true, collisionBytesPreserved: true, sourceAndBinaryUnchanged: true,
  start: { exit: result.status, stdout: result.stdout, stderr: result.stderr },
  state: { exit: reload.status, stdout: reload.stdout, stderr: reload.stderr },
  boundary: 'Reviewer-authored focused program, executed unchanged by native binary owner. After-correction observation only; no uncorrected native binary was executed for this edge.'
};
fs.writeFileSync(path.join(output, 'receiving.json'), JSON.stringify(receipt, null, 2) + '\n');
process.stdout.write(JSON.stringify(receipt, null, 2) + '\n');
