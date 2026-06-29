# Argus standalone install scripts (dev/CI only)

These scripts are for local refresh and CI — **not published on GitHub releases**. End users should install the full desktop bundle (`.deb`, `.rpm`, NSIS `.exe`, `.dmg`) which includes the app, CLI, and redirector with PATH setup.

See [install-sidecars.md](../docs/install-sidecars.md).

## Scripts

| Script | Purpose |
|--------|---------|
| `install-cli.sh` / `install-cli.ps1` | CLI only (`argus`) |
| `install-sandbox.sh` / `install-sandbox.ps1` | Platform redirector + deps |
| `install-sidecars.sh` | CLI + redirector bundle |

## Flags (all shell installers)

| Flag | Effect |
|------|--------|
| `--version v0.3.0` | Pin release tag |
| `--prefix ~/.local` | Install location |
| `--dry-run` | Print URLs and paths only |
