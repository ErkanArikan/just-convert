// linuxdeploy rewrites and resolves every ELF file in an AppDir, which LibreOffice's optional
// modules and the frozen worker cannot survive; they are added to the AppImage after it is built.
import { execFileSync } from "node:child_process";
import { cp, readdir, readFile, rm } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const run = (program, args, options = {}) =>
  execFileSync(program, args, {
    cwd: projectRoot,
    stdio: ["ignore", "inherit", "inherit"],
    ...options,
  });

const config = JSON.parse(
  await readFile(resolve(projectRoot, "src-tauri/tauri.conf.json"), "utf8"),
);
const bundleDirectory = resolve(projectRoot, "src-tauri/target/release/bundle");
const lateResources = ["libreoffice", "pdf-to-docx"];

const engineConfig = resolve(projectRoot, "src-tauri/engines/linux.conf.json");
const extraConfig = process.argv.slice(2);

run("pnpm", [
  "tauri",
  "build",
  "--bundles",
  "deb",
  "--config",
  engineConfig,
  ...extraConfig,
]);
run("pnpm", [
  "tauri",
  "build",
  "--bundles",
  "appimage",
  "--config",
  engineConfig,
  ...extraConfig,
  "--config",
  JSON.stringify({
    bundle: {
      resources: Object.fromEntries(
        lateResources.map((name) => [`../vendor/${name}/`, null]),
      ),
    },
  }),
]);

const appimageDirectory = join(bundleDirectory, "appimage");
const appimageName = (await readdir(appimageDirectory)).find((name) =>
  name.endsWith(".AppImage"),
);
const appimage = join(appimageDirectory, appimageName);
const extracted = join(appimageDirectory, "squashfs-root");
await rm(extracted, { recursive: true, force: true });
run(appimage, ["--appimage-extract"], {
  cwd: appimageDirectory,
  stdio: "ignore",
});

const vendor = join(extracted, "usr/lib", config.productName, "vendor");
for (const name of lateResources) {
  await cp(resolve(projectRoot, "vendor", name), join(vendor, name), {
    recursive: true,
    verbatimSymlinks: true,
  });
}

// LibreOffice links NSS and NSPR, which desktops ship but minimal systems may not.
const nssLibraries = [
  "libnss3.so",
  "libnssutil3.so",
  "libsmime3.so",
  "libssl3.so",
  "libsoftokn3.so",
  "libfreeblpriv3.so",
  "libnspr4.so",
  "libplc4.so",
  "libplds4.so",
];
const libraryIndex = execFileSync("ldconfig", ["-p"], { encoding: "utf8" });
const libraryPath = (name) => {
  const line = libraryIndex
    .split("\n")
    .find((entry) => entry.trim().startsWith(`${name} `));
  return line?.split(" => ")[1]?.trim();
};
for (const name of nssLibraries) {
  let source = libraryPath(name);
  if (!source) {
    const nssDirectory = dirname(libraryPath("libnss3.so") ?? "") + "/nss";
    source = join(nssDirectory, name);
  }
  await cp(source, join(vendor, "libreoffice/program", name), {
    dereference: true,
  });
}

const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
const toolDirectory = join(homedir(), ".cache/tauri");
const pluginName = (await readdir(toolDirectory)).find((name) =>
  name.startsWith("linuxdeploy-plugin-appimage"),
);
if (!pluginName)
  throw new Error(`linuxdeploy-plugin-appimage not found in ${toolDirectory}`);
const plugin = join(toolDirectory, pluginName);
await rm(appimage, { force: true });
run(plugin, ["--appimage-extract-and-run", "--appdir", extracted], {
  cwd: appimageDirectory,
  env: { ...process.env, ARCH: arch, OUTPUT: appimageName },
});
await rm(extracted, { recursive: true, force: true });
console.log(`Packaged ${appimage}`);
