import { createRequire } from "node:module";
import { mkdtempSync, readdirSync, readFileSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

// Exercise the checked-in TypeScript with the real React runtime on Node 20+.
// Compilation is temporary; npm run typecheck checks the full program separately.
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceRoot = process.env.UMA_UI_TEST_SOURCE || join(packageRoot, "src");
const outputRoot = mkdtempSync(join(packageRoot, ".test-modules-"));
writeFileSync(join(outputRoot, "package.json"), '{"type":"commonjs"}\n');

function compile(directory, relative = "") {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    const target = join(relative, entry.name);
    if (entry.isDirectory()) compile(path, target);
    else if (/\.tsx?$/.test(entry.name)) {
      const result = ts.transpileModule(readFileSync(path, "utf8"), {
        compilerOptions: {
          target: ts.ScriptTarget.ES2022,
          module: ts.ModuleKind.CommonJS,
          jsx: ts.JsxEmit.ReactJSX,
          esModuleInterop: true,
        },
        fileName: path,
      });
      const output = join(outputRoot, target.replace(/\.tsx?$/, ".js"));
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, result.outputText);
    }
  }
}
compile(sourceRoot);
const require = createRequire(join(outputRoot, "package.json"));
export const loadUiModule = (path) => require(join(outputRoot, `${path}.js`));
export const cleanCompiledUi = () => rmSync(outputRoot, { recursive: true, force: true });
