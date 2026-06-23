# `argus run` — OS-level secret injection

`argus run` wraps any command and intercepts outbound HTTP/HTTPS at the operating system level, routing traffic through the Argus bucket MITM proxy for placeholder rewriting — without per-library SDK wiring.

## Prerequisites

1. **Argus desktop** installed, signed in, and IPC server active (`~/.argus/argus.sock` on Linux/macOS, `\\.\pipe\argus` on Windows).
2. **Bucket proxy enabled** in the Argus app for your target bucket.
3. **Platform privileges** (requested automatically at redirector start — no admin shell required):
   - **Linux:** `sudo` for eBPF load (kernel ≥ 6.8, native Linux — not WSL2). You may be prompted once per sudo timeout.
   - **Windows:** UAC prompt for `argus-redirector-windows.exe` (WinDivert). The CLI itself stays unprivileged.
   - **macOS:** Network Extension (M4 — not yet available).

## Basic usage

```bash
argus run uvicorn app:main --reload
argus run node server.js
argus run --bucket acme-backend -- cargo run --release
```

Use `--` when Argus flags precede the command:

```bash
argus run --bucket my-bucket -- uvicorn app:main --reload --port 8080
```

## Flags

| Flag | Description |
|------|-------------|
| `--bucket` | Bucket UUID or name (default: `ARGUS_BUCKET_ID` from `.env`) |
| `--env` | Path to `.env` (default: `./.env`) |
| `--dry-run` | Validate plan without executing |
| `--no-proxy` | Inject env without OS capture |

## Environment variables set in sandbox mode

```bash
ARGUS_SANDBOX=1
ARGUS_SANDBOX_SESSION=sess_...
SSL_CERT_FILE=~/.argus/ca-bundle.pem
REQUESTS_CA_BUNDLE=...
NODE_EXTRA_CA_CERTS=...
```

`HTTP_PROXY` is **not** set in sandbox mode — capture is OS-level.

## Hot reload (uvicorn `--reload`)

The CLI registers the root PID and watches the process tree; new worker PIDs are registered via IPC automatically.

## Limitations

See [sandbox_temp_plan.md §17](../sandbox_temp_plan.md) — certificate pinning, gRPC, WSL2, Docker bridge networks, QUIC, etc.

## Related

- [dev-sandbox.md](./dev-sandbox.md) — local dev workflow (Tauri + CLI)
- [install-sidecars.md](./install-sidecars.md) — bundled vs standalone install
- Library mode (SDK wiring) remains supported via `load_env()` / py-argus / node-argus
