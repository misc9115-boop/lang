// Native WebView + real Tauri IPC + SQLite smoke test, using the optional
// embedded W3C driver. No renderer mocks and no live learner data are used.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  openSync,
  closeSync,
  writeFileSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { setTimeout as pause } from "node:timers/promises";

mkdirSync("test-results", { recursive: true });
const root = mkdtempSync(
  path.join(path.resolve("test-results"), "native-profile-"),
);
const executable = path.resolve(
  "target/debug",
  process.platform === "win32" ? "lang-desktop.exe" : "lang-desktop",
);
const seed = path.resolve(
  "target/debug/examples",
  process.platform === "win32" ? "ui_driver.exe" : "ui_driver",
);
const seeded = spawnSync(seed, ["--seed-native-root", root], {
  stdio: "inherit",
  windowsHide: true,
});
assert.equal(seeded.status, 0, "Native seed creation must succeed");
const base = "http://127.0.0.1:19445";
let processHandle;
let sessionId;
let log;
let exitCode;

async function request(method, route, body) {
  const response = await fetch(base + route, {
    method,
    headers: body ? { "Content-Type": "application/json" } : {},
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(35000),
  });
  const result = await response.json();
  if (!response.ok || result.value?.error)
    throw new Error(JSON.stringify(result.value ?? result));
  return result.value;
}
async function wait(check, message) {
  const deadline = Date.now() + 30000;
  let lastError;
  while (Date.now() < deadline) {
    if (exitCode !== undefined)
      throw new Error(
        `Native app exited (${exitCode}). See ${path.join(root, "native.log")}`,
      );
    try {
      if (await check()) return;
    } catch (e) {
      lastError = e;
    }
    await pause(200);
  }
  throw new Error(`${message}${lastError ? `: ${lastError}` : ""}`);
}
async function script(code, args = []) {
  return request("POST", `/session/${sessionId}/execute/sync`, {
    script: code,
    args,
  });
}
async function invoke(command, args = {}) {
  const value = await request("POST", `/session/${sessionId}/execute/async`, {
    script:
      "const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({value}),error=>done({error:String(error)}));",
    args: [command, args],
  });
  if (value.error) throw new Error(value.error);
  return value.value;
}
async function textIncludes(text) {
  await wait(
    () =>
      script("return document.body.innerText.includes(arguments[0]);", [text]),
    `Missing native UI text: ${text}`,
  );
}
async function click(label) {
  await wait(
    () =>
      script(
        'const button=[...document.querySelectorAll("button")].find(b=>{const clone=b.cloneNode(true);clone.querySelectorAll("[aria-hidden=true]").forEach(n=>n.remove());return clone.textContent.replace(/\\s+/g,"")===arguments[0].replace(/\\s+/g,"");});if(!button||button.disabled)return false;button.click();return true;',
        [label],
      ),
    `Native button not usable: ${label}`,
  );
}
async function fill(selector, text) {
  await script(
    'const field=document.querySelector(arguments[0]);if(!field)throw new Error("Missing field");field.value=arguments[1];field.dispatchEvent(new Event("input",{bubbles:true}));field.dispatchEvent(new Event("change",{bubbles:true}));',
    [selector, text],
  );
}
async function start() {
  exitCode = undefined;
  log = openSync(path.join(root, "native.log"), "a");
  processHandle = spawn(executable, [], {
    env: {
      ...process.env,
      LANG_NATIVE_TEST_DATA_DIR: root,
      TAURI_WEBDRIVER_PORT: "19445",
    },
    stdio: ["ignore", log, log],
    windowsHide: true,
  });
  processHandle.on("error", (e) => {
    exitCode = e.message;
  });
  processHandle.on("exit", (code) => {
    exitCode = code;
  });
  await wait(async () => {
    const r = await fetch(base + "/status", {
      signal: AbortSignal.timeout(1000),
    });
    return r.ok;
  }, "Native test server did not start");
  const session = await request("POST", "/session", {
    capabilities: {
      alwaysMatch: { "wdio:tauriServiceOptions": { windowLabel: "main" } },
    },
  });
  sessionId = session.sessionId;
}
async function stop() {
  if (sessionId) {
    try {
      await request("DELETE", `/session/${sessionId}`);
    } catch {}
    sessionId = undefined;
  }
  if (processHandle) {
    const child = processHandle;
    processHandle = undefined;
    if (child.exitCode === null) {
      child.kill();
      await Promise.race([
        new Promise((resolve) => child.once("exit", resolve)),
        pause(5000),
      ]);
    }
  }
  if (log !== undefined) {
    closeSync(log);
    log = undefined;
  }
}
try {
  await start();
  await textIncludes("A place to begin your Italian.");
  await click("C2 CEFR");
  // The time zone field uses a stable placeholder in the real application.
  await fill('input[placeholder="America/Chicago"]', "America/Chicago");
  await click("Open my study space");
  await textIncludes("Your learning path");
  assert.equal((await invoke("app_state")).settings.level, "C2");
  await click("Settings");
  await textIncludes("Pre-import recovery copies");
  await click("Preview recovery copy");
  await textIncludes("Review before restoring");
  assert.equal(
    await script(
      'return [...document.querySelectorAll("button")].find(b=>b.innerText.trim()==="Restore this backup").disabled;',
    ),
    true,
  );
  await click("Cancel restore");
  assert.equal((await invoke("app_state")).settings.level, "C2");
  await click("Preview recovery copy");
  await wait(
    () =>
      script(
        'const label=[...document.querySelectorAll("label")].find(l=>l.textContent.replace(/\\s+/g," ").includes("I understand this replaces"));const field=label?.querySelector("input");if(!field||field.disabled)return false;if(!field.checked)field.click();return field.checked;',
      ),
    "Restore confirmation did not become usable",
  );
  await click("Restore this backup");
  await textIncludes("Backup restored.");
  assert.equal((await invoke("app_state")).settings.level, "A1");
  const recoveryCopies = await invoke("list_recovery_backups");
  assert.equal(recoveryCopies.length, 2);
  const created = recoveryCopies.find(
    (entry) => entry.id !== "pre-import-native-fixture.langbackup",
  );
  assert.ok(created, "Restore must create a separate recovery archive");
  const recovery = JSON.parse(
    readFileSync(path.join(root, "backups", created.id), "utf8"),
  );
  assert.equal(recovery.body.data.settings.level, "C2");
  assert.equal(recovery.body.data.sessions.length, 0);
  await click("Learn");
  await click("Resume saved lesson →");
  await textIncludes("Translate into Italian.");
  assert.equal(
    await script('return document.querySelector("textarea").value;'),
    "Un ca",
  );
  await fill("textarea", "Un caffè per favore");
  await click("← Save & exit");
  await textIncludes("Your learning path");
  const before = await invoke("app_state");
  assert.equal(
    before.activeSession.exercise.draft.value,
    "Un caffè per favore",
  );
  const image = await request("GET", `/session/${sessionId}/screenshot`);
  writeFileSync("test-results/native-window.png", Buffer.from(image, "base64"));
  await stop();
  await pause(800);
  await start();
  await textIncludes("Your learning path");
  await click("Resume saved lesson →");
  await textIncludes("Translate into Italian.");
  assert.equal(
    await script('return document.querySelector("textarea").value;'),
    "Un caffè per favore",
  );
  const after = await invoke("app_state");
  assert.equal(after.settings.level, "A1");
  assert.equal(after.progress.totalXp, 0);
  console.log(
    "PASS native startup, real IPC, backup preview/cancel/restore, recovery creation, and saved draft after process restart",
  );
  writeFileSync(
    "test-results/native-status.json",
    JSON.stringify(
      {
        passed: true,
        profile: root,
        checks: [
          "native-startup",
          "ipc",
          "recovery-preview",
          "cancel",
          "restore",
          "save-resume-after-process-restart",
        ],
      },
      null,
      2,
    ),
  );
} catch (error) {
  console.error(String(error));
  try {
    console.error(
      readFileSync(path.join(root, "native.log"), "utf8").slice(0, 6000),
    );
  } catch {}
  writeFileSync(
    "test-results/native-status.json",
    JSON.stringify(
      { passed: false, profile: root, error: String(error) },
      null,
      2,
    ),
  );
  process.exitCode = 1;
} finally {
  await stop();
}
