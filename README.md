# Argus

Local-first secrets vault and approval gateway for developer environments. One encrypted database per machine. Zero cloud.

## Quick start

**Requirements:** Rust ≥ 1.88, Node.js 20+, pnpm. On Windows, see [docs/build-deps.md](docs/build-deps.md) for SQLCipher/OpenSSL via vcpkg.

```bash
cd argus
pnpm install
pnpm tauri dev
```

## Documentation

| Document | Description |
|----------|-------------|
| [docs/plan.md](docs/plan.md) | 20-milestone development plan |
| [docs/architecture.md](docs/architecture.md) | System design, modules, IPC |
| [docs/design.md](docs/design.md) | UI screens and components |
| [docs/security.md](docs/security.md) | Threat model and crypto spec |
| [docs/build-deps.md](docs/build-deps.md) | SQLCipher build notes |

## Scripts

| Command | Description |
|---------|-------------|
| `pnpm dev` | Vite dev server (frontend only) |
| `pnpm build` | Production frontend build |
| `pnpm tauri dev` | Desktop app with hot reload |
| `pnpm tauri build` | Release binaries |

## Data directory

Runtime data lives at `~/.argus/` (see [architecture.md](docs/architecture.md)): encrypted `argus.db`, local socket/pipe when signed in.

## License

MIT (see LICENSE when published).
