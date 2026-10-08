import fs from "node:fs";
import path from "node:path";
import { spawn, execFileSync } from "node:child_process";
import { createServer } from "node:net";
import { once } from "node:events";
import { createHash } from "node:crypto";
const root = "/Users/me/workspace/estate/release-receiving-49f845d0dece";
const source = path.join(root, "source"), dist = path.join(source, "packages/uma-sim-ui/dist");
const packageRoot = path.join(root, "extracted/uma-sim-macos-arm64"), binary = path.join(packageRoot, "uma-sim");
const output = path.join(root, "receiving"), cwd = path.join(root, "ui-working");
fs.mkdirSync(cwd);
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const index = fs.readFileSync(path.join(dist, "index.html"));
const assets = [...new Set([...index.toString("utf8").matchAll(/(?:src|href)="(\/assets\/[^"]+)"/g)].map(match => match[1]))];
if (!assets.length) throw new Error("No generated UI asset paths found");
const expected = new Map([["/", digest(index)], ...assets.map(url => [url, digest(fs.readFileSync(path.join(dist, url.slice(1))))])]);
const result = { schema: "uma-sim.embedded-ui-receiving.v1", selectedSource: "b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c", startedAt: new Date().toISOString(), binary, cwd, packageDataRoot: packageRoot, repoRootEnvironment: "unset; require ordinary adjacent-data discovery", requests: [], result: "failed" };
const moved = [];
let child, exited, childOutput = "", spawnError;
try {
  for (const [name, from] of [["research", path.join(source, "research")], ["knowledge", path.join(source, "knowledge")], ["ui-dist", dist]]) {
    const hidden = path.join(root, "ui-probe-hidden-" + name);
    if (!fs.existsSync(from) || fs.existsSync(hidden)) throw new Error("Source-isolation precondition failed");
    fs.renameSync(from, hidden); moved.push({ from, hidden });
  }
  result.sourceDataAndUiDirectoriesUnavailable = moved.every(item => !fs.existsSync(item.from));
  const reservation = createServer(); reservation.listen(0, "127.0.0.1"); await once(reservation, "listening");
  const port = reservation.address().port; await new Promise(resolve => reservation.close(resolve));
  const env = { ...process.env }; delete env.UMA_REPO_ROOT; delete env.GIT_DIR; delete env.GIT_WORK_TREE;
  child = spawn(binary, ["serve", "--port=" + port], { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
  result.pid = child.pid; result.port = port;
  exited = new Promise(resolve => child.once("close", (code, signal) => resolve({ code, signal })));
  child.once("error", error => { spawnError = error; });
  child.stdout.on("data", bytes => { childOutput += bytes; }); child.stderr.on("data", bytes => { childOutput += bytes; });
  const base = "http://127.0.0.1:" + port, deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    if (spawnError || child.exitCode !== null) throw spawnError || new Error("Packaged server exited before health");
    try { const response = await fetch(base + "/v1/health", { signal: AbortSignal.timeout(200) }); if (response.ok) { result.health = await response.json(); break; } } catch {}
    await new Promise(resolve => setTimeout(resolve, 20));
  }
  if (result.health?.repoRootPath !== packageRoot) throw new Error("Packaged adjacent data root was not used");
  for (const [url, sha256] of expected) {
    const response = await fetch(base + url, { signal: AbortSignal.timeout(2000) });
    const bytes = Buffer.from(await response.arrayBuffer());
    const received = digest(bytes);
    result.requests.push({ url, status: response.status, bytes: bytes.length, expectedSha256: sha256, receivedSha256: received, match: response.status === 200 && received === sha256 });
  }
  if (!result.requests.every(request => request.match)) throw new Error("Embedded UI bytes differ from the selected build");
  result.result = "passed";
} catch (error) {
  result.error = String(error.stack || error);
} finally {
  if (child) {
    try {
      if (child.exitCode === null && child.signalCode === null) child.kill("SIGTERM");
      let timer;
      result.exit = await Promise.race([exited, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Owned UI server failed to exit")), 5000); })]).finally(() => clearTimeout(timer));
    } catch (error) { result.cleanupError = String(error); result.result = "failed"; }
  }
  fs.writeFileSync(path.join(output, "embedded-ui-server.log"), childOutput);
  const errors = [];
  for (const item of moved.reverse()) {
    try { if (!fs.existsSync(item.from) && fs.existsSync(item.hidden)) fs.renameSync(item.hidden, item.from); else throw new Error("Source restoration refused"); }
    catch (error) { errors.push(String(error)); }
  }
  result.sourceDirectoriesRestored = errors.length === 0;
  if (errors.length) { result.restoreErrors = errors; result.result = "failed"; }
  result.trackedSourceAfter = execFileSync("git", ["status", "--short"], { cwd: source, encoding: "utf8" }).trim();
  if (result.trackedSourceAfter) result.result = "failed";
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(path.join(output, "embedded-ui.json"), JSON.stringify(result, null, 2) + "\n");
  console.log(JSON.stringify(result, null, 2));
  if (result.result !== "passed") process.exitCode = 1;
}
