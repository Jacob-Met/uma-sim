import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

export const REVIEW_ROOT = dirname(fileURLToPath(import.meta.url));
export const BASELINE = process.env.UMA_SIM_TEST_REPO_ROOT ?? fileURLToPath(new URL('../../../../', import.meta.url));
export const SOURCE = process.env.UMA_MCP_REVIEW_SOURCE ?? fileURLToPath(new URL('../../mcp-stdio.js', import.meta.url));

export class McpClient {
  constructor(api, source = SOURCE) {
    this.nextId = 1;
    this.pending = new Map();
    this.unclaimed = [];
    this.stderr = '';
    this.buffer = '';
    this.child = spawn(process.execPath, [source], {
      env: { ...process.env, UMA_SIM_API: api },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.child.stdout.setEncoding('utf8');
    this.child.stderr.setEncoding('utf8');
    this.child.stderr.on('data', data => { this.stderr = (this.stderr + data).slice(-16000); });
    this.child.stdout.on('data', data => {
      this.buffer += data;
      for (let boundary; (boundary = this.buffer.indexOf('\n')) >= 0;) {
        const line = this.buffer.slice(0, boundary);
        this.buffer = this.buffer.slice(boundary + 1);
        if (!line.trim()) continue;
        let response;
        try { response = JSON.parse(line); }
        catch (error) { this.rejectAll(new Error(`Non-JSON MCP stdout: ${line.slice(0, 200)}`)); continue; }
        const request = this.pending.get(response.id);
        if (request) {
          this.pending.delete(response.id);
          clearTimeout(request.timeout);
          request.resolve(response);
        } else {
          this.unclaimed.push(response);
        }
      }
    });
    this.child.once('error', error => this.rejectAll(error));
    this.child.once('exit', (code, signal) => this.rejectAll(new Error(`MCP exited ${code}/${signal}: ${this.stderr}`)));
  }

  rejectAll(error) {
    for (const item of this.pending.values()) {
      clearTimeout(item.timeout);
      item.reject(error);
    }
    this.pending.clear();
  }

  request(method, params, { id = this.nextId++, fragment = false } = {}) {
    const response = new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`MCP response timeout for ${method} ${id}: ${this.stderr}`));
      }, 15000);
      this.pending.set(id, { resolve, reject, timeout });
    });
    const wire = Buffer.from(JSON.stringify({ jsonrpc: '2.0', id, method, ...(params === undefined ? {} : { params }) }) + '\r\n');
    if (fragment) {
      // Byte fragments also split multibyte UTF-8. A valid transport must decode
      // the stream, not each incoming buffer independently.
      for (const byte of wire) this.child.stdin.write(Buffer.from([byte]));
    } else {
      this.child.stdin.write(wire);
    }
    return response;
  }

  call(name, args = {}) { return this.request('tools/call', { name, arguments: args }); }

  async stop() {
    if (this.child.exitCode !== null || this.child.signalCode !== null) return;
    const exited = once(this.child, 'exit');
    this.child.kill('SIGTERM');
    await exited;
  }
}

export async function fixture(t, handler) {
  const requests = [];
  const server = createServer(async (req, res) => {
    try {
      let raw = '';
      for await (const chunk of req) raw += chunk.toString('utf8');
      const url = new URL(req.url, 'http://fixture.invalid');
      const entry = { method: req.method, path: url.pathname, query: [...url.searchParams], body: raw ? JSON.parse(raw) : undefined };
      requests.push(entry);
      if (handler) await handler(entry, req, res);
      else {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ received: entry }));
      }
    } catch (error) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ fixtureError: error.message }));
    }
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const api = `http://127.0.0.1:${server.address().port}`;
  const client = new McpClient(api);
  t.after(async () => {
    await client.stop();
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  });
  return { api, client, requests, server };
}

export function toolText(response) {
  assert.equal(response.error, undefined, `Unexpected JSON-RPC error: ${JSON.stringify(response)}`);
  assert.ok(Array.isArray(response.result?.content), `Missing tool content: ${JSON.stringify(response)}`);
  return response.result.content.filter(item => item.type === 'text').map(item => item.text).join('\n');
}

export function toolJson(response) {
  assert.notEqual(response.result?.isError, true, `Unexpected tool execution error: ${JSON.stringify(response)}`);
  return JSON.parse(toolText(response));
}

export function paramsError(response) {
  assert.equal(response.error?.code, -32602, `Expected rejected tool arguments: ${JSON.stringify(response)}`);
  assert.equal(response.result, undefined);
}

export function executionError(response, marker) {
  assert.equal(response.result?.isError, true, `Expected unsuccessful tool result: ${JSON.stringify(response)}`);
  assert.equal(response.error, undefined, 'HTTP execution failures should remain readable tool results');
  assert.ok(toolText(response).includes(marker), `Lost backend diagnostic ${marker}: ${JSON.stringify(response)}`);
}

const UTC_TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;

export function comparableComparison(value) {
  assert.match(value.comparedAt, UTC_TIMESTAMP);
  assert.ok(Number.isFinite(Date.parse(value.comparedAt)));
  // The Rust receiver generates this field on each comparison request. All
  // experiment identities, decisions, outcomes and caveats must still match.
  const { comparedAt, ...experiment } = value;
  return experiment;
}

export function comparableReport(value) {
  const lines = value.split('\n');
  const generated = lines.filter(line => line.startsWith('- Compared at: '));
  assert.equal(generated.length, 1);
  const rendered = generated[0].slice('- Compared at: '.length);
  assert.match(rendered, /^\d{4}\\-\d{2}\\-\d{2}T\d{2}\\:\d{2}\\:\d{2}Z$/);
  // Decode only the accepted timestamp field; preserve all report data bytes.
  const timestamp = rendered.replace(/\\([:-])/g, '$1');
  const parsed = Date.parse(timestamp);
  assert.ok(Number.isFinite(parsed));
  assert.equal(new Date(parsed).toISOString(), timestamp.slice(0, -1) + '.000Z');
  return lines.map(line => line === generated[0] ? '- Compared at: <generated per request>' : line).join('\n');
}
