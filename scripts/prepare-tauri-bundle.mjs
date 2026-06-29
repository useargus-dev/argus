#!/usr/bin/env node
/**
 * Sync platform-specific sidecar paths in tauri.conf.json bundle.resources.
 *
 * Tauri validates resource paths literally; on Windows binaries are *.exe.
 *
 * Usage:
 *   node scripts/prepare-tauri-bundle.mjs [linux|windows|macos]   # release bundle
 *   node scripts/prepare-tauri-bundle.mjs --dev [platform]        # tauri dev (CLI only)
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const confPath = path.join(root, "src-tauri", "tauri.conf.json");

const argv = process.argv.slice(2);
const devMode = argv.includes("--dev");
const allowMissing = argv.includes("--allow-missing");
const platformArg = argv.find((a) => a !== "--dev" && a !== "--allow-missing");

const platform =
  platformArg ||
  (process.platform === "win32"
    ? "windows"
    : process.platform === "darwin"
      ? "macos"
      : "linux");

const conf = JSON.parse(fs.readFileSync(confPath, "utf8"));
const resources = {};

function releaseBinaryRel(baseName) {
  const rel = `../target/release/${baseName}`;
  return platform === "windows" ? `${rel}.exe` : rel;
}

function binaryExists(srcRel) {
  const abs = path.join(root, srcRel.replace(/^\.\.\//, ""));
  return fs.existsSync(abs);
}

function sidecarDest(baseName) {
  return platform === "windows"
    ? `lib/argus/${baseName}.exe`
    : `lib/argus/${baseName}`;
}

function addReleaseBinary(baseName, dest, { required = false } = {}) {
  const srcRel = releaseBinaryRel(baseName);
  if (binaryExists(srcRel)) {
    resources[srcRel] = dest;
    return true;
  }
  const msg = `prepare-tauri-bundle: missing ${srcRel} (cargo build --release -p ${baseName})`;
  if (required) {
    console.error(msg);
    process.exit(1);
  }
  console.warn(`${msg}; skipping`);
  return false;
}

addReleaseBinary("argus-cli", sidecarDest("argus-cli"), {
  required: !allowMissing,
});

if (!devMode) {
  if (platform === "linux") {
    addReleaseBinary(
      "argus-redirector-linux",
      sidecarDest("argus-redirector-linux"),
      { required: true },
    );
  }

  if (platform === "windows") {
    addReleaseBinary(
      "argus-redirector-windows",
      sidecarDest("argus-redirector-windows"),
      { required: true },
    );
    const windivertDir = path.join(root, "third_party", "windivert");
    for (const f of ["WinDivert.dll", "WinDivert64.sys"]) {
      const staged = path.join(windivertDir, f);
      if (fs.existsSync(staged)) {
        resources[`../third_party/windivert/${f}`] = `lib/argus/${f}`;
      } else {
        console.error(
          `prepare-tauri-bundle: missing ${staged} (run scripts/stage-windivert.ps1 after building redirector)`,
        );
        process.exit(1);
      }
    }
  }
}

conf.bundle.resources = resources;
fs.writeFileSync(confPath, `${JSON.stringify(conf, null, 2)}\n`);
console.log(
  `prepare-tauri-bundle: ${devMode ? "dev" : "bundle"} resources for ${platform}`,
);
