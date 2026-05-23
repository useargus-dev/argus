# Argus — 20-Milestone Development Plan

> Execute milestones **in order**. Each milestone has clear entry criteria, deliverables, and verification steps.  
> Do not skip ahead — later milestones depend on security boundaries established earlier.

**Related:** [architecture.md](./architecture.md) · [design.md](./design.md) · [security.md](./security.md)

---

## How to use this plan

| Symbol | Meaning |
|---|---|
| **Goal** | What you are building in this step |
| **Deliverables** | Files / features that must exist |
| **Verify** | Commands or manual checks before moving on |
| **Depends on** | Prior milestone numbers |

**Current scaffold:** Tauri 2 + React 19 + Tailwind v4 + Zustand (`argus/`). Rust backend is minimal — Milestones 1–4 build the security core before UI polish.

---

## Milestone 1 — Repository & toolchain baseline

**Goal:** Reproducible dev environment on Windows, macOS, and Linux.

**Depends on:** —

**Deliverables:**

- [ ] Rust toolchain ≥ **1.88** (required by current Tauri deps)
- [ ] `pnpm` scripts: `dev`, `build`, `tauri dev`, `tauri build`
- [ ] `docs/` linked from root `README.md`
- [ ] `.gitignore` covers `target/`, `dist/`, `node_modules/`, `~/.argus` test paths
- [ ] `rust-toolchain.toml` pins minimum Rust version
- [ ] CI skeleton (optional): `cargo check`, `pnpm build` on three OS targets

**Verify:**

```bash
rustc --version    # >= 1.88
pnpm tauri dev     # window opens
pnpm build         # succeeds
```

**Exit criteria:** Clean clone → running app in &lt;15 minutes on your primary OS.

---

## Milestone 2 — Frontend shell, bento layout & routing

**Goal:** Navigable UI with bento dashboard, custom components, and Boneyard wired — no real secrets yet.

**Depends on:** 1

**Deliverables:**

- [ ] React Router: `/register`, `/login`, `/dashboard`, `/vault`, `/buckets`, `/settings` ([design.md](./design.md) §2)
- [ ] Custom UI primitives: `BentoGrid`, `BentoCard`, `Avatar`, `Button`, `Input`, `PasswordInput`
- [ ] `AppShell` + `Sidebar` + `SidebarProfile` (avatar, username, email placeholders)
- [ ] Dashboard page with bento placeholder cells
- [ ] `boneyard-js` Vite plugin + `boneyard.config.json` + `import './bones/registry'` in `main.tsx`
- [ ] Skeleton wrappers on dashboard tiles (`name="dashboard-*"`)
- [ ] Zustand `auth` store: `email`, `username`, `avatarUrl`, `isSignedIn`
- [ ] `lib/tauri-bridge.ts` typed stubs
- [ ] Tailwind `@theme` tokens in `globals.css`
- [ ] Lucide icons

**Verify:**

- All routes render; unauthenticated users land on `/login`
- `pnpm dev` generates/updates bones on HMR
- Dashboard shows shimmer skeletons when `loading={true}`

**Exit criteria:** Bento dashboard + sidebar profile visible with skeletons; no `/expiring` or `/audit` routes.

---

## Milestone 3 — `~/.argus` data directory & SQLCipher foundation

**Goal:** Encrypted database file created and opened **only from Rust**.

**Depends on:** 1

**Deliverables:**

- [ ] `dirs`-based path resolution → `~/.argus/argus.db`
- [ ] `libsqlite3-sys` with `bundled-sqlcipher` (or vendored OpenSSL on Windows per platform notes)
- [ ] `db/mod.rs`: open, `PRAGMA key`, close, `is_open` guard
- [ ] `db/migrations/001_initial.sql` — all tables from architecture §9
- [ ] Migration runner on first open after unlock
- [ ] File permissions `0600` on DB creation (Unix); Windows ACL hardening documented

