/**
 * Capture Argus UI screenshots for docs (mocked Tauri IPC — no desktop app required).
 *
 *   pnpm screenshots:capture
 *
 * Output: docs/assets/screenshots/*.png
 *
 * Requires Chromium once: pnpm exec playwright install chromium
 */

import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const OUT_DIR = path.join(ROOT, "docs", "assets", "screenshots");
const BASE_URL = "http://127.0.0.1:1420";
const VIEWPORT = { width: 1100, height: 720 };

/** Fixture data serialized into the browser mock (no real secrets). */
const FIXTURES = {
  profile: {
    email: "sushant@example.com",
    username: "robosushie",
    firstName: "Sushant",
    lastName: "Samuel",
  },
  scopes: {
    app: true,
    vault: true,
    buckets: true,
    vaultExpiresAt: null,
    bucketsExpiresAt: null,
  },
  bucketId: "550e8400-e29b-41d4-a716-446655440000",
  bucketToken: "demo-bucket-token-not-real",
  secrets: [
    {
      id: "sec-anthropic",
      name: "Anthropic Production Key",
      secretType: "api_key",
      organization: "Acme",
      environment: "prod",
      description: "Claude API for backend services",
      tags: ["llm", "anthropic"],
      expiresAt: null,
      isArchived: false,
      createdAt: "2026-01-10T12:00:00Z",
      updatedAt: "2026-03-01T09:00:00Z",
    },
    {
      id: "sec-openai",
      name: "OpenAI Dev Key",
      secretType: "api_key",
      organization: "Acme",
      environment: "dev",
      description: null,
      tags: ["llm"],
      expiresAt: "2026-06-01T00:00:00Z",
      isArchived: false,
      createdAt: "2026-01-15T12:00:00Z",
      updatedAt: "2026-02-20T09:00:00Z",
    },
    {
      id: "sec-db",
      name: "Supabase Database URL",
      secretType: "connection_string",
      organization: "Acme",
      environment: "prod",
      description: "Primary Postgres",
      tags: ["database"],
      expiresAt: null,
      isArchived: false,
      createdAt: "2025-12-01T12:00:00Z",
      updatedAt: "2026-01-05T09:00:00Z",
    },
  ],
  mappings: [
    {
      id: "map-anthropic",
      bucketId: "550e8400-e29b-41d4-a716-446655440000",
      envLabel: "ANTHROPIC_API_KEY",
      mappingType: "secret" as const,
      secretId: "sec-anthropic",
      secretName: "Anthropic Production Key",
      secretType: "api_key",
      textValue: null,
      proxyEnabled: true,
      proxyPlaceholder: "argus-proxy-demoAnthropicKey",
      allowedHosts: ["api.anthropic.com"],
      createdAt: "2026-02-01T12:00:00Z",
    },
    {
      id: "map-openai",
      bucketId: "550e8400-e29b-41d4-a716-446655440000",
      envLabel: "OPENAI_API_KEY",
      mappingType: "secret" as const,
      secretId: "sec-openai",
      secretName: "OpenAI Dev Key",
      secretType: "api_key",
      textValue: null,
      proxyEnabled: false,
      proxyPlaceholder: null,
      allowedHosts: [],
      createdAt: "2026-02-01T12:00:00Z",
    },
    {
      id: "map-db",
      bucketId: "550e8400-e29b-41d4-a716-446655440000",
      envLabel: "DATABASE_URL",
      mappingType: "secret" as const,
      secretId: "sec-db",
      secretName: "Supabase Database URL",
      secretType: "connection_string",
      textValue: null,
      proxyEnabled: false,
      proxyPlaceholder: null,
      allowedHosts: [],
      createdAt: "2026-02-01T12:00:00Z",
    },
  ],
  pendingRequest: {
    requestId: "req-demo-001",
    bucketId: "550e8400-e29b-41d4-a716-446655440000",
    bucketName: "Acme Backend",
    fingerprint: "abc123fingerprint",
    pid: 12840,
    exePath: "C:\\Program Files\\nodejs\\node.exe",
    cwd: "E:\\Projects\\acme-backend",
    cwdVerified: true,
    runArgs: "node scripts/run_langchain.js",
    gitRemote: "https://github.com/acme/backend.git",
    processName: "node",
    machineId: "demo-machine-id",
    accessTtlMinutes: 60,
    createdAt: new Date(Date.now() - 45_000).toISOString(),
  },
  activeGrant: {
    id: "grant-demo-001",
    bucketId: "550e8400-e29b-41d4-a716-446655440000",
    bucketName: "Acme Backend",
    fingerprint: "def456fingerprint",
    clientLabel: "python",
    grantedAt: new Date(Date.now() - 3_600_000).toISOString(),
    expiresAt: new Date(Date.now() + 7_200_000).toISOString(),
    lastSeenAt: new Date(Date.now() - 120_000).toISOString(),
    isActive: true,
    cwd: "E:\\Projects\\acme-backend",
    exePath: "C:\\Python312\\python.exe",
    gitRemote: "https://github.com/acme/backend.git",
    runArgs: "python -m app.main",
  },
  settings: {
    auto_lock_minutes: "15",
    lock_on_screen_lock: "true",
    run_in_background: "true",
    notify_client_access: "true",
    expiry_notify_days: "7",
  },
  totpSetup: {
    secret: "JBSWY3DPEHPK3PXP",
    otpauthUri:
      "otpauth://totp/Argus:robosushie?secret=JBSWY3DPEHPK3PXP&issuer=Argus",
  },
  secondFactorStatus: {
    activeSecondFactor: "totp" as const,
    totpEnrolled: true,
    biometricEnrolled: false,
  },
  /** Demo recovery code for register-complete screenshot (not real). */
  recoveryCode: "DEM2K7M9",
};

