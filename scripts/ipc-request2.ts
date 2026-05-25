/**
 * Send one NDJSON IPC request to a running Argus instance (sign-in + app unlocked).
 *
 * Identity (URI / cwd) is derived by Argus from this process — not from JSON.
 * Run from the project directory you want to authorize:
 *
 *   cd E:/Projects/argus-project/argus
 *   pnpm ipc:test -- --bucket-id ID --token TOKEN
 *
 * Windows: \\.\pipe\argus — macOS/Linux: ~/.argus/argus.sock
 */

import { randomUUID } from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { parseArgs } from "node:util";

interface IpcRequest {
  request_id: string;
  bucket_id: string;
  client_token: string;
  cwd?: string;
}

interface IpcResponse {
  status: string;
  request_id?: string;
  env?: Record<string, string>;
  code?: string;
  message?: string;
}

function socketPath(): string {
  return path.join(os.homedir(), ".argus", "argus.sock");
}

function readLine(socket: net.Socket, timeoutMs: number): Promise<string> {
  return new Promise((resolve, reject) => {
    let buf = "";
    let settled = false;
    const settle = (run: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.removeAllListeners("data");
      socket.removeAllListeners("error");
      socket.removeAllListeners("end");
      run();
    };

    const timer = setTimeout(() => {
      socket.destroy();
      settle(() => reject(new Error("timed out waiting for response")));
    }, timeoutMs);

    socket.on("data", (chunk: Buffer) => {
      buf += chunk.toString("utf8");
      const nl = buf.indexOf("\n");
      if (nl !== -1) {
        settle(() => resolve(buf.slice(0, nl).trim()));
      }
    });
    socket.on("error", (e) => settle(() => reject(e)));
    socket.on("end", () => {
      if (buf.length > 0) {
        settle(() => resolve(buf.trim()));
      } else {
        settle(() => reject(new Error("connection closed without response")));
      }
    });
  });
}

async function sendUnix(sockPath: string, payload: IpcRequest): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ path: sockPath });
    const line = `${JSON.stringify(payload)}\n`;

    socket.once("connect", () => {
      readLine(socket, 130_000).then(resolve).catch(reject);
      socket.write(line, "utf8");
    });
    socket.once("error", reject);
  });
}

async function sendWindows(payload: IpcRequest): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = net.connect("\\\\.\\pipe\\argus");
    const line = `${JSON.stringify(payload)}\n`;

    socket.once("connect", () => {
      readLine(socket, 130_000).then(resolve).catch(reject);
      socket.write(line, "utf8");
    });
    socket.once("error", reject);
  });
}

function usage(): never {
  console.error(`Usage: pnpm ipc:test -- --bucket-id ID --token TOKEN

  Run from the project directory to authorize (Argus reads this process cwd).

  --bucket-id   ARGUS_BUCKET_ID from the bucket .env panel
  --token       ARGUS_BUCKET_TOKEN

  Current cwd: ${process.cwd()}`);
  process.exit(1);
}

async function main(): Promise<number> {
  const { values } = parseArgs({
    options: {
      "bucket-id": { type: "string" },
      token: { type: "string" },
    },
    allowPositionals: false,
  });

  const bucketId = values["bucket-id"];
  const token = values.token;
  if (!bucketId || !token) usage();

  const payload: IpcRequest = {
    request_id: randomUUID(),
    bucket_id: bucketId,
    client_token: token,
    cwd: process.cwd(),
  };

  console.error(`cwd: ${process.cwd()}`);

  let raw: string;
  try {
    if (process.platform === "win32") {
      raw = await sendWindows(payload);
    } else {
      const sock = socketPath();
      if (!fs.existsSync(sock)) {
        console.error(`socket not found: ${sock}`);
        console.error("Sign in to Argus and keep the app unlocked.");
        return 1;
      }
      raw = await sendUnix(sock, payload);
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.error(`connection failed: ${msg}`);
    console.error(
      process.platform === "win32"
        ? "Is Argus signed in? Named pipe \\\\.\\pipe\\argus must exist."
        : "Is Argus signed in?",
    );
    return 1;
  }

  console.log(raw);

  let resp: IpcResponse;
  try {
    resp = JSON.parse(raw) as IpcResponse;
  } catch {
    return 1;
  }

  if (resp.status === "ok" && resp.env) {
    const keys = Object.keys(resp.env).sort();
    console.log(`\n${keys.length} env var(s):`);
    for (const k of keys) {
      console.log(`  ${k}=***`);
    }
    return 0;
  }

  if (resp.status === "locked") {
    console.error("Argus is locked or signed out — unlock the app and retry.");
  } else if (resp.status === "denied") {
    console.error("Access denied (or approval timed out).");
  } else if (resp.status === "error") {
    console.error(`error ${resp.code ?? "?"}: ${resp.message ?? "unknown"}`);
  }
  return 1;
}

main().then((code) => process.exit(code));
