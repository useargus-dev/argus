/**
 * Reset local Argus data via the Rust `reset_db` binary (~/.argus by default).
 *
 *   pnpm db:reset
 *   ARGUS_DATA_DIR=/tmp/argus-test pnpm db:reset
 */

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = path.join(root, "src-tauri", "Cargo.toml");

const result = spawnSync(
  "cargo",
  ["run", "--manifest-path", manifest, "--bin", "reset_db", "--quiet"],
  { stdio: "inherit", env: process.env, cwd: root, shell: process.platform === "win32" },
);

process.exit(result.status ?? 1);
