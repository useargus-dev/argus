/**
 * Sync app version fields from a release tag (e.g. v0.2.1 → 0.2.1).
 * Used by .github/workflows/release.yml before tauri build so bundle
 * artifact names match the GitHub release tag.
 *
 *   node scripts/set-release-version.mjs v0.2.1
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tag = process.argv[2];

if (!tag) {
  console.error("Usage: node scripts/set-release-version.mjs <tag>");
  console.error("Example: node scripts/set-release-version.mjs v0.2.1");
  process.exit(1);
}

const version = tag.replace(/^v/i, "");
const semver =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][\w.-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][\w.-]*))*))?(?:\+([\w.-]+))?$/;

if (!semver.test(version)) {
  console.error(`Invalid release version "${version}" (from tag "${tag}")`);
  process.exit(1);
}

function updateJson(file, mutator) {
  const full = path.join(root, file);
  const data = JSON.parse(fs.readFileSync(full, "utf8"));
  mutator(data);
  fs.writeFileSync(full, `${JSON.stringify(data, null, 2)}\n`, "utf8");
  console.log(`  ${file} → ${version}`);
}

function updateCargoToml() {
  const appToml = path.join(root, "src-tauri", "Cargo.toml");
  const appText = fs.readFileSync(appToml, "utf8");
  const appPattern = /^version\s*=\s*"[^"]*"/m;
  if (!appPattern.test(appText)) {
    console.error("Could not find [package].version in src-tauri/Cargo.toml");
    process.exit(1);
  }
  fs.writeFileSync(
    appToml,
    appText.replace(appPattern, `version = "${version}"`),
    "utf8",
  );
  console.log(`  src-tauri/Cargo.toml → ${version}`);

  const workspaceToml = path.join(root, "Cargo.toml");
  const workspaceText = fs.readFileSync(workspaceToml, "utf8");
  const workspacePattern = /^version\s*=\s*"[^"]*"/m;
  if (!workspacePattern.test(workspaceText)) {
    console.error('Could not find [workspace.package].version in Cargo.toml');
    process.exit(1);
  }
  fs.writeFileSync(
    workspaceToml,
    workspaceText.replace(workspacePattern, `version = "${version}"`),
    "utf8",
  );
  console.log(`  Cargo.toml (workspace) → ${version}`);
}

console.log(`Setting release version to ${version} (from tag ${tag})`);

updateJson("src-tauri/tauri.conf.json", (data) => {
  data.version = version;
});
updateJson("package.json", (data) => {
  data.version = version;
});
updateCargoToml();

console.log("Done.");
