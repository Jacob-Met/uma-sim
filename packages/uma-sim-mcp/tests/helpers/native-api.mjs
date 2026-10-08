import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { mkdir, mkdtemp, writeFile, readFile, access, stat } from 'node:fs/promises';
import { constants } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { BASELINE, SOURCE, McpClient } from './mcp-client.mjs';
const ROOT = process.env.UMA_SIM_TEST_OUTPUT_DIR ?? join(tmpdir(), 'uma-sim-mcp-native');
const CONFIGURED_BINARY = process.env.UMA_SIM_TEST_API_BIN;
const BINARY = CONFIGURED_BINARY ? resolve(CONFIGURED_BINARY) : CONFIGURED_BINARY;

export async function actualServer(t) {
  assert.ok(BINARY, 'UMA_SIM_TEST_API_BIN must name an executable API binary');
  assert.ok((await stat(BINARY)).isFile(), 'UMA_SIM_TEST_API_BIN must name a file');
  await access(BINARY, constants.X_OK);
  await mkdir(`${ROOT}/native-runs`, { recursive: true });
  const cwd = await mkdtemp(`${ROOT}/native-runs/review-`);
  const reserver = createServer();
  reserver.listen(0, '127.0.0.1');
  await once(reserver, 'listening');
  const port = reserver.address().port;
  await new Promise(resolve => reserver.close(resolve));
  const api = `http://127.0.0.1:${port}`;
  const env = { ...process.env, UMA_REPO_ROOT: BASELINE };
  delete env.UMA_POLICY_CMD;
  const child = spawn(BINARY, [String(port)], { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  for (const stream of [child.stdout, child.stderr]) stream.on('data', data => { output = (output + data).slice(-50000); });
  const client = new McpClient(api);
  t.after(async () => {
    await client.stop();
    if (child.exitCode === null && child.signalCode === null) {
      const exited = once(child, 'exit');
      child.kill('SIGTERM');
      await exited;
    }
    await writeFile(`${cwd}/server.log`, output);
    await writeFile(`${cwd}/provenance.json`, JSON.stringify({ receivingCommit: process.env.UMA_SIM_TEST_RECEIVING_COMMIT ?? null, repoRoot: BASELINE, mcpSource: SOURCE, mcpSha256: createHash('sha256').update(await readFile(SOURCE)).digest('hex'), binary: BINARY, binarySha256: createHash('sha256').update(await readFile(BINARY)).digest('hex'), cwd }, null, 2) + '\n');
  });
  for (let attempt = 0; attempt < 200; attempt++) {
    if (child.exitCode !== null) throw new Error(`API exited: ${output}`);
    try {
      const health = await fetch(`${api}/v1/health`, { signal: AbortSignal.timeout(200) });
      if (health.ok) break;
    } catch {}
    if (attempt === 199) throw new Error(`API failed readiness: ${output}`);
    await new Promise(resolve => setTimeout(resolve, 25));
  }
  const direct = async (method, path, body) => {
    const response = await fetch(`${api}${path}`, {
      method,
      ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      signal: AbortSignal.timeout(15000),
    });
    const text = await response.text();
    let data;
    try { data = JSON.parse(text); } catch {}
    return { status: response.status, data, text, type: response.headers.get('content-type') };
  };
  const state = async session => {
    const response = await direct('GET', `/v1/run/state?session=${encodeURIComponent(session)}`);
    assert.equal(response.status, 200, response.text);
    return response.data;
  };
  return { api, client, direct, state, cwd };
}
