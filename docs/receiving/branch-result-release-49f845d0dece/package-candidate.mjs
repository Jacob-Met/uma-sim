import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
const root = "/Users/me/workspace/estate/release-receiving-49f845d0dece";
const source = path.join(root, "source");
const selected = "b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c";
const input = JSON.parse(fs.readFileSync(path.join(root, "build/input-verification.json"), "utf8"));
if (input.selectedSource !== selected || input.sourceStateAfterPreservingGeneratedEvidence !== "clean at selected source") throw new Error("Build input qualification absent");
const git = args => execFileSync("git", args, { cwd: source, encoding: "utf8" }).trim();
if (git(["rev-parse", "HEAD"]) !== selected || git(["status", "--short"])) throw new Error("Source identity changed");
const artifact = "uma-sim-macos-arm64";
const stageParent = path.join(root, "stage");
const stage = path.join(stageParent, artifact);
const artifactRoot = path.join(root, "artifacts");
const zip = path.join(artifactRoot, artifact + "-b8e3da0c-candidate.zip");
const extraction = path.join(root, "extracted");
if ([stage, zip, extraction].some(p => fs.existsSync(p))) throw new Error("Package destination exists; preserve prior evidence");
fs.mkdirSync(stage, { recursive: true }); fs.mkdirSync(artifactRoot, { recursive: true });
fs.copyFileSync(input.executable, path.join(stage, "uma-sim"));
fs.chmodSync(path.join(stage, "uma-sim"), 0o755);
for (const name of ["research", "content_packs"]) fs.cpSync(path.join(source, name), path.join(stage, name), { recursive: true });
const canonical = "knowledge/canonical/by_kind";
fs.mkdirSync(path.join(stage, canonical), { recursive: true });
for (const name of fs.readdirSync(path.join(source, canonical)).filter(n => n.endsWith(".json"))) fs.copyFileSync(path.join(source, canonical, name), path.join(stage, canonical, name));
for (const name of ["LICENSE", "NOTICE", "README.md"]) fs.copyFileSync(path.join(source, name), path.join(stage, name));
const digest = file => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
function inventory(dir) {
  const files = [];
  function walk(relative) {
    for (const entry of fs.readdirSync(path.join(dir, relative), { withFileTypes: true }).sort((a,b) => a.name.localeCompare(b.name))) {
      const rel = path.join(relative, entry.name), file = path.join(dir, rel);
      if (entry.isDirectory()) walk(rel);
      else if (entry.isFile()) files.push({ path: rel, bytes: fs.statSync(file).size, mode: fs.statSync(file).mode & 0o777, sha256: digest(file) });
      else throw new Error("Unexpected nonregular package entry: " + rel);
    }
  }
  walk(""); return files;
}
const stagedFiles = inventory(stage);
if (digest(path.join(stage, "uma-sim")) !== input.executableSha256) throw new Error("Staged binary differs");
execFileSync("zip", ["-qr", zip, artifact], { cwd: stageParent });
const entries = execFileSync("unzip", ["-Z1", zip], { encoding: "utf8" }).trim().split("\n");
if (entries.some(name => !name.startsWith(artifact + "/") || name.split("/").includes("..") || name.includes("\\"))) throw new Error("Archive path outside expected package");
fs.mkdirSync(extraction);
execFileSync("unzip", ["-q", zip, "-d", extraction]);
const packageRoot = path.join(extraction, artifact), extractedFiles = inventory(packageRoot);
if (JSON.stringify(extractedFiles) !== JSON.stringify(stagedFiles)) throw new Error("ZIP extraction differs from stage");
const manifest = {
  schema: "uma-sim.local-release-package.v1", createdAt: new Date().toISOString(),
  repository: "Jacob-Met/uma-sim", selectedSource: selected, sourceTree: git(["rev-parse", "HEAD^{tree}"]),
  kind: "local optimized release candidate; no public release or version tag",
  platform: "macos-arm64", workflowRecipe: ".github/workflows/release.yml",
  workflowRecipeBlob: "1ba20a73631aa35a3a733a538ef5effc3cb94ee4",
  buildInputReceipt: "../build/input-verification.json",
  zip, zipBytes: fs.statSync(zip).size, zipSha256: digest(zip),
  packageRoot, executable: path.join(packageRoot, "uma-sim"), executableSha256: input.executableSha256,
  files: extractedFiles, extractionVerification: "all file names, bytes, hashes and permission modes match staged package",
  platformScope: "Built and pending receiving on this Mac; no Linux or Windows package built"
};
fs.writeFileSync(path.join(artifactRoot, "package-manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(JSON.stringify({ zip: manifest.zip, zipBytes: manifest.zipBytes, zipSha256: manifest.zipSha256, fileCount: manifest.files.length, packageRoot, executableSha256: manifest.executableSha256, source: selected }, null, 2));
