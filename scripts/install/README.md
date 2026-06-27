# Argus standalone install scripts

Optional refresh/CI installers for sidecars shipped by default with the desktop app.
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

Release assets: `argus-cli-*`, `argus-sandbox-*` on GitHub releases.