type Scenario =
  | "no-account"
  | "register-complete"
  | "has-account"
  | "login-totp"
  | "authenticated"
  | "approvals";

type CaptureStep = {
  file: string;
  path: string;
  scenario: Scenario;
  setup?: (page: import("playwright").Page) => Promise<void>;
  viewport?: { width: number; height: number };
};

const CAPTURES: CaptureStep[] = [
  {
    file: "01-register-account.png",
    path: "/register",
    scenario: "no-account",
  },
  {
    file: "02-register-second-factor.png",
    path: "/register",
    scenario: "no-account",
    async setup(page) {
      await page.getByLabel("First name").fill("Sushant");
      await page.getByLabel("Last name").fill("Samuel");
      await page.getByLabel("Username").fill("robosushie");
      await page.getByLabel("Master password", { exact: true }).fill("demo-password-12");
      await page.getByLabel("Confirm master password").fill("demo-password-12");
      await page.getByRole("button", { name: "Continue" }).click();
      await page.getByText("Add a second factor").waitFor();
    },
  },
  {
    file: "03-register-provisioning.png",
    path: "/register/provisioning",
    scenario: "no-account",
  },
  {
    file: "04-register-complete.png",
    path: "/register/provisioning",
    scenario: "register-complete",
    async setup(page) {
      await page.getByText("Account secured").waitFor();
      await page.getByText("DEM2-K7M9").waitFor();
    },
  },
];

