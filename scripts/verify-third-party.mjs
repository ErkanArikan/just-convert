import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { access, readFile, readdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestDirectory = resolve(projectRoot, "vendor/manifests");
const licenseIndexPath = resolve(
  projectRoot,
  "docs/licenses/THIRD_PARTY_LICENSES.md",
);

function projectPath(relativePath) {
  return resolve(projectRoot, relativePath);
}

async function requirePath(relativePath, expectedType = "file") {
  const absolutePath = projectPath(relativePath);
  await access(absolutePath);
  const details = await stat(absolutePath);
  if (expectedType === "file" && !details.isFile()) {
    throw new Error(`${relativePath} is not a file`);
  }
  if (expectedType === "directory" && !details.isDirectory()) {
    throw new Error(`${relativePath} is not a directory`);
  }
  return absolutePath;
}

async function sha256(relativePath) {
  const absolutePath = await requirePath(relativePath);
  const hash = createHash("sha256");
  await new Promise((resolvePromise, rejectPromise) => {
    const stream = createReadStream(absolutePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", resolvePromise);
    stream.on("error", rejectPromise);
  });
  return hash.digest("hex");
}

async function verifyHash(relativePath, expectedHash) {
  const actualHash = await sha256(relativePath);
  if (actualHash.toLowerCase() !== expectedHash.toLowerCase()) {
    throw new Error(
      `${relativePath} checksum mismatch: expected ${expectedHash}, received ${actualHash}`,
    );
  }
}

function requiredMetadata(manifest, fileName) {
  const license = manifest.license_identifier ?? manifest.licenseIdentifier;
  const source =
    manifest.source?.distribution_url ??
    manifest.source?.release_url ??
    manifest.sourceUrl;
  const date =
    manifest.download_date ?? manifest.downloadDate ?? manifest.detectedDate;
  for (const [field, value] of [
    ["component", manifest.component],
    ["version", manifest.version],
    ["license identifier", license],
    ["source URL", source],
    ["download/detection date", date],
  ]) {
    if (!value) throw new Error(`${fileName} is missing ${field}`);
  }
}

async function verifyManifest(fileName, licenseIndex) {
  const manifestPath = resolve(manifestDirectory, fileName);
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  requiredMetadata(manifest, fileName);
  if (!licenseIndex.includes(`| ${manifest.component}`)) {
    throw new Error(
      `${manifest.component} is missing from THIRD_PARTY_LICENSES.md`,
    );
  }

  const bundledFiles = manifest.bundled_files ?? manifest.runtime ?? [];
  for (const file of bundledFiles) {
    await verifyHash(file.path, file.sha256);
  }

  if (manifest.license_file) {
    await requirePath(manifest.license_file);
  }
  for (const licenseFile of manifest.license_files ?? []) {
    await verifyHash(licenseFile.path, licenseFile.sha256);
  }
  if (manifest.licenseFiles) {
    await requirePath(manifest.licenseFiles.directory, "directory");
    await verifyHash(
      `${manifest.licenseFiles.directory}LICENSE.txt`,
      manifest.licenseFiles.packageLicenseSha256,
    );
  }

  return bundledFiles.length;
}

async function verifyPackagedResources() {
  const config = JSON.parse(
    await readFile(resolve(projectRoot, "src-tauri/tauri.conf.json"), "utf8"),
  );
  const resources = Object.keys(config.bundle?.resources ?? {});
  for (const source of resources) {
    const relativePath = source.replace(/^\.\.\//, "");
    await requirePath(
      relativePath,
      source.endsWith("/") ? "directory" : "file",
    );
  }
  return resources.length;
}

const licenseIndex = await readFile(licenseIndexPath, "utf8");
for (const component of [
  "FFmpeg",
  "qpdf",
  "PDFium",
  "LibreOffice",
  "Python",
  "PyMuPDF",
  "pdf2docx",
  "rusqlite / SQLite",
]) {
  if (!licenseIndex.includes(`| ${component}`)) {
    throw new Error(`${component} is missing from THIRD_PARTY_LICENSES.md`);
  }
}
await requirePath("docs/licenses/rusqlite-LICENSE.txt");

const manifestFiles = (await readdir(manifestDirectory))
  .filter((fileName) => fileName.endsWith(".json"))
  .sort();
let bundledFileCount = 0;
for (const fileName of manifestFiles) {
  bundledFileCount += await verifyManifest(fileName, licenseIndex);
}
const resourceCount = await verifyPackagedResources();

console.log(
  `Third-party verification passed: ${manifestFiles.length} manifests, ` +
    `${bundledFileCount} checksummed runtime files, ${resourceCount} packaged resources.`,
);
