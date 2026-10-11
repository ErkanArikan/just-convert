// Starts the installed Just Convert app as an MCP server; the app bundles every conversion engine.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const home = homedir();
const candidates = {
  darwin: [
    "/Applications/Just Convert.app/Contents/MacOS/just-convert",
    join(home, "Applications/Just Convert.app/Contents/MacOS/just-convert"),
  ],
  win32: [
    join(
      process.env.LOCALAPPDATA || join(home, "AppData", "Local"),
      "Just Convert",
      "just-convert.exe",
    ),
    join(
      process.env.ProgramFiles || "C:\\Program Files",
      "Just Convert",
      "just-convert.exe",
    ),
  ],
  linux: ["/usr/bin/just-convert"],
};

const configured = (process.env.JUST_CONVERT_APP_PATH || "").trim();
const executable =
  configured ||
  (candidates[process.platform] || []).find((path) => existsSync(path));

if (!executable || !existsSync(executable)) {
  process.stderr.write(
    "Just Convert is not installed. Download it from " +
      "https://github.com/ErkanArikan/just-convert/releases, or set the path to the app " +
      "executable in this extension's settings.\n",
  );
  process.exit(1);
}

const child = spawn(executable, ["--mcp"], { stdio: "inherit" });
child.on("error", (error) => {
  process.stderr.write(`Just Convert could not be started: ${error.message}\n`);
  process.exit(1);
});
child.on("exit", (code, signal) => process.exit(signal ? 1 : (code ?? 0)));
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => child.kill(signal));
}