function buildMockInvokeSource(): string {
  return `
(function () {
  const FIXTURES = ${JSON.stringify(FIXTURES)};

  function scenario() {
    const params = new URLSearchParams(window.location.search);
    return params.get("argus-scenario") || "authenticated";
  }

  function bucket() {
    return {
      id: FIXTURES.bucketId,
      name: "Acme Backend",
      description: "Node + Python services with Argus proxy",
      isActive: true,
      accessTtlMinutes: 60,
      refreshTtlMinutes: null,
      sessionTtlMinutes: 480,
      mappingCount: FIXTURES.mappings.length,
      activeGrantCount: 1,
      proxyEnabled: bucketProxyEnabled,
      proxyPort: bucketProxyEnabled ? 9001 : null,
      createdAt: "2026-01-01T12:00:00Z",
      updatedAt: "2026-03-15T09:00:00Z",
    };
  }

  let bucketProxyEnabled = false;

  const callbacks = new Map();
  const eventListeners = new Map();
  let callbackSeq = 1;

  function emitEvent(event, payload) {
    const handlers = eventListeners.get(event) || [];
    for (const handlerId of handlers) {
      const cb = callbacks.get(handlerId);
      if (cb) cb({ event, payload });
    }
  }

  window.__TAURI_INTERNALS__ = {
    transformCallback(cb) {
      const id = callbackSeq++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback(id) {
      callbacks.delete(id);
    },
    runCallback(id, data) {
      const cb = callbacks.get(id);
      if (cb) cb(data);
    },
    async invoke(cmd, args) {
      const sc = scenario();

      if (cmd === "has_account") {
        return sc !== "no-account" && sc !== "register-complete";
      }
      if (cmd === "is_signed_in") {
        return sc === "authenticated" || sc === "approvals";
      }
      if (cmd === "sign_in") {
        if (sc === "login-totp" && !(args?.req?.totpCode)) {
          throw JSON.stringify({
            code: "SECOND_FACTOR_REQUIRED",
            message: "Second factor required",
            secondFactorType: "totp",
          });
        }
        return FIXTURES.profile;
      }
      if (cmd === "sign_out") return;
      if (cmd === "get_scope_status") return FIXTURES.scopes;
      if (cmd === "get_profile") return FIXTURES.profile;
      if (cmd === "unlock_app") return FIXTURES.scopes;
      if (cmd === "lock_app") return { ...FIXTURES.scopes, app: false };
      if (cmd === "prepare_totp_setup") return FIXTURES.totpSetup;
      if (cmd === "register_validate") return;
      if (cmd === "register_finalize") {
        if (sc === "register-complete") {
          setTimeout(function () {
            emitEvent("register-progress", {
              step: "complete",
              status: "done",
              recoveryCode: FIXTURES.recoveryCode,
            });
          }, 100);
        }
        return;
      }
      if (cmd === "take_registration_recovery_code") return FIXTURES.recoveryCode;
      if (cmd === "verify_biometric") return;
      if (cmd === "get_second_factor_status") return FIXTURES.secondFactorStatus;
      if (cmd === "get_second_factor_type") return "totp";
      if (cmd === "get_settings") return FIXTURES.settings;
      if (cmd === "set_setting") return;
      if (cmd === "search_secrets") return FIXTURES.secrets;
      if (cmd === "get_secret") {
        const id = args?.id;
        const meta = FIXTURES.secrets.find((s) => s.id === id) || FIXTURES.secrets[0];
        return {
          ...meta,
          value: { key: "sk-ant-demo-placeholder-not-real" },
        };
      }
      if (cmd === "list_buckets") return [bucket()];
      if (cmd === "set_bucket_proxy_enabled") {
        bucketProxyEnabled = Boolean(args?.enabled);
        return bucket();
      }
      if (cmd === "get_bucket_token") return FIXTURES.bucketToken;
      if (cmd === "list_bucket_mappings") return FIXTURES.mappings;
      if (cmd === "upsert_bucket_mapping") {
        const req = args?.req ?? {};
        const idx = FIXTURES.mappings.findIndex(
          (m) => m.envLabel === req.envLabel,
        );
        const base = idx >= 0 ? FIXTURES.mappings[idx] : FIXTURES.mappings[0];
        return {
          ...base,
          ...req,
          id: base.id,
          bucketId: FIXTURES.bucketId,
          proxyPlaceholder: req.proxyEnabled
            ? base.proxyPlaceholder ?? "argus-proxy-demoAnthropicKey"
            : null,
        };
      }
      if (cmd === "list_pending") {
        const path = window.location.pathname;
        if (path === "/approvals" || path === "/requests") {
          return [FIXTURES.pendingRequest];
        }
        return [];
      }
      if (cmd === "list_grants") {
        return [FIXTURES.activeGrant];
      }
      if (cmd === "pending_count") {
        const path = window.location.pathname;
        if (path === "/dashboard") return 1;
        if (path === "/approvals" || path === "/requests") return 1;
        return 0;
      }
      if (cmd === "plugin:event|listen") {
        const event = args?.event;
        const handler = args?.handler;
        if (event && handler != null) {
          if (!eventListeners.has(event)) eventListeners.set(event, []);
          eventListeners.get(event).push(handler);
        }
        return handler;
      }
      if (cmd === "plugin:event|unlisten") {
        const event = args?.event;
        const eventId = args?.eventId;
        if (event && eventListeners.has(event)) {
          const list = eventListeners.get(event);
          const idx = list.indexOf(eventId);
          if (idx >= 0) list.splice(idx, 1);
        }
        callbacks.delete(eventId);
        return;
      }
      if (cmd.startsWith("plugin:")) return;

      return null;
    },
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(event, eventId) {
      callbacks.delete(eventId);
      if (eventListeners.has(event)) {
        const list = eventListeners.get(event);
        const idx = list.indexOf(eventId);
        if (idx >= 0) list.splice(idx, 1);
      }
    },
  };

  const style = document.createElement("style");
  style.textContent = \`
    button[aria-label*="Switch to"] { display: none !important; }
    [data-sonner-toaster] { display: none !important; }
  \`;
  document.documentElement.appendChild(style);
})();
`;
}

async function waitForServer(url: string, timeoutMs = 60_000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      await new Promise<void>((resolve, reject) => {
        const req = http.get(url, (res) => {
          res.resume();
          resolve();
        });
        req.on("error", reject);
        req.setTimeout(2000, () => {
          req.destroy();
          reject(new Error("timeout"));
        });
      });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  throw new Error(`Vite dev server did not start at ${url}`);
}

function startVite(): ChildProcess {
  const child = spawn("pnpm", ["exec", "vite", "--host", "127.0.0.1", "--port", "1420"], {
    cwd: ROOT,
    shell: true,
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, FORCE_COLOR: "0" },
  });
  return child;
}

