import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const [name, cwd, program, ...args] = process.argv.slice(2);
assert.ok(name && cwd && program);
assert.match(name, /^[a-z0-9-]+$/);
const output = path.dirname(fileURLToPath(import.meta.url));
for (const extension of ['stdout', 'stderr', 'json']) assert.ok(!fs.existsSync(path.join(output, name + '.' + extension)), 'Use a new gate name.');
const root = '/workspace/scratch/acd057031fb2/agents/production';
const toolchain = '/workspace/scratch/7183b7a84620/agents/native/toolchain/bin';
const overrides = {
  RUSTC: toolchain + '/rustc', RUSTDOC: toolchain + '/rustdoc',
  CARGO_HOME: root + '/uma-report-cargo-home', CARGO_TARGET_DIR: root + '/uma-release-target',
  CARGO_NET_OFFLINE: 'true', CARGO_INCREMENTAL: '0', CARGO_BUILD_JOBS: '2',
  CARGO_PROFILE_RELEASE_DEBUG: '0', CARGO_PROFILE_RELEASE_STRIP: 'symbols',
  npm_config_cache: root + '/uma-release-npm-cache', TMPDIR: root + '/uma-release-tmp',
  PYTHONDONTWRITEBYTECODE: '1',
};
const startedAt = new Date().toISOString();
const result = spawnSync(program, args, { cwd, env: { ...process.env, ...overrides }, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
const stdout = result.stdout || '';
const stderr = result.stderr || '';
fs.writeFileSync(path.join(output, name + '.stdout'), stdout);
fs.writeFileSync(path.join(output, name + '.stderr'), stderr);
const hash = value => createHash('sha256').update(value).digest('hex');
const receipt = {
  name, program, args, cwd, startedAt, finishedAt: new Date().toISOString(),
  exitCode: result.status, signal: result.signal, error: result.error?.message ?? null,
  environmentOverrides: overrides, stdoutSha256: hash(stdout), stderrSha256: hash(stderr),
  passed: result.status === 0 && !result.error,
};
fs.writeFileSync(path.join(output, name + '.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log(JSON.stringify({ ...receipt, stdout, stderr }));
if (!receipt.passed) process.exitCode = 1;
