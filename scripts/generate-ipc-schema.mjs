#!/usr/bin/env node
/**
 * Emit docs/ipc-schema.json — manual sync list for TS/Python SDK authors.
 * Run: node scripts/generate-ipc-schema.mjs
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const out = path.join(root, "docs", "ipc-schema.json");

const schema = {
  version: 4,
  description: "Argus desktop IPC (newline-delimited JSON). Source of truth: crates/protocol.",
  requests: {
    fetch_env: {
      note: "v3 — no type field",
      fields: ["request_id", "bucket_id", "client_token", "cwd?"],
    },
    sandbox_create: {
      type: "sandbox_create",
      fields: [
        "request_id",
        "bucket_id",
        "client_token",
        "cwd?",
        "command_preview?",
      ],
    },
    sandbox_register_pids: {
      type: "sandbox_register_pids",
      fields: ["request_id", "session_id", "pids"],
    },
    sandbox_revoke: {
      type: "sandbox_revoke",
      fields: ["request_id", "session_id"],
    },
    sandbox_list: {
      type: "sandbox_list",
      fields: ["request_id"],
    },
  },
  responses: {
    ok: {
      fields: [
        "request_id",
        "env?",
        "proxy?",
        "session_id?",
        "proxy_port?",
        "expires_at?",
        "ca_bundle_path?",
        "relay_secret?",
        "sessions?",
      ],
    },
    denied: { fields: ["request_id", "code", "message"] },
    locked: { fields: ["request_id", "message"] },
    error: { fields: ["request_id", "code", "message"] },
  },
};

fs.writeFileSync(out, JSON.stringify(schema, null, 2) + "\n");
console.log(`Wrote ${out}`);
