// Packs mcpb/ into dist/just-convert-<version>.mcpb and, with --server-json <owner/repo> <tag>,
// writes the registry server.json pointing at that release asset.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [outputDirectory = "dist", flag, repository, tag] = process.argv.slice(2);
const { version } = JSON.parse(
  await readFile(resolve(projectRoot, "src-tauri/tauri.conf.json"), "utf8"),
);

const stage = await mkdtemp(join(tmpdir(), "just-convert-mcpb-"));
const output = resolve(
  projectRoot,
  outputDirectory,
  `just-convert-${version}.mcpb`,
);
try {
  await cp(resolve(projectRoot, "mcpb"), stage, { recursive: true });
  const manifestPath = join(stage, "manifest.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  manifest.version = version;
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await mkdir(dirname(output), { recursive: true });
  execFileSync(
    "npx",
    ["--yes", "@anthropic-ai/mcpb@2.1.2", "pack", stage, output],
    {
      stdio: ["ignore", "inherit", "inherit"],
      shell: process.platform === "win32",
    },
  );
} finally {
  await rm(stage, { recursive: true, force: true });
}

const sha256 = createHash("sha256")
  .update(await readFile(output))
  .digest("hex");
console.log(`Packed ${output}\nSHA-256 ${sha256}`);

if (flag === "--server-json") {
  if (!repository || !tag)
    throw new Error("--server-json needs <owner/repo> <tag>");
  const [owner, name] = repository.split("/");
  const server = JSON.parse(
    await readFile(resolve(projectRoot, "server.json"), "utf8"),
  );
  server.name = `io.github.${owner}/${name}`;
  server.version = version;
  server.websiteUrl = `https://github.com/${repository}`;
  server.repository.url = `https://github.com/${repository}`;
  server.packages[0].identifier = `https://github.com/${repository}/releases/download/${tag}/just-convert-${version}.mcpb`;
  server.packages[0].fileSha256 = sha256;
  await writeFile(
    resolve(projectRoot, "server.json"),
    `${JSON.stringify(server, null, 2)}\n`,
  );
  console.log(`Wrote server.json for ${server.name} ${version}`);
}
