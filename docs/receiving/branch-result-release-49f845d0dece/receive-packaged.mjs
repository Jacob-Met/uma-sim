import fs from "node:fs";
import path from "node:path";
import { spawnSync, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
const root = "/Users/me/workspace/estate/release-receiving-49f845d0dece";
const source = path.join(root, "source"), output = path.join(root, "receiving");
const packageRoot = path.join(root, "extracted/uma-sim-macos-arm64");
const binary = path.join(packageRoot, "uma-sim");
const selected = "b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c";
const adapter = path.join(root, "receiver/branch_publication.packaged.api.mjs");
const fixture = path.join(root, "fixture/portable-checkpoint.json");
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const manifest = JSON.parse(fs.readFileSync(path.join(root, "artifacts/package-manifest.json"), "utf8"));
if (digest(fs.readFileSync(binary)) !== manifest.executableSha256) throw new Error("Extracted executable differs");
if (digest(fs.readFileSync(adapter)) !== "feb19ed7e10a728651dfb032487161ef38999d948c69e3349242beedf325cf78") throw new Error("Adapter differs");
const resultFile = path.join(output, "packaged-branch-receiving.json");
if (fs.existsSync(resultFile)) throw new Error("Receiving result exists; preserve it");
const env = {
  ...process.env, GIT_DIR: path.join(source, ".git"),
  UMA_LAB_REPO: packageRoot, UMA_LAB_BINARY: binary, UMA_LAB_FIXTURE: fixture,
  UMA_LAB_OUTPUT_ROOT: path.join(output, "branch"), UMA_LAB_SOURCE_REF: selected,
  UMA_LAB_BINARY_BUILD: selected, UMA_LAB_EXPECT: "preserved"
};
delete env.GIT_WORK_TREE;
const observed = execFileSync("git", ["rev-parse", "HEAD"], { cwd: packageRoot, env, encoding: "utf8" }).trim();
if (observed !== selected) throw new Error("Real source object store pin differs");
if (execFileSync("git", ["status", "--short"], { cwd: source, encoding: "utf8" }).trim()) throw new Error("Source tree is not clean");
const result = {
  schema: "uma-sim.packaged-branch-receiving.v1", startedAt: new Date().toISOString(),
  selectedSource: selected, executable: binary, executableSha256: manifest.executableSha256,
  zipSha256: manifest.zipSha256, packageDataRoot: packageRoot,
  originalReceiverSha256: "d0dfc775ed80d91cbd38c33dc04dd234694a541fc27398cf0be018d60abec7ac",
  adapterSha256: digest(fs.readFileSync(adapter)),
  sourceProvenance: { gitDir: env.GIT_DIR, commandWorkingDirectory: packageRoot, ref: selected, method: "Real pinned Git object store supplies provenance; runtime data root is the extracted ZIP" },
  sourceDirectoriesUnavailableDuringReceiver: false, sourceDirectoriesRestored: false,
  command: ["node", adapter], environmentOverrides: Object.fromEntries(Object.entries(env).filter(([name]) => name === "GIT_DIR" || name.startsWith("UMA_LAB_")))
};
const moved = [];
try {
  for (const name of ["research", "knowledge"]) {
    const from = path.join(source, name), hidden = path.join(source, name + ".packaged-receiving-hidden");
    if (!fs.existsSync(from) || fs.existsSync(hidden)) throw new Error("Owned-source data isolation precondition failed");
    fs.renameSync(from, hidden); moved.push({ from, hidden });
  }
  result.sourceDirectoriesUnavailableDuringReceiver = moved.every(item => !fs.existsSync(item.from));
  fs.writeFileSync(resultFile, JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  const run = spawnSync("node", [adapter], { cwd: path.join(root, "launcher-check"), env, encoding: "utf8", timeout: 120000 });
  fs.writeFileSync(path.join(output, "packaged-branch-receiver.stdout.log"), run.stdout || "");
  fs.writeFileSync(path.join(output, "packaged-branch-receiver.stderr.log"), run.stderr || "");
  Object.assign(result, { exitCode: run.status, signal: run.signal });
  const summary = JSON.parse((run.stdout || "").trim().split("\n").at(-1));
  const corePath = path.join(summary.output, "receipt.json");
  const core = JSON.parse(fs.readFileSync(corePath, "utf8"));
  Object.assign(result, { coreReceipt: corePath, coreReceiptSha256: digest(fs.readFileSync(corePath)), coreSummary: summary,
    allBootsUsePackagedData: core.boots.every(boot => boot.health.repoRootPath === packageRoot),
    allBootsExited: core.boots.every(boot => Boolean(boot.exitedAt)),
    nativeSourceBindingMatches: core.source.main === selected && core.source.sourceRef === selected && core.nativeBinary.sha256 === manifest.executableSha256 });
  result.result = run.status === 0 && summary.pass === 7 && summary.fail === 0 && result.allBootsUsePackagedData && result.allBootsExited && result.nativeSourceBindingMatches ? "passed" : "failed";
} catch (error) {
  result.result = "failed"; result.error = String(error.stack || error); process.exitCode = 1;
} finally {
  const restoreErrors = [];
  for (const item of moved.reverse()) {
    try { if (fs.existsSync(item.hidden) && !fs.existsSync(item.from)) fs.renameSync(item.hidden, item.from); else throw new Error("Owned-source data restoration refused"); }
    catch (error) { restoreErrors.push(String(error)); }
  }
  result.sourceDirectoriesRestored = restoreErrors.length === 0 && ["research", "knowledge"].every(name => fs.existsSync(path.join(source, name)));
  if (restoreErrors.length) { result.restoreErrors = restoreErrors; result.result = "failed"; }
  result.trackedSourceAfter = execFileSync("git", ["status", "--short"], { cwd: source, encoding: "utf8" }).trim();
  if (result.trackedSourceAfter) result.result = "failed";
  result.finishedAt = new Date().toISOString();
  fs.writeFileSync(resultFile, JSON.stringify(result, null, 2) + "\n");
  console.log(JSON.stringify(result, null, 2));
  if (result.result !== "passed") process.exitCode = 1;
}
