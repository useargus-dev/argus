# Argus documentation

Public technical docs for the Argus desktop app. Start with the [root README](../README.md) for install and license.

| Document | Purpose |
|----------|---------|
| [architecture.md](./architecture.md) | System design: auth, vault, buckets, IPC, fingerprint, tray, SQLCipher, file layout |
| [security.md](./security.md) | Threat model, cryptography, hardening checklist |
| [build-deps.md](./build-deps.md) | SQLCipher / OpenSSL setup (Windows, macOS, Linux) |
| [design.md](./design.md) | UI screens, components, user flows |
| [plan.md](./plan.md) | **Roadmap** — ordered milestones; many items are not implemented yet |

## Auth model (implemented today)

| Action | Requirement |
|--------|-------------|
| **Register** | Email, username, password + **TOTP setup** or **biometric** |
| **Sign in / unlock app** | Password + TOTP **or** biometric → access to vault, buckets, settings |
| **Soft lock** | Window lock; keys stay in memory; unlock with TOTP/biometric only |
| **Vault & bucket CRUD** | App unlocked (no separate per-scope elevation in current builds) |

Older docs may mention separate “elevate vault/buckets” steps; that was simplified to **app unlock** unless noted otherwise in [architecture.md](./architecture.md).

## Before you ship or fork

- Read [security.md](./security.md) § Known limitations.
- **IPC** (socket/pipe, advanced fingerprint, grants, requests popup window) and **tray** (hide-on-close, left-click opens requests) are **shipped**. **Client libraries** are **planned** — see [architecture.md](./architecture.md) §11 and `pnpm ipc:test` (`scripts/ipc-request.ts`).
- **Approvals page** in main app sidebar — view and revoke all client grants.
- **Licensing:** community code is [AGPL-3.0](../LICENSE); proprietary/self-hosted offerings use [COMMERCIAL_LICENSE.md](../COMMERCIAL_LICENSE.md).
