# Argus — System Architecture

> **Argus** is a local-first secrets vault and approval gateway for developer environments.  
> One encrypted database. One OS user. Zero cloud. Secrets leave the machine only when a human approves a specific process identity.

> **Implementation note:** **IPC** (local socket/pipe + OS-verified fingerprint + grants + requests popup window + approvals page) and **tray** (hide-on-close, left-click opens requests) are **shipped** on desktop. **Python/Node client libraries** and advanced tray menus (per-bucket submenu, pause IPC) remain **planned**. See [README](../README.md).

**Related documents:** [plan.md](./plan.md) · [design.md](./design.md) · [security.md](./security.md)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [System Context](#2-system-context)
3. [Trust Boundaries](#3-trust-boundaries)
4. [High-Level Architecture](#4-high-level-architecture)
5. [Runtime Topology](#5-runtime-topology)
6. [Platform Interfaces](#6-platform-interfaces)
7. [Rust Backend Architecture](#7-rust-backend-architecture)
8. [Frontend Architecture](#8-frontend-architecture)
9. [Data Layer](#9-data-layer)
10. [Cryptography Pipeline](#10-cryptography-pipeline)
11. [IPC & Socket Server](#11-ipc--socket-server)
12. [Authorization Scopes](#12-authorization-scopes)
13. [Tray & Background Service](#13-tray--background-service)
14. [Session, Client Grants & Audit](#14-session-client-grants--audit)
15. [Three Access Tiers](#15-three-access-tiers)
16. [Client Libraries & CLI](#16-client-libraries--cli)
17. [Background Services](#17-background-services)
18. [Configuration & File Layout](#18-configuration--file-layout)
19. [Technology Decisions](#19-technology-decisions)
20. [Explicit Non-Goals](#20-explicit-non-goals)

---

## 1. Executive Summary

Argus replaces plaintext `.env` files in projects with:

| In the project | In Argus |
|---|---|
| `ARGUS_BUCKET_ID=<uuid>` | Full secret values, encrypted |
| Optional `.env` fallback keys | Typed schemas, expiry, audit |

**Core guarantees (always at rest):**

- `~/.argus/argus.db` is opaque (SQLCipher AES-256).

**Core guarantees (when signed out):**

- No socket / named pipe endpoint exists; `db_key` zeroized; SQLCipher pool closed.
- No secret material in frontend memory.

**Core guarantees (when signed in — including while app is soft-locked):**

- **Argus core** may run in the **system tray** with the main window closed; active buckets stay visible and IPC stays up.
- External apps connect via **local IPC** with `bucket_id` + `client_token`. Client identity is derived server-side via OS process inspection and hashed into a **fingerprint**.
- **New clients** always trigger a user approval popup; returning clients use **per-bucket TTL / refresh** policy.
- **Process access requests and grant approvals** continue while the app is soft-locked (only sign-in required).
- **App soft lock** blocks vault, buckets, dashboard, and settings UI — not IPC or approvals (see §12).
- Injectable secret types are enforced in **Rust**, not the UI.

---

## 2. System Context

```
                    ┌─────────────────────────────────────┐
                    │         Developer Machine            │
                    │                                      │
  Project repos     │   ┌─────────────────────────────┐   │
  (.env w/ bucket)  │   │      Argus (Tauri 2)         │   │
        │           │   │  React UI ◄──► Rust Core     │   │
        │           │   │         │                    │   │
        └───────────┼──►│    ~/.argus/argus.db        │   │
                    │   │    ~/.argus/argus.sock      │   │
                    │   │    System tray (active buckets)│   │
                    │   └──────────────┬──────────────┘   │
                    │                  │ IPC               │
                    │     ┌────────────┼────────────┐      │
                    │     ▼            ▼            ▼      │
                    │  Python      Node/Bun      CLI     │
                    │  library     library        argus  │
                    └─────────────────────────────────────┘

        ✗ No cloud sync    ✗ No telemetry    ✗ No remote API
```

**Scope of this architecture:** desktop application for **Windows, macOS, and Linux** only. Self-hosted servers, cloud vaults, and team sync are **out of scope**.

---

## 3. Trust Boundaries

| Zone | Trust level | Can access |
|---|---|---|
| **Rust core** (`src-tauri/`) | Highest | DB file, crypto keys, socket, process inspection, notifications |
| **WebView (React)** | Untrusted UI | Only `invoke()` commands and events exposed via Tauri capabilities |
| **Client libraries** | Untrusted external | Socket only; no DB; no decrypt without approval |
| **Project `.env`** | Public to repo | `ARGUS_BUCKET_ID` only (meaningless without Argus + approval) |
| **OS / root attacker** | Out of scope | Memory dumps, kernel hooks — documented in [security.md](./security.md) |

### Rules enforced at boundaries

1. Frontend **never** opens `argus.db`, **never** binds the socket, **never** holds `db_key`.
2. Socket handler **never** returns `credential`, `recovery_codes`, or `note` types to libraries (error `SECRET_TYPE_NOT_INJECTABLE`).
3. All `invoke()` inputs are validated in Rust (length limits, UUID format, enum variants).
4. Tauri **capabilities** whitelist commands per window; default deny.

References: [Tauri Security Model](https://v2.tauri.app/security/), [Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/).

---

## 4. High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         ARGUS DESKTOP APPLICATION                         │
│                                                                           │
│  ┌─────────────────────────────┐    ┌──────────────────────────────────┐ │
│  │     Presentation (React)     │    │         Domain Core (Rust)        │ │
│  │                              │    │                                   │ │
│  │  pages/                      │    │  commands/  ← Tauri invoke API    │ │
│  │  components/                 │◄──►│  db/        ← SQLCipher + repos    │ │
│  │  state/ (Zustand)            │    │  crypto/    ← KDF, AES-GCM, TOTP   │ │
│  │  hooks/                      │    │  ipc/       ← IPC server + peer     │ │
│  │  lib/tauri-bridge.ts         │    │  sessions/  ← pending approvals     │ │
│  │                              │    │  background/← timers, lock watcher  │ │
│  └─────────────────────────────┘    └──────────────────────────────────┘ │
│              │ events: client-access-requested, locked, expiry-alert     │
│              │ invoke: unlock, secrets.*, buckets.*, ...                   │
└──────────────────────────────┬───────────────────────────────────────────┘
                               │
                    Local IPC (see §6)
                               │
         ┌─────────────────────┼─────────────────────┐
         ▼                     ▼                     ▼
   argus-secrets-py     @argus-secrets/node      argus CLI
```

### Layer responsibilities

| Layer | Responsibility |
|---|---|
| **Presentation** | UX, forms, search UI, requests popup, approvals page, masked display |
| **Tauri bridge** | Typed wrappers over `invoke`, event subscriptions |
| **Commands** | Authorize UI actions, decrypt for display, CRUD |
| **Domain services** | Business rules: access matrix, approval TTL, audit writes |
| **Infrastructure** | SQLCipher connection, socket I/O, `sysinfo`, notifications |

---

## 5. Runtime Topology

Argus has **three operational states**:

| State | Socket / tray | DB connection | UI |
|---|---|---|---|
| **Signed out** | Stopped; tray hidden | Closed / key zeroized | `/login` |
| **Signed in, app unlocked** | Listening + tray active | Open SQLCipher pool | Dashboard / vault / buckets / settings / approvals |
| **Signed in, app locked** | Listening + tray active (IPC **unchanged**) | Open SQLCipher pool (keys in memory) | Vault/buckets/settings blocked; **requests + approvals still work** |
| **Window closed** | Tray + IPC **remain** if user enabled “Run in background” | Stays open until sign-out | None (tray only) |
| **First run** | Stopped | Creating schema + account + **mandatory 2FA** | `/register` |

### Sign-in sequence (Scope 1 — App shell)

```
User enters email/username + password
        │
        ▼
Second factor (required — configured at register):
        EITHER valid TOTP code
        OR successful biometric (Windows Hello / Touch ID)
        │
        ▼
crypto/kdf.rs ──► Argon2id verify + derive db_key
        │
        ▼
Open SQLCipher → spawn background jobs → start socket server → show tray
        │
        └──► emit "signed-in" → /dashboard
        └──► Scopes in memory: APP unlocked; VAULT and BUCKETS follow APP (no separate timers)
```

**Register (first run)** must complete **one** second-factor method before the account is usable:

| Method | Stored | Platforms |
|---|---|---|
| **TOTP** | Encrypted `totp_secret` | All |
| **Biometric** | OS-wrapped unlock key + `second_factor_type = biometric` | Windows, macOS (Linux: TOTP only) |

User chooses **TOTP or biometric** at setup — **not optional** to skip both.

### Sign-out vs soft app lock

**Soft app lock** (`lock_app`, idle `auto_lock_minutes`, or planned screen-lock hook):

```
soft_lock_app() or auto-lock fires
        │
        ├──► app_locked = true; vault/bucket UI scopes cleared
        ├──► IPC server, tray, and DB pool **stay running**
        ├──► pending client requests + approve/deny + grant list/revoke **still work**
        └──► emit "app-locked" → AppLockModal on vault/buckets/settings routes
```

**Sign-out** (Settings → Sign out, tray menu, or planned screen-lock → sign-out):

```
sign_out()
        │
        ├──► socket server shutdown + unlink socket file
        ├──► cancel background IPC tasks
        ├──► zeroize keys + clear session + close DB pool
        └──► emit "signed-out" → frontend navigates to /login
```

**Scope 2 / 3 (Vault & bucket CRUD) — shipped:** Vault and bucket mutations require **app unlock** only. Separate per-scope elevation was removed; `elevate_vault` / `elevate_buckets` are legacy no-ops when the app is unlocked. Setting `vault_read_requires_elevation` exists in the DB but is not used for a separate elevation step in current builds. **IPC client access is not gated by app lock** — only by sign-in and grant policy.

---

## 6. Platform Interfaces

### 6.1 Data directory (`~/.argus/`)

| File / path | Platform | Permissions | Lifecycle |
|---|---|---|---|
| `argus.db` | All | `0600` (user read/write) | Created first run |
| `argus.sock` | macOS, Linux | `0600` after bind | Created on sign-in, removed on sign-out |
| `\\.\pipe\argus` | Windows | DACL: current user only | Same lifecycle |
| `logs/` (optional) | All | `0700` dir | Debug builds only; no secrets |

Use `dirs` crate → `data_local_dir()` / equivalent, then `~/.argus` (documented, not hardcoded to `$HOME` on Windows).

### 6.2 IPC transport matrix **(shipped)**

| Platform | Mechanism | Path / name | Security notes |
|---|---|---|---|
| **Linux** | `AF_UNIX` `SOCK_STREAM` | `~/.argus/argus.sock` | `chmod 0600` after `bind`; peer UID check optional hardening |
| **macOS** | Same as Linux | Same | Same; screen lock via `NSWorkspace` notification |
| **Windows** | Named pipe | `\\.\pipe\argus` | Custom security descriptor: **no** `Everyone` / `Anonymous`; current-user SID only; consider `FILE_FLAG_FIRST_PIPE_INSTANCE` |

**Protocol:** newline-delimited JSON (NDJSON), one request per connection, max message size **64 KiB** (configurable).

**Why not TCP localhost:** avoids port enumeration, firewall prompts, and accidental remote exposure. OS-enforced local IPC only.

### 6.3 OS integration

| Feature | macOS | Windows | Linux |
|---|---|---|---|
| Screen lock detection | `NSWorkspace` | `WTS_SESSION_LOCK` | DBus `ScreenSaver` |
| Notifications | `tauri-plugin-notification` | Same | Same (freedesktop) |
| Process identity | Native OS APIs + `sysinfo` fallback → fingerprint | Same | Same |
| Clipboard clear | WebView + Rust timer | Same | Same |
| Code signing (Phase 4) | Notarization | Authenticode | AppImage + GPG |

### 6.4 WebView / Tauri shell

- **Tauri 2.x** with system WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux).
- **CSP** enabled in `tauri.conf.json` — restrict `connect-src` to `ipc:` only; no remote scripts ([Tauri CSP docs](https://v2.tauri.app/security/csp/)).
- **Isolation pattern** recommended for production ([Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/)): sandbox validates `invoke` payloads before Rust core.
- **Capabilities** per command group: `auth`, `secrets`, `buckets`, `approvals`, `audit`, `settings`.

---

## 7. Rust Backend Architecture

### 7.1 Module map

```
src-tauri/src/
├── main.rs                 # Entry, plugin registration
├── lib.rs                  # Module tree, AppState
├── state.rs                # AppState: pool, unlock flag, socket handle, pending map
│
├── db/
│   ├── mod.rs              # Pool lifecycle, PRAGMA key, migrations runner
│   ├── migrations/         # Versioned SQL (001_initial.sql, ...)
│   ├── secrets.rs
│   ├── buckets.rs
│   ├── approvals.rs
│   ├── audit.rs
│   └── settings.rs
│
├── crypto/
│   ├── mod.rs
│   ├── kdf.rs              # Argon2id hash + db_key derivation
│   ├── value_enc.rs        # AES-256-GCM for secrets.value blob
│   └── totp.rs             # RFC 6238 verify/generate
│
├── ipc/
│   ├── mod.rs              # Public API
│   ├── server.rs           # Listener start/stop (cfg unix / windows)
│   ├── handler.rs          # Per-connection state machine + grant logic
│   ├── protocol.rs         # serde IpcRequest / IpcResponse types
│   └── peer/
│       ├── mod.rs          # from_connected_stream (unix/windows)
│       ├── resolve.rs      # PID → VerifiedClient + fingerprint
│       ├── machine_id.rs   # OS-specific stable machine identifier
│       └── proc_info.rs    # Native process inspection (cmdline, cwd)
│
├── sessions/
│   ├── pending.rs          # ClientAccessRequestEvent queue (15-min window)
│   └── store.rs            # request_id → oneshot::Sender<ApprovalDecision>
│
├── background/
│   ├── mod.rs
│   ├── expiry_checker.rs
│   ├── approval_cleanup.rs
│   └── lock_watcher.rs
│
└── commands/
    ├── mod.rs
    ├── auth.rs
    ├── secrets.rs
    ├── buckets.rs
    ├── clients.rs          # IPC grant management + requests window commands
    ├── audit.rs
    └── settings.rs
```

### 7.2 Recommended crates

| Concern | Crate | Notes |
|---|---|---|
| Async runtime | `tokio` | Socket server + background tasks |
| DB | `rusqlite` + `libsqlite3-sys` feature `bundled-sqlcipher` | **Not** `tauri-plugin-sql` for vault — Rust must own the pool ([community guidance](https://codeforreal.com/blogs/setup-encrypted-sqlitedb-in-tauri-with-drizzle-orm/)) |
| Password KDF | `argon2` | Argon2id per [OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html) |
| Value encryption | `aes-gcm` | Per-value nonce |
| HKDF | `hkdf` + `sha2` | `value_key` from `db_key` |
| Secrets in RAM | `secrecy` + `zeroize` | Wrap keys and decrypted values |
| TOTP | `totp-rs` | SHA-1, 6 digits, ±1 window |
| Process info | `sysinfo` + native OS APIs | Exe path, cwd, cmdline, UID + fingerprint |
| UUID | `uuid` v4 | All PKs |
| Serialize | `serde`, `serde_json` | Value blobs + IPC |
| Notifications | `tauri-plugin-notification` | Approval prompts |
| Directories | `dirs` | `~/.argus` resolution |

### 7.3 `AppState` (shared)

```rust
pub struct AppState {
    pub db: Option<DbPool>,              // None when locked
    pub db_key: Option<Secret<[u8; 32]>>,
    pub value_key: Option<Secret<[u8; 32]>>,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub socket_shutdown: Option<tokio::sync::watch::Sender<()>>,
    pub pending_approvals: Arc<Mutex<HashMap<Uuid, oneshot::Sender<ApprovalDecision>>>>,
    pub last_activity: Arc<AtomicU64>,    // for auto-lock
}
```

Managed via `tauri::Manager::manage()` — **single instance** per app process.

### 7.4 Tauri command surface (complete)

| Namespace | Commands |
|---|---|
| **auth** | `register`, `sign_in`, `sign_out`, `unlock_app`, `lock_app`, `elevate_vault` (legacy), `get_scope_status`, `get_profile`, … |
| **secrets** | `create_secret`, `update_secret`, `delete_secret`, `get_secret`, `search_secrets`, `archive_secret` |
| **buckets** | `create_bucket`, `update_bucket`, `delete_bucket`, `get_buckets`, `upsert_mapping`, `remove_mapping` |
| **approvals** | `get_active_approvals`, `revoke_approval`, `respond_to_approval_request` |
| **audit** | `get_audit_log` (dashboard widget), `export_audit_csv` (settings danger zone, optional) |
| **settings** | `get_settings`, `set_setting` |

**Return types for list/search:** metadata only (`SecretMeta`) — no decrypted `value` unless `get_secret` with explicit UI action.

### 7.5 Socket handler flow

```
accept connection (pipe/socket)
    │
    ▼
read line → parse IpcRequest (bucket_id, client_token, optional cwd)
    │
    ▼
resolve peer from OS (PID → exe, cwd, uid, args, machine_id, git_remote → fingerprint)
    │
    ▼
validate bucket_id exists + token hash matches
    │
    ▼
query client_grants (bucket_id, fingerprint, token_hash) WHERE expires_at > now()
    │
    ├─ HIT ──► load mappings → decrypt injectable secrets → write audit → respond ok
    │
    └─ MISS ──► insert pending in sessions/store
              → emit client-access-requested + show requests window
              → wait oneshot (timeout 120s)
                    ├─ approved → INSERT grant, audit GRANTED, respond ok
                    └─ denied  → audit DENIED, respond denied
```

---

## 8. Frontend Architecture

### 8.1 Directory structure (target)

```
src/
├── main.tsx
├── app.tsx                    # Root layout + router outlet
├── styles/
│   └── globals.css            # @import "tailwindcss"
├── components/
│   ├── ui/                    # primitives (Button, Input, Badge, ...)
│   ├── layout/                # Sidebar, Shell, LockScreen
│   ├── secrets/               # SecretList, SecretForm, TypeFields/*
│   ├── buckets/               # BucketList, MappingTable
│   ├── approvals/             # ApprovalBanner, ApprovalList
│   └── audit/                 # AuditTable, Filters
├── pages/
│   ├── login.tsx
│   ├── register.tsx
│   ├── dashboard.tsx
│   ├── vault.tsx
│   ├── buckets.tsx
│   ├── approvals.tsx          # Grant management (main window)
│   ├── requests.tsx           # IPC access requests (tray popup window)
│   └── settings.tsx
├── bones/                      # boneyard-js generated skeletons
├── state/
│   ├── auth.store.ts
│   ├── vault.store.ts
│   └── ui.store.ts
├── hooks/
│   ├── useTauriEvent.ts
│   └── useUnlockState.ts
├── lib/
│   ├── tauri-bridge.ts        # typed invoke wrappers
│   └── fuse-search.ts         # client-side FTS fallback
└── types/
    ├── secret.ts
    ├── bucket.ts
    └── audit.ts
```

### 8.2 State management (Zustand)

| Store | Holds | Never holds |
|---|---|---|
| `auth` | `isSignedIn`, `email`, `username`, `avatarUrl` | Password, `db_key` |
| `vault` | `SecretMeta[]`, filters, selection id | Decrypted values (fetch on demand into component local state, clear on unmount) |
| `ui` | sidebar badges, modal open flags, pending approval payload | — |

### 8.3 Event bridge (Rust → UI)

| Event | Payload | UI action |
|---|---|---|
| `client-access-requested` | `ClientAccessRequestEvent` | Refresh requests list (requests window) |
| `signed-out` | `{}` | Navigate to `/login`, clear vault + auth stores |
| `signed-in` | `UserProfile` | Hydrate sidebar profile; load dashboard data |
| `expiry-alert` | `{ secretId, daysRemaining }` | Update dashboard + vault badges |

### 8.4 Routing

| Route | Guard | Window |
|---|---|---|
| `/` | Redirect → `/login`, `/register`, or `/dashboard` | Main |
| `/register` | Only if no `users` row | Main |
| `/login` | When signed out | Main |
| `/dashboard`, `/vault`, `/buckets`, `/settings` | Requires sign-in + app unlocked | Main |
| `/approvals` | Requires sign-in only (works while app locked) | Main |
| `/requests` | Requires sign-in only (works while app locked) | Requests popup |

The `/requests` route renders in a separate compact window opened from the system tray. It shows pending access requests from the last 15 minutes and allows the user to accept/deny them even when the main app is locked.

The `/approvals` route is in the main app sidebar and lists all active/expired grants with a revoke option. **App lock does not block approvals** — only vault and bucket management UI require unlock. Sign-out stops IPC entirely.

---

## 9. Data Layer

**Single file:** `~/.argus/argus.db` — full-file encryption via SQLCipher.

### 9.1 Schema summary

| Table | Purpose |
|---|---|
| `users` | Single row: local account (email, username, avatar_url, password hash, optional TOTP) |
| `secrets` | All secret types; `value` = AES-GCM blob |
| `secrets_fts` | FTS5 on `name`, `description` |
| `app_buckets` | Named env groupings |
| `bucket_mappings` | `env_label` → `secret_id` or encrypted `text_value` |
| `approvals` | Process identity + TTL |
| `audit_log` | Append-only events |
| `settings` | Key-value preferences |

See design document §4 for full DDL. Migrations are **forward-only** SQL files executed on unlock.

### 9.2 Search strategy

1. **Primary:** SQLite FTS5 virtual table (`secrets_fts`).
2. **UI filter chips:** `type`, `organization`, `environment`, `is_archived`, expiring window — SQL `WHERE`.
3. **Fuse.js:** optional fuzzy fallback on metadata already loaded in memory post-unlock.

**Never** full-table scan decrypt for search.

### 9.3 Repository pattern

Each `db/*.rs` module exposes:

- `create`, `update`, `delete`, `get_by_id`
- Domain-specific queries (e.g. `list_expiring_within_days`)

Commands call repositories; repositories **never** emit Tauri events.

---

## 10. Cryptography Pipeline

```
Master password
      │
      ▼ Argon2id (unique 32-byte salt per user, stored alongside hash)
  password_hash (stored)          db_key [32 bytes] (ephemeral, zeroized on lock)
      │                                │
      │                                ├──► SQLCipher PRAGMA key
      │                                │
      │                                └──► HKDF-SHA256(info="argus-value-v1")
      │                                         │
      │                                         ▼
      │                                   value_key [32 bytes]
      │                                         │
      │                                         ▼ AES-256-GCM per secret
      │                                   base64(nonce ‖ ciphertext ‖ tag)
      │
      └──► TOTP seed encrypted with key derived from master password
```

| Layer | Algorithm | Purpose |
|---|---|---|
| File | SQLCipher AES-256-CBC | Opaque DB on disk at rest |
| Field | AES-256-GCM | Defense in depth for `value` column |
| Password | Argon2id | OWASP minimum: m=19 MiB, t=2, p=1; Argus target: **m=64 MiB, t=3, p=4** (tunable) |
| TOTP | RFC 6238 | Second factor on unlock |

Full threat analysis: [security.md](./security.md).

---

## 11. IPC & Socket Server **(shipped)**

Socket server runs while the user is **signed in** (starts on sign-in, stops on sign-out). The main window may be hidden; IPC stays up when **Run in background** is enabled.

### 11.1 Request (client → Argus) — v3 protocol

Client identity is **never** trusted from JSON. The server derives PID, exe, cwd, uid, git remote, and machine ID from OS APIs. The client only sends credentials:

```json
{
  "request_id": "uuid-v4",
  "bucket_id": "550e8400-e29b-41d4-a716-446655440000",
  "client_token": "<ARGUS_BUCKET_TOKEN value>",
  "cwd": "/Users/dev/projects/acme-backend"
}
```

| Field | Required | Purpose |
|---|---|---|
| `bucket_id` | Yes | App bucket / “app id” (`ARGUS_BUCKET_ID` in project) |
| `client_token` | Yes | Secret issued in bucket settings |
| `cwd` | No | Fallback working directory (Windows only, when OS can't read peer cwd) |

All other identity fields (exe path, process name, PID, UID, machine ID, git remote, command-line args) are resolved **server-side** from the peer process attached to the pipe/socket. See [security.md](./security.md) §9.1 for the fingerprint computation.

### 11.2 Grant check flow

```
IPC request received (core signed in, socket up)
        │
        ▼
Validate bucket_id exists and bucket is "active" (tray / user toggle)
        │
        ▼
Lookup client_grants WHERE bucket_id + fingerprint + token_hash
        AND expires_at > now()
        │
        ├─ HIT ──► return secrets per mappings; audit SECRET_ACCESSED
        │
        └─ MISS / expired ──► emit client-access-requested
                              Show requests window (bottom-right popup)
                              User Accept → INSERT grant
                                expires_at = now + bucket.access_ttl_minutes
                                (or global default)
                              User Deny → audit DENIED
```

**Refresh:** If `refresh_ttl_minutes` is set on the bucket, a grant within refresh window may extend `expires_at` without popup (configurable). First connection always requires approval.

### 11.3 Response statuses

| `status` | Meaning | Client action |
|---|---|---|
| `ok` | Secrets map returned | Inject env / use values |
| `pending` | Awaiting user | Wait second NDJSON line (up to `timeout_seconds`) |
| `denied` | User rejected | Fail closed |
| `locked` | Argus signed out / core stopped | Fallback `.env` if enabled |
| `error` | e.g. `INVALID_TOKEN`, `SECRET_TYPE_NOT_INJECTABLE` | Surface code |

### 11.4 Connection policy

- Short-lived connection per request (or keep-alive v2 — default: one shot).
- Rate limit per `bucket_id` + `fingerprint`.
- Max message 64 KiB.

---

## 12. Authorization Scopes

One local account with **capability scopes** in Rust `AppState` (not separate users).

| Scope | ID | Gates | Typical operations |
|---|---|---|---|
| **App shell** | `APP` | Dashboard, **Settings** | Cleared on idle **app lock** or sign-out |
| **Vault** | `VAULT` | `/vault` secret CRUD | Same as **APP** while the app is unlocked (no separate vault TTL) |
| **Buckets** | `BUCKETS` | `/buckets` mutations | Same as **APP** while the app is unlocked |
| **IPC / approvals** | *(signed-in only)* | Client access requests, grant list/revoke | **Not** gated by app lock; requires sign-in + running core |

### Session model (implemented)

```
sign_in()        → password + TOTP or biometric; keys in memory; APP + VAULT effective
unlock_app()     → TOTP or biometric after idle app lock (no password)
sign_out()       → zeroize keys; full sign-in required on next start
auto_lock idle   → soft app lock (keys stay in memory); VAULT follows APP
elevate_buckets() → legacy no-op when app unlocked; buckets follow APP
```

| Command | Validates |
|---|---|
| `sign_in` | email/username + password + TOTP **or** biometric |
| `unlock_app` | TOTP **or** biometric only (after idle app lock) |
| `elevate_vault` | Legacy no-op when app unlocked; vault follows APP |
| `elevate_buckets` | Legacy no-op when app unlocked; buckets follow APP |

**Vault & buckets:** No separate elevation timers. Available whenever `APP` is unlocked. Idle **`auto_lock_minutes`** soft-locks vault/buckets/settings UI (`AppLockModal`, TOTP or biometric to resume). **IPC, process requests, and the approvals page are unaffected** — only sign-out stops the socket server.

**Not three separate accounts** — one `users` row, scope flags in memory (never stored plaintext in DB).

---

## 13. Tray & Background Service

### 13.1 Process model

**Shipped:** system tray icon, IPC socket server, access requests popup window, `run_in_background` setting.

```
┌─────────────────────────────────────────────────────────┐
│ Argus (Tauri + Rust) — when signed in                    │
│  • SQLCipher pool (shipped)                              │
│  • Tray icon + menu (shipped)                            │
│  • Socket server (shipped)                               │
│  • Access requests popup window (shipped)                │
├─────────────────────────────────────────────────────────┤
│ Main Window (React) — user can hide via window close     │
│  • Dashboard, vault, buckets, approvals, settings      │
├─────────────────────────────────────────────────────────┤
│ Requests Window (React) — compact bottom-right popup     │
│  • Pending access requests (last 15 min)                │
│  • Accept / Deny with TTL selection                     │
└─────────────────────────────────────────────────────────┘
```

Closing the **main window** hides it (tray remains) when `run_in_background` is enabled. **Sign out** clears session keys and returns to login.

### 13.2 Tray behavior

**Left-click:** If signed in → open **Requests window** (bottom-right popup showing pending access requests). If not signed in → open **Main window** for sign-in.

| Menu item | Status | Action |
|---|---|---|
| Open Argus | Shipped | Show/focus main window |
| Access Requests | Shipped | Show requests popup (pending access requests) |
| Sign out | Shipped | Full sign-out |
| Active buckets submenu | Planned | Open bucket detail |
| Pause all IPC | Planned | Emergency deny new grants |

### 13.3 Plugins

| Plugin | Use |
|---|---|
| `tauri-plugin-tray` | Icon, menu, click handlers |
| `tauri-plugin-notification` | New client access requests |
| Biometry plugin | Login + unlock on Win/macOS (shipped) |

---

## 14. Session, Client Grants & Audit

### Two-layer model

| Layer | Storage | Survives |
|---|---|---|
| **Client grant** | `client_grants` table | Argus restart, reboot (until `expires_at`) |
| **Wire request** | Ephemeral | Per IPC connection |

### `client_grants` table (new)

```sql
CREATE TABLE client_grants (
  id              TEXT PRIMARY KEY,
  bucket_id       TEXT NOT NULL REFERENCES app_buckets(id) ON DELETE CASCADE,
  fingerprint     TEXT NOT NULL,           -- SHA-256 of (machine_id|git_remote|cwd|exe_path|uid|run_args)
  token_hash      TEXT NOT NULL,           -- SHA-256 of client_token
  client_label    TEXT,                    -- optional friendly name
  granted_at      TEXT NOT NULL,
  expires_at      TEXT NOT NULL,
  last_seen_at    TEXT,
  UNIQUE(bucket_id, fingerprint, token_hash)
);
```

Grant identity is `(bucket_id, fingerprint, token_hash)`. Fingerprint is computed from OS-verified peer attributes — see [security.md](./security.md) §9.1.

### Per-bucket access settings (`app_buckets` columns)

| Column | Default | Description |
|---|---|---|
| `access_ttl_minutes` | from global | How long a **new** client grant lasts after Accept |
| `refresh_ttl_minutes` | NULL | Silent extension window before re-prompt |
| `is_tray_active` | `1` | Show in tray when grants exist |

### Global settings (Settings page)

| Key | Default | Description |
|---|---|---|
| `default_access_ttl_minutes` | `60` | Used when bucket has no override |
| `default_refresh_ttl_minutes` | `NULL` | Global refresh policy |
| `run_in_background` | `1` | Close window → tray, keep IPC |
| `auto_lock_minutes` | `30` | Idle app lock (vault/buckets UI only; IPC and approvals unchanged) |

### Client access popup (requests window)

When a new IPC request arrives and no active grant matches, the **requests window** (bottom-right popup) opens automatically. All pending requests from the last 15 minutes are shown in a scrollable list. Each request card displays:

| Field | Content |
|---|---|
| Bucket name | Which bucket the client wants to access |
| Folder (cwd) | Full working directory path (with unverified badge if OS fallback) |
| Exe path | Full path to the connecting executable |
| Git remote | Repository URL (if detected) |
| Args | Process command line (tokens stripped from display) |
| Accept options | Use bucket TTL or pick 15m / 1h / 3h / 8h |

The requests window works **even when the app is locked** (only requires signed-in status). If the user is not signed in, the window shows a sign-in prompt instead.

### Audit events (append-only)

`SECRET_ACCESSED`, `CLIENT_GRANTED`, `CLIENT_DENIED`, `CLIENT_EXPIRED`, `SCOPE_ELEVATED`, `SIGNED_IN`, `SIGNED_OUT`, `SECRET_*`, `BUCKET_*`

---

## 15. Three Access Tiers

| Tier | Consumer | Mechanism | Types allowed |
|---|---|---|---|
| **1 — Library** | App runtimes | Socket → `os.environ` | `api_key`, `access_token`, `connection_string` |
| **2 — CLI** | Shell | Same socket; `argus` binary path | Above + `ssh_key`, `certificate` (with confirm) |
| **3 — UI** | Human | Tauri `invoke` | All types; copy-only for `credential` |

Access matrix enforced in `ipc/handler.rs` and `commands/secrets.rs`.

---

## 16. Client Libraries & CLI

### Monorepo layout (recommended)

```
argus/
├── apps/desktop/          # Tauri app (current `argus/`)
├── crates/argus-core/     # Shared protocol types (optional)
├── clients/
│   ├── python/argus_secrets/
│   ├── node/@argus-secrets/node/
│   └── cli/               # Rust binary, shares socket client code
└── docs/
```

### Library contract (future Python/Node — not v1 UI scope)

1. Read `ARGUS_BUCKET_ID` and `ARGUS_BUCKET_TOKEN` from `.env` (or env).
2. Connect to `~/.argus/argus.sock` / `\\.\pipe\argus`.
3. Send v3 request JSON (§11.1) — only `bucket_id`, `client_token`, optional `cwd`.
4. Handle `ok` / `pending` / `denied` / `locked`.
5. On first run, user approves in requests popup; grant TTL from bucket settings.
6. Server derives client fingerprint from OS process inspection (no client-side identity).

### CLI commands (Phase 3)

```bash
argus get DATABASE_URL
argus list --bucket "Acme Backend"
argus export --bucket "Acme Backend"   # eval-safe pattern, no history leakage
argus expiring --days 30
```

---

## 17. Background Services

Spawned when **signed in** (tray core), cancelled on **sign out**:

| Task | Interval | Action |
|---|---|---|
| `approval_cleanup` | 10 min | Delete expired approvals + audit |
| `expiry_checker` | 6 h + on unlock | Bitmask notifications 30/7/1 day |
| `lock_watcher` | Event-driven | Screen lock → `lock()` if setting on |
| `auto_lock` | Activity-based | Reset on every `invoke()` |

---

## 18. Configuration & File Layout

### `settings` defaults

| Key | Default | Description |
|---|---|---|
| `auto_lock_minutes` | `30` | Idle app lock (vault/buckets UI only; IPC and approvals unchanged) |
| `default_ttl_minutes` | `60` | Pre-selected approval TTL |
| `expiry_notify_30d` | `1` | Enable 30-day warnings |
| `expiry_notify_7d` | `1` | Enable 7-day warnings |
| `expiry_notify_1d` | `1` | Enable 1-day warnings |
| `verify_process_path` | `1` | Cross-check PID with sysinfo |
| `lock_on_screen_lock` | `1` | OS lock triggers app lock (planned) |
| `fallback_to_dotenv` | `1` | Libraries may read `.env` when locked |
| `socket_max_message_bytes` | `65536` | Protocol limit |

### Open-source repository layout

```
argus-project/
├── argus/                    # Desktop app (this repo path)
│   ├── src/                  # React frontend
│   ├── src-tauri/            # Rust backend
│   └── docs/                 # architecture, plan, design, security
├── clients/                  # Phase 2+ (Python, Node, CLI)
├── LICENSE
├── SECURITY.md               # Public vulnerability disclosure
└── README.md
```

---

## 19. Technology Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Desktop shell | Tauri 2 | Rust security boundary; small binary; native APIs |
| DB access | `rusqlite` + SQLCipher in Rust | Frontend must not touch DB; plugin-sql insufficient for crypto control |
| UI | React 19 + TypeScript | Ecosystem; matches current scaffold |
| CSS | Tailwind CSS v4 + `@tailwindcss/vite` | Bento layout, custom tokens |
| Skeletons | boneyard-js + Vite plugin | Auto-captured loading UI |
| UI components | Custom only | BentoCard, Avatar, SidebarProfile — see [design.md](./design.md) |
| State | Zustand | Profile in auth store; secrets never persisted |
| Auth UX | Local account + mandatory 2FA | Password + (TOTP **or** biometric); 3 scopes |
| Tray | `tauri-plugin-tray` | Active buckets when window closed |
| Biometric | `tauri-plugin-biometry` (community) | Win/macOS; Linux → TOTP only |
| IPC to apps | Unix socket / named pipe | OS-local; no network attack surface |
| No cloud | — | Threat model: local-only secrets |

---

## 20. Explicit Non-Goals

The following are **not** part of this architecture:

- Cloud sync, team vaults, SSO
- Self-hosted Argus server
- Browser extension
- Mobile apps
- Hardware security key as primary unlock (future consideration only)
- Automatic `.env` rewriting in git repos (user copies `ARGUS_BUCKET_ID` manually)
- Protection of secrets **after** injection into `os.environ` (client app responsibility)

---

## Appendix A — `users` table (local account)

Single row per installation. **Not** a cloud account — metadata for UI and login identifier only.

```sql
CREATE TABLE users (
  id                  TEXT PRIMARY KEY DEFAULT 'local',
  email               TEXT NOT NULL UNIQUE,
  username            TEXT NOT NULL UNIQUE,
  avatar_url          TEXT,                 -- HTTPS URL or NULL (initials fallback)
  password_hash       TEXT NOT NULL,        -- Argon2id
  totp_secret         TEXT,                 -- encrypted; set if user picks TOTP at register
  second_factor_type  TEXT NOT NULL,        -- 'totp' | 'biometric'
  totp_enabled        INTEGER DEFAULT 0,    -- 1 if second_factor_type = totp
  biometric_enrolled  INTEGER DEFAULT 0,
  created_at          TEXT NOT NULL,
  last_signed_in_at   TEXT
);
```

| Command | Behavior |
|---|---|
| `register` | Insert row; **require** TOTP setup **or** biometric enroll; then sign in |
| `sign_in` | Password + TOTP **or** biometric per `second_factor_type` |
| `unlock_app` | TOTP **or** biometric after idle app lock |
| `elevate_buckets` | Legacy no-op when app unlocked |
| `sign_out` | Stop tray, socket, zeroize keys → `signed-out` |
| `get_scope_status` | `{ app, vault, buckets }` + `expires_at` per scope |
| `get_profile` | Sidebar profile fields |
| `update_profile` | `email`, `username` |

---

## Appendix B — Secret type access matrix

| Type | Library | CLI | UI |
|---|---|---|---|
| `api_key` | ✅ | ✅ | ✅ |
| `access_token` | ✅ | ✅ | ✅ |
| `credential` | ❌ | ❌ | ✅ copy |
| `recovery_codes` | ❌ | ❌ | ✅ mark used |
| `ssh_key` | ❌ | ✅ confirm | ✅ |
| `certificate` | ❌ | ✅ | ✅ |
| `connection_string` | ✅ | ✅ | ✅ |
| `note` | ❌ | ❌ | ✅ read |

---

## Appendix C — References

- [Tauri 2 Security](https://v2.tauri.app/security/)
- [Tauri CSP](https://v2.tauri.app/security/csp/)
- [Tauri Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [SQLCipher](https://www.zetetic.net/sqlcipher/)
- [RFC 9106 Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)
- [Microsoft Named Pipe Security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
- [Boneyard — skeleton framework](https://github.com/0xGF/boneyard)

---

*Argus never blinks. Neither do your secrets.*
