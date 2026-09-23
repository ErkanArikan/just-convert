import { access, readFile, readdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function pathFromRoot(relativePath) {
  return resolve(projectRoot, relativePath);
}

async function requireFile(relativePath) {
  const absolutePath = pathFromRoot(relativePath);
  await access(absolutePath);
  if (!(await stat(absolutePath)).isFile()) {
    throw new Error(`${relativePath} is not a file`);
  }
  return absolutePath;
}

async function requireMissing(relativePath) {
  try {
    await access(pathFromRoot(relativePath));
  } catch {
    return;
  }
  throw new Error(`Template asset must be removed: ${relativePath}`);
}

function requireText(source, expected, sourceName) {
  if (!source.includes(expected)) {
    throw new Error(`${sourceName} is missing: ${expected}`);
  }
}

async function collectFiles(directory, extension) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(path, extension)));
    } else if (entry.isFile() && path.endsWith(extension)) {
      files.push(path);
    }
  }
  return files;
}

const license = await readFile(await requireFile("LICENSE"), "utf8");
for (const expected of [
  "SPDX-License-Identifier: MIT OR Apache-2.0",
  "MIT License",
  "Apache License",
  "END OF TERMS AND CONDITIONS",
]) {
  requireText(license, expected, "LICENSE");
}

for (const brandedAsset of [
  "branding/app-icon-master.png",
  "public/app-icon.png",
  "src-tauri/icons/icon.ico",
  "src-tauri/icons/icon.png",
  "src-tauri/icons/32x32.png",
  "src-tauri/icons/128x128.png",
]) {
  await requireFile(brandedAsset);
}
for (const templateAsset of [
  "public/tauri.svg",
  "public/vite.svg",
  "src/assets/react.svg",
]) {
  await requireMissing(templateAsset);
}

const packageJson = JSON.parse(
  await readFile(await requireFile("package.json"), "utf8"),
);
if (packageJson.license !== "MIT OR Apache-2.0") {
  throw new Error("package.json must declare MIT OR Apache-2.0");
}
if (packageJson.name !== "just-convert") {
  throw new Error("package.json must use the final Just Convert package name");
}

const cargoToml = await readFile(
  await requireFile("src-tauri/Cargo.toml"),
  "utf8",
);
requireText(cargoToml, 'license = "MIT OR Apache-2.0"', "Cargo.toml");
requireText(cargoToml, 'name = "just-convert"', "Cargo.toml");
requireText(cargoToml, 'name = "just_convert_lib"', "Cargo.toml");
requireText(cargoToml, 'rusqlite = { version = "0.32.1", features = ["bundled"] }', "Cargo.toml");

const jobHistorySource = await readFile(
  await requireFile("src-tauri/src/application/job_history.rs"),
  "utf8",
);
requireText(jobHistorySource, 'const DATABASE_NAME: &str = "jobs.sqlite3"', "job history");
requireText(jobHistorySource, "TransactionBehavior::Immediate", "job history");

const platformSourcePath = pathFromRoot("src-tauri/src/platform/mod.rs");
const platformSource = await readFile(platformSourcePath, "utf8");
requireText(platformSource, "CREATE_NO_WINDOW", "platform process helper");
requireText(
  platformSource,
  "command.creation_flags(CREATE_NO_WINDOW)",
  "platform process helper",
);
for (const rustFile of await collectFiles(
  pathFromRoot("src-tauri/src"),
  ".rs",
)) {
  if (rustFile === platformSourcePath) continue;
  const source = await readFile(rustFile, "utf8");
  if (source.includes("Command::new(")) {
    throw new Error(
      `${rustFile} launches a process directly; use platform::background_command`,
    );
  }
}

const tauriConfig = JSON.parse(
  await readFile(await requireFile("src-tauri/tauri.conf.json"), "utf8"),
);
if (
  tauriConfig.productName !== "Just Convert" ||
  tauriConfig.identifier !== "com.erkan.justconvert" ||
  tauriConfig.app?.windows?.[0]?.title !== "Just Convert"
) {
  throw new Error("Tauri metadata must use the final Just Convert branding");
}
if (JSON.stringify(tauriConfig.bundle?.targets) !== JSON.stringify(["nsis"])) {
  throw new Error("Windows v1 must configure NSIS as its sole bundle target");
}

const defaultCapability = JSON.parse(
  await readFile(
    await requireFile("src-tauri/capabilities/default.json"),
    "utf8",
  ),
);
for (const permission of ["dialog:allow-open", "dialog:allow-save"]) {
  if (!defaultCapability.permissions?.includes(permission)) {
    throw new Error(`Default capability is missing ${permission}`);
  }
}

const gitignore = await readFile(await requireFile(".gitignore"), "utf8");
for (const expected of [
  "node_modules",
  "dist",
  "src-tauri/target/",
  "src-tauri/gen/",
  ".env",
  "*.pfx",
  ".codex/",
]) {
  requireText(gitignore, expected, ".gitignore");
}

const attributes = await readFile(await requireFile(".gitattributes"), "utf8");
requireText(attributes, "vendor/ffmpeg/bin/*.exe filter=lfs", ".gitattributes");

const workflow = await readFile(
  await requireFile(".github/workflows/quality.yml"),
  "utf8",
);
for (const expected of [
  "contents: read",
  "lfs: true",
  "pnpm install --frozen-lockfile",
  "pnpm check",
]) {
  requireText(workflow, expected, "quality workflow");
}

const readme = await readFile(await requireFile("README.md"), "utf8");
for (const expected of [
  "# Just Convert",
  "NSIS installer",
  "unsigned",
  "SmartScreen",
  "MIT or Apache 2.0",
]) {
  requireText(readme, expected, "README.md");
}

console.log(
  "Release policy verification passed: dual license, final branding, NSIS-only packaging, Git LFS, CI, and unsigned-build notice.",
);
