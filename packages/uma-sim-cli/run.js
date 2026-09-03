#!/usr/bin/env node
/**
 * Spawns uma-sim CLI. Prefers Rust binary (uma-sim-core), falls back to Gradle JVM.
 * Usage: node run.js start --seed=42 --speed=20
 */
const { spawnSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "../..");
const args = process.argv.slice(2);

const rustCandidates = [
  path.join(repoRoot, "uma-sim-core", "target", "release", process.platform === "win32" ? "uma-sim.exe" : "uma-sim"),
  path.join(repoRoot, "uma-sim-core", "target", "debug", process.platform === "win32" ? "uma-sim.exe" : "uma-sim"),
];

const rustBin = rustCandidates.find((p) => fs.existsSync(p));

let result;
if (rustBin) {
  result = spawnSync(rustBin, args, { cwd: repoRoot, stdio: "inherit", shell: false });
} else {
  const legacy = path.join(repoRoot, "legacy", "uma-sim-kotlin");
  const gradlew = path.join(legacy, process.platform === "win32" ? "gradlew.bat" : "gradlew");
  const simArgs = args.join(" ");
  result = spawnSync(
    gradlew,
    [`:sim-engine:runSim`, `-PsimArgs=${simArgs}`],
    { cwd: legacy, stdio: "inherit", shell: true },
  );
}

process.exit(result.status ?? 1);
