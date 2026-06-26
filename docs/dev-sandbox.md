# Sandbox dev workflow (`argus run`)

How to run the desktop app and CLI sidecar together during development.

## Layout

| Artifact | Build output | Role |
|----------|--------------|------|
| Desktop app | `target/debug/argus.exe` (package `argus`) | IPC server + transparent proxy |
| CLI | `target/debug/argus-cli.exe` (package `argus-cli`) | `argus run` sidecar |
| Redirector | `target/debug/argus-redirector-windows.exe` (`crates/redirectors/windows`) | OS capture (Windows) |

The CLI and desktop **must** have different binary names so `cargo build` does not overwrite the Tauri app.

## Terminal 1 — desktop

```powershell
cd argus
pnpm install          # once
cargo build --release -p argus-cli   # required before first tauri dev (bundled sidecar)
pnpm tauri dev
```

`pnpm tauri dev` runs `scripts/prepare-tauri-bundle.mjs --dev` to set the correct CLI resource path per OS (`.exe` on Windows).

Sign in and **enable Argus Proxy** on your bucket.

**Important:** run the desktop from source (`pnpm tauri dev`) after pulling CLI/sandbox changes. An older installed Argus build only supports library-mode IPC (`fetch_env`) and will not create sandbox sessions for `argus run`.

## Terminal 2 — build sidecars

```powershell
cd argus
cargo build -p argus-cli
cargo build -p argus-redirector-windows   # full capture only
./scripts/stage-windivert.ps1             # copies WinDivert next to redirector
```

## `.env` for your app

```env
ARGUS_BUCKET_ID=<bucket-uuid>
ARGUS_BUCKET_TOKEN=<token-from-app>
```

## Quick checks

```powershell
.\target\debug\argus-cli.exe status
.\target\debug\argus-cli.exe run --dry-run -- cmd /c echo hello
```

## Env injection only (no admin)

```powershell
.\target\debug\argus-cli.exe run --no-proxy -- node app.js
```

## Full OS capture (UAC / sudo prompt)

Stage the redirector next to the CLI dev layout, then run from a **normal** shell (not Administrator):

```powershell
mkdir target\debug\lib\argus -Force
copy target\debug\argus-redirector-windows.exe target\debug\lib\argus\
copy target\debug\WinDivert.dll target\debug\lib\argus\ -ErrorAction SilentlyContinue
copy target\debug\WinDivert64.sys target\debug\lib\argus\ -ErrorAction SilentlyContinue
$env:ARGUS_HOME = "$PWD\target\debug"

.\target\debug\argus-cli.exe run -- curl https://your-allowed-host.example/
```

When capture starts, approve the **UAC** prompt for `argus-redirector-windows.exe`. Denying UAC fails with a clear error; use `--no-proxy` to skip OS capture.

On Linux, run from a normal shell — you may be prompted for **sudo** when capture starts. Optional polkit policy: [packaging/linux/README.md](../packaging/linux/README.md).

## Installed layout (release)

After NSIS/deb install, CLI is on PATH as `argus` (copied from bundled `lib/argus/argus-cli.exe`). Redirector lives at `$ARGUS_HOME/lib/argus/argus-redirector-windows.exe`.

See also [run-mode.md](./run-mode.md) and [install-sidecars.md](./install-sidecars.md).
