// Real Rust API + production UI + Chromium regression for a mandatory career event.
// Run with UMA_EVENT_BINARY=... and UMA_EVENT_CHROME=... after building the embedded UI.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const executable = process.env.UMA_EVENT_BINARY
  ?? join(process.env.CARGO_TARGET_DIR ?? join(root, "target"), "debug", "uma-sim");
const chrome = process.env.UMA_EVENT_CHROME;
const screenshot = process.env.UMA_EVENT_SCREENSHOT;

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => server.once("error", reject).listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
}
const port = await freePort();
const base = `http://127.0.0.1:${port}`;
const server = spawn(executable, ["serve", `--port=${port}`], {
  cwd: root,
  env: { ...process.env, UMA_REPO_ROOT: root },
  stdio: ["ignore", "pipe", "pipe"],
});
let stderr = "";
server.stderr.on("data", b => { stderr += String(b).slice(0, 500); });
let browser;
try {
  let healthy = false;
  for (let i = 0; i < 100; i++) {
    if (server.exitCode !== null) throw new Error(`Server exited ${server.exitCode}: ${stderr}`);
    try {
      const r = await fetch(base + "/v1/health");
      if (r.ok) { healthy = true; break; }
    } catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  assert.ok(healthy, `Rust API not available: ${stderr}`);
  browser = await chromium.launch({
    headless: true,
    ...(chrome ? { executablePath: chrome } : {}),
    args: ["--no-sandbox"],
  });
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  const pageErrors = [];
  page.on("pageerror", e => pageErrors.push(e.message));
  const actionRequests = [];
  page.on("request", r => {
    if (new URL(r.url()).pathname === "/v1/run/action") actionRequests.push(JSON.parse(r.postData()));
  });
  await page.goto(base, { waitUntil: "domcontentloaded" });
  const start = page.getByRole("button", { name: "Start run" });
  await start.waitFor({ state: "visible" });
  await page.waitForFunction(() => {
    const b = [...document.querySelectorAll("button")].find(b => b.textContent?.trim() === "Start run");
    return b && !b.disabled;
  }, null, { timeout: 60_000 });
  await start.click();
  await page.getByRole("button", { name: "Auto step" }).waitFor();
  let advances = 0;
  for (; advances < 65; advances++) {
    if (await page.locator("dialog.event-modal[open]").count()) break;
    await page.getByRole("button", { name: "Auto step" }).click();
    await page.waitForTimeout(80);
  }
  const dialog = page.locator("dialog.event-modal");
  assert.ok(await dialog.count(), `No pending event after ${advances} native career auto actions`);
  await dialog.waitFor({ state: "visible" });
  const initial = await dialog.evaluate(el => ({
    modal: el.matches(":modal"),
    focusInside: el.contains(document.activeElement),
    options: el.querySelectorAll("button.event").length,
    title: el.querySelector("p")?.textContent,
  }));
  assert.equal(initial.modal, true, "event must occupy browser native modal top layer");
  assert.equal(initial.focusInside, true, "focus must enter the event");
  assert.ok(initial.options > 0, "required event choices must be visible");
  await page.keyboard.press("Tab");
  assert.equal(await dialog.evaluate(el => el.contains(document.activeElement)), true,
    "Tab must keep focus inside the modal");
  if (screenshot) await page.screenshot({ path: screenshot });
  await page.setViewportSize({ width: 390, height: 780 });
  const mobileBounds = await dialog.boundingBox();
  assert.ok(mobileBounds && mobileBounds.x >= -0.5 &&
    mobileBounds.x + mobileBounds.width <= 390.5,
    "modal must fit the mobile viewport horizontally");
  assert.ok(mobileBounds.y >= -0.5 &&
    mobileBounds.y + mobileBounds.height <= 780.5,
    "modal must remain scrollable within the mobile viewport");
  await page.setViewportSize({ width: 1200, height: 900 });
  await page.keyboard.press("Escape");
  assert.equal(await dialog.evaluate(el => el.open), true, "Escape must not skip required event");
  const beforeKey = actionRequests.length;
  await page.keyboard.press("1");
  await page.waitForTimeout(125);
  assert.equal(actionRequests.length, beforeKey, "number key must not dispatch background career choice");
  await dialog.locator("button.event").first().click();
  await page.waitForTimeout(125);
  assert.equal(await page.locator("dialog.event-modal[open]").count(), 0,
    "accepted event choice must close modal");
  assert.equal(actionRequests.length, beforeKey + 1);
  assert.equal(actionRequests.at(-1).action, "event_0", "first event option must keep its original ordinal");
  assert.deepEqual(pageErrors, [], "the real UI must not emit uncaught page errors");
  console.log(JSON.stringify({ result: "PASS", advances, event: initial.title, options: initial.options,
    escapeBlocked: true, backgroundShortcutBlocked: true, chosenAction: "event_0", pageErrors }));
} finally {
  if (browser) await browser.close();
  server.kill("SIGTERM");
}
