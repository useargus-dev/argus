# Argus install paths vs runtime data

Argus uses two different directory concepts:

## `ARGUS_HOME` — install / sidecars

Set by the installer or dev workflow. Contains bundled binaries:

| Path | Contents |
|------|----------|
| `{ARGUS_HOME}/lib/argus/argus-cli.exe` | CLI sidecar (`argus run`, Windows) |
| `{ARGUS_HOME}/lib/argus/argus-redirector-windows.exe` | WinDivert redirector (Windows) |
| `{ARGUS_HOME}/lib/argus/argus-redirector-linux` | eBPF redirector (Linux) |

Dev default when building locally: `argus/target/debug` with `lib/argus/` staged beside `argus-cli.exe`.

## `~/.argus` / `ARGUS_DATA_DIR` — runtime data

| Path | Contents |
|------|----------|
| `argus.sock` (Unix) or `\\.\pipe\argus-{sessionId}` (Windows) | Desktop IPC |
| `ca-bundle.pem` | MITM CA bundle for sandbox children |
| SQLCipher DB | Buckets, grants, sandbox sessions |

The CLI connects to IPC in the data dir but loads redirectors from `ARGUS_HOME`.