**Verify:**

```bash
# After test unlock command (temporary):
ls -la ~/.argus/argus.db   # mode 600 on Unix
# Open DB file in hex editor — not readable plaintext
```

**Exit criteria:** DB file exists, migrations applied, cannot read without key.

---

## Milestone 4 — Cryptography module

**Goal:** Master password → keys, TOTP, value encrypt/decrypt — unit tested in isolation.

**Depends on:** 3

**Deliverables:**

- [ ] `crypto/kdf.rs` — Argon2id hash + verify (params: m=64 MiB, t=3, p=4 — document in security.md)
- [ ] `crypto/value_enc.rs` — AES-256-GCM encrypt/decrypt with random 12-byte nonce
- [ ] `crypto/totp.rs` — generate secret, verify code, QR URI for setup
- [ ] `zeroize` + `secrecy` on all key material
- [ ] `#[cfg(test)]` unit tests: round-trip encrypt, wrong password fails, TOTP window

**Verify:**

```bash
cargo test -p argus crypto::
```

**Exit criteria:** 100% crypto tests pass; no keys in logs.

---

## Milestone 5 — Local account, mandatory 2FA, scopes & sign-out

**Goal:** Register with **required** TOTP or biometric; sign-in with password + 2FA; APP/VAULT/BUCKETS scope elevations.

**Depends on:** 3, 4

**Deliverables:**

- [ ] `users` table: `second_factor_type`, `totp_*`, `biometric_enrolled` ([architecture.md](./architecture.md) Appendix A)
- [ ] `commands/auth.rs`: `register`, `sign_in`, `sign_out`, `elevate_vault`, `elevate_buckets`, `get_scope_status`, `get_profile`, `update_profile`
- [ ] Register wizard: account → **mandatory** TOTP QR **or** biometric enroll → dashboard
- [ ] Login: password → TOTP **or** biometric step
- [ ] `ElevateVaultModal` / `ElevateBucketsModal` ([design.md](./design.md) §8)
- [ ] Route guards: all pages need APP; vault writes need VAULT; bucket writes need BUCKETS
- [ ] Settings: sign out; global TTL + elevation minutes (APP scope)
- [ ] Biometry: `tauri-plugin-biometry` (Win/macOS); Linux TOTP-only path
- [ ] Events: `signed-in`, `signed-out`, `scope-changed`

**Verify:**

- Cannot register without completing TOTP or biometric
- Vault “Add secret” prompts elevation when scope missing
- Bucket “Create” prompts BUCKETS elevation
- Sign out blocks all routes

**Exit criteria:** Three-scope auth model working in UI + Rust.

---

## Milestone 6 — Secrets CRUD (all types)

**Goal:** Full secret lifecycle through Rust commands only.

**Depends on:** 5

**Deliverables:**

- [ ] `db/secrets.rs` repository
- [ ] `commands/secrets.rs`: create, update, delete, get, search — **check VAULT scope** on mutations
- [ ] Type-specific JSON schemas validated before encrypt ([architecture.md](./architecture.md) Appendix A)
- [ ] FTS5 triggers working on insert/update/delete
- [ ] Frontend: type selector + forms per type ([design.md](./design.md) §4)
- [ ] UI: masked values, reveal-on-click, copy with 30s clipboard clear (timer in Rust or frontend)

**Verify:**

- Create one secret of each type
- Search by name finds correct row
- `get_secret` returns decrypted value; list/search does not

**Exit criteria:** Vault page fully functional for CRUD.

---

## Milestone 7 — Vault UX polish

**Goal:** Production-quality list, filters, and detail panel.

**Depends on:** 6

**Deliverables:**

- [ ] Filter chips: type, org, environment, expiring (no `/expiring` page), archived
- [ ] Boneyard skeletons: `vault-list`, `vault-detail`
- [ ] Fuse.js fuzzy search on loaded metadata
- [ ] Secret detail side panel (or modal)
- [ ] Expiry badges (30d / 7d / 1d / expired)
- [ ] Recovery codes: mark-as-used UI
- [ ] Archive / unarchive

