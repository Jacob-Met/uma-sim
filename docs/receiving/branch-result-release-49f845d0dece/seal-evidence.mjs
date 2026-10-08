import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
const root = "/Users/me/workspace/estate/release-receiving-49f845d0dece";
const selected = "b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c";
const source = path.join(root, "source");
const read = relative => fs.readFileSync(path.join(root, relative));
const json = relative => JSON.parse(read(relative));
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const sha = relative => digest(read(relative));
const write = (relative, value) => fs.writeFileSync(path.join(root, relative), JSON.stringify(value, null, 2) + "\n", { flag: "wx" });
const git = args => execFileSync("git", args, { cwd: source, encoding: "utf8" }).trim();
const manifest = json("artifacts/package-manifest.json");
const build = json("build/build.json"), inputs = json("build/input-verification.json");
const branch = json("receiving/packaged-branch-receiving.json"), ui = json("receiving/embedded-ui.json"), layout = json("receiving/layout-smoke.json");
const coreRelative = path.relative(root, branch.coreReceipt), core = json(coreRelative);
assert.equal(git(["rev-parse", "HEAD"]), selected);
assert.equal(git(["rev-parse", "HEAD^{tree}"]), manifest.sourceTree);
assert.equal(git(["status", "--short"]), "");
assert.equal(sha("source/Cargo.lock"), build.cargoLockSha256);
assert.equal(sha("source/packages/uma-sim-ui/package-lock.json"), build.uiLockSha256);
assert.equal(build.state, "failed");
assert.equal(build.stages.length, 2);
assert(build.stages.every(stage => stage.state === "passed" && stage.exitCode === 0));
assert.equal(inputs.onlyTrackedChange.path, "packages/uma-sim-ui/tsconfig.tsbuildinfo");
assert.equal(sha("source/packages/uma-sim-ui/tsconfig.tsbuildinfo"), inputs.onlyTrackedChange.beforeSha256);
assert.equal(sha("build/tsconfig.generated.tsbuildinfo"), inputs.onlyTrackedChange.afterSha256);
assert.equal(digest(fs.readFileSync(manifest.zip)), manifest.zipSha256);
assert.equal(fs.statSync(manifest.zip).size, manifest.zipBytes);
assert.equal(digest(fs.readFileSync(manifest.executable)), manifest.executableSha256);
assert.equal(sha("target/release/uma-sim"), manifest.executableSha256);
assert.equal(branch.result, "passed");
assert.equal(branch.coreSummary.pass, 7); assert.equal(branch.coreSummary.fail, 0);
assert.equal(branch.coreSummary.observedLoss, false);
assert.equal(branch.allBootsUsePackagedData, true); assert.equal(branch.allBootsExited, true);
assert.equal(branch.nativeSourceBindingMatches, true); assert.equal(branch.sourceDirectoriesRestored, true);
assert.equal(sha(coreRelative), branch.coreReceiptSha256);
assert.equal(core.pass, 7); assert.equal(core.fail, 0); assert.equal(core.observedLoss, false);
assert.equal(core.boots.length, 4);
assert(core.boots.every(boot => boot.health.repoRootPath === manifest.packageRoot && boot.exitedAt));
assert.equal(sha("receiver/branch_publication.original.api.mjs"), branch.originalReceiverSha256);
assert.equal(sha("receiver/branch_publication.packaged.api.mjs"), branch.adapterSha256);
const original = read("receiver/branch_publication.original.api.mjs").toString("utf8");
const adapted = read("receiver/branch_publication.packaged.api.mjs").toString("utf8");
assert.equal(adapted, original.replace("spawn(binary, [String(port)],", 'spawn(binary, ["serve", "--port=" + port],'));
assert.equal(layout.result, "passed"); assert.equal(layout.layoutSmoke.exitCode, 0);
assert.equal(sha("receiving/layout-smoke.log"), layout.layoutSmoke.outputSha256);
assert.equal(ui.result, "passed"); assert.equal(ui.requests.length, 3);
assert(ui.requests.every(request => request.status === 200 && request.match && request.receivedSha256 === request.expectedSha256));
assert.equal(ui.sourceDataAndUiDirectoriesUnavailable, true); assert.equal(ui.sourceDirectoriesRestored, true);
assert.equal(ui.health.repoRootPath, manifest.packageRoot); assert.equal(ui.exit.signal, "SIGTERM");
for (const name of ["research", "knowledge", "packages/uma-sim-ui/dist"]) assert(fs.statSync(path.join(source, name)).isDirectory());
const inventory = (directory, prefix = "") => fs.readdirSync(directory).flatMap(name => {
  const relative = prefix ? prefix + "/" + name : name, file = path.join(directory, name), stat = fs.lstatSync(file);
  if (stat.isDirectory()) return inventory(file, relative);
  assert(stat.isFile(), "Unexpected non-regular file: " + file);
  return [{ path: relative, bytes: stat.size, mode: stat.mode & 0o777, sha256: digest(fs.readFileSync(file)) }];
}).sort((a, b) => a.path.localeCompare(b.path));
const expectedInventory = [...manifest.files].sort((a, b) => a.path.localeCompare(b.path));
assert.equal(expectedInventory.length, 63);
assert.deepEqual(inventory(manifest.packageRoot), expectedInventory);
assert.deepEqual(inventory(path.join(root, "stage/uma-sim-macos-arm64")), expectedInventory);
const final = {
  schema: "uma-sim.mac-release-candidate-receiving-final.v1", sealedAt: new Date().toISOString(),
  repository: "Jacob-Met/uma-sim", selectedSource: selected, sourceTree: manifest.sourceTree,
  selectedRef: "receiving/branch-result-release-49f845d0dece",
  result: "passed: local optimized macOS arm64 ZIP receiving",
  retainedRoot: root, platform: "macos-arm64",
  recipe: { path: manifest.workflowRecipe, gitBlob: manifest.workflowRecipeBlob },
  candidate: { zip: manifest.zip, zipBytes: manifest.zipBytes, zipSha256: manifest.zipSha256,
    executable: manifest.executable, executableBytes: fs.statSync(manifest.executable).size,
    executableSha256: manifest.executableSha256, files: expectedInventory.length,
    inventoryReceipt: "artifacts/package-manifest.json", stagedAndExtractedBytesAndModesMatchAfterAllReceiving: true },
  build: { actualRecipe: build.stages.map(({ exe, args, cwd, state, exitCode }) => ({ exe, args, cwd, state, exitCode })),
    originalReceiptState: build.state, originalGuardError: build.error, guardDispositionReceipt: "build/input-verification.json",
    onlyGeneratedTrackedChange: inputs.onlyTrackedChange, optimizedCompilations: 1, secondBuild: false,
    node: build.node, npm: build.npm, cargo: build.cargo, rustc: build.rustc, platformVersion: build.platform,
    cargoLockSha256: build.cargoLockSha256, uiLockSha256: build.uiLockSha256 },
  receiving: {
    branch: { result: branch.result, pass: core.pass, fail: core.fail, observedLoss: core.observedLoss,
      cases: core.cases, rawReceipt: coreRelative, rawReceiptSha256: branch.coreReceiptSha256,
      wrapperReceipt: "receiving/packaged-branch-receiving.json", startedAt: branch.startedAt, finishedAt: branch.finishedAt,
      sourceDataUnavailable: branch.sourceDirectoriesUnavailableDuringReceiver, allFourBootsUseExtractedData: true, allFourBootsExited: true,
      provenance: branch.sourceProvenance },
    layout: { result: layout.result, receipt: "receiving/layout-smoke.json", existingScript: "scripts/smoke_release_layout.sh",
      sourceDataUnavailable: layout.layoutSmoke.scriptHidOnlyOwnedSourceData, repoRootEnvironment: layout.repoRootEnvironment },
    embeddedUi: { result: ui.result, receipt: "receiving/embedded-ui.json", matchedHttp200Assets: ui.requests.length,
      requests: ui.requests, repoRootEnvironment: ui.repoRootEnvironment, sourceDataAndUiUnavailable: true, childExited: true,
      initialHarnessPreflight: "receiving/embedded-ui-preflight-attempt-1.json", executableRebuilt: false }
  },
  adapter: { source: "uma-sim-core/tests/branch_publication.api.mjs", originalSha256: branch.originalReceiverSha256,
    adaptedSha256: branch.adapterSha256, exactDiff: "receiver/adapter.diff", scope: "Only packaged CLI child argv changed to serve --port; all core receiver logic preserved" },
  fixture: { path: "fixture/portable-checkpoint.json", sha256: sha("fixture/portable-checkpoint.json") },
  finalSource: { head: selected, trackedAndUntrackedStatus: "", researchRestored: true, knowledgeRestored: true, uiDistRestored: true },
  hostedWorkflow: { attemptCount: 1, outcome: "HTTP 401; zero workflow runs", attemptedAt: json("dispatch.json").startedAt,
    attemptReceipt: "dispatch.json", zeroRunReceipt: "dispatch-runs-after-attempt.json", retry: false, runId: null, artifactIds: [] },
  scope: { linuxBuilt: false, windowsBuilt: false, publicReleaseCreated: false, versionTagCreated: false, installedServiceUpdated: false,
    browserInteractionClaimed: false, candidatePackageRetainedNatively: true, completeRawEvidenceRetainedNatively: true }
};
write("final-receipt.json", final);
const prefixes = ["build", "receiver", "fixture", "receiving"];
const evidenceFiles = prefixes.flatMap(prefix => inventory(path.join(root, prefix)).map(item => ({ ...item, path: prefix + "/" + item.path })));
for (const name of ["README.md", "final-receipt.json", "preflight.json", "dispatch.json", "dispatch-runs-after-attempt.json",
  "build-candidate.mjs", "package-candidate.mjs", "receive-packaged.mjs", "receive-embedded-ui.mjs", "receive-embedded-ui-v2.mjs",
  "seal-evidence.mjs", "artifacts/package-manifest.json"]) {
  const stat = fs.statSync(path.join(root, name));
  evidenceFiles.push({ path: name, bytes: stat.size, mode: stat.mode & 0o777, sha256: sha(name) });
}
evidenceFiles.sort((a, b) => a.path.localeCompare(b.path));
assert.equal(new Set(evidenceFiles.map(item => item.path)).size, evidenceFiles.length);
write("evidence-manifest.json", { schema: "uma-sim.receiving-evidence-manifest.v1", createdAt: new Date().toISOString(), selectedSource: selected,
  candidateZipSha256: manifest.zipSha256, files: evidenceFiles });
