import { spawn } from "node:child_process";
import path from "node:path";
import { createServer } from "vite";
import { coreTestTransport } from "./transport.mjs";

const driver = spawn(
  path.resolve(
    "target/debug/examples",
    process.platform === "win32" ? "ui_driver.exe" : "ui_driver",
  ),
  [],
  { stdio: "inherit", windowsHide: true },
);
const server = await createServer({ plugins: [coreTestTransport] });
await server.listen();
server.printUrls();
console.log(
  "TEST WORKBENCH ONLY: disposable SQLite database. Not the native app.",
);
async function stop() {
  driver.kill();
  await server.close();
  process.exit();
}
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