async function snap(page: import("playwright").Page, file: string): Promise<void> {
  await page.waitForTimeout(350);
  const outPath = path.join(OUT_DIR, file);
  await page.screenshot({ path: outPath, fullPage: false });
  console.log(`  ✓ ${file}`);
}

async function captureAuthScreens(page: import("playwright").Page): Promise<void> {
  for (const step of CAPTURES) {
    const viewport = step.viewport ?? VIEWPORT;
    await page.setViewportSize(viewport);
    await page.goto(routeUrl(step.path, step.scenario), {
      waitUntil: "networkidle",
    });
    if (step.setup) await step.setup(page);
    await snap(page, step.file);
  }
}

async function captureSignedInScreens(page: import("playwright").Page): Promise<void> {
  await page.setViewportSize(VIEWPORT);
  await signInForAuthenticated(page);
  await snap(page, "05-dashboard.png");

  await page.getByRole("link", { name: "Vault", exact: true }).click();
  await page.waitForURL("**/vault**");
  await snap(page, "06-vault.png");

  await page.getByRole("link", { name: "Buckets", exact: true }).click();
  await page.waitForURL("**/buckets**");
  await snap(page, "07-buckets.png");

  await page.getByRole("link", { name: "Open bucket" }).click();
  await page.waitForURL(`**/buckets/${FIXTURES.bucketId}**`);
  await page.getByRole("heading", { name: "Acme Backend" }).waitFor();

  // 08 — bucket-level Argus Proxy disabled (switch off)
  await page.locator("button[aria-expanded]").filter({ hasText: "Argus Proxy" }).click();
  await page.getByRole("switch", { name: "Enable proxy" }).waitFor();
  await snap(page, "08-bucket-detail-proxy.png");

  // Enable bucket proxy, then configure ANTHROPIC mapping
  await page.getByRole("switch", { name: "Enable proxy" }).click();
  await page.getByText("127.0.0.1:9001").waitFor();

  await page.getByText("ANTHROPIC_API_KEY").click();
  await page.getByText("Inject proxy token").waitFor();
  await page.getByText("Allowed domains", { exact: true }).waitFor();
  await page.getByText("api.anthropic.com").waitFor();
  await snap(page, "09-bucket-mapping-proxy.png");

  // 10 — injected proxy token revealed for ANTHROPIC_API_KEY
  await page.getByRole("button", { name: "Show proxy token" }).click();
  await page.getByText("argus-proxy-demoAnthropicKey").waitFor();
  await snap(page, "10-bucket-mapping-inject-token.png");

  await page.getByRole("link", { name: "Approvals", exact: true }).click();
  await page.waitForURL("**/approvals**");
  await page.getByRole("heading", { name: /Pending \(\d+\)/ }).waitFor();
  await snap(page, "11-approvals.png");

  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page.waitForURL("**/settings**");
  await snap(page, "12-settings.png");
}

async function signInForAuthenticated(page: import("playwright").Page): Promise<void> {
  await page.goto(`${BASE_URL}/login?argus-scenario=authenticated`, {
    waitUntil: "networkidle",
  });
  const url = page.url();
  if (url.includes("/dashboard")) return;
  await page.getByLabel("Email or username").fill("robosushie");
  await page.getByLabel("Password", { exact: true }).fill("demo-password-12");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL("**/dashboard", { timeout: 15_000 });
}

function routeUrl(path: string, scenario: Scenario): string {
  const sep = path.includes("?") ? "&" : "?";
  return `${BASE_URL}${path}${sep}argus-scenario=${encodeURIComponent(scenario)}`;
}

async function main(): Promise<void> {
  fs.mkdirSync(OUT_DIR, { recursive: true });

  let playwright: typeof import("playwright");
  try {
    playwright = await import("playwright");
  } catch {
    console.error(
      "Playwright is required. Run:\n  pnpm add -D playwright\n  pnpm exec playwright install chromium",
    );
    process.exit(1);
  }

  console.log("Starting Vite dev server…");
  const vite = startVite();
  vite.stderr?.on("data", (chunk: Buffer) => {
    const line = chunk.toString();
    if (line.toLowerCase().includes("error")) process.stderr.write(line);
  });

  try {
    await waitForServer(BASE_URL);
    console.log(`Vite ready at ${BASE_URL}`);

    const browser = await playwright.chromium.launch();
    const context = await browser.newContext({
      viewport: VIEWPORT,
      deviceScaleFactor: 1,
      colorScheme: "dark",
    });

    await context.addInitScript({ content: buildMockInvokeSource() });

    const page = await context.newPage();

    await captureAuthScreens(page);
    await captureSignedInScreens(page);

    await browser.close();
    console.log("\nDone — 12 screenshots in docs/assets/screenshots/");
  } finally {
    vite.kill("SIGTERM");
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
