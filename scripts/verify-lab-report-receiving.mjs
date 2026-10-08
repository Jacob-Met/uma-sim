#!/usr/bin/env node
/**
 * Independent receiving check for reports from an actual uma-sim-api binary.
 *
 * node scripts/verify-lab-report-receiving.mjs API_BINARY REPO_ROOT OUTPUT_DIR
 *
 * The supplied native server runs on loopback in a fresh, disposable state
 * directory. Eight cases create branches solely through the public HTTP API
 * using the built-in bot and stub race model. One additional case explicitly
 * authors string fields in two disposable persisted branch files to challenge
 * report metadata that normal simulation cannot freely name. Original and
 * authored branch bytes are both preserved. Chromium downloads actual API
 * attachments; marked parses those saved bytes, and Chromium inspects them.
 * No HTTP report payload or simulator response is replaced.
 *
 * Dependencies are supplied by the receiving environment, never installed:
 * PLAYWRIGHT_MODULE: package name or absolute playwright-core package directory
 * MARKED_MODULE: package name or absolute marked ESM entry file
 * CHROMIUM_EXECUTABLE: existing browser executable
 */
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import fs from "node:fs";
import { createRequire } from "node:module";
import net from "node:net";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright-core");
const markedRef = process.env.MARKED_MODULE || "marked";
const marked = await import(path.isAbsolute(markedRef) ? pathToFileURL(markedRef).href : markedRef);
const [binaryArg, repoArg, outputArg] = process.argv.slice(2);
if (!binaryArg || !repoArg || !outputArg || !process.env.CHROMIUM_EXECUTABLE) {
  throw new Error("Supply API_BINARY REPO_ROOT OUTPUT_DIR and CHROMIUM_EXECUTABLE.");
}
const binary = fs.realpathSync(binaryArg);
const repo = fs.realpathSync(repoArg);
const output = path.resolve(outputArg);
fs.mkdirSync(output, { recursive: true });
assert.equal(fs.readdirSync(output).length, 0, "Use a new output directory to preserve prior receipts.");
const stateDir = path.join(output, "state");
fs.mkdirSync(stateDir);
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const fileDigest = filename => digest(fs.readFileSync(filename));
const packageInfo = moduleRef => {
  const entry = require.resolve(moduleRef);
  let directory = path.dirname(entry);
  while (directory !== path.dirname(directory)) {
    const metadata = path.join(directory, "package.json");
    if (fs.existsSync(metadata)) {
      const parsed = JSON.parse(fs.readFileSync(metadata, "utf8"));
      return { name: parsed.name, version: parsed.version, entry, entrySha256: fileDigest(entry) };
    }
    directory = path.dirname(directory);
  }
  throw new Error(`No package metadata found for ${moduleRef}`);
};
const writeJson = (filename, value) => fs.writeFileSync(filename, JSON.stringify(value, null, 2) + "\n");
const normalizeLines = text => text.replace(/\r\n?/g, "\n");
const normalizeCode = text => normalizeLines(text).replaceAll("\n", " ");
const htmlText = text => String(text).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const git = (...args) => {
  const result = spawnSync("git", ["-C", repo, ...args], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
};
const sourceFiles = [
  "Cargo.toml", "Cargo.lock", "uma-sim-core/Cargo.toml",
  "uma-sim-core/src/career_lab.rs", "uma-sim-core/src/api.rs",
  "uma-sim-core/src/bin/uma-sim-api.rs",
];
const sourcePins = () => Object.fromEntries(sourceFiles.map(filename => [filename, fileDigest(path.join(repo, filename))]));
const scenarios = [
  { id: "ordinary_names", names: ["Rest early", "Plain control"] },
  { id: "pipe", names: ["Rest | train", "Plain control"] },
  { id: "markdown", names: ["Rest **early** _pace_ `draft` [note](#anchor)", "Control ~~steady~~"] },
  { id: "html", names: ["Pace <em>fast</em> & <br> recover", "Control > baseline"] },
  { id: "line_breaks", names: ["Pace\r\nREST\n\n# Recovered heading", "B\rSecond line"] },
  { id: "backslash_pipe_backticks", names: ["Left \\| right `draft`", "Two ``ticks`` and \\\\ tail"] },
  { id: "unicode", names: ["早起き ☀️ | 休む", "« contrôle » · café"] },
  { id: "literal_entities", names: ["R&D &amp; &#124; literal", "Control &lt;done&gt;"] },
  { id: "authored_persisted_fields", names: ["Authored metadata A", "Authored metadata B"], persistedFixture: true },
];
const authoredCheckpoint = "cp` `long``fence``` <tag> &amp; | marker\r\nnext";
const authorBranchStrings = (branch, index) => {
  branch.checkpointName = authoredCheckpoint;
  branch.timeline[0].dateLabel = "Date [anchor](#target) | <i>tag</i>\nsecond";
  branch.timeline[0].phase = "`phase` <em> &amp; | x\rY";
  branch.timeline[0].actionId = index === 0
    ? "`a``` | &amp; <tag> \\ path\nnext`" : "b`` ` &raw; |\r\nfinish";
  branch.timeline[0].actionLabel = index === 0
    ? "A | **literal** `draft` <br> &amp;\nnext" : "B \\| _label_ <em>text</em>\r\nlast";
  branch.outcome.mood = index === 0 ? "GOOD | *literal* <br>\r\nnext" : "B | &amp; _mood_\rstep";
  branch.outcome.completedRaces = ["race | <tag>", "literal `race`\nnew"];
  branch.outcome.learnedSkills = ["skill **x** | &amp;", "second `slot`"];
  branch.outcome.sparks = [{ color: "red | <b>tone</b>", factorId: "receipt-factor", stars: 3, label: "spark _star_ `tick`\nline" }];
  return branch;
};
const receipt = {
  schema: "uma-sim-native-report-receiving-v1", startedAt: new Date().toISOString(),
  boundary: "Actual native API and disposable filesystem storage; eight natural API branch-name cases plus one explicitly authored persisted-string fixture; built-in bot with stub races; real Chromium downloads; marked GFM render. No deployed estate service or external policy.",
  source: { root: repo, commit: git("rev-parse", "HEAD"), status: git("status", "--short"), before: sourcePins() },
  binary: { path: binary, sha256: fileDigest(binary) },
  harnessSha256: fileDigest(fileURLToPath(import.meta.url)),
  browser: { executable: fs.realpathSync(process.env.CHROMIUM_EXECUTABLE), sha256: fileDigest(process.env.CHROMIUM_EXECUTABLE) },
  parser: { module: markedRef, gfm: true, ...packageInfo(markedRef) },
  playwright: packageInfo(process.env.PLAYWRIGHT_MODULE || "playwright-core"),
  cases: [], apiRequests: [], browserRequests: [], outsideRequests: [], pageErrors: [],
};
let native;
let browser;
let context;
let serverLog;
let nativeExited = false;
let fatal;

try {
  const reserver = net.createServer();
  reserver.listen(0, "127.0.0.1");
  await once(reserver, "listening");
  const port = reserver.address().port;
  await new Promise((resolve, reject) => reserver.close(error => error ? reject(error) : resolve()));
  const base = `http://127.0.0.1:${port}`;
  receipt.nativeOrigin = base;
  const env = { ...process.env, UMA_REPO_ROOT: repo };
  delete env.UMA_POLICY_CMD;
  native = spawn(binary, [String(port)], { cwd: stateDir, env, stdio: ["ignore", "pipe", "pipe"] });
  const nativeExit = once(native, "exit").then(([code, signal]) => {
    nativeExited = true;
    receipt.nativeExit = { code, signal };
  });
  serverLog = fs.createWriteStream(path.join(output, "server.log"));
  native.stdout.pipe(serverLog, { end: false });
  native.stderr.pipe(serverLog, { end: false });
  for (let attempt = 0; ; attempt++) {
    if (nativeExited) throw new Error("Native server exited before health was available.");
    try {
      const response = await fetch(base + "/v1/health", { signal: AbortSignal.timeout(1000) });
      if (response.ok) {
        receipt.health = await response.json();
        break;
      }
    } catch (error) {
      if (attempt >= 99) throw error;
    }
    assert.ok(attempt < 99, "Native server did not become healthy.");
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  const api = async (method, route, body) => {
    const response = await fetch(base + route, {
      method, headers: body ? { "Content-Type": "application/json" } : undefined,
      body: body ? JSON.stringify(body) : undefined, signal: AbortSignal.timeout(30000),
    });
    const bytes = Buffer.from(await response.arrayBuffer());
    receipt.apiRequests.push({ method, route, input: body, status: response.status,
      contentType: response.headers.get("content-type"), bytes: bytes.length, sha256: digest(bytes) });
    assert.equal(response.status, 200, `${method} ${route}: ${bytes.toString("utf8")}`);
    return JSON.parse(bytes.toString("utf8"));
  };
  receipt.run = await api("POST", "/v1/run/start", {
    seed: 4242, scenario: "ura", trainee: "Special Week", raceModel: "stub", policy: "bot",
  });
  receipt.checkpoint = await api("POST", "/v1/library/save", { name: "report-receiving" });

  browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_EXECUTABLE, headless: true,
    args: ["--disable-dev-shm-usage", "--disable-gpu"],
  });
  receipt.browser.version = browser.version();
  context = await browser.newContext({ acceptDownloads: true, viewport: { width: 1180, height: 1000 } });
  const responseHeaders = [];
  const pendingHeaders = [];
  context.on("response", response => {
    const capture = response.allHeaders().then(headers => responseHeaders.push({
      url: response.url(), status: response.status(), headers,
    }));
    pendingHeaders.push(capture);
  });
  await context.route("**/*", route => {
    const url = route.request().url();
    receipt.browserRequests.push({ method: route.request().method(), url });
    if (new URL(url).origin !== base) {
      receipt.outsideRequests.push(url);
      return route.abort("blockedbyclient");
    }
    return route.continue();
  });
  const page = await context.newPage();
  page.on("pageerror", error => receipt.pageErrors.push(String(error)));
  await page.goto(base + "/v1/health");

  for (const scenario of scenarios) {
    const caseDir = path.join(output, scenario.id);
    fs.mkdirSync(caseDir);
    const result = { id: scenario.id, submittedNames: scenario.names,
      inputBoundary: scenario.persistedFixture ? "Explicitly authored disposable persisted string fields after actual API branch creation." : "Actual API branch creation; no persisted input changes.",
      checks: [], downloads: [] };
    receipt.cases.push(result);
    const check = (name, fn) => {
      try { fn(); result.checks.push({ name, passed: true }); }
      catch (error) { result.checks.push({ name, passed: false, error: error.message }); }
    };
    try {
      const branches = [];
      for (const name of scenario.names) {
        branches.push(await api("POST", "/v1/lab/branch", {
          checkpoint: "report-receiving", name, policy: "bot", maxActions: 2,
        }));
      }
      writeJson(path.join(caseDir, "created-branches.json"), branches);
      const ids = branches.map(branch => branch.branch.id);
      const admittedNames = branches.map(branch => branch.branch.name);
      let authored;
      if (scenario.persistedFixture) {
        authored = ids.map((id, index) => {
          assert.match(id, /^br-[a-zA-Z0-9-]+$/);
          const filename = path.join(stateDir, ".uma-sim", "lab", `${id}.json`);
          const original = fs.readFileSync(filename);
          fs.writeFileSync(path.join(caseDir, `simulation-branch-${index}.json`), original);
          const fixture = authorBranchStrings(JSON.parse(original.toString("utf8")), index);
          writeJson(filename, fixture);
          writeJson(path.join(caseDir, `authored-branch-${index}.json`), fixture);
          return fixture;
        });
        result.authoredStoreFiles = ids.map((id, index) => ({
          branchId: id, originalSha256: fileDigest(path.join(caseDir, `simulation-branch-${index}.json`)),
          authoredSha256: fileDigest(path.join(caseDir, `authored-branch-${index}.json`)),
        }));
      }
      result.branchIds = ids;
      result.admittedNames = admittedNames;
      check("API admits each intended name without changing internal data", () => {
        assert.deepEqual(admittedNames, scenario.names.map(name => name.trim()));
        assert.notEqual(ids[0], ids[1]);
      });
      const stem = `branch-compare-${ids[0]}-vs-${ids[1]}`;
      const params = `a=${encodeURIComponent(ids[0])}&b=${encodeURIComponent(ids[1])}`;
      const reportUrls = Object.fromEntries(["markdown", "json"].map(format => [format,
        `${base}/v1/lab/report?${params}&format=${format}`]));
      await page.setContent(`<html><head><meta charset="utf-8"></head><body>${Object.entries(reportUrls).map(([format, url]) =>
        `<a id="${format}" href="${htmlText(url)}">Download ${format}</a>`).join("<br>")}</body></html>`);
      const attachments = {};
      for (const format of ["markdown", "json"]) {
        const downloadPromise = page.waitForEvent("download", { timeout: 10000 });
        await page.locator(`#${format}`).click();
        const download = await downloadPromise;
        const extension = format === "markdown" ? "md" : "json";
        const filename = path.join(caseDir, `report.${extension}`);
        await download.saveAs(filename);
        const bytes = fs.readFileSync(filename);
        attachments[format] = bytes;
        const downloadInfo = {
          format, url: download.url(), suggestedFilename: download.suggestedFilename(),
          failure: await download.failure(), bytes: bytes.length, sha256: digest(bytes),
        };
        result.downloads.push(downloadInfo);
        check(`${format} browser attachment identity and bytes`, () => {
          assert.equal(downloadInfo.url, reportUrls[format]);
          assert.equal(downloadInfo.suggestedFilename, `${stem}.${extension}`);
          assert.equal(downloadInfo.failure, null);
          assert.ok(bytes.length > 100);
          assert.deepEqual(Buffer.from(bytes.toString("utf8"), "utf8"), bytes, "Report must be valid UTF-8.");
        });
      }
      const json = JSON.parse(attachments.json.toString("utf8"));
      check("Downloaded JSON preserves ordered branch IDs and original names", () => {
        assert.deepEqual([json.aId, json.bId], ids);
        assert.deepEqual([json.aName, json.bName], admittedNames);
        assert.equal(json.checkpointName, scenario.persistedFixture ? authoredCheckpoint : "report-receiving");
        assert.equal(json.seed, 4242);
        assert.equal(json.sameCheckpoint, true);
        if (authored) {
          assert.equal(json.firstDivergence.phase, authored[0].timeline[0].phase);
          assert.equal(json.firstDivergence.dateLabel, authored[0].timeline[0].dateLabel);
          assert.deepEqual([json.firstDivergence.actionA, json.firstDivergence.actionB], authored.map(branch => branch.timeline[0].actionId));
          assert.deepEqual([json.outcomeA, json.outcomeB], authored.map(branch => branch.outcome));
        }
      });
      const markdown = attachments.markdown.toString("utf8");
      const rendered = marked.marked.parse(markdown, { gfm: true });
      const html = `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src 'none'; script-src 'none'"><style>body{font:15px/1.5 system-ui,sans-serif;margin:32px;color:#17202a;background:white}main{max-width:1090px}h1{font-size:25px;line-height:1.35}h2{font-size:19px;margin-top:25px}table{border-collapse:collapse;width:100%;font-size:13px}th,td{border:1px solid #ccd4dc;padding:7px 9px;text-align:left;vertical-align:top}thead{background:#edf2f7}code{font-size:13px}li{margin:4px 0}</style></head><body><main>${rendered}</main></body></html>`;
      fs.writeFileSync(path.join(caseDir, "report.html"), html);
      await page.setContent(html, { waitUntil: "load" });
      const dom = await page.evaluate(() => {
        const main = document.querySelector("main");
        const tables = [...main.querySelectorAll("table")];
        const divergenceHeading = [...main.querySelectorAll("h2")].find(node => node.innerText === "First divergence");
        return {
          headings: [...main.querySelectorAll("h1")].map(node => node.innerText),
          sectionHeadings: [...main.querySelectorAll("h2")].map(node => node.innerText),
          tables: tables.map(table => ({
            headers: [...table.querySelectorAll("thead tr")].map(row => [...row.children].map(cell => cell.innerText)),
            rows: [...table.querySelectorAll("tbody tr")].map(row => [...row.children].map(cell => cell.innerText)),
          })),
          codeSpans: [...main.querySelectorAll("code")].map(node => node.innerText),
          divergenceItems: divergenceHeading?.nextElementSibling?.tagName === "UL"
            ? [...divergenceHeading.nextElementSibling.children].map(node => node.innerText) : [],
          mainText: main.innerText,
        };
      });
      writeJson(path.join(caseDir, "rendered-dom.json"), dom);
      check("Rendered heading retains each name as literal data", () => {
        assert.deepEqual(dom.headings.map(normalizeLines), [normalizeLines(`Branch comparison: ${admittedNames[0]} vs ${admittedNames[1]}`)]);
      });
      check("Embedded name data cannot split sections or tables", () => {
        assert.deepEqual(dom.sectionHeadings, ["First divergence", "Timeline (per step)", "Final outcomes", "Caveats"]);
        assert.equal(dom.tables.length, 2);
        assert.equal(dom.tables[0].rows.length, json.aligned.length);
        assert.ok(dom.tables[0].rows.every(row => row.length === 8));
        assert.ok(dom.tables[1].rows.every(row => row.length === 3));
        assert.equal(dom.tables[1].rows.length, 12);
      });
      check("Rendered final Branch row preserves ordered original cell text", () => {
        const branchRows = dom.tables.at(-1)?.rows.filter(row => row[0] === "Branch");
        assert.deepEqual(branchRows?.map(row => row.map(normalizeLines)), [["Branch", ...admittedNames.map(normalizeLines)]]);
      });
      check("Rendered numeric and deterministic outcome cells match downloaded JSON", () => {
        const rows = new Map(dom.tables.at(-1)?.rows.map(row => [row[0], row.slice(1)]) || []);
        for (const [label, field] of [["Steps", "steps"], ["Career complete", "careerComplete"], ["Final turn", "finalTurn"], ["Fans", "fans"], ["Skill points", "skillPoints"], ["RNG calls", "totalRngCalls"]]) {
          assert.deepEqual(rows.get(label), [String(json.outcomeA[field]), String(json.outcomeB[field])], label);
        }
        assert.deepEqual(rows.get("Energy / mood")?.map(normalizeLines), [json.outcomeA, json.outcomeB].map(outcome => normalizeLines(`${outcome.energy} / ${outcome.mood}`)));
      });
      if (authored) {
        const divergence = json.firstDivergence;
        check("Code-span metadata preserves literal backticks, pipes, entities and HTML", () => {
          assert.deepEqual(dom.codeSpans, [json.checkpointName, divergence.phase, divergence.kind,
            divergence.actionA, divergence.actionB].map(normalizeCode));
        });
        check("Divergence labels and generated note remain literal within four list items", () => {
          const expected = [
            `Step ${divergence.stepIndex} · turn ${divergence.turn} (${normalizeLines(divergence.dateLabel)}) · phase ${normalizeCode(divergence.phase)} · kind ${normalizeCode(divergence.kind)}`,
            `A: ${normalizeCode(divergence.actionA)} — ${normalizeLines(divergence.labelA)}`,
            `B: ${normalizeCode(divergence.actionB)} — ${normalizeLines(divergence.labelB)}`,
            normalizeLines(divergence.note),
          ];
          assert.deepEqual(dom.divergenceItems.map(normalizeLines), expected);
        });
        check("Authored timeline, race, skill and spark strings survive table receiving", () => {
          assert.deepEqual(dom.tables[0].rows[0].slice(2, 4).map(normalizeLines), [divergence.labelA, divergence.labelB].map(normalizeLines));
          const rows = new Map(dom.tables.at(-1)?.rows.map(row => [row[0], row.slice(1).map(normalizeLines)]) || []);
          const outcomes = [json.outcomeA, json.outcomeB];
          assert.deepEqual(rows.get("Races"), outcomes.map(outcome => normalizeLines(`${outcome.completedRaces.length} (${outcome.completedRaces.join(", ")})`)));
          assert.deepEqual(rows.get("Skills learned"), outcomes.map(outcome => normalizeLines(`${outcome.learnedSkills.length}: ${outcome.learnedSkills.join(", ")}`)));
          assert.deepEqual(rows.get("Sparks"), outcomes.map(outcome => normalizeLines(outcome.sparks.map(spark => `${spark.color}★${spark.stars} ${spark.label}`).join("; "))));
        });
      }
      await page.screenshot({ path: path.join(caseDir, "rendered.png"), fullPage: true });
    } catch (error) {
      result.checks.push({ name: "Native API and receiving execution", passed: false, error: error.stack });
    }
    result.passed = result.checks.every(check => check.passed);
    writeJson(path.join(caseDir, "case.json"), result);
    console.log(`${result.passed ? "PASS" : "FAIL"} ${scenario.id}: ${result.checks.filter(check => check.passed).length}/${result.checks.length} checks`);
  }
  await Promise.all(pendingHeaders);
  receipt.browserResponses = responseHeaders;
  for (const result of receipt.cases) {
    for (const attachment of result.downloads) {
      const response = responseHeaders.find(response => response.url === attachment.url);
      attachment.response = response || null;
      // A downloaded navigation may not expose a Playwright Response. In that
      // case the saved attachment filename, URL and bytes remain the evidence;
      // do not invent HTTP status or header observations.
      if (response) {
        const expected = attachment.format === "markdown" ? "text/markdown" : "application/json";
        const passed = response.status === 200 && response.headers["content-type"]?.startsWith(expected)
          && response.headers["content-disposition"]?.includes(attachment.suggestedFilename)
          && Number(response.headers["content-length"]) === attachment.bytes;
        result.checks.push({ name: `${attachment.format} observed HTTP attachment headers`, passed: Boolean(passed), observed: response });
        result.passed = result.checks.every(check => check.passed);
      }
    }
    writeJson(path.join(output, result.id, "case.json"), result);
  }
  assert.deepEqual(receipt.outsideRequests, [], "Receiver attempted no requests outside the native loopback origin.");
  assert.deepEqual(receipt.pageErrors, [], "Browser emitted no unhandled page errors.");
  await context.close();
  context = undefined;
  await browser.close();
  browser = undefined;
  native.kill("SIGTERM");
  await nativeExit;
} catch (error) {
  fatal = error;
  receipt.fatal = error.stack;
} finally {
  if (context) await context.close().catch(() => {});
  if (browser) await browser.close().catch(() => {});
  if (native && !nativeExited) {
    native.kill("SIGTERM");
    await once(native, "exit");
  }
  if (serverLog) await new Promise(resolve => serverLog.end(resolve));
  receipt.source.after = sourcePins();
  receipt.source.unchanged = JSON.stringify(receipt.source.before) === JSON.stringify(receipt.source.after);
  receipt.binary.afterSha256 = fileDigest(binary);
  receipt.binary.unchanged = receipt.binary.sha256 === receipt.binary.afterSha256;
  receipt.summary = {
    passed: receipt.cases.filter(result => result.passed).length,
    failed: receipt.cases.filter(result => !result.passed).length,
    total: receipt.cases.length, planned: scenarios.length,
    checksPassed: receipt.cases.flatMap(result => result.checks).filter(check => check.passed).length,
    checksFailed: receipt.cases.flatMap(result => result.checks).filter(check => !check.passed).length,
  };
  receipt.finishedAt = new Date().toISOString();
  receipt.accepted = !fatal && receipt.summary.failed === 0 && receipt.summary.total === scenarios.length
    && receipt.source.unchanged && receipt.binary.unchanged;
  writeJson(path.join(output, "receipt.json"), receipt);
  console.log(JSON.stringify({ accepted: receipt.accepted, ...receipt.summary, sourceUnchanged: receipt.source.unchanged, binaryUnchanged: receipt.binary.unchanged }));
  process.exitCode = receipt.accepted ? 0 : 1;
}
