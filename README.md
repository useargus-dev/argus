# Argus

**Privacy-first secrets vault** for developers. Store API keys and credentials in an encrypted database on your machine—no cloud sync. Built with [Tauri 2](https://v2.tauri.app/) and React.

> **Status:** Early development (v0.2). Not security-audited. Use at your own risk for non-production or personal workflows.

## Features (current)

- Encrypted vault (**SQLCipher**) at `~/.argus/argus.db`
- Registration with master password + **TOTP** or **biometric** unlock; **recovery code** at setup
- **Vault** — typed secrets, tags, filters, expiry
- **App buckets** — map env names to vault secrets; per-bucket client tokens
- **Argus Proxy** (optional) — per-bucket loopback HTTP MITM; proxy-enabled mappings inject `argus-proxy-*` placeholders instead of real keys; per-mapping allowed domains
- **Settings** — security, notifications, tray preference (close hides to tray when signed in)
- **Local IPC** — apps request bucket env vars via `\\.\pipe\argus` (Windows) or `~/.argus/argus.sock` (macOS/Linux); client identity derived server-side via OS process inspection + SHA-256 fingerprint; grants stored with per-bucket TTL
- **Requests window** — compact bottom-right popup from system tray showing pending access requests with Accept/Deny per request; works even while app is locked
- **Approvals page** — view and revoke all active/expired client grants from the main app sidebar
- **System tray** — left-click opens requests window (if signed in); hide on close; IPC server runs while signed in

**Client libraries:** [Node.js `@useargus/node`](https://www.npmjs.com/package/@useargus/node) ([source](https://github.com/useargus-dev/node-argus)) and [Python `useargus`](https://pypi.org/project/useargus/) ([source](https://github.com/useargus-dev/py-argus)) are published (v0.2). Go, Ruby, and Java SDKs are in development. See [docs/architecture.md](docs/architecture.md) §11.5 (proxy) and §16 (SDKs).

Test IPC without a library: `pnpm ipc:test` (see [docs/architecture.md](docs/architecture.md) §11).

## Quick start

**Requirements:** Rust ≥ 1.88, Node.js 20+, [pnpm](https://pnpm.io/).  
**Windows:** [SQLCipher / OpenSSL build notes](docs/build-deps.md) (vcpkg).

```bash
git clone <your-repo-url>
cd <repository-name>   # root contains package.json and src-tauri/
pnpm install
pnpm tauri dev
```

| Command                  | Description                    |
| ------------------------ | ------------------------------ |
| `pnpm tauri dev`         | Desktop app with hot reload    |
| `pnpm tauri build`       | Release binaries               |
| `pnpm build`             | Frontend production build only |
| `pnpm exec tsc --noEmit` | Typecheck                      |
| `pnpm screenshots:capture` | Regenerate docs UI screenshots |

## Data on disk

| Path                  | Purpose                                             |
| --------------------- | --------------------------------------------------- |
| `~/.argus/argus.db`   | Encrypted database                                  |
| `~/.argus/`           | App data (see [architecture](docs/architecture.md)) |
| `~/.argus/argus.sock` | IPC endpoint (Unix; while signed in)                |
| `~/.argus/ca-bundle.pem` | Argus Proxy MITM CA bundle (when proxy used)     |
| `\\.\pipe\argus`      | IPC endpoint (Windows; while signed in)             |

## Documentation

| Document | Description |
| -------- | ----------- |
| [docs/README.md](docs/README.md) | Doc index |
| [docs/architecture.md](docs/architecture.md) | System design, IPC, proxy |
| [docs/security.md](docs/security.md) | Threat model, crypto, limitations |
| [docs/build-deps.md](docs/build-deps.md) | SQLCipher build per OS |

## Contact

Questions and feedback: [ssamuel.sushant@gmail.com](mailto:ssamuel.sushant@gmail.com)

## License

AGPL-3.0-or-later — see [LICENSE](LICENSE). Commercial licensing: [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md).
