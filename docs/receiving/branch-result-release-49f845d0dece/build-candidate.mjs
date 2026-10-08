import fs from "node:fs";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const root = "/Users/me/workspace/estate/release-receiving-49f845d0dece";
const source = path.join(root, "source");
const selected = "b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c";
const output = path.join(root, "build");
fs.mkdirSync(output, { recursive: true });
const receiptPath = path.join(output, "build.json");
if (fs.existsSync(receiptPath)) throw new Error("A bounded build record already exists; inspect before any retry");
const command = (exe, args, cwd = source) => {
  const r = spawnSync(exe, args, { cwd, encoding: "utf8" });
  if (r.status !== 0) throw new Error(exe + " preflight failed");
  return r.stdout.trim();
};
const digest = file => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const available = () => { const s = fs.statfsSync(root); return s.bavail * s.bsize; };
if (command("git", ["rev-parse", "HEAD"]) !== selected) throw new Error("Source pin differs");
if (spawnSync("git", ["diff", "--quiet", "HEAD"], { cwd: source }).status !== 0) throw new Error("Tracked source is dirty");
if (available() < 1.5 * 1024 ** 3) throw new Error("Less than 1.5 GiB free before build");
const uiLock = path.join(source, "packages/uma-sim-ui/package-lock.json");
const cargoLock = path.join(source, "Cargo.lock");
const receipt = {
  schema: "uma-sim.local-release-build.v1", selectedSource: selected,
  sourceTree: command("git", ["rev-parse", "HEAD^{tree}"]),
  startedAt: new Date().toISOString(), source, target: path.join(root, "target"),
  cargoHome: "/Users/me/.cargo", node: process.version, npm: command("npm", ["--version"]),
  cargo: command("cargo", ["--version"]), rustc: command("rustc", ["--version"]),
  architecture: command("uname", ["-m"]), platform: command("sw_vers", ["-productVersion"]),
  cargoLockSha256: digest(cargoLock), uiLockSha256: digest(uiLock),
  dependencyCustody: "UI dependency tree cloned from the retired owner's matching lock; all output paths isolated",
  initialAvailableBytes: available(), minimumAvailableBytes: available(),
  diskFloorBytes: 512 * 1024 ** 2, maxCargoMilliseconds: 600000,
  state: "running", stages: []
};
const save = () => fs.writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + "\n");
fs.writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + "\n", { flag: "wx" });
const env = { ...process.env, CARGO_HOME: receipt.cargoHome, CARGO_TARGET_DIR: receipt.target };
async function run(name, exe, args, cwd, maximumMilliseconds) {
  const log = path.join(output, name + ".log");
  const fd = fs.openSync(log, "wx");
  const stage = { name, exe, args, cwd, log, startedAt: new Date().toISOString(), state: "running" };
  receipt.stages.push(stage); save();
  console.log(JSON.stringify({ stage: name, state: "started", startedAt: stage.startedAt }));
  await new Promise((resolve, reject) => {
    const child = spawn(exe, args, { cwd, env, detached: true, stdio: ["ignore", fd, fd] });
    stage.pid = child.pid; save();
    let settled = false, stopped = false, killTimer;
    const stop = reason => {
      if (stopped || settled) return;
      stopped = true; stage.stopReason = reason; save();
      try { process.kill(-child.pid, "SIGTERM"); } catch {}
      killTimer = setTimeout(() => { try { process.kill(-child.pid, "SIGKILL"); } catch {} }, 5000);
    };
    const timeout = setTimeout(() => stop("bounded stage timeout"), maximumMilliseconds);
    const disk = setInterval(() => {
      const bytes = available();
      receipt.minimumAvailableBytes = Math.min(receipt.minimumAvailableBytes, bytes);
      if (bytes < receipt.diskFloorBytes) stop("free disk below 512 MiB floor");
    }, 10000);
    const finish = (code, signal, error) => {
      if (settled) return;
      settled = true; clearTimeout(timeout); clearInterval(disk); clearTimeout(killTimer); fs.closeSync(fd);
      Object.assign(stage, { finishedAt: new Date().toISOString(), exitCode: code, signal, state: code === 0 && !stopped ? "passed" : "failed" });
      save(); console.log(JSON.stringify({ stage: name, state: stage.state, exitCode: code, signal }));
      if (stage.state === "passed") resolve(); else reject(error || new Error(name + " failed; see retained log"));
    };
    child.once("error", error => finish(null, null, error));
    child.once("close", (code, signal) => finish(code, signal));
  });
}
try {
  await run("ui", "npm", ["run", "build"], path.join(source, "packages/uma-sim-ui"), 120000);
  await run("cargo-release", "cargo", ["build", "--locked", "--release", "--features", "embed-ui", "-p", "uma-sim-core", "--jobs", "1"], source, receipt.maxCargoMilliseconds);
  if (command("git", ["rev-parse", "HEAD"]) !== selected || spawnSync("git", ["diff", "--quiet", "HEAD"], { cwd: source }).status !== 0) throw new Error("Source changed during build");
  if (digest(cargoLock) !== receipt.cargoLockSha256 || digest(uiLock) !== receipt.uiLockSha256) throw new Error("Lockfile changed");
  const binary = path.join(receipt.target, "release/uma-sim");
  Object.assign(receipt, { state: "passed", executable: binary, executableSha256: digest(binary), executableBytes: fs.statSync(binary).size, executableFile: command("file", [binary]), finalAvailableBytes: available() });
} catch (error) {
  Object.assign(receipt, { state: "failed", error: String(error.message), finalAvailableBytes: available() });
  process.exitCode = 1;
} finally {
  receipt.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ state: receipt.state, selectedSource: selected, executableSha256: receipt.executableSha256, receipt: receiptPath, error: receipt.error }));
}
