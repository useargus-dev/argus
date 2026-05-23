# Argus — Security Model & Hardening Guide

> This document is the authoritative security specification for Argus.  
> It defines threats, controls, cryptographic parameters, and release checklists.  
> A shorter public `SECURITY.md` at the repo root should link here for researchers.

**Related:** [architecture.md](./architecture.md) · [plan.md](./plan.md)

---

## Table of Contents

1. [Security Goals](#1-security-goals)
2. [Threat Model](#2-threat-model)
3. [Trust Boundaries](#3-trust-boundaries)
4. [Cryptographic Specification](#4-cryptographic-specification)
5. [Data Protection](#5-data-protection)
6. [Authentication & Authorization Scopes](#6-authentication--authorization-scopes)
7. [Second Factor (Mandatory)](#7-second-factor-mandatory)
8. [IPC, Client Tokens & Socket Hardening](#8-ipc-client-tokens--socket-hardening)
9. [Client Grant Security](#9-client-grant-security)
10. [Tray & Background Process](#10-tray--background-process)
11. [Secret Type Access Matrix](#11-secret-type-access-matrix)
12. [Frontend & Tauri Hardening](#12-frontend--tauri-hardening)
13. [Memory & Runtime Safety](#13-memory--runtime-safety)
14. [Audit & Logging](#14-audit--logging)
15. [Client Library Security (Future)](#15-client-library-security-future)
16. [Platform-Specific Controls](#16-platform-specific-controls)
17. [Operational Security](#17-operational-security)
18. [Known Limitations](#18-known-limitations)
19. [Release Security Checklist](#19-release-security-checklist)
20. [Vulnerability Disclosure](#20-vulnerability-disclosure)

---

## 1. Security Goals

| Goal | Success metric |
|---|---|
| **Confidentiality at rest** | Stolen `argus.db` without password is unusable |
| **Confidentiality in transit (local)** | Only approved OS-user processes receive secrets |
| **Integrity** | Tampered ciphertext fails GCM verification |
| **Accountability** | Every secret injection is audit-logged |
| **Least privilege** | WebView cannot read DB or bind socket |
| **Fail secure** | Lock destroys socket and zeroizes keys |
| **Usable security** | `.env` fallback optional; approvals show full path |

---

## 2. Threat Model

### 2.1 In scope — Argus mitigates

| Threat | Mitigation summary |
|---|---|
| **T1** Supply chain reads project `.env` | Only `ARGUS_BUCKET_ID` in repo |
| **T2** Casual filesystem access to DB | SQLCipher + Argon2id |
| **T3** Unknown app requests secrets | Client grant gate (bucket + uri + token) + popup |
| **T3b** Stolen `client_token` | Token stored hashed; rotate from bucket settings |
| **T4** Secret scraping via screen share in terminal | No values in shell history (CLI design) |
| **T5** Accidental commit of real secrets | Vault is outside project tree |
| **T6** Stale approval after laptop sleep | `expires_at` + cleanup job |
| **T7** XSS in WebView stealing invoke | CSP + capabilities + isolation pattern |
| **T8** Type confusion (inject credentials) | Rust access matrix on socket |

### 2.2 Out of scope — document honestly

| Threat | Why out of scope |
|---|---|
| **O1** Root / kernel attacker | Can dump process memory of unlocked Argus |
| **O2** User approves malicious binary | User must read `process_path` — social engineering |
| **O3** Malware after env injection | Client app holds secrets in `os.environ` |
| **O4** Physical forensic disk imaging while unlocked | Memory keys exist while running |
| **O5** Nation-state hardware implants | — |
| **O6** Coerced user unlocks vault | — |

### 2.3 Attacker personas

| Persona | Capability | Primary defense |
|---|---|---|
| **Script kiddy** | Reads `.env` in repo | Bucket ID useless without Argus |
| **Malicious npm dep** | Scans project files | No secrets in tree |
| **Local unprivileged user** | Connects to socket | `0600` socket + different UID blocked |
| **Compromised dev tool** | Spawns process | Approval shows real `process_path` |
| **Forensic thief** | Steals laptop disk | SQLCipher at rest |

---

## 3. Trust Boundaries

```
┌─────────────────────────────────────────────────────────┐
│ UNTRUSTED: WebView (React)                               │
│  - Treat all input as hostile                            │
│  - No direct file/network access to vault                │
└───────────────────────────┬─────────────────────────────┘
                            │ Tauri IPC (capabilities)
┌───────────────────────────▼─────────────────────────────┐
│ TRUSTED: Rust core                                         │
│  - Crypto, DB, socket, audit                             │
└───────────────────────────┬─────────────────────────────┘
                            │ NDJSON / local socket
┌───────────────────────────▼─────────────────────────────┐
│ UNTRUSTED: Client apps (Python, Node, CLI)               │
│  - Must not forge process identity without user approval │
└─────────────────────────────────────────────────────────┘
```

**Invariant:** Decryption of `secrets.value` happens **only** in Rust, in commands or socket handler after authorization check.

---

## 4. Cryptographic Specification

### 4.1 Master password (Argon2id)

| Parameter | Value | Source |
|---|---|---|
| Algorithm | **Argon2id** | [OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html) |
| Memory (m) | **65536 KiB (64 MiB)** | Stricter than OWASP minimum (19 MiB) for desktop KDF |
| Iterations (t) | **3** | Balance unlock latency vs strength |
| Parallelism (p) | **4** | Match typical CPU; queue if needed |
| Salt | **32 bytes** random per user | Stored with `password_hash` |
| Output | 32 bytes → `db_key` + `password_hash` encoding | |

**Storage format for `password_hash`:**

```
$argon2id$v=19$m=65536,t=3,p=4$<salt_b64>$<hash_b64>
```

**Never** store master password plaintext. **Never** log KDF inputs/outputs.

### 4.2 Database encryption (SQLCipher)

| Parameter | Value |
|---|---|
| Engine | SQLCipher (AES-256-CBC page encryption) |
| Key | `db_key` from Argon2id |
| Activation | `PRAGMA key = "x'...hex...'"` immediately after open |
| Verification | `SELECT count(*) FROM sqlite_schema` — fails if wrong key |

**Implementation note:** Use `rusqlite` + `bundled-sqlcipher` in Rust core — **not** frontend `tauri-plugin-sql` for the vault ([rationale](https://codeforreal.com/blogs/setup-encrypted-sqlitedb-in-tauri-with-drizzle-orm/)).

### 4.3 Per-value encryption (AES-256-GCM)

| Parameter | Value |
|---|---|
| Algorithm | AES-256-GCM |
| Key | `value_key = HKDF-SHA256(db_key, salt=app_salt, info="argus-value-v1")` |
| Nonce | 12 bytes random **per encryption** |
| AAD | optional: `secret_id` as associated data (bind ciphertext to row) |
| Stored form | `base64(nonce ‖ ciphertext ‖ tag)` |

**Rotation:** Changing master password must re-encrypt all `value` blobs (document in settings flow).

### 4.4 TOTP (second factor)

| Parameter | Value |
|---|---|
| Standard | RFC 6238 |
| Algorithm | SHA-1 (authenticator app compatibility) |
| Digits | 6 |
| Period | 30 seconds |
| Window | ±1 step |
| Seed storage | Encrypted with key derived from master password |

### 4.5 Optional pepper (future)

OWASP recommends application-level **pepper** (secret not in DB). For Argus open-source:

- Pepper could be machine-specific key in OS keychain (Phase 5)
- Not required for v1 if Argon2id parameters remain strong

---

## 5. Data Protection

### 5.1 Filesystem layout

| Path | Mode (Unix) | Content sensitivity |
|---|---|---|
| `~/.argus/` | `0700` | Directory |
| `~/.argus/meta.json` | `0600` | Bootstrap flags only: `has_account`, Argon2id `password_hash` (PHC string), `second_factor_type`, encrypted TOTP blob reference for pre-unlock 2FA step |
| `~/.argus/argus.db` | `0600` | Encrypted vault |
| `~/.argus/argus.sock` | `0600` | IPC endpoint (exists only when unlocked) |

### 5.2 Plaintext in database

These columns are **searchable plaintext inside encrypted DB** (protected by SQLCipher, not by field-level crypto):

- `secrets.name`, `description`, `tags`, `organization`, `environment`, `type`, `expires_at`
- `approvals.process_path`, `working_dir`
- `audit_log` metadata (never secret values)

**Rationale:** FTS and filtering without decrypting every row. Trade-off documented.

### 5.3 Backup & export

| Operation | Security requirement |
|---|---|
| Audit CSV export | Re-enter master password |
| Vault backup (Phase 5) | Separate AES-GCM file + Argon2id |
| Clear audit | Password + typed confirmation phrase |

---

## 6. Authentication & Authorization Scopes

### 6.1 What “login” means

**Not** three separate user accounts. **One** local `users` row with **three capability scopes** in Rust `AppState`:

| Scope | Required to… |
|---|---|
| **APP** | Open any page (dashboard, vault view, buckets view, **settings**) |
| **VAULT** | Create/update/delete/reveal secrets |
| **BUCKETS** | Create/edit buckets, mappings, client tokens, per-bucket TTL |

### 6.2 Sign-in (APP scope)

Always requires:

1. Email or username + password (Argon2id)
2. **Second factor** — TOTP **or** biometric (mandatory from register; see §7)

### 6.3 Elevation (VAULT / BUCKETS scopes)

Same factor strength as sign-in:

- `elevate_vault`: password + TOTP **or** biometric → `vault_elevated_until`
- `elevate_buckets`: password + TOTP **or** biometric → `buckets_elevated_until`

Durations from global settings (`vault_elevation_minutes`, `bucket_elevation_minutes`). Defaults: 15 minutes.

**Settings page** requires APP scope only (full sign-in). Changing global TTL or security prefs does **not** require BUCKETS scope.

### 6.4 Profile data

| Field | Storage | Notes |
|---|---|---|
| `email`, `username` | Plaintext inside SQLCipher | Login identifiers only |
| `avatar_url` | HTTPS URL only | No `data:` URIs |

### 6.5 Sign-out

Stops **tray core**, socket, zeroizes `db_key`, clears all scopes. `client_grants` remain on disk until `expires_at` but IPC cannot decrypt without core running and signed in.

### 6.6 Rate limiting

Failed sign-in / elevation: exponential backoff + lockout after 10 failures / 15 min. Audit `AUTH_FAILED` without password in log.

---

## 7. Second Factor (Mandatory)

At **register**, user must complete **exactly one**:

| Option | `second_factor_type` | Notes |
|---|---|---|
| **TOTP** | `totp` | QR + verify 6 digits; seed encrypted in DB |
| **Biometric** | `biometric` | Windows Hello / Touch ID; enroll via OS APIs |

| Rule | Detail |
|---|---|
| Cannot skip both | Register API returns error if neither completed |
| Login | Use same method as `second_factor_type` |
| Elevation | Same method (do not mix TOTP login + biometric elevate unless user enabled both in future) |
| Linux | Biometric unavailable → **TOTP required** |

Biometric unlocks a **device-bound** key in OS secure storage — it does **not** replace Argon2id or SQLCipher keys. Password is still required for register, elevation, and recovery flows.

---

## 8. IPC, Client Tokens & Socket Hardening

### 8.1 Transport

Unix socket `0600` / Windows named pipe with user-only DACL. Socket owned by **tray core**, not only when UI window is open.

### 8.2 Client token security

| Property | Implementation |
|---|---|
| Generation | CSPRNG 32+ bytes, prefix `ag_live_` |
| Storage | Only `SHA-256(token)` in `client_grants.token_hash` |
| Transmission | Client sends token once per IPC request; never log full token |
| Rotation | Bucket settings “Regenerate token” invalidates old hashes (BUCKETS scope) |
| Leak | User revokes grant or rotates token; shorten TTL |

### 8.3 Request validation (v2)

- `bucket_id`, `uri`, `client_token` required
- Normalize `uri` before hash
- Constant-time compare on token hash
- Optional `sysinfo` PID/path cross-check

### 8.4 Core must be signed in

`locked` response if tray core stopped (user signed out). Libraries may fall back to `.env` per global setting.

---

## 9. Client Grant Security

### 9.1 Grant identity

```
(bucket_id, uri_hash, token_hash)
```

First seen triple → **popup** (OS notification + modal). No auto-approve for new URIs.

### 9.2 TTL hierarchy

| Level | Setting |
|---|---|
| Per-bucket | `access_ttl_minutes`, `refresh_ttl_minutes` |
| Global default | `default_access_ttl_minutes`, `default_refresh_ttl_minutes` in Settings |

On Accept, `expires_at = now + effective_access_ttl`. **Refresh** (if enabled): silent extension when same client reconnects before expiry and within refresh window — reduces notification fatigue.

### 9.3 User must see

- Bucket name
- Full normalized URI
- Token suffix (last 4 chars of prefix display)
- Process name/path when provided

### 9.4 Revocation

- Per-grant revoke from bucket detail
- “Revoke all clients” in settings danger zone (BUCKETS scope + password)

---

## 10. Tray & Background Process

| Control | Rationale |
|---|---|
| Tray runs only when signed in | No IPC without `db_key` |
| Close window ≠ sign out | User expects buckets to keep working |
| Sign out from tray | Same as Settings — full teardown |
| Tray shows active buckets only | Reduces accidental exposure surface |
| Pending request badge | User notices blocked clients |

**Threat:** Unlocked laptop + tray running → approved clients can fetch secrets. Mitigate: auto-lock, screen lock → sign-out, short access TTL.

---

## 11. Secret Type Access Matrix

Enforced in **`socket/handler.rs`** and **`commands/secrets.rs`** (double-check UI cannot bypass).

| `type` | Socket inject | CLI | UI `get_secret` |
|---|---|---|---|
| `api_key` | ✅ | ✅ | ✅ |
| `access_token` | ✅ | ✅ | ✅ |
| `connection_string` | ✅ | ✅ | ✅ |
| `credential` | ❌ `SECRET_TYPE_NOT_INJECTABLE` | ❌ | ✅ copy only |
| `recovery_codes` | ❌ | ❌ | ✅ |
| `ssh_key` | ❌ | ✅ + confirm | ✅ |
| `certificate` | ❌ | ✅ | ✅ |
| `note` | ❌ | ❌ | ✅ read-only |

---

## 12. Frontend & Tauri Hardening

### 10.1 Content Security Policy

Enable strict CSP per [Tauri CSP docs](https://v2.tauri.app/security/csp/):

```json
{
  "csp": {
    "default-src": "'self' asset:",
    "script-src": "'self'",
    "style-src": "'self' 'unsafe-inline'",
    "img-src": "'self' asset: data:",
    "connect-src": "ipc: http://ipc.localhost",
    "font-src": "'self'"
  }
}
```

- **No** remote CDN scripts or styles in production
- Bundle fonts locally

### 10.2 Capabilities (Tauri 2)

- Split permissions: `auth-default`, `secrets-default`, `admin-default`
- Deny `fs` read of `~/.argus` from frontend
- Deny `shell` unless explicitly required (prefer none)

### 10.3 Isolation pattern

Recommended for release builds ([Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/)):

- Minimal isolation app — validate `invoke` argument shapes
- Keep isolation bundle tiny (reduced supply chain risk)

### 10.4 WebView data

- Disable remote debugging in release
- Clear sensitive component state on `locked` event
- No `localStorage` for secrets

---

## 13. Memory & Runtime Safety

| Control | Implementation |
|---|---|
| Secret bytes in RAM | `secrecy::Secret<T>` wrapper |
| Zero on drop | `zeroize` on keys and decrypted buffers |
| `db_key` lifetime | `Option<Secret<[u8; 32]>>` cleared on lock |
| SQLCipher cache | Close connection on lock |
| `mlock` (optional) | Lock `db_key` pages — platform-dependent, best-effort |
| Swap risk | Document: unlocked vault may swap; OS encryption recommended |

**Rust safety:** `#![deny(unsafe_code)]` in security-critical crates where possible; audit any `unsafe` for SQLCipher FFI.

---

## 14. Audit & Logging

### 12.1 Audit log properties

- **Append-only** in normal operation
- **No secret values** in any column
- Includes: `event_type`, `bucket_id`, `secret_ids[]`, `env_labels[]`, process metadata, `pid`, `occurred_at`

### 12.2 Application logs

| Allowed | Forbidden |
|---|---|
| Event type, UUIDs, paths | Passwords, TOTP codes, decrypted values |
| Error codes | Full ciphertext dumps |

Release builds: log level `WARN` default.

---

## 15. Client Library Security (Future)

### 13.1 Trust model

Libraries are **untrusted**. They must:

- Never cache secrets to disk
- Never log env values
- Clear sensitive strings after setting env (language-dependent best effort)

### 13.2 Fallback to `.env`

When `locked`:

| `fallback_to_dotenv` | Behavior |
|---|---|
| `1` (default) | Load `.env`; stderr warning |
| `0` | Raise `ArgusLockedError` — strict mode |

### 13.3 CLI history safety

- `argus get KEY` — output suitable for piping, not echoed in history if used via `$(argus get ...)` patterns documented
- Document **bash** `HISTCONTROL` / **zsh** `incognito` for paranoid users

---

## 16. Platform-Specific Controls

### 14.1 macOS

- Keychain integration (future pepper storage)
- Notarization + Hardened Runtime for distribution
- Screen lock: `NSWorkspace` notification

### 14.2 Windows

- Named pipe ACL hardening (see §7.1)
- SQLCipher build: OpenSSL via vcpkg documented
- Authenticode signing
- WebView2 evergreen runtime dependency documented

### 14.3 Linux

- AppArmor/SELinux profiles (optional community contribution)
- `~/.argus` permissions enforced on create
- WebKitGTK security updates via distro

---

## 17. Operational Security

### 15.1 Developer machine recommendations

- Full-disk encryption (FileVault / BitLocker / LUKS)
- Screen lock when away
- Separate bucket per environment (`prod` vs `dev`)
- Short TTL approvals for production buckets (15m default)

### 15.2 Project `.env` hygiene

```env
# Safe to commit (with team agreement):
ARGUS_BUCKET_ID=550e8400-e29b-41d4-a716-446655440000

# Never commit:
# DATABASE_URL=postgresql://...
```

### 15.3 Dependency hygiene

```bash
cargo audit
cargo deny check   # optional
pnpm audit
```

Pin versions in `Cargo.lock` / `pnpm-lock.yaml`.

---

## 18. Known Limitations

Publish these in the public threat model (Phase 4):

1. **Post-injection exposure** — Argus cannot control client app memory after `os.environ` set.
2. **User-approved malware** — Approval UI is the last line; misleading process names possible (show full path).
3. **Root attacker** — Can attach debugger to unlocked Argus.
4. **Plaintext metadata** — Secret names/orgs searchable inside encrypted DB file structure (SQLCipher hides content, not existence of a DB).
5. **TOTP seed** — Requires master password to decrypt; loss of password = loss of vault (unless backup export exists).

---

## 19. Release Security Checklist

Before tagging `v0.1.0`:

- [ ] CSP enabled and tested
- [ ] Capabilities least-privilege review
- [ ] `debug_simulate_*` commands disabled in release
- [ ] `cargo audit` clean (or documented exceptions)
- [ ] No secrets in test fixtures committed
- [ ] Socket `0600` / pipe ACL verified on all three OS
- [ ] Access matrix integration tests pass
- [ ] Lock zeroizes keys (unit test)
- [ ] Wrong password does not leak timing oracle (constant-time compare via argon2 verify)
- [ ] Signed binaries (platform certs)
- [ ] `SECURITY.md` disclosure policy published
- [ ] Reproducible build instructions tested

---

## 20. Vulnerability Disclosure

**Template for root `SECURITY.md`:**

```markdown
# Security Policy

Report vulnerabilities to: security@<your-domain> (or GitHub private advisory)

Please include:
- Platform (Windows/macOS/Linux)
- Argus version
- Steps to reproduce
- Impact assessment

Do not disclose publicly until fix is released.

We aim to respond within 72 hours.
```

**In-scope reports:**

- Bypass of approval flow
- Secret leakage via IPC without approval
- SQLCipher bypass
- XSS → secret exfiltration via Tauri invoke

**Out-of-scope:**

- User-approved malicious process (social engineering)
- Post-injection memory scraping

---

## Appendix — Security feature summary

| Feature | Status (v1 target) |
|---|---|
| SQLCipher full-DB encryption | Required |
| Local account + mandatory 2FA (TOTP or biometric) | Required |
| Three authorization scopes | Required |
| Client grants (bucket + uri + token) | Required |
| System tray + background IPC | Required |
| Sign out | Required |
| Per-value AES-GCM | Required |
| Process approval gate | Required |
| Append-only audit | Required |
| Auto-lock + screen lock | Required |
| Type-based inject blocking | Required |
| CSP + capabilities | Required |
| Isolation pattern | Recommended |
| OS keychain pepper | Future |
| Hardware security key | Future |

---

*Security is not a feature flag. If a control in this document conflicts with convenience, security wins for production defaults.*