const archiveRelative = "artifacts/uma-sim-macos-arm64-b8e3da0c-receiving-evidence.tar.gz";
const archive = path.join(root, archiveRelative);
assert(!fs.existsSync(archive));
const archiveMembers = [...evidenceFiles.map(item => item.path), "evidence-manifest.json"].sort();
execFileSync("/usr/bin/tar", ["-czf", archive, ...archiveMembers], { cwd: root, env: { ...process.env, COPYFILE_DISABLE: "1" } });
const archivedMembers = execFileSync("/usr/bin/tar", ["-tzf", archive], { cwd: root, encoding: "utf8" }).trim().split("\n").sort();
assert.deepEqual(archivedMembers, archiveMembers);
const archiveReceipt = { schema: "uma-sim.receiving-evidence-archive.v1", createdAt: new Date().toISOString(), selectedSource: selected,
  archive, archiveBytes: fs.statSync(archive).size, archiveSha256: digest(fs.readFileSync(archive)),
  evidenceManifest: path.join(root, "evidence-manifest.json"), evidenceManifestSha256: sha("evidence-manifest.json"),
  finalReceipt: path.join(root, "final-receipt.json"), finalReceiptSha256: sha("final-receipt.json"),
  files: archiveMembers.length, archiveMembersVerified: true, candidateZipIncluded: false, candidateZip: manifest.zip, candidateZipSha256: manifest.zipSha256 };
write("archive-receipt.json", archiveReceipt);
console.log(JSON.stringify({ finalResult: final.result, candidate: final.candidate, archive: archiveReceipt, evidenceFiles: evidenceFiles.length }, null, 2));