**Verify:**

- 50+ test secrets: scroll and search remain responsive
- Expiring filter matches `expires_at` logic

**Exit criteria:** Vault usable daily without raw SQL.

---

## Milestone 8 — App buckets & mappings

**Goal:** Env label ↔ secret mapping; bucket ID for projects.

**Depends on:** 6

**Deliverables:**

- [ ] `db/buckets.rs` + `commands/buckets.rs` — **check BUCKETS scope** on mutations
- [ ] Bucket columns: `access_ttl_minutes`, `refresh_ttl_minutes`, `is_tray_active`
- [ ] Generate/display `client_token` (show once on create); store hash only
- [ ] Frontend `/buckets` list + detail with mapping table
- [ ] Copy bucket ID button + helper text for `.env`
- [ ] `UNIQUE(bucket_id, env_label)` enforced
- [ ] Default `session_ttl_minutes` per bucket

**Verify:**

- Create bucket, add 4 mappings, copy UUID
- Delete secret referenced by mapping → `RESTRICT` error surfaces in UI

**Exit criteria:** Bucket detail matches design doc wireframe.

---

## Milestone 9 — Audit log (backend + dashboard widget)

**Goal:** Append-only audit trail; surface recent events on Dashboard only (no `/audit` page).

**Depends on:** 5

**Deliverables:**

- [ ] `db/audit.rs` + `commands/audit.rs`
- [ ] Audit writes on: sign-in, sign-out, secret CRUD, approvals
- [ ] `get_audit_log` with limit (e.g. 5 rows for dashboard)
- [ ] Dashboard bento tile: `RecentActivityList` + `Skeleton name="dashboard-recent-activity"`

**Verify:**

- Milestone 6 actions create audit rows
- Dashboard shows last 5 events (no secret values)

**Exit criteria:** Audit data visible on dashboard; no dedicated audit route.

---

## Milestone 10 — In-memory session store & approval commands

**Goal:** Rust-side pending approval channels (no socket yet).

**Depends on:** 8, 9

**Deliverables:**

- [ ] `sessions/store.rs` — `HashMap<request_id, oneshot::Sender<_>>`
- [ ] `commands/approvals.rs`: `respond_to_approval_request`, `get_active_approvals`, `revoke_approval`
- [ ] `db/approvals.rs` persistence
- [ ] Simulated approval request from a **test command** `debug_simulate_approval` (dev only, feature flag)

**Verify:**

- Simulate request → UI banner → approve → oneshot resolves
- Deny path clears pending map

**Exit criteria:** Approval UI works without socket.

---

## Milestone 11 — Tray core + Unix socket (macOS & Linux)

**Goal:** Background tray with active buckets; IPC v2 (`bucket_id` + `client_token` + `uri`).

**Depends on:** 8, 10

**Deliverables:**

- [ ] `tauri-plugin-tray`: menu, active bucket list, sign out, pending badge
- [ ] `run_in_background`: close window → tray only; core keeps socket
- [ ] `client_grants` table + migration
- [ ] `socket/mod.rs` — lifecycle tied to **signed-in core**, not window visibility
- [ ] `socket/protocol.rs` — v2 request/response ([architecture.md](./architecture.md) §11)
- [ ] `socket/handler.rs` — grant lookup + new-client pending flow
- [ ] Socket `0600`; access matrix enforcement
- [ ] Audit: `CLIENT_GRANTED`, `CLIENT_DENIED`, `SECRET_ACCESSED`

**Verify:**

- Close main window → tray remains; bucket list visible
- Test IPC with bucket_id + token + uri → popup on first connect
- Accept → secrets returned until TTL expires
- Sign out → tray gone; IPC `locked`

