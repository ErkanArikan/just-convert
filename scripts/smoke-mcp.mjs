// Starts `<executable> --mcp` and checks that it answers initialize and tools/list over stdio.
import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";

const executable = process.argv[2];
const child = spawn(executable, ["--mcp"], {
  stdio: ["pipe", "pipe", "inherit"],
  env: {
    ...process.env,
    JUST_CONVERT_DATA_DIR: mkdtempSync(join(tmpdir(), "just-convert-mcp-")),
  },
});
const pending = new Map();
createInterface({ input: child.stdout }).on("line", (line) => {
  const message = JSON.parse(line);
  pending.get(message.id)?.(message);
});
const request = (id, method, params) =>
  new Promise((resolve) => {
    pending.set(id, resolve);
    child.stdin.write(
      `${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`,
    );
  });
const timeout = setTimeout(() => {
  console.error("MCP smoke test timed out");
  process.exit(1);
}, 60000);

const init = await request(1, "initialize", {
  protocolVersion: "2025-06-18",
  capabilities: {},
  clientInfo: { name: "smoke", version: "0" },
});
child.stdin.write(
  `${JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })}\n`,
);
const list = await request(2, "tools/list", {});
const converters = await request(3, "tools/call", {
  name: "list_converters",
  arguments: {},
});
clearTimeout(timeout);
child.kill();

const tools = list.result?.tools?.map((tool) => tool.name) ?? [];
const statuses = converters.result?.structuredContent?.converters ?? [];
console.log(
  `server ${init.result?.serverInfo?.name} ${init.result?.serverInfo?.version}`,
);
console.log(
  `${tools.length} tools; converters: ${statuses.map((c) => `${c.id}=${c.status}`).join(", ")}`,
);
if (tools.length < 19 || statuses.length === 0) process.exit(1);
if (
  process.argv.includes("--require-ready") &&
  statuses.some((c) => c.status !== "ready")
) {
  console.error("Some bundled converters are not ready");
  process.exit(1);
}
