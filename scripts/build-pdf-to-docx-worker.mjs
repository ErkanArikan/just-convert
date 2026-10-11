import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const windows = process.platform === "win32";
const lockFile = resolve(projectRoot, "workers/requirements.lock");
const workerScript = resolve(projectRoot, "workers/pdf_to_docx.py");
const destination = resolve(projectRoot, "vendor/pdf-to-docx");
const executable = join(
  destination,
  windows ? "pdf-to-docx.exe" : "pdf-to-docx",
);
const pythonVersion = "3.12";

const fingerprint = createHash("sha256")
  .update(await readFile(lockFile))
  .update(await readFile(workerScript))
  .update(await readFile(fileURLToPath(import.meta.url)))
  .update(`${process.platform}-${process.arch}-${pythonVersion}`)
  .digest("hex");
const marker = resolve(
  projectRoot,
  `vendor/.engines/pdf-to-docx-${fingerprint}.done`,
);

async function exists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

if ((await exists(marker)) && (await exists(executable))) {
  console.log("pdf-to-docx worker: up to date");
  process.exit(0);
}

const run = (program, args) =>
  execFileSync(program, args, { stdio: ["ignore", "inherit", "inherit"] });

const work = await mkdtemp(join(tmpdir(), "just-convert-worker-"));
try {
  const venv = join(work, "venv");
  const bin = join(venv, windows ? "Scripts" : "bin");
  const python = join(bin, windows ? "python.exe" : "python");
  run("uv", [
    "venv",
    "--quiet",
    "--python-preference",
    "only-managed",
    "--python",
    pythonVersion,
    venv,
  ]);
  run("uv", [
    "pip",
    "install",
    "--quiet",
    "--python",
    python,
    "--require-hashes",
    "-r",
    lockFile,
  ]);
  run(join(bin, windows ? "pyinstaller.exe" : "pyinstaller"), [
    "--noconfirm",
    "--clean",
    "--log-level",
    "WARN",
    "--onedir",
    "--exclude-module",
    "tkinter",
    "--name",
    "pdf-to-docx",
    "--distpath",
    join(work, "dist"),
    "--workpath",
    join(work, "build"),
    "--specpath",
    work,
    workerScript,
  ]);
  await rm(destination, { recursive: true, force: true });
  await cp(join(work, "dist", "pdf-to-docx"), destination, {
    recursive: true,
    verbatimSymlinks: true,
  });

  const smoke = spawnSync(executable, [], { encoding: "utf8" });
  if (smoke.status !== 2 || !smoke.stderr.includes("Usage")) {
    throw new Error(
      `Frozen worker smoke test failed: ${smoke.stderr || smoke.error}`,
    );
  }
  await mkdir(dirname(marker), { recursive: true });
  await writeFile(marker, `${fingerprint}\n`);
  console.log(`pdf-to-docx worker: built ${executable}`);
} finally {
  await rm(work, { recursive: true, force: true });
}