**Exit criteria:** Tray + v2 IPC on Linux/macOS.

---

## Milestone 12 — Windows named pipe server

**Goal:** Feature parity on Windows.

**Depends on:** 11

**Deliverables:**

- [ ] `cfg(windows)` named pipe at `\\.\pipe\argus`
- [ ] Security descriptor: current user only (no Everyone/Anonymous)
- [ ] Same protocol.rs handler shared via trait
- [ ] Document OpenSSL/vcpkg build steps in README if needed for SQLCipher

**Verify:**

- `pnpm tauri dev` on Windows → pipe accepts connection
- Cross-test: locked → `locked` response

**Exit criteria:** All three OS families pass manual socket test.

---

## Milestone 13 — Client access popup & notifications

**Goal:** New-app popup (URI + token + bucket); per-bucket/global TTL on accept.

**Depends on:** 11, 12, 10

**Deliverables:**

- [ ] `tauri-plugin-notification` for `client-access-requested`
- [ ] `ClientAccessDialog` ([design.md](./design.md) §9)
- [ ] `respond_to_client_access` command with TTL from bucket or global default
- [ ] Settings UI: global `default_access_ttl_minutes`, `default_refresh_ttl_minutes`
- [ ] Bucket detail: per-bucket TTL overrides + “Active in tray” toggle
- [ ] 120s timeout → `denied` if no response

**Verify:**

- New uri+token → popup; repeat within TTL → no popup
- Deny → audit `CLIENT_DENIED`
- Tray “Pending (N)” opens queue

**Exit criteria:** Full client onboarding flow without Python lib (test harness only).

---

## Milestone 14 — Python client library (future — post design)

**Goal:** `load_secrets()` sending v2 IPC (`ARGUS_BUCKET_ID`, `ARGUS_CLIENT_TOKEN`, `uri`).

**Depends on:** 13

**Status:** **Out of v1 implementation scope** per product direction — document protocol only until core stable.

**Deliverables (when started):**

- [ ] `argus_secrets` package; v2 protocol; `.env` keys documented
- [ ] Example script; not required for Milestone 20 release

**Exit criteria:** Deferred — track as Phase 2b in README.

---

## Milestone 15 — Node/Bun client library (future — post design)

**Goal:** Same as M14 for Node. **Deferred** — protocol in [architecture.md](./architecture.md) §11.

**Depends on:** 13

**Exit criteria:** Deferred.

---

## Milestone 16 — Background services

**Goal:** Expiry notifications, approval cleanup, auto-lock, screen lock.

**Depends on:** 5, 9, 11

**Deliverables:**

- [ ] `background/approval_cleanup.rs` — 10 min interval
- [ ] `background/expiry_checker.rs` — bitmask notifications
- [ ] `background/lock_watcher.rs` — platform screen lock
- [ ] Auto-lock timer reset on every `invoke()`
- [ ] Settings toggles wired from `/settings`

**Verify:**

- Set `auto_lock_minutes=1` → idle → auto lock
- Expire a test approval → row deleted + audit `APPROVAL_EXPIRED`

**Exit criteria:** App maintains itself without manual DB hygiene.

---

## Milestone 17 — Argus CLI

**Goal:** Shell access tier with history-safe patterns.

**Depends on:** 11, 12

**Deliverables:**

- [ ] Rust binary `argus` in `clients/cli/`
- [ ] Commands: `get`, `list`, `export`, `expiring`
- [ ] Separate approval identity (`process_path` = CLI binary)
- [ ] `export` designed for `eval` without echoing secrets to history
- [ ] Man pages or `--help`

**Verify:**

```bash
argus get DATABASE_URL   # triggers approval if needed
argus list --bucket "Acme Backend"  # names only
```

**Exit criteria:** CLI documented in README; no secrets in shell history file.

---

## Milestone 18 — Settings, profile, danger zone

**Goal:** User-configurable security posture + profile + sign out.

