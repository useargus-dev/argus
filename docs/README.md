# Argus documentation

Public technical docs for the Argus desktop app. Start with the [root README](../README.md) for install and license.

| Document                             | Purpose                                                                             |
| ------------------------------------ | ----------------------------------------------------------------------------------- |
| [architecture.md](./architecture.md) | System design: auth, vault, buckets, IPC, fingerprint, tray, SQLCipher, file layout |
| [security.md](./security.md)         | Threat model, cryptography, hardening checklist                                     |
| [build-deps.md](./build-deps.md)     | SQLCipher / OpenSSL setup (Windows, macOS, Linux)                                   |
| [design.md](./design.md)             | UI screens, components, user flows                                                  |
| [plan.md](./plan.md)                 | **Roadmap** — ordered milestones; many items are not implemented yet                |

## Auth model (implemented today)

| Action                   | Requirement                                                           |
| ------------------------ | --------------------------------------------------------------------- |
| **Register**             | Email, username, password + **TOTP setup** or **biometric**           |
| **Sign in / unlock app** | Password + TOTP **or** biometric → access to vault, buckets, settings |
| **Soft lock**            | Blocks vault, buckets, dashboard, and settings UI; keys stay in memory; unlock with TOTP/biometric only |
| **Vault & bucket CRUD**  | App unlocked (no separate per-scope elevation in current builds)      |
| **IPC & client approvals** | Signed in only — **not** blocked by soft lock (requests window, approve/deny, approvals page) |
| **Sign out**             | Stops IPC, zeroizes keys, closes DB pool — full sign-in required again |

**App lock vs sign-out:** Soft lock protects the **main app UI** (vault and buckets). It does **not** stop the IPC server, process access requests, or grant approvals/revokes. Only **sign-out** tears down IPC and secret access.

Older docs may mention separate “elevate vault/buckets” steps; that was simplified to **app unlock** unless noted otherwise in [architecture.md](./architecture.md).

## Before you ship or fork

- Read [security.md](./security.md) § Known limitations.
- **IPC** (socket/pipe, advanced fingerprint, grants, requests popup window) and **tray** (hide-on-close, left-click opens requests) are **shipped**.
- **Client libraries:** **Node.js** [`@useargus/node`](https://www.npmjs.com/package/@useargus/node) is published ([source](https://github.com/useargus-dev/node-argus)). Python, Go, Ruby, and Java are **in development** — see [architecture.md](./architecture.md) §16. Test IPC with `pnpm ipc:test` (`scripts/ipc-request.ts`).
- **Approvals page** in main app sidebar — view and revoke all client grants (works while app is locked; vault/buckets UI require unlock).
- **Licensing:** community code is [AGPL-3.0](../LICENSE). Team servers and cloud sync are out of scope — see [architecture.md](./architecture.md). Other commercial licensing uses [COMMERCIAL_LICENSE.md](../COMMERCIAL_LICENSE.md).
