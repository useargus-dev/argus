# Argus

**Local-first secrets vault** for developers. Store API keys and credentials in an encrypted database on your machine—no cloud sync. Built with [Tauri 2](https://v2.tauri.app/) and React.

> **Status:** Early development (v0.1). Not security-audited. Use at your own risk for non-production or personal workflows.

## Features (current)

- Encrypted vault (**SQLCipher**) at `~/.argus/argus.db`
- Registration with password + **TOTP** or **biometric** unlock
- **Vault** — typed secrets, tags, filters, expiry
- **App buckets** — map env names to vault secrets; per-bucket client tokens
- **Settings** — security, notifications, tray preference (close hides to tray when signed in)
- **Local IPC** — apps request bucket env vars via `\\.\pipe\argus` (Windows) or `~/.argus/argus.sock` (macOS/Linux); first connect shows an approval dialog; grants stored with TTL
- **System tray** — hide on close, Open / Sign out; IPC server runs while signed in

**Planned:** packaged client libraries wrapping IPC. Test with `pnpm ipc:test`. See [docs/architecture.md](docs/architecture.md) §11.

## Quick start

**Requirements:** Rust ≥ 1.88, Node.js 20+, [pnpm](https://pnpm.io/).  
**Windows:** [SQLCipher / OpenSSL build notes](docs/build-deps.md) (vcpkg).

```bash
git clone <your-repo-url>
cd <repository-name>   # root contains package.json and src-tauri/
pnpm install
pnpm tauri dev
```

| Command | Description |
|---------|-------------|
| `pnpm tauri dev` | Desktop app with hot reload |
| `pnpm tauri build` | Release binaries |
| `pnpm build` | Frontend production build only |
| `pnpm exec tsc --noEmit` | Typecheck |

## Data on disk

| Path | Purpose |
|------|---------|
| `~/.argus/argus.db` | Encrypted database |
| `~/.argus/` | App data (see [architecture](docs/architecture.md)) |
| `~/.argus/argus.sock` | IPC endpoint (Unix; while signed in) |
| `\\.\pipe\argus` | IPC endpoint (Windows; while signed in) |

Never commit real bucket IDs, tokens, or database files.

## Documentation

| Document | Audience |
|----------|----------|
| [docs/architecture.md](docs/architecture.md) | System design, modules, data layout |
| [docs/security.md](docs/security.md) | Threat model, crypto, hardening |
| [docs/build-deps.md](docs/build-deps.md) | SQLCipher / platform build setup |
| [docs/design.md](docs/design.md) | UI screens and flows |
| [docs/plan.md](docs/plan.md) | Development roadmap (aspirational) |

## Security

Argus handles sensitive material. Read [SECURITY.md](SECURITY.md) before reporting issues. For vulnerabilities, use **GitHub Security Advisories** (private) on this repository—not public issues.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). By contributing, you agree that your contributions are licensed under the same terms as this project.

## Third-party software

Argus depends on open-source libraries (React, Tauri, SQLCipher, etc.) under their own licenses. This repository’s license applies to **Argus source code** only, not to dependencies.

## License

**Community edition** (this repository) is free software under the **[GNU Affero General Public License v3.0](LICENSE)** (AGPL-3.0).

| You can (under AGPL) | You need a [commercial license](COMMERCIAL_LICENSE.md) |
|----------------------|--------------------------------------------------------|
| Use on your own machine (personal, freelance, or company internal) | Ship a **closed-source** product that includes Argus |
| View, fork, and modify the source | Offer a **hosted/SaaS** service without AGPL source compliance |
| Distribute changes **if** you comply with AGPL (source available to users) | Resell a proprietary derivative without open-sourcing it |

A **paid self-hosted / team edition** for startups is planned as a separate product; see [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) for inquiries.

The **Argus** name and logo are not covered by AGPL — do not imply endorsement for competing products.

## Contact

- **Security:** [SECURITY.md](SECURITY.md) (GitHub Security Advisories)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Commercial licensing:** [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) or GitHub Discussions on this repository
