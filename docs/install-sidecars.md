# Installing Argus (desktop + CLI + redirector)

GitHub releases ship **full installers only** — no standalone CLI/sandbox scripts or sidecar archives.

| Platform | Installer | What you get |
|----------|-----------|--------------|
| Windows | `argus_*_x64-setup.exe` (NSIS) | Desktop app, `argus` CLI on system PATH, WinDivert redirector |
| Linux | `.deb` or `.rpm` | Desktop app, `/usr/local/bin/argus`, eBPF redirector |
| macOS | `.dmg` | Desktop app (CLI/`argus run` capture not supported on macOS yet) |

## Bundled layout

| Platform | CLI | Redirector | `ARGUS_HOME` |
|----------|-----|------------|--------------|
| Linux | `/usr/local/bin/argus` → `{ARGUS_HOME}/lib/argus/argus-cli` | `{ARGUS_HOME}/lib/argus/argus-redirector-linux` | `/usr/lib/argus` (set in `/etc/profile.d/argus.sh`) |
| Windows | `{ARGUS_HOME}\bin\argus.exe` (from bundled `argus-cli.exe`) | `{ARGUS_HOME}\lib\argus\argus-redirector-windows.exe` + WinDivert | `%ProgramFiles%\argus` (`ARGUS_HOME` env + registry `InstallPath`) |

The Windows NSIS installer sets `ARGUS_HOME` (user + machine), registers **App Paths** for `argus.exe`, and drops `argus.cmd` into `%LOCALAPPDATA%\Microsoft\WindowsApps` (typically already on user Path). **It does not modify the Path environment variable.** Linux package postinst symlinks the CLI into `/usr/local/bin` and exports `ARGUS_HOME` for login shells.

## Dev / CI refresh (not on releases)

For local sidecar refresh without reinstalling the GUI, use scripts under [argus/scripts/install/](../scripts/install/) — these are **not published** on GitHub releases.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `IPC socket not found` | Start Argus desktop and sign in |
| `PROXY_DISABLED` | Enable proxy on bucket in Argus app |
| Linux sudo denied / no TTY | Approve sudo when prompted, configure polkit ([packaging/linux/README.md](../packaging/linux/README.md)), or use `--no-proxy` |
| Windows UAC denied | Approve the Administrator prompt for `argus-redirector-windows.exe`, or use `--no-proxy` |
| `argus-redirector-linux not found` | Reinstall the `.deb`/`.rpm` bundle (redirector is bundled; standalone archives are not published) |
| `argus` not found after Windows install | Use `*_setup.exe` (not MSI), open a **new** terminal after install |

## Code signing (Windows)

Release builds are signed in CI when signing secrets are configured. SmartScreen reputation requires an EV certificate — document your org's signing process for production releases.

## Version lock

CLI, redirectors, and desktop share the same release version (`CARGO_PKG_VERSION`). Mismatch may cause IPC compatibility issues — reinstall matching versions together.
