import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestDirectory = resolve(projectRoot, "vendor/manifests");
const markerDirectory = resolve(projectRoot, "vendor/.engines");

async function exists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function sha256File(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}

async function download(url, destination, expectedSha256) {
  const response = await fetch(url);
  if (!response.ok || !response.body) {
    throw new Error(`Download failed (HTTP ${response.status}): ${url}`);
  }
  const hash = createHash("sha256");
  const body = Readable.fromWeb(response.body);
  body.on("data", (chunk) => hash.update(chunk));
  await pipeline(body, createWriteStream(destination));
  const actual = hash.digest("hex");
  if (actual !== expectedSha256) {
    throw new Error(
      `Checksum mismatch for ${url}: expected ${expectedSha256}, received ${actual}`,
    );
  }
}

function run(program, args) {
  execFileSync(program, args, { stdio: ["ignore", "ignore", "inherit"] });
}

async function extractDebBundle(archive, root) {
  const unpacked = join(dirname(archive), "unpacked");
  await mkdir(unpacked, { recursive: true });
  run("tar", ["-xzf", archive, "-C", unpacked]);
  const debs = [];
  async function collect(directory) {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) await collect(path);
      else if (entry.name.endsWith(".deb")) debs.push(path);
    }
  }
  await collect(unpacked);
  for (const deb of debs) {
    run("dpkg-deb", ["-x", deb, root]);
  }
}

async function extract(entry, archive, root) {
  await mkdir(root, { recursive: true });
  switch (entry.kind) {
    case "tar":
      run("tar", ["-xf", archive, "-C", root]);
      return async () => {};
    case "zip":
      if (process.platform === "win32")
        run("tar", ["-xf", archive, "-C", root]);
      else run("unzip", ["-q", "-o", archive, "-d", root]);
      return async () => {};
    case "dmg": {
      const mountPoint = join(root, "mount");
      await mkdir(mountPoint);
      run("hdiutil", [
        "attach",
        "-nobrowse",
        "-readonly",
        "-noautoopen",
        "-mountpoint",
        mountPoint,
        archive,
      ]);
      return async () => run("hdiutil", ["detach", mountPoint, "-force"]);
    }
    case "deb-bundle":
      await extractDebBundle(archive, root);
      return async () => {};
    case "msi":
      run("msiexec", ["/a", archive, "/qn", `TARGETDIR=${root}`]);
      return async () => {};
    default:
      throw new Error(`Unknown archive kind: ${entry.kind}`);
  }
}

// `from` may use `*` for exactly one unknown path segment.
async function resolvePattern(base, pattern) {
  let candidates = [base];
  for (const segment of pattern.split("/")) {
    const next = [];
    for (const candidate of candidates) {
      if (segment === "*") {
        for (const child of await readdir(candidate))
          next.push(join(candidate, child));
      } else if (await exists(join(candidate, segment))) {
        next.push(join(candidate, segment));
      }
    }
    candidates = next;
  }
  if (candidates.length !== 1) {
    throw new Error(
      `Expected one match for ${pattern}, found ${candidates.length}`,
    );
  }
  return candidates[0];
}

async function locate(base, relative, depth = 5) {
  if (await exists(join(base, relative))) return base;
  if (depth === 0) return null;
  for (const entry of await readdir(base, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.isSymbolicLink()) continue;
    const found = await locate(join(base, entry.name), relative, depth - 1);
    if (found) return found;
  }
  return null;
}

// Prune and keep patterns may use `*` inside any path segment.
async function expandGlob(base, pattern) {
  let matches = [base];
  for (const segment of pattern.split("/")) {
    const next = [];
    if (!segment.includes("*")) {
      for (const match of matches) {
        if (await exists(join(match, segment))) next.push(join(match, segment));
      }
    } else {
      const regex = new RegExp(
        `^${segment
          .split("*")
          .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, "\\$&"))
          .join(".*")}$`,
      );
      for (const match of matches) {
        if (!(await stat(match)).isDirectory()) continue;
        for (const child of await readdir(match)) {
          if (regex.test(child)) next.push(join(match, child));
        }
      }
    }
    matches = next;
  }
  return matches;
}

async function prune(destination, file) {
  const kept = new Set();
  for (const pattern of file.keep ?? []) {
    for (const path of await expandGlob(destination, pattern)) kept.add(path);
  }
  for (const pattern of file.prune ?? []) {
    for (const path of await expandGlob(destination, pattern)) {
      if (!kept.has(path)) await rm(path, { recursive: true, force: true });
    }
  }
}

async function install(source, destination) {
  if ((await stat(source)).isFile()) source = await realpath(source);
  await rm(destination, { recursive: true, force: true });
  await mkdir(dirname(destination), { recursive: true });
  if (process.platform === "darwin") {
    run("ditto", [source, destination]);
  } else {
    await cp(source, destination, { recursive: true, dereference: true });
  }
}

async function fetchEntry(component, entry) {
  // Keyed on the whole entry so changing a prune list reinstalls the engine.
  const entryHash = createHash("sha256")
    .update(JSON.stringify(entry))
    .digest("hex");
  const marker = join(markerDirectory, `${entryHash}.done`);
  const outputs = entry.files.map((file) => resolve(projectRoot, file.to));
  if (
    (await exists(marker)) &&
    (await Promise.all(outputs.map(exists))).every(Boolean)
  ) {
    console.log(`${component}: up to date (${entry.url.split("/").pop()})`);
    return;
  }

  const work = await mkdtemp(join(tmpdir(), "just-convert-engine-"));
  try {
    const archive = join(work, entry.url.split("/").pop());
    const cached = process.env.ENGINE_CACHE_DIR
      ? join(process.env.ENGINE_CACHE_DIR, entry.sha256)
      : null;
    if (
      cached &&
      (await exists(cached)) &&
      (await sha256File(cached)) === entry.sha256
    ) {
      await cp(cached, archive);
      console.log(`${component}: using cached ${entry.url.split("/").pop()}`);
    } else {
      console.log(`${component}: downloading ${entry.url}`);
      await download(entry.url, archive, entry.sha256);
      if (cached) {
        await mkdir(dirname(cached), { recursive: true });
        await cp(archive, cached);
      }
    }
    const root = join(work, "root");
    const cleanup = await extract(entry, archive, root);
    try {
      const base = entry.kind === "dmg" ? join(root, "mount") : root;
      for (const file of entry.files) {
        const source = file.locate
          ? await locate(base, file.locate)
          : await resolvePattern(base, file.from);
        if (!source)
          throw new Error(`${component}: ${file.locate} not found in archive`);
        await install(source, resolve(projectRoot, file.to));
        await prune(resolve(projectRoot, file.to), file);
      }
    } finally {
      await cleanup();
    }
    await mkdir(markerDirectory, { recursive: true });
    await writeFile(marker, `${entry.url}\n`);
    console.log(
      `${component}: installed ${entry.files.map((file) => file.to).join(", ")}`,
    );
  } finally {
    await rm(work, { recursive: true, force: true });
  }
}

const manifests = (await readdir(manifestDirectory)).filter((name) =>
  name.endsWith(".json"),
);
for (const name of manifests.sort()) {
  const manifest = JSON.parse(
    await readFile(join(manifestDirectory, name), "utf8"),
  );
  const entries = (manifest.platformArchives ?? []).filter(
    (entry) =>
      entry.platform === process.platform && entry.arch === process.arch,
  );
  for (const entry of entries) {
    await fetchEntry(manifest.component, entry);
  }
}
