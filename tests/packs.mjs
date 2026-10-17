// Exercise the real offline library executable and SQLite on both CI hosts.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  closeSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

mkdirSync("test-results", { recursive: true });
const root = mkdtempSync(path.resolve("test-results", "pack caffè "));
const source = path.join(root, "authoring");
mkdirSync(source);
for (const file of ["pack.json", "cafe.json"])
  copyFileSync(path.join("content/development", file), path.join(source, file));
const db = path.join(root, "packs.sqlite3");
const executable = path.resolve(
  "target/debug/examples",
  process.platform === "win32" ? "pack_library.exe" : "pack_library",
);
const id = "dev.pack.cafe";
let call = 0;
function run(args, expected = 0) {
  // Keep real CLI diagnostics and avoid requiring child-pipe creation on hosts
  // where ordinary file-backed child I/O is available but anonymous pipes are not.
  const prefix = path.join(root, `call-${++call}`);
  const stdout = openSync(`${prefix}.out`, "w");
  const stderr = openSync(`${prefix}.err`, "w");
  let result;
  try {
    result = spawnSync(executable, args, {
      stdio: ["ignore", stdout, stderr],
      timeout: 20000,
      windowsHide: true,
    });
  } finally {
    closeSync(stdout);
    closeSync(stderr);
  }
  const output = readFileSync(`${prefix}.out`, "utf8");
  const diagnostics = readFileSync(`${prefix}.err`, "utf8");
  assert.ifError(result.error);
  assert.equal(result.status, expected, diagnostics);
  return expected === 0 ? JSON.parse(output) : diagnostics;
}
const dev = (...args) => run(["--development", db, ...args]);
run([], 2);
run([db, "unknown"], 2);
run([db, "activate", id, "0"], 2);
run([db, "list"], 1);
assert.match(run([db, "install", source], 1), /development library/);
assert.equal(dev("install", source).active, false);
assert.deepEqual(dev("active"), []);
dev("activate", id, "1");
assert.equal(dev("verify", id, "1").trustedReview, false);
assert.equal(dev("active")[0].id, "dev.a1.cafe");
// An authoring folder is never referenced after installation.
writeFileSync(path.join(source, "cafe.json"), "corrupt mutable source");
assert.equal(dev("active")[0].title, "At the café");
copyFileSync("content/development/cafe.json", path.join(source, "cafe.json"));
const lesson = JSON.parse(readFileSync(path.join(source, "cafe.json"), "utf8"));
lesson.version = 2;
lesson.title = "Synthetic CLI update";
const bytes = JSON.stringify(lesson);
writeFileSync(path.join(source, "cafe.json"), bytes);
const manifest = JSON.parse(
  readFileSync(path.join(source, "pack.json"), "utf8"),
);
manifest.version = 2;
manifest.lessons[0].file.bytes = Buffer.byteLength(bytes);
manifest.lessons[0].file.sha256 = createHash("sha256")
  .update(bytes)
  .digest("hex");
writeFileSync(path.join(source, "pack.json"), JSON.stringify(manifest));
dev("install", source);
assert.equal(dev("active")[0].title, "At the café");
dev("activate", id, "2");
assert.equal(dev("active")[0].title, "Synthetic CLI update");
dev("activate", id, "1");
assert.equal(dev("active")[0].title, "At the café");
run(["--development", db, "remove", id, "1"], 1);
run([db, "active"], 1); // Reviewed-only default cannot load cached drafts.
dev("deactivate", id);
dev("remove", id, "1");
assert.equal(dev("list")[0].stored, false);
run(["--development", db, "activate", id, "1"], 1);
for (const file of ["pack.json", "cafe.json"])
  copyFileSync(path.join("content/development", file), path.join(source, file));
dev("install", source); // Exact historical reinstall restores payloads.
dev("activate", id, "1");
assert.equal(dev("active").length, 1);
console.log(
  "PASS offline pack install, verify, update, rollback, removal, repair, source independence, draft isolation and CLI exit codes",
);
