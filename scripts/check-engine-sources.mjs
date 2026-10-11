import { readFile, readdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const manifestDirectory = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../vendor/manifests",
);
const failures = [];
let checked = 0;
for (const name of (await readdir(manifestDirectory)).sort()) {
  if (!name.endsWith(".json")) continue;
  const manifest = JSON.parse(
    await readFile(join(manifestDirectory, name), "utf8"),
  );
  for (const entry of manifest.platformArchives ?? []) {
    checked += 1;
    const label = `${manifest.component} ${entry.platform}-${entry.arch}`;
    try {
      const response = await fetch(entry.url, {
        method: "HEAD",
        redirect: "follow",
      });
      if (!response.ok)
        failures.push(`${label}: HTTP ${response.status} ${entry.url}`);
      else console.log(`ok   ${label}`);
    } catch (error) {
      failures.push(`${label}: ${error.message} ${entry.url}`);
    }
  }
}
if (failures.length > 0) {
  console.error(
    `\n${failures.length} of ${checked} pinned engine downloads are unavailable:`,
  );
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log(`\nAll ${checked} pinned engine downloads are available.`);