**Depends on:** 5, 9, 16

**Deliverables:**

- [ ] `/settings` bento sections ([design.md](./design.md) §5.6)
- [ ] Profile: avatar URL, display username/email
- [ ] **Sign out** with confirm dialog
- [ ] `commands/settings.rs` get/set all keys
- [ ] Change password (re-encrypt optional TOTP + SQLCipher rekey)
- [ ] Optional: enable TOTP (QR setup)
- [ ] Optional: audit export CSV from settings danger zone (no audit page)

**Verify:**

- Sign out returns to `/login`; sidebar empty
- Avatar URL update reflects in sidebar
- `fallback_to_dotenv` toggle works

**Exit criteria:** Settings + sign out + profile complete.

---

## Milestone 19 — Tauri hardening & capabilities

**Goal:** Production security configuration.

**Depends on:** 5–18

**Deliverables:**

- [ ] Strict CSP in `tauri.conf.json` (no remote scripts)
- [ ] Capabilities: least-privilege per command namespace
- [ ] Isolation pattern build (`dist-isolation`) — optional but recommended
- [ ] Remove `debug_simulate_approval` from release builds
- [ ] `tauri.conf.json` — no overly broad `fs` or `shell` permissions
- [ ] Dependency audit: `cargo audit`, `pnpm audit`

**Verify:**

- `tauri.conf.json` CSP blocks inline script injection test
- `cargo audit` zero critical (or documented exceptions)

**Exit criteria:** Matches [security.md](./security.md) release checklist.

---

## Milestone 20 — Release readiness (open source)

**Goal:** Shippable signed binaries and public documentation.

**Depends on:** 19

**Deliverables:**

- [ ] `SECURITY.md` — disclosure policy (root repo)
- [ ] Threat model summary (link to `docs/security.md`)
- [ ] README: install, quickstart, `.env` setup, library examples
- [ ] CHANGELOG.md
- [ ] GitHub Actions: build artifacts for win/mac/linux
- [ ] Code signing pipeline documented (notarization / Authenticode)
- [ ] Reproducible build notes
- [ ] License file (e.g. MIT or Apache-2.0 — choose before publish)

**Verify:**

- Fresh user follows README → unlock → bucket → Python lib → secret in app
- GitHub Release contains three platform binaries

**Exit criteria:** v0.1.0 public release tag.

---

## Post-v1 backlog (not in the 20 milestones)

| Feature | Phase |
|---|---|
| Java / Go / Ruby libraries | Phase 5 |
| Encrypted vault export/import | Phase 5 |
| `.env` scanner for plaintext secrets | Phase 5 |
| Import bulk from `.env` | Phase 5 |
| Hardware key (WebAuthn) as optional 3rd factor | Future |

---

## Suggested timeline (solo developer)

| Milestones | Calendar (estimate) |
|---|---|
| 1–2 | Week 1 |
| 3–5 | Weeks 2–3 |
| 6–9 | Weeks 4–6 |
| 10–13 | Weeks 7–10 (tray + v2 IPC + popups) |
| 14–15 | Deferred (client libs) |
| 16–20 | Weeks 11–14 |
| 18–20 | Weeks 13–14 |

Adjust ±30% based on SQLCipher platform build pain on Windows.

---

## Milestone dependency graph

```
1 ──► 2
 │
 └──► 3 ──► 4 ──► 5 ──► 6 ──► 7
                  │       │
                  │       └──► 8 ──► 10 ──► 11 ──► 12 ──► 13
                  │              │                    │
                  └──► 9 ◄───────┘                    ├──► 14
                  │                                  ├──► 15
                  └──► 16 ◄──────────────────────────├──► 17
                                                     │
                  18 ◄── 5,9,16                      │
                  19 ◄── all                         │
                  20 ◄── 19                          │
```

---

*Work the list top to bottom. Security first, socket second, polish last.*
