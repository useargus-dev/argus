# Argus

**Local-first secrets vault** for developers. Store API keys and credentials in an encrypted database on your machine—no cloud sync. Built with [Tauri 2](https://v2.tauri.app/) and React.

> **Status:** Early development (v0.1). Not security-audited. Use at your own risk for non-production or personal workflows.

## Features (current)

- Encrypted vault (**SQLCipher**) at `~/.argus/argus.db`
- Registration with password + **TOTP** or **biometric** unlock
- **Vault** — typed secrets, tags, filters, expiry
- **App buckets** — map env names to vault secrets; per-bucket client tokens
- **Settings** — security, notifications, tray preference (close hides to tray when signed in)
- **Local IPC** — apps request bucket env vars via `\\.\pipe\argus` (Windows) or `~/.argus/argus.sock` (macOS/Linux); client identity derived server-side via OS process inspection + SHA-256 fingerprint; grants stored with per-bucket TTL
- **Requests window** — compact bottom-right popup from system tray showing all pending access requests (last 15 min) with Accept/Deny per request; works even while app is locked
- **Approvals page** — view and revoke all active/expired client grants from the main app sidebar; works even while app is locked (vault and buckets UI require unlock)
- **System tray** — left-click opens requests window (if signed in); hide on close; IPC server runs while signed in

**Client libraries:** [Node.js `@useargus/node`](https://www.npmjs.com/package/@useargus/node) is available on npm. Python, Go, Ruby, and Java SDKs are in development. See [docs/architecture.md](docs/architecture.md) §16 and the [node-argus](https://github.com/useargus-dev/node-argus) repository.

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

## Data on disk

| Path                  | Purpose                                             |
| --------------------- | --------------------------------------------------- |
| `~/.argus/argus.db`   | Encrypted database                                  |
| `~/.argus/`           | App data (see [architecture](docs/architecture.md)) |
| `~/.argus/argus.sock` | IPC endpoint (Unix; while signed in)                |
| `\\.\pipe\argus`      | IPC endpoint (Windows; while signed in)             |

Never commit real bucket IDs, tokens, or database files.

## Documentation

| Document                                     | Audience                            |
| -------------------------------------------- | ----------------------------------- |
| [docs/architecture.md](docs/architecture.md) | System design, modules, data layout |
| [docs/security.md](docs/security.md)         | Threat model, crypto, hardening     |
| [docs/build-deps.md](docs/build-deps.md)     | SQLCipher / platform build setup    |
| [docs/design.md](docs/design.md)             | UI screens and flows                |
| [docs/plan.md](docs/plan.md)                 | Development roadmap (aspirational)  |
| [@useargus/node](https://www.npmjs.com/package/@useargus/node) | Node.js SDK (`loadEnv`) — [source](https://github.com/useargus-dev/node-argus) |

## Security

Argus handles sensitive material. Read [SECURITY.md](SECURITY.md) before reporting issues. For vulnerabilities, use **GitHub Security Advisories** (private) on this repository—not public issues.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). By contributing, you agree that your contributions are licensed under the same terms as this project.

## Third-party software

Argus depends on open-source libraries (React, Tauri, SQLCipher, etc.) under their own licenses. This repository’s license applies to **Argus source code** only, not to dependencies.

## License

**Community edition** (this repository) is free software under the **[GNU Affero General Public License v3.0](LICENSE)** (AGPL-3.0).

| You can (under AGPL)                                                       | You need a [commercial license](COMMERCIAL_LICENSE.md)         |
| -------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Use on your own machine (personal, freelance, or company internal)         | Ship a **closed-source** product that includes Argus           |
| View, fork, and modify the source                                          | Offer a **hosted/SaaS** service without AGPL source compliance |
| Distribute changes **if** you comply with AGPL (source available to users) | Resell a proprietary derivative without open-sourcing it       |

Team servers and cloud sync are **out of scope** for the current desktop app. See [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) for uses that need a separate commercial license.

The **Argus** name and logo are not covered by AGPL — do not imply endorsement for competing products.

## Contact

- **Security:** [SECURITY.md](SECURITY.md) (GitHub Security Advisories)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Commercial licensing:** [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) or GitHub Discussions on this repository
