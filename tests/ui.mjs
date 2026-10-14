// Browser integration checks use the real Rust/SQLite core through a test-only
// IPC transport. The driver and injected transport are absent from app builds.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { setTimeout as pause } from "node:timers/promises";
import { chromium, expect } from "@playwright/test";
import { createServer } from "vite";
import { coreTestTransport } from "./transport.mjs";

const driverPath = path.resolve(
  "target",
  "debug",
  "examples",
  process.platform === "win32" ? "ui_driver.exe" : "ui_driver",
);
const driver = spawn(driverPath, [], { stdio: "inherit", windowsHide: true });
const vite = await createServer({ plugins: [coreTestTransport] });
let browser;
let browserProcess;
let browserDirectory;
const errors = [];
const externalRequests = [];
const waitFor = async (url) => {
  for (let i = 0; i < 100; i++) {
    try {
      const response = await fetch(url);
      if (response.ok) return await response.json();
    } catch {}
    await pause(100);
  }
  throw new Error(`Test service did not start: ${url}`);
};
try {
  await vite.listen();
  await waitFor("http://127.0.0.1:1421/health");
  if (process.env.LANG_TEST_CDP_URL) {
    browser = await chromium.connectOverCDP(process.env.LANG_TEST_CDP_URL);
  } else if (process.platform === "win32") {
    // Ignored stdio also works in desktop sandboxes that prohibit child pipes.
    const executable =
      process.env.LANG_TEST_BROWSER_PATH ||
      [
        "C:/Program Files/Google/Chrome/Application/chrome.exe",
        "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
      ].find(existsSync);
    assert.ok(executable, "Install Chrome/Edge or set LANG_TEST_BROWSER_PATH");
    browserDirectory = mkdtempSync(path.join(tmpdir(), "lang-ui-"));
    browserProcess = spawn(
      executable,
      [
        "--headless=new",
        "--remote-debugging-address=127.0.0.1",
        "--remote-debugging-port=19223",
        `--user-data-dir=${browserDirectory}`,
        "--no-first-run",
        "--no-default-browser-check",
        "about:blank",
      ],
      { stdio: "ignore", windowsHide: true },
    );
    await waitFor("http://127.0.0.1:19223/json/version");
    browser = await chromium.connectOverCDP("http://127.0.0.1:19223");
  } else {
    browser = await chromium.launch();
  }

  const context = await browser.newContext({
    viewport: { width: 1280, height: 1000 },
    reducedMotion: "reduce",
  });
  await context.addInitScript(() => {
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args = {}) => {
        const response = await fetch("http://127.0.0.1:1421/invoke", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ cmd, args }),
        });
        const payload = await response.json();
        if (payload.error) throw new Error(payload.error);
        return payload.value;
      },
    };
  });
  const page = await context.newPage();
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/*", (route) => {
    const url = new URL(route.request().url());
    if (url.hostname === "127.0.0.1" && ["1420", "1421"].includes(url.port))
      return route.continue();
    externalRequests.push(url.href);
    return route.abort();
  });
  await page.goto("http://127.0.0.1:1420");
  await expect(
    page.getByRole("heading", { name: "A place to begin your Italian." }),
  ).toBeVisible();
  await page.getByLabel("Study time zone").fill("not/a-zone");
  await page.getByRole("button", { name: "Open my study space" }).click();
  await expect(page.getByRole("alert")).toContainText("valid IANA");
  await page.getByLabel("Study time zone").fill("America/Chicago");
  await page.getByRole("button", { name: "Open my study space" }).click();
  await expect(
    page.getByRole("heading", { name: "Your learning path" }),
  ).toBeVisible();
  await expect(
    page.getByText("No reviewed course packs are installed.", { exact: false }),
  ).toBeVisible();
  mkdirSync("test-results", { recursive: true });
  await page.screenshot({
    path: "test-results/learning-path.png",
    fullPage: true,
  });
  console.log("PASS onboarding validation and honest empty course state");

  await page.getByRole("button", { name: "Open test lesson" }).click();
  await page.getByRole("radio", { name: /Good evening/ }).check();
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await expect(
    page.getByText("Try again — you have unlimited retries"),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "My answer should be accepted" })
    .click();
  await page
    .getByLabel("Describe your answer")
    .fill("UI integration test report, not an actual editorial claim.");
  await page.getByRole("button", { name: "Save local report" }).click();
  await expect(page.getByRole("status")).toContainText("Report saved locally");
  await page.getByRole("button", { name: "Show hint", exact: true }).click();
  await page.getByRole("radio", { name: /A coffee, please/ }).check();
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(
    page.getByRole("heading", { name: "Translate into Italian." }),
  ).toBeFocused();
  await page
    .getByRole("textbox", { name: "Your answer", exact: true })
    .fill("Un caffe per favore");
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await expect(
    page.getByText("Try again — you have unlimited retries"),
  ).toBeVisible();
  await page
    .getByRole("textbox", { name: "Your answer", exact: true })
    .fill("Un caffè per favore");
  await page.getByRole("button", { name: "Save & exit" }).click();
  await page.reload();
  await page.getByRole("button", { name: "Resume saved lesson" }).click();
  await expect(
    page.getByRole("textbox", { name: "Your answer", exact: true }),
  ).toHaveValue("Un caffè per favore");
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  console.log(
    "PASS real SQLite retry, hint, accent, report, and exit/reload/resume flow",
  );

  const bank = page.getByRole("group", { name: "Available words" });
  for (const word of ["Il", "conto,", "per", "favore"])
    await bank.getByRole("button", { name: word, exact: true }).click();
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page
    .getByRole("button", { name: "Review solution", exact: true })
    .click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("radio", { name: /A coffee/ }).check();
  await page.getByRole("button", { name: "Check answer", exact: true }).click();
  await page.getByRole("button", { name: "Finish lesson" }).click();
  await expect(
    page.getByRole("heading", { name: "Test lesson complete" }),
  ).toBeVisible();
  await expect(
    page.getByText("1 items with hints · 1 solutions reviewed"),
  ).toBeVisible();
  const finalState = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("app_state"),
  );
  assert.equal(finalState.progress.totalXp, 0);
  assert.equal(finalState.progress.completedLessons, 0);
  assert.deepEqual(finalState.progress.achievements, []);
  await page.screenshot({
    path: "test-results/lesson-summary.png",
    fullPage: true,
  });
  console.log(
    "PASS ordering, reveal, reading, completion, and draft exclusion from XP",
  );

  await page.getByRole("button", { name: "Back to my path" }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "C2 CEFR", exact: true }).click();
  await page.getByLabel("Appearance", { exact: true }).selectOption("dark");
  await page.getByLabel("Text size", { exact: true }).selectOption("200");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Find the precise expression" }),
  ).toBeVisible();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    ),
    false,
    "No horizontal overflow at 200% text size",
  );
  await page.screenshot({
    path: "test-results/dark-large-text.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 760, height: 900 });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    ),
    false,
    "No horizontal overflow at narrow width and 200% text",
  );
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Text size", { exact: true }).selectOption("100");
  await page.getByLabel("Appearance", { exact: true }).selectOption("light");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await page.keyboard.press("Tab");
  const focusedTag = await page.evaluate(() => document.activeElement?.tagName);
  assert.ok(["BUTTON", "A", "MAIN"].includes(focusedTag));
  assert.deepEqual(errors, []);
  assert.deepEqual(externalRequests, []);
  console.log(
    "PASS level changes, dark theme, 200% text, narrow layout, keyboard, and no external page requests",
  );
  await context.close();
} finally {
  if (browser) await browser.close();
  browserProcess?.kill();
  driver.kill();
  await vite.close();
  // Chrome can briefly retain profile locks. These are disposable test assets.
  if (browserDirectory) {
    const resolved = path.resolve(browserDirectory);
    if (
      path.dirname(resolved) === path.resolve(tmpdir()) &&
      path.basename(resolved).startsWith("lang-ui-")
    ) {
      try {
        rmSync(resolved, { recursive: true, force: true });
      } catch {}
    }
  }
}
