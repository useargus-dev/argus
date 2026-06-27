# Installing Argus sidecars (CLI + sandbox redirector)

Every **standard Argus desktop install** ships the CLI and platform redirector automatically. Use standalone installers only to refresh sidecars without reinstalling the GUI.

## Bundled layout

| Platform | CLI | Redirector |
|----------|-----|------------|
| Linux | `/usr/local/bin/argus` (symlink from `argus-cli`) | `$ARGUS_HOME/lib/argus/argus-redirector-linux` |
| Windows | `%ProgramFiles%\Argus\bin\argus.exe` or `%LOCALAPPDATA%\argus\bin\argus.exe` (from bundled `argus-cli.exe`) | `{ARGUS_HOME}/lib/argus/argus-redirector-windows.exe` + WinDivert |

`ARGUS_HOME` defaults to the desktop install directory. Windows sets `InstallPath` under `HKCU` (per-user install) and/or `HKLM` (per-machine install). The NSIS installer adds `bin` to the user PATH always, and to machine PATH for per-machine installs.

## Standalone install scripts

From [argus/scripts/install/](../scripts/install/):

```bash
# Linux / macOS — CLI only
curl -fsSL https://get.argus.dev/install-cli.sh | sh

# Linux — redirector only
curl -fsSL https://get.argus.dev/install-sandbox.sh | sh

# CLI + redirector
curl -fsSL https://get.argus.dev/install-sidecars.sh | sh
```

```powershell
# Windows — CLI only
irm https://get.argus.dev/install-cli.ps1 | iex

# Windows — redirector + WinDivert
irm https://get.argus.dev/install-sandbox.ps1 | iex
```

Flags: `--version v0.2.1`, `--prefix ~/.local`, `--dry-run`.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `IPC socket not found` | Start Argus desktop and sign in |
| `PROXY_DISABLED` | Enable proxy on bucket in Argus app |
| Linux sudo denied / no TTY | Approve sudo when prompted, configure polkit ([packaging/linux/README.md](../packaging/linux/README.md)), or use `--no-proxy` |
| Windows UAC denied | Approve the Administrator prompt for `argus-redirector-windows.exe`, or use `--no-proxy` |
| `argus-redirector-linux not found` | Reinstall desktop bundle or run `install-sandbox.sh` |

## Code signing (Windows)

Release builds are signed in CI when signing secrets are configured. SmartScreen reputation requires an EV certificate — document your org's signing process for production releases.

## Version lock

CLI, redirectors, and desktop share the same release version (`CARGO_PKG_VERSION`). Mismatch may cause IPC compatibility issues — reinstall matching versions together.
