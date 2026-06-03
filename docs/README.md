# Argus documentation

Public technical docs for the Argus desktop app (v0.2). Start with the [root README](../README.md) for install and license.

| Document                             | Purpose                                                                             |
| ------------------------------------ | ----------------------------------------------------------------------------------- |
| [architecture.md](./architecture.md) | System design: auth, vault, buckets, IPC, **Argus Proxy**, fingerprint, tray        |
| [security.md](./security.md)         | Threat model, cryptography, hardening checklist, known limitations                    |
| [build-deps.md](./build-deps.md)     | SQLCipher / OpenSSL setup (Windows, macOS, Linux)                                   |
| [design.md](./design.md)             | UI screens, components, user flows                                                  |
| [plan.md](./plan.md)                 | **Roadmap** — ordered milestones; not kept in sync with main                        |

**Screenshots:** `docs/assets/screenshots/` — regenerate with `pnpm screenshots:capture` (see `docs/assets/screenshots/README.md`).

## Auth model (implemented today)

| Action                   | Requirement                                                           |
| ------------------------ | --------------------------------------------------------------------- |
| **Register**             | Username, master password + **TOTP setup** or **biometric**; recovery code at end |
| **Sign in / unlock app** | Password + TOTP **or** biometric → access to vault, buckets, settings |
| **Soft lock**            | Blocks vault, buckets, dashboard, and settings UI; keys stay in memory; unlock with TOTP/biometric only |
| **Vault & bucket CRUD**  | App unlocked (no separate per-scope elevation in current builds)      |
| **IPC & client approvals** | Signed in only — **not** blocked by soft lock (requests window, approve/deny, approvals page) |
| **Sign out**             | Stops IPC, zeroizes keys, closes DB pool — full sign-in required again |
| **Password reset**       | Recovery code flow only (not Settings) — re-encrypts vault; second factor unchanged |

**App lock vs sign-out:** Soft lock protects the **main app UI** (vault and buckets). It does **not** stop the IPC server, process access requests, or grant approvals/revokes. Only **sign-out** tears down IPC and secret access.

## Client libraries (v0.2)

- **Node.js** [`@useargus/node`](https://www.npmjs.com/package/@useargus/node) — `loadEnv()`; with Argus Proxy off use any HTTP client; with proxy on use config helpers and builders (`createArgusUndiciDispatcher`, `argusAxiosClientConfig`, …). Guides: [node-argus/docs/usage](https://github.com/useargus-dev/node-argus/tree/main/docs/usage)
- **Python** [`useargus`](https://pypi.org/project/useargus/) — `load_env()`; with proxy on use config helpers and builders (`argus_httpx_config`, `create_argus_requests_proxy_adapter`, …). Guides: [py-argus/docs/usage](https://github.com/useargus-dev/py-argus/tree/main/docs/usage)
- Go, Ruby, Java — **in development** (see [architecture.md](./architecture.md) §16)
- Test IPC: `pnpm ipc:test` (`scripts/ipc-request.ts`)

## Before you ship or fork

- Read [security.md](./security.md) § Known limitations.
- **Argus Proxy** is shipped — understand local MITM CA trust and placeholder vs real env exposure.
- **Licensing:** community code is [AGPL-3.0](../LICENSE). Team servers and cloud sync are out of scope — see [architecture.md](./architecture.md).
