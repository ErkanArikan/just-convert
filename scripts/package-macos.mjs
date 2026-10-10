// Tauri's resource copier follows symlinks, which breaks the signature of the embedded
// LibreOffice.app; engines are therefore copied in with ditto after the app is bundled.
import { execFileSync } from "node:child_process";
import { mkdtemp, readdir, readFile, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const run = (program, args) =>
  execFileSync(program, args, {
    cwd: projectRoot,
    stdio: ["ignore", "inherit", "inherit"],
  });

const config = JSON.parse(
  await readFile(resolve(projectRoot, "src-tauri/tauri.conf.json"), "utf8"),
);
const bundleDirectory = resolve(projectRoot, "src-tauri/target/release/bundle");
const app = join(bundleDirectory, "macos", `${config.productName}.app`);
const vendor = join(app, "Contents/Resources/vendor");

// CI passes unset secrets as empty strings, which Tauri would treat as a signing identity.
for (const name of [
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
]) {
  if (!process.env[name]) delete process.env[name];
}
const identity = process.env.APPLE_SIGNING_IDENTITY;
const notarize =
  identity &&
  process.env.APPLE_ID &&
  process.env.APPLE_PASSWORD &&
  process.env.APPLE_TEAM_ID;

const engineEntitlements = resolve(
  projectRoot,
  "src-tauri/macos/engines.entitlements",
);

function sign(path, entitlements) {
  run("codesign", [
    "--force",
    "--timestamp",
    "--options",
    "runtime",
    ...(entitlements ? ["--entitlements", entitlements] : []),
    "--sign",
    identity,
    path,
  ]);
}

function notarizeAndStaple(path) {
  let submission = path;
  if (path.endsWith(".app")) {
    submission = `${path}.zip`;
    run("ditto", ["-c", "-k", "--keepParent", path, submission]);
  }
  run("xcrun", [
    "notarytool",
    "submit",
    submission,
    "--apple-id",
    process.env.APPLE_ID,
    "--password",
    process.env.APPLE_PASSWORD,
    "--team-id",
    process.env.APPLE_TEAM_ID,
    "--wait",
  ]);
  run("xcrun", ["stapler", "staple", path]);
  if (submission !== path) execFileSync("rm", [submission]);
}

async function machOFiles(directory) {
  const found = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      found.push(...(await machOFiles(path)));
      continue;
    }
    if (!entry.isFile()) continue;
    const kind = execFileSync("file", ["-b", path], { encoding: "utf8" });
    if (kind.startsWith("Mach-O")) {
      found.push({ path, executable: kind.includes("executable") });
    }
  }
  return found;
}

run("pnpm", ["tauri", "build", "--bundles", "app"]);

for (const engine of [
  "ffmpeg/bin/ffmpeg",
  "ffmpeg/bin/ffprobe",
  "qpdf/bin/qpdf",
  "qpdf/lib",
  "pdfium/bin/libpdfium.dylib",
  "libreoffice",
  "pdf-to-docx",
  "whisper",
]) {
  await rm(join(vendor, engine), { recursive: true, force: true });
  run("ditto", [resolve(projectRoot, "vendor", engine), join(vendor, engine)]);
}

if (identity) {
  // The embedded LibreOffice keeps its notarized Document Foundation signature. Engine
  // executables load third-party libraries, so they opt out of library validation.
  for (const engine of ["ffmpeg", "qpdf", "pdfium", "pdf-to-docx"]) {
    const files = await machOFiles(join(vendor, engine));
    for (const file of files.filter((file) => !file.executable))
      sign(file.path);
    for (const file of files.filter((file) => file.executable)) {
      sign(file.path, engineEntitlements);
    }
  }
  sign(app);
} else {
  run("codesign", ["--force", "--sign", "-", app]);
}
run("codesign", ["--verify", "--deep", "--strict", app]);
if (notarize) notarizeAndStaple(app);

const arch = process.arch === "arm64" ? "aarch64" : "x64";
const dmg = join(
  bundleDirectory,
  "dmg",
  `${config.productName}_${config.version}_${arch}.dmg`,
);
const stage = await mkdtemp(join(tmpdir(), "just-convert-dmg-"));
try {
  run("ditto", [app, join(stage, `${config.productName}.app`)]);
  await symlink("/Applications", join(stage, "Applications"));
  run("mkdir", ["-p", dirname(dmg)]);
  run("hdiutil", [
    "create",
    "-quiet",
    "-volname",
    config.productName,
    "-srcfolder",
    stage,
    "-format",
    "ULMO",
    "-ov",
    dmg,
  ]);
} finally {
  await rm(stage, { recursive: true, force: true });
}
if (identity) {
  run("codesign", ["--force", "--timestamp", "--sign", identity, dmg]);
}
if (notarize) notarizeAndStaple(dmg);
console.log(
  `Packaged ${app}\nPackaged ${dmg}\nSigning: ${identity ? `Developer ID${notarize ? ", notarized" : ""}` : "ad-hoc"}`,
);
