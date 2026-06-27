# Argus `argus run` — Sandbox Development Plan

> **Status:** Draft / temporary planning document  
> **Version:** Argus v0.3 target feature  
> **Last updated:** 2026-06-07  
> **Purpose:** Milestone-based implementation guide for OS-level transparent network capture (`argus run`) using proven third-party libraries and a sidecar architecture.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Goals and Non-Goals](#3-goals-and-non-goals)
4. [Architecture Overview](#4-architecture-overview)
5. [Third-Party Libraries (Do Not Build From Scratch)](#5-third-party-libraries-do-not-build-from-scratch)
6. [Component Design](#6-component-design)
7. [CLI Design](#7-cli-design)
8. [Sidecar Architecture](#8-sidecar-architecture)
9. [Argus Core Changes](#9-argus-core-changes)
10. [IPC and Protocol Design](#10-ipc-and-protocol-design)
11. [Sandbox Session and Grant Model](#11-sandbox-session-and-grant-model)
12. [Transparent vs Explicit Proxy Modes](#12-transparent-vs-explicit-proxy-modes)
13. [Platform Implementation](#13-platform-implementation)
14. [Distribution, Signing, and User Experience](#14-distribution-signing-and-user-experience)
15. [SDK Changes (py-argus / node-argus)](#15-sdk-changes-py-argus--node-argus)
16. [Hot Reload and Process Trees](#16-hot-reload-and-process-trees)
17. [Known Limitations](#17-known-limitations)
18. [Milestone Plan](#18-milestone-plan)
19. [Testing Strategy](#19-testing-strategy)
20. [Risks and Mitigations](#20-risks-and-mitigations)
21. [References](#21-references)

---

## 1. Executive Summary

Argus v0.2 ships **library mode**: `load_env()` injects secrets (or placeholders), and developers manually wire HTTP clients (requests, httpx, axios, fetch, etc.) to the per-bucket loopback MITM proxy.

**`argus run`** is the v0.3 feature that wraps any command and intercepts outbound HTTP/HTTPS at the **operating system level**, routing traffic through the existing Argus proxy for credential injection — **without per-library SDK wiring**.

### Recommended approach (correct, not from scratch)

| Layer | Use | Build |
|-------|-----|-------|
| OS traffic redirect (Linux/Windows/macOS) | **[mitmproxy_rs](https://github.com/mitmproxy/mitmproxy_rs)** ecosystem | Thin adapter only |
| CLI orchestration | **`argus` CLI sidecar** (new Rust binary) | Yes (small) |
| MITM + CA + rewrite + allowlists | **Existing Argus `proxy/` crate** | Transparent acceptor only |
| Grant / approval / vault | **Existing Argus IPC + DB** | Sandbox session extension |

### Primary command

```bash
argus run uvicorn app:main --reload
argus run node server.js
argus run --bucket acme-backend -- cargo run --release
```

Pattern: `docker run`, `cargo run` — **`run` is the verb**, everything after it is the user command. Use `--` when Argus flags precede the command.

**Install:** Every Argus desktop installer ships the **CLI + sandbox redirector** automatically. Optional standalone installs via curl / PowerShell / npm — see [§14](#14-distribution-signing-and-user-experience).

### Repository scope — all sandbox code under `argus/`

**Yes.** All v0.3 **sandbox implementation code** lives under **`argus/`**. Sibling folders (`py-argus/`, `node-argus/`, `website/`) are not modified for core sandbox logic — but may receive **optional doc links** (§15).

| Area | Path | Change level | What changes |
|------|------|--------------|--------------|
| **Desktop core + UI** | `argus/src-tauri/`, `argus/src/` | **Major** | Transparent proxy acceptor, sandbox session IPC/DB, audit, React UI for active run sessions |
| **CLI sidecar** | `argus/cli/` | **Major (new)** | `argus run`, `status`, `sessions`, sidecar lifecycle |
| **Platform redirectors** | `argus/redirector-linux/`, `argus/redirector-windows/`, `argus/macos-redirector/` | **Major (new)** | OS capture binaries / macOS System Extension (Swift fork) |
| **Shared Rust crates** | `argus/crates/argus-protocol/`, `argus-ipc-client/`, etc. | **Major (new)** | IPC types, CLI IPC client, mitmproxy_rs adapter, redirector glue, PID tree helpers |
| **Install scripts** | `argus/scripts/install/` | **Moderate (new)** | Standalone CLI/sandbox installers (curl / PowerShell one-liners) |
| **Installer bundling** | `argus/src-tauri/tauri.conf.json`, NSIS/DMG/deb hooks | **Major** | Ship CLI + redirectors with every Argus desktop install |
| **Smoke tests** | `argus/tests/sandbox/` | **Moderate** | `argus run` integration smoke scripts |
| **Docs** | `argus/docs/` | **Moderate** | `run-mode.md`, install-sidecars.md |
| **This plan** | `argus/sandbox_temp_plan.md` | **Docs** | Implementation guide |
| **Workspace root** | `Cargo.toml` (repo root) | **Config only** | Workspace member paths under `argus/*` — no feature logic |

See [§6 Component Design](#6-component-design) and [§14 Distribution](#14-distribution-signing-and-user-experience) for layout, bundled install, and standalone install commands.

---

## 2. Problem Statement

### What works today (library mode)

- Argus desktop provides per-bucket loopback HTTP MITM proxy (ports 9000–9100).
- IPC returns env + optional `proxy` config (`httpProxy`, `caBundlePath`, etc.).
- py-argus / node-argus expose per-library wiring helpers.
- Proxy validates `Proxy-Authorization`, peer PID, fingerprint grant, and host allowlist on `CONNECT`.

### Pain points

1. **Every HTTP library needs explicit wiring** — requests needs custom adapter; axios needs `https-proxy-agent`; Node fetch needs undici dispatcher; LangChain needs monkey-patches.
2. **Non-Python/Node stacks are unsupported** — Rust reqwest, Go net/http, Java HttpClient, BAML, etc.
3. **Documentation burden** — one guide per library, ongoing maintenance.
4. **Grant mismatch for wrappers** — a parent launcher and child app have different fingerprints.

### What `argus run` solves

Transparent OS-level capture sends **all outbound TCP HTTP/HTTPS** (from the sandboxed process tree) through Argus before it leaves the machine. The app uses placeholders in env; Argus rewrites at MITM time — same security model as proxy mode, zero library patches.

---

## 3. Goals and Non-Goals

### Goals

- [ ] `argus run <command>` works on **Linux, macOS, Windows** (native, not WSL).
- [ ] Reuse **existing** bucket ID, bucket token, CA, rewrite rules, host allowlists, audit log.
- [ ] No changes required in user application code for standard HTTP/HTTPS stacks.
- [ ] Support **process trees** (uvicorn `--reload`, npm scripts spawning node, etc.).
- [ ] **Every Argus desktop install** ships CLI + platform sandbox redirector(s) on PATH (or standard symlinks); no separate download required for `argus run`.
- [ ] **Optional standalone installs** for CLI-only or sandbox-only via documented curl / PowerShell / npm one-liners (§14.4).
- [ ] Sidecar CLI talks to running Argus desktop over existing IPC socket.
- [ ] Library mode remains fully supported (no breaking changes).

### Non-Goals (v0.3)

- Filesystem isolation (`--isolate`) — design hook only; implement in v0.4+.
- gRPC, WebSocket-first, database drivers (PostgreSQL, Redis, MongoDB).
- Cloud sync, remote sandbox, container orchestration.
- Replacing mitmproxy's redirector with custom eBPF/WinDivert code.
- WSL2 support (eBPF disabled by default; mitmproxy officially unsupported).
- Certificate pinning bypass.
- Linux kernels below 6.8 for eBPF path (document fallback or fail gracefully).

---

## 4. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Developer shell                                                         │
│    $ argus run uvicorn app:main --reload                                 │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  argus CLI (sidecar binary)                                              │
│    1. Parse CLI, read .env                                               │
│    2. IPC → Argus: fetch env + create sandbox session                    │
│    3. Start platform redirector (mitmproxy_rs)                           │
│    4. exec child with placeholders + CA env vars                         │
│    5. Wait, forward signals, teardown on exit                            │
└───────┬─────────────────────────────┬───────────────────────────────────┘
        │ IPC (existing + new)         │ Redirector IPC (mitmproxy_rs proto)
        ▼                              ▼
┌───────────────────────┐    ┌────────────────────────────────────────────┐
│  Argus Desktop        │    │  Platform redirector                        │
│  (Tauri + Rust)       │    │  Linux:   mitmproxy-linux (eBPF / Aya)      │
│                       │    │  Windows: mitmproxy-windows (WinDivert)     │
│  • Vault / grants     │    │  macOS:   mitmproxy-macos (Network Ext.)    │
│  • Explicit proxy     │◄───│  Per-process-tree capture                   │
│  • Transparent proxy  │    └────────────────────────────────────────────┘
│    (NEW acceptor)     │                    ▲
│  • rewrite.rs         │                    │ TCP (redirected)
│  • ca.rs              │                    │
└───────────────────────┘                    │
                                             │
                              ┌──────────────┴──────────────┐
                              │  Child process tree          │
                              │  uvicorn / node / cargo …    │
                              └─────────────────────────────┘
```

### Two proxy front doors, one rewrite engine

| Mode | Entry | Auth | Status |
|------|-------|------|--------|
| **Explicit** (library mode) | HTTP `CONNECT` + `Proxy-Authorization` | Bucket token in proxy URL | Shipped |
| **Transparent** (`argus run`) | Raw TCP/TLS after OS redirect | Sandbox session + PID/tree | **New** |

Both paths converge on the same `handle_mitm_request` → `rewrite.rs` → upstream.

---

## 5. Third-Party Libraries (Do Not Build From Scratch)

### Primary dependency: mitmproxy_rs

**Repository:** https://github.com/mitmproxy/mitmproxy_rs  
**License:** MIT  
**Maturity:** Local capture shipped on all three platforms (mitmproxy 10.2+ Windows, 10.1.5+ macOS, 11.1+ Linux).

| Package | Platform | Mechanism | Notes |
|---------|----------|-----------|-------|
| `mitmproxy-linux` | Linux | eBPF via **Aya** (`cgroup/connect4`) | Kernel ≥6.8; requires sudo to load BPF |
| `mitmproxy-windows` | Windows | **WinDivert** via `windivert-rust` | Elevated redirector; bundles pre-signed `WinDivert64.sys` |
| `mitmproxy-macos` | macOS | Swift **Network Extension** (App Proxy Provider) | System Extension; separate signing pipeline |
| `mitmproxy-rs` (core) | All | Rust IPC, stream handling, `LocalRedirector` API | Protobuf over Unix socket / named pipe |

**Why mitmproxy_rs over alternatives:**

| Alternative | Why not primary |
|-------------|-----------------|
| Raw **Aya** alone | You'd reimplement redirector + original-dest lookup + TCP reassembly |
| Raw **windivert** alone | Same; mitmproxy already solved Windows packet→stream |
| **tun2proxy** | System-wide TUN/VPN model; wrong for per-command sandbox |
| **trans_proxy** | Transparent front-end only; no per-PID capture; needs iptables/pf setup |
| **relay-core** | Full MITM replacement; less mature for local PID capture |
| **sshuttle-rs** | Orchestrator reference; not a drop-in MITM integration |

### Transitive / reference libraries

| Crate | Role | Direct dependency? |
|-------|------|-------------------|
| [aya](https://github.com/aya-rs/aya) | eBPF load/attach | Via mitmproxy-linux only |
| [windivert](https://docs.rs/windivert) | WinDivert bindings | Via mitmproxy-windows only |
| **hyper**, **tokio-rustls**, **rcgen** | MITM (already in Argus) | Keep existing |

### Integration strategy

**Phase A — Evaluate:** Spike with stock `mitmproxy --mode local:curl` on each OS.

**Phase B — Embed:** Create `argus/crates/argus-intercept/` (and related `argus/crates/argus-redirector-core/`) workspace crates that wrap `LocalRedirector` from mitmproxy_rs, forwarding streams to Argus transparent port instead of mitmproxy's generic handler.

**Phase C — macOS fork:** Fork `mitmproxy-macos` redirector into `argus/macos-redirector/`; re-sign with Argus Developer ID; embed in `Argus.app/Contents/Library/SystemExtensions/`.

**License note:** mitmproxy_rs is MIT; Argus is AGPL-3.0. Using MIT library as dependency is compatible; do not copy MIT code into AGPL files without noting provenance.

---

## 6. Component Design

### Monorepo layout (target)

All sandbox paths live under **`argus/`**. The Cargo workspace is defined in the **repo root** `Cargo.toml` with members pointing at `argus/*` (config only — no feature code at repo root).

```
argus-project/
├── Cargo.toml                      # workspace members: argus/src-tauri, argus/cli, argus/crates/*, …
│
└── argus/                          # ALL v0.3 sandbox work lives here
    ├── src/                        # React UI — minor (active run sessions in M5)
    ├── docs/                       # architecture.md, run-mode.md, install-sidecars.md
    ├── sandbox_temp_plan.md        # this document
    ├── scripts/
    │   └── install/                # standalone component installers (§14.4)
    │       ├── install-cli.sh
    │       ├── install-cli.ps1
    │       ├── install-sandbox.sh
    │       ├── install-sandbox.ps1
    │       └── README.md
    ├── tests/
    │   └── sandbox/                # argus run smoke scripts (M2+)
    │
    ├── cli/                        # `argus` on PATH: run, status, sessions
    │   └── src/
    │       ├── main.rs
    │       ├── cmd/run.rs
    │       ├── cmd/status.rs
    │       └── cmd/sessions.rs
    ├── redirector-linux/           # mitmproxy-linux wrapper binary
    ├── redirector-windows/         # mitmproxy-windows wrapper + WinDivert bundle
    ├── macos-redirector/           # fork mitmproxy-macos sysex (Swift)
    │
    ├── crates/                     # shared libraries
    │   ├── argus-protocol/         # IPC v4 types (sandbox_create, etc.)
    │   ├── argus-ipc-client/       # socket client used by CLI + tests
    │   ├── argus-intercept/        # mitmproxy_rs LocalRedirector adapter
    │   ├── argus-redirector-core/  # platform dispatch, lifecycle glue
    │   └── argus-process-tree/     # PID tree walk / register helpers
    │
    └── src-tauri/
        ├── tauri.conf.json         # bundle CLI + redirectors with desktop (§14)
        ├── src/
        │   ├── proxy/
        │   │   ├── server.rs       # existing CONNECT proxy
        │   │   ├── transparent.rs  # NEW
        │   │   ├── session.rs      # NEW sandbox session auth
        │   │   ├── rewrite.rs      # existing (reuse)
        │   │   ├── ca.rs           # existing (reuse)
        │   │   └── auth.rs         # extend
        │   └── ipc/
        │       └── handler.rs      # extend for run session IPC (v4)
        └── tests/                  # integration tests for transparent acceptor
```

CLI and redirectors are **separate binaries** (not inside `src-tauri/`) so they can ship on PATH, be signed independently, and run elevated without loading the Tauri runtime — but they are **built, versioned, and installed from the same `argus/` tree** and **bundled in the main Argus installer by default**.

### Component responsibilities

| Component | Binary | Runs as | Responsibility |
|-----------|--------|---------|----------------|
| **Argus Desktop** | `Argus.app` / `argus.exe` | User | Vault, UI, IPC server, both proxy modes, session store |
| **argus CLI** | `argus` | User | `run`, `status`, `sessions`; orchestrates sandbox lifecycle |
| **Platform redirector** | `mitmproxy-linux-redirector`, `windows-redirector.exe`, macOS NE | Root/Admin/Extension | OS capture; stream IPC to Argus |
| **User command** | `uvicorn`, `node`, etc. | User (child) | Normal app; no Argus imports required |

---

## 7. CLI Design

### Design principles

1. **`run` is the primary verb** — not `sandbox run`.
2. **No quotes required** for normal commands — shell parses args naturally.
3. **`--` separator** when Argus flags precede the command (Unix convention).
4. **Fail fast** with actionable errors if Argus desktop is not running / locked / proxy disabled.
5. **Exit code propagation** — child exit code becomes `argus run` exit code.

### Command reference

#### Primary

```bash
# Basic — intercept all outbound HTTP/HTTPS from command
argus run uvicorn app:main --reload
argus run node server.js
argus run cargo run --release
argus run python -m pytest tests/

# Argus flags before command (requires --)
argus run --bucket acme-backend -- uvicorn app:main --reload --port 8080

# Explicit bucket from flag (no .env bucket needed for this run)
argus run --bucket 550e8400-e29b-41d4-a716-446655440000 -- node app.js
```

#### Flags (`argus run`)

| Flag | Description | Default |
|------|-------------|---------|
| `--bucket <id\|name>` | Bucket UUID or display name | From `.env` `ARGUS_BUCKET_ID` |
| `--env <path>` | Path to `.env` file | `./.env` |
| `--traffic` | Print live outbound requests to terminal (summary lines) | off |
| `--dry-run` | Validate preflight + print intercept plan; do not exec | off |
| `--no-proxy` | Inject real secrets (proxy off path); still uses `run` wrapper without capture | off |
| `--isolate` | Enable filesystem isolation (v0.4; flag reserved in v0.3) | off |
| `-h, --help` | Help | |

#### Supporting commands (phased)

```bash
# v0.3.0 — ship with run
argus status              # vault locked?, proxy up?, bucket proxy enabled?, active run sessions
argus run --help

# v0.3.1 — sessions
argus sessions            # list active IPC grants + run sessions
argus sessions revoke [--all | <session_id>]

# v0.3.2 — vault from terminal (optional)
argus unlock              # prompt password + TOTP (if feasible via IPC extension)
argus lock                # no-op or trigger app lock via IPC

# v0.4.0 — traffic
argus traffic [--session <id>]   # tail audit/proxy events for active run
```

### CLI parsing rules

```
argus run [ARGUS_FLAGS...] [--] COMMAND [COMMAND_ARGS...]
```

- If `--` present: everything before is Argus flags; everything after is exec argv.
- If `--` absent: first token not starting with `-` begins COMMAND; no Argus flags after COMMAND start.
- Unknown Argus flag → error with suggestion.

### Example UX

```text
$ argus run uvicorn app:main --reload
✓ Argus connected (bucket: acme-backend, proxy: 127.0.0.1:9001)
✓ Sandbox session: sess_a1b2c3 (expires in 60m)
⚠ Linux: you may be prompted for sudo to start network capture (once per sudo timeout)
→ Running: uvicorn app:main --reload
  [traffic] POST api.anthropic.com/v1/messages → 200 (rewrote ANTHROPIC_API_KEY)
```

### Error messages (required)

| Condition | Message direction |
|-----------|-------------------|
| Argus not running | "Start Argus and sign in. IPC socket not found at …" |
| Bucket proxy disabled | "Enable Argus Proxy on bucket '…' in the Argus app." |
| No grant | "Approve this client in Argus Requests window (120s timeout)." |
| Linux no sudo | "Network capture requires sudo. Approve the sudo prompt, configure polkit, or use --no-proxy." |
| macOS extension not approved | "Approve Argus Network Extension in System Settings → Privacy & Security." |
| Windows UAC denied | "Network capture was cancelled or denied. Approve the Administrator prompt, or use --no-proxy." |

---

## 8. Sidecar Architecture

### Why sidecar (not in-process Tauri command)

| Reason | Detail |
|--------|--------|
| **TTY / signals** | `argus run` must attach to user's terminal; inherit stdin/stdout/stderr; forward SIGINT/SIGTERM |
| **Process replacement semantics** | Child should be direct descendant for clean process tree capture |
| **Privilege separation** | Elevated redirector on Windows can be separate from UI process |
| **Developer ergonomics** | `argus` on PATH in CI/scripts without launching GUI |
| **Cross-platform** | Same CLI entry point; platform redirector selected at runtime |

### Sidecar ↔ Desktop relationship

```
argus CLI  ──IPC (Unix socket / named pipe)──►  Argus Desktop (always running when using run)
     │
     ├── fetch env (existing IPC)
     ├── create sandbox session (new IPC)
     └── revoke sandbox session (new IPC)

argus CLI  ──redirector IPC──►  mitmproxy redirector  ──streams──►  Argus transparent port
```

**Requirement:** Argus desktop must be signed in (IPC server active). Document prominently.

### Installation model — bundled by default

Every **standard Argus desktop install** (NSIS on Windows, DMG/pkg on macOS, deb/AppImage/tar.gz on Linux) ships:

| Component | Shipped with desktop? | On PATH / discoverable? |
|-----------|----------------------|-------------------------|
| Argus desktop (`Argus.app` / `argus.exe`) | ✅ Always | N/A (GUI) |
| **`argus` CLI** (`argus run`, `status`, …) | ✅ Always | ✅ Yes |
| **Platform sandbox redirector** | ✅ Always | Invoked by CLI (not user-facing) |
| macOS Network Extension | ✅ Embedded in app bundle | Activated on first `argus run` |
| WinDivert (`WinDivert64.sys`, `.dll`) | ✅ Bundled (Windows) | Next to redirector |

**User expectation:** Install Argus once → open terminal → `argus run …` works (subject to OS privilege steps: sudo / admin / sysex approval).

### Sidecar binary layout (installed files)

All sidecars share the **same version** as the desktop app (single release artifact set).

| Platform | Desktop | CLI | Redirector | Shared lib dir |
|----------|---------|-----|------------|----------------|
| **Linux** | `/opt/Argus/argus` (or AppImage mount) | `/usr/local/bin/argus` → `…/bin/argus` | `…/lib/argus/argus-redirector-linux` | `…/lib/argus/` |
| **Windows** | `C:\Program Files\Argus\argus.exe` | `…\bin\argus.exe` | `…\lib\argus\argus-redirector-windows.exe` + WinDivert | `…\lib\argus\` |
| **macOS** | `/Applications/Argus.app` | `Argus.app/Contents/MacOS/argus` + symlink `/usr/local/bin/argus` | NE in `Contents/Library/SystemExtensions/` | `Contents/Resources/lib/argus/` |

Environment variable for non-PATH discovery (optional fallback):

```bash
ARGUS_HOME=/opt/Argus          # Linux
ARGUS_HOME="/Applications/Argus.app/Contents/Resources"  # macOS
# Windows: HKLM\Software\Argus\InstallPath
```

CLI resolves redirector via `$ARGUS_HOME/lib/argus/` (or registry on Windows).

### Tauri bundling (implementation)

| Mechanism | Use |
|-----------|-----|
| **`tauri.conf.json` → `bundle.resources`** | Ship redirector binaries + WinDivert under `Resources/lib/argus/` |
| **`bundle.externalBin`** (optional) | Include CLI as sidecar binary in bundle; postinstall symlinks to PATH |
| **NSIS custom hook** | Add `bin\` to PATH; copy `lib\argus\*` |
| **DMG / pkg postinstall** | Symlink `/usr/local/bin/argus`; notarize all binaries together |
| **Linux `.deb` / AppImage** | `postinst` symlinks; `Depends` unchanged |

**Recommendation:** **Bundled full stack (default)** + **`argus/crates/argus-protocol`** shared types. Build all sidecars in CI before `tauri build`; pass artifact paths into bundle config. Version lock: CLI, redirectors, and desktop share `CARGO_PKG_VERSION`.

### Standalone component installs (optional)

For users who already have Argus desktop but need to **refresh** or **install sidecars on a headless/CI machine** without re-running the full GUI installer:

| Package | Contents | Requires desktop? |
|---------|----------|-------------------|
| **`argus-cli`** | `argus` binary only | Yes (IPC to running Argus) |
| **`argus-sandbox-<platform>`** | Platform redirector + deps (WinDivert on Windows) | Yes |
| **`argus-sidecars-<platform>`** | CLI + redirector (no GUI) | Yes |

Published as GitHub release assets: `argus-cli-x86_64-unknown-linux-gnu.tar.gz`, `argus-sandbox-windows-x86_64.zip`, etc.

### Standalone install one-liners (§14.4)

Hosted scripts live in-repo at `argus/scripts/install/` and are published to `https://releases.argus.dev/` (or GitHub `raw` + release URLs) at cut time.

#### CLI only

```bash
# Linux / macOS
curl -fsSL https://get.argus.dev/install-cli.sh | sh

# Windows (PowerShell)
irm https://get.argus.dev/install-cli.ps1 | iex
```

#### Sandbox redirector only (platform auto-detected)

```bash
# Linux / macOS
curl -fsSL https://get.argus.dev/install-sandbox.sh | sh

# Windows (PowerShell) — includes WinDivert bundle
irm https://get.argus.dev/install-sandbox.ps1 | iex
```

#### CLI + sandbox (sidecars bundle, no GUI)

```bash
curl -fsSL https://get.argus.dev/install-sidecars.sh | sh
# Installs both CLI and platform redirector into $ARGUS_HOME or ~/.local/argus
```

#### npm (optional convenience wrapper — M5)

```bash
# Global CLI shim that downloads matching release binary
npm install -g @useargus/cli

# Or run without global install
npx @useargus/cli run uvicorn app:main --reload
```

The npm package is a **thin installer/shim** (not a reimplementation): it downloads the same signed binaries from GitHub releases and places them on PATH. Publish from `argus/cli/npm/` when ready.

#### Flags (all shell installers)

| Flag | Effect |
|------|--------|
| `--version v0.3.0` | Pin release tag |
| `--prefix ~/.local` | Install location (default: `/usr/local` or `%LOCALAPPDATA%\Argus`) |
| `--cli-only` | Skip redirector (install-cli.sh default) |
| `--sandbox-only` | Skip CLI |
| `--dry-run` | Print download URLs and paths only |

Scripts verify **Ed25519/minisign** or **SHA256 checksums** from `SHA256SUMS` on the release page before installing.

### Sidecar lifecycle (detailed)

```
1. PARSE
   └─ cmd/run.rs: clap parse, resolve bucket from --bucket or .env

2. PREFLIGHT
   ├─ IPC ping / fetch_bucket_env (existing)
   ├─ Verify bucket.proxy_enabled == true
   ├─ Verify grant (existing approval flow for `argus` CLI fingerprint)
   └─ IPC sandbox_create → { session_id, proxy_port, expires_at }

3. REDIRECTOR SETUP
   ├─ Platform: start LocalRedirector (mitmproxy_rs)
   ├─ Intercept spec: process tree rooted at upcoming child PID
   │   (see §16 — use tree/cgroup, not single PID)
   └─ Redirect target: 127.0.0.1:{bucket.proxy_port} (9000–9100)

4. ENV PREP
   ├─ Inject bucket env (placeholders for proxy mappings)
   ├─ SSL_CERT_FILE = ~/.argus/ca-bundle.pem
   ├─ REQUESTS_CA_BUNDLE = same (Python)
   ├─ NODE_EXTRA_CA_CERTS = same (Node)
   ├─ ARGUS_SANDBOX = 1
   ├─ ARGUS_SANDBOX_SESSION = {session_id}
   └─ Apply .env overrides (existing precedence rules)

5. SPAWN
   ├─ posix_spawn / Command::spawn (no shell unless user command needs it)
   ├─ Register root PID + descendants with session (in redirector)
   └─ Optional: --traffic goroutine reading audit stream

6. WAIT
   ├─ waitpid, forward signals
   └─ On Ctrl+C: SIGINT to process group, then teardown

7. TEARDOWN (always, including panic)
   ├─ Stop redirector
   ├─ IPC sandbox_revoke(session_id)
   └─ Print summary if --traffic
```

---

## 9. Argus Core Changes

### 9.1 Transparent proxy acceptor (`proxy/transparent.rs`)

**Problem:** Current proxy expects HTTP `CONNECT` first (`proxy/server.rs`). Transparent capture delivers **raw TLS ClientHello** — no `Proxy-Authorization`.

**Solution:** Second acceptor on dedicated port (or same port with protocol sniff):

```
First bytes     → Route
"CONNECT "      → existing handle_connect (library mode)
TLS record 0x16 → transparent handle_transparent (argus run)
HTTP GET/...    → 501 or transparent plain HTTP (out of scope v0.3)
```

**Reuse from existing code:**

- `ca::server_config_for_host(host)` — leaf cert generation
- `rewrite.rs` — header/body placeholder substitution
- `bucket_mappings::list_proxy_rewrite_entries` — host-scoped rewrite
- `audit::proxy_request` — extend with `session_id`, `mode=transparent`

### 9.2 Sandbox session store (`proxy/session.rs` + DB migration)

New table `sandbox_sessions`:

```sql
CREATE TABLE sandbox_sessions (
  id              TEXT PRIMARY KEY,
  bucket_id       TEXT NOT NULL,
  grant_id        TEXT NOT NULL,          -- parent argus CLI grant
  parent_fingerprint TEXT NOT NULL,
  command_preview TEXT,
  root_pid        INTEGER,                -- set after spawn
  created_at      TEXT NOT NULL,
  expires_at      TEXT NOT NULL,
  revoked_at      TEXT,
  FOREIGN KEY (bucket_id) REFERENCES app_buckets(id)
);

CREATE TABLE sandbox_session_pids (
  session_id TEXT NOT NULL,
  pid        INTEGER NOT NULL,
  added_at   TEXT NOT NULL,
  PRIMARY KEY (session_id, pid)
);
```

Session TTL: default match bucket `access_ttl_minutes`; extend on activity optional v0.3.1.

### 9.3 Port allocation — shared bucket port (9000–9100)

| Mode | Port | Notes |
|------|------|-------|
| Library mode (CONNECT) | `app_buckets.proxy_port` (9000–9100) | Existing per-bucket listener |
| Sandbox mode (transparent TLS) | **Same** `app_buckets.proxy_port` | Protocol sniff on first bytes |

**Decision (M0):** No second port range. One listener per bucket; peek first bytes → `CONNECT` (explicit) or `0x16` TLS Handshake (transparent). Redirector targets `127.0.0.1:{bucket.proxy_port}`.

### 9.4 IPC handler extensions (`ipc/handler.rs`)

New request types on existing socket (v4 protocol extension):

- `sandbox_create`
- `sandbox_register_pids`
- `sandbox_revoke`

See §10.

### 9.5 Audit extensions

New audit actions:

- `SANDBOX_SESSION_CREATED`
- `SANDBOX_SESSION_REVOKED`
- `SANDBOX_PID_REGISTERED`
- Existing `PROXY_REQUEST` gains optional `session_id`, `capture_mode`

---

## 10. IPC and Protocol Design

### 10.1 Existing IPC (unchanged for library mode)

**Socket:** `~/.argus/argus.sock` (Unix) / `\\.\pipe\argus` (Windows)

**Request (v3):**

```json
{
  "request_id": "uuid",
  "bucket_id": "uuid",
  "client_token": "tok_...",
  "cwd": "/path/to/project"
}
```

**Response `ok`:**

```json
{
  "status": "ok",
  "env": { "ANTHROPIC_API_KEY": "argus-proxy-..." },
  "proxy": {
    "enabled": true,
    "httpProxy": "http://tok@127.0.0.1:9001",
    "httpsProxy": "http://tok@127.0.0.1:9001",
    "noProxy": "localhost,127.0.0.1,::1",
    "caBundlePath": "/Users/dev/.argus/ca-bundle.pem"
  }
}
```

### 10.2 New IPC messages (v4 — `argus run` only)

#### `sandbox_create`

```json
{
  "type": "sandbox_create",
  "request_id": "uuid",
  "bucket_id": "uuid",
  "client_token": "tok_...",
  "cwd": "/path/to/project",
  "command_preview": "uvicorn app:main --reload"
}
```

**Response:**

```json
{
  "status": "ok",
  "session_id": "sess_...",
  "proxy_port": 9001,
  "expires_at": "2026-06-05T18:00:00Z",
  "env": { "...": "argus-proxy-..." },
  "ca_bundle_path": "/Users/dev/.argus/ca-bundle.pem"
}
```

**Errors:** `PROXY_DISABLED`, `GRANT_REQUIRED`, `SESSION_NOT_FOUND`, `LOCKED`

#### `sandbox_register_pids`

Called after spawn and on reload when new worker PIDs appear:

```json
{
  "type": "sandbox_register_pids",
  "request_id": "uuid",
  "session_id": "sess_...",
  "pids": [12345, 12346]
}
```

#### `sandbox_revoke`

```json
{
  "type": "sandbox_revoke",
  "request_id": "uuid",
  "session_id": "sess_..."
}
```

### 10.3 Redirector ↔ Argus stream protocol

**Do not invent a new protocol if mitmproxy_rs can be adapted.**

mitmproxy_rs uses **Protobuf over Unix socket** (`src/ipc/mitmproxy_ipc.proto`). Each flow gets a dedicated socket connection with metadata (original destination, PID).

**Argus adapter approach:**

1. Redirector sends streams to the bucket's existing `proxy_port` (9000–9100).
2. Start `LocalRedirector` pointing at `127.0.0.1:{proxy_port}` (or implement `StreamHandler` trait if exposed).
3. Map flow metadata → sandbox session auth → existing MITM pipeline.

**M1 spike deliverable:** Document exact mitmproxy_rs hook point (`local_redirector.rs`, `start_local_redirector`) and whether Argus can implement a custom upstream handler without forking.

### 10.4 Transparent connection auth (replaces Proxy-Authorization)

For each transparent connection:

```
1. Extract src_pid from redirector metadata
2. Lookup sandbox_session_pids WHERE pid = src_pid AND revoked_at IS NULL
3. Load session → bucket_id
4. Verify session.expires_at > now
5. Verify host allowed (existing allowlist)
6. Load rewrite entries for host
7. MITM + rewrite + forward
```

If PID not registered → `407`/close + audit `PROXY_GRANT_DENIED`.

---

## 11. Sandbox Session and Grant Model

### Problem

- `argus` CLI obtains IPC grant (fingerprint of `argus` binary + `run uvicorn...` args).
- Child `uvicorn` worker has **different** fingerprint.
- Transparent path has **no** `Proxy-Authorization` header.

### Solution: grant delegation

```
argus CLI grant (parent)
        │
        ▼
sandbox_session (delegated authority)
        │
        ├── register root_pid at spawn
        ├── register child PIDs (reload, subprocess)
        └── transparent acceptor checks PID ∈ session
```

**First-time UX:** User approves **`argus`** CLI once (same Requests popup as today). Child processes inherit via session — **no per-uvicorn approval**.

### Fingerprint for `argus run`

Include in grant label:

```
client_label: "argus run (acme-backend)"
run_args: "/usr/bin/argus run uvicorn app:main --reload"
```

Store `command_preview` in `sandbox_sessions` for audit UI.

---

## 12. Transparent vs Explicit Proxy Modes

| Aspect | Explicit (library mode) | Transparent (`argus run`) |
|--------|-------------------------|---------------------------|
| User entry | `load_env()` + SDK wiring | `argus run <cmd>` |
| Client config | HTTP proxy URL + CA | None (OS capture) |
| Auth | `Proxy-Authorization` + grant fingerprint | Session + PID registry |
| Ingress | HTTP CONNECT | Raw TLS after redirect |
| py-argus / node-argus | Required wiring docs | Not required |
| Privileges | None | OS-specific (sudo/admin/extension) |
| Inbound server ports | Unaffected | Unaffected (egress only) |

**Both modes share:** placeholders in env, CA at `~/.argus/ca-bundle.pem`, rewrite rules, host allowlists, audit.

---

## 13. Platform Implementation

### 13.1 Linux

| Item | Detail |
|------|--------|
| **Library** | `mitmproxy-linux` (Aya eBPF) |
| **Kernel** | ≥6.8 officially ([mitmproxy docs](https://docs.mitmproxy.org/stable/concepts/modes/)) |
| **Privileges** | sudo to load BPF program |
| **WSL** | Unsupported (eBPF disabled) |
| **Containers** | Only `--network host` |
| **Process match** | First 16 chars of comm (`TASK_COMM_LEN`); prefer **PID tree registration** over name |
| **uvicorn reload** | Register new PIDs via `sandbox_register_pids` |

**Known issue (mitmproxy #7787):** eBPF must exclude Argus proxy worker threads (`tgid` not `tid`). Verify fix present in pinned mitmproxy_rs version.

**Fallback (v0.4 optional):** nftables REDIRECT + `SO_ORIGINAL_DST` for kernels <6.8 (no per-PID without cgroup setup).

### 13.2 Windows

| Item | Detail |
|------|--------|
| **Library** | `mitmproxy-windows` (WinDivert) |
| **Privileges** | Administrator for `WinDivertOpen()` |
| **Driver** | Bundle **official pre-signed** `WinDivert64.sys` from WinDivert distribution — **do not** attestation-sign custom driver |
| **Architecture** | Elevated `windows-redirector.exe` ↔ named pipe ↔ Argus |
| **IPv4** | Argus peer PID lookup already IPv4-only on Windows — consistent |

**App signing:** Sign desktop `argus.exe`, CLI `bin\argus.exe`, and `lib\argus\argus-redirector-windows.exe` with your Windows cert (EV recommended for SmartScreen).

### 13.3 macOS

| Item | Detail |
|------|--------|
| **Library** | Fork `mitmproxy-macos` (Swift Network Extension) |
| **API** | App Proxy Provider (not Packet Tunnel) — per-app/PID filtering |
| **Packaging** | System Extension in `Argus.app/Contents/Library/SystemExtensions/` |
| **Entitlements** | `app-proxy-provider-systemextension`, `com.apple.developer.system-extension.install` |
| **Signing** | Developer ID for app + sysex; **manual re-sign** (Xcode Direct Distribution broken for NE — [Apple forum](https://developer.apple.com/forums/thread/737894)) |
| **User step** | Approve System Extension in System Settings (one-time) |
| **Redirector flow** | mitmproxy pattern: copy helper to `/Applications`, activate extension, Unix socket IPC |

**Tauri note:** macOS NE requires Xcode/Swift target; not generatable from Tauri alone. Use [tauri-macos-xcode](https://github.com/Choochmeque/tauri-macos-xcode) or CI script.

### 13.4 Platform matrix summary

| Feature | Linux | Windows | macOS |
|---------|-------|---------|-------|
| `argus run` HTTP capture | ✅ M2 | ✅ M3 | ✅ M4 |
| Per-command (not system-wide) | ✅ eBPF | ✅ WinDivert PID | ✅ NE spec |
| `--reload` support | ✅ PID register | ✅ PID register | ✅ PID register |
| Sidecars bundled in desktop installer | ✅ M2 | ✅ M3 | ✅ M4 |
| OS privilege at `argus run` time | sudo | admin | extension approval |
| Code signing for capture | N/A | App only | App + sysex |

---

## 14. Distribution, Signing, and User Experience

### 14.1 Default install — full stack with Argus desktop

**Policy:** Users who install Argus get sandbox + CLI **automatically**. There is no separate “enable sandbox” installer step.

Post-install verification:

```bash
argus --version          # CLI on PATH
argus status             # IPC to desktop OK
argus doctor             # redirector present, OS support OK (M5)
```

First `argus run` may still prompt for **OS privileges** (sudo / UAC / System Extension) — that is runtime, not install-time.

### 14.2 Signing and certificates

| Platform | Certificate | Enables |
|----------|-------------|---------|
| **macOS** | Developer ID Application | Signed + notarized `Argus.app`, `argus` CLI; Gatekeeper pass |
| **macOS** | + NE provisioning profile | Signed System Extension (manual workflow) |
| **Windows** | EV/OV code signing | SmartScreen reputation for installer + binaries |
| **Linux** | None required | AppImage/deb/tar.gz; sidecar tarballs on GitHub releases |

Sign **all shipped binaries** in a release: desktop, `argus` CLI, and platform redirector(s). macOS notarizes app + sysex + CLI together.

### 14.3 What certificates do NOT remove

- macOS System Extension approval dialog (first run)
- Windows UAC elevation for WinDivert
- Linux sudo for eBPF load

### 14.4 Standalone install scripts (source of truth)

In-repo scripts (published to CDN on release):

| Script | Purpose |
|--------|---------|
| `argus/scripts/install/install-cli.sh` | Download + install CLI binary only |
| `argus/scripts/install/install-cli.ps1` | Windows CLI only |
| `argus/scripts/install/install-sandbox.sh` | Download + install platform redirector only |
| `argus/scripts/install/install-sandbox.ps1` | Windows redirector + WinDivert |
| `argus/scripts/install/install-sidecars.sh` | CLI + redirector bundle |
| `argus/scripts/install/README.md` | Flags, env vars, troubleshooting |

CI (`release.yml`) uploads these scripts alongside release binaries. `get.argus.dev` redirects to the matching tag’s GitHub release assets.

See also [§8 Installation model](#installation-model--bundled-by-default) for one-liner examples (curl, PowerShell, npm).

### 14.5 Installer checklist

- [ ] **Bundled by default:** every desktop installer ships CLI + platform redirector(s)
- [ ] Ship `argus` CLI on PATH (symlink or NSIS PATH update)
- [ ] Ship platform redirector under `$ARGUS_HOME/lib/argus/`
- [ ] Ship WinDivert.dll + pre-signed WinDivert64.sys (Windows)
- [ ] macOS: embed and notarize System Extension with main app
- [ ] Publish standalone release assets + install scripts for CLI-only / sandbox-only
- [ ] Document privilege requirements in README, `argus/docs/install-sidecars.md`, and `argus run --help`
- [ ] `argus doctor` (v0.3.2): check sidecars installed, version match, OS support, Argus connectivity

---

## 15. SDK Changes (py-argus / node-argus)

`argus run` makes per-library proxy wiring **unnecessary** for sandbox mode — that is the point. SDKs still matter for **library mode** and for apps that detect sandbox vs library context.

### What “out of scope” means here (repo boundary, not the feature)

| In scope for v0.3 | Out of scope for v0.3 |
|-------------------|------------------------|
| All sandbox **implementation** in `argus/` (CLI, redirectors, desktop) | New sandbox **logic** inside `py-argus/` or `node-argus/` source |
| Primary docs in **`argus/docs/run-mode.md`**, **`argus/docs/install-sidecars.md`** | Rewriting SDKs to duplicate `argus run` orchestration |
| Optional one-line links in SDK READMEs → Argus docs | Required code changes in SDKs for sandbox to work |
| Optional helpers (`is_sandbox_mode()`, skip wiring warnings) | Breaking changes to `load_env()` / `loadEnv()` |

**Sandbox is fully in scope for v0.3.** Only the **location of code changes** is constrained to `argus/`.

### v0.3 SDK work (minimal)

| Change | Where | Priority |
|--------|-------|----------|
| Document `argus run` as preferred path when proxy enabled | **`argus/docs/run-mode.md`** (primary) | **Required** |
| Link from `py-argus/README.md` / `node-argus/README.md` to Argus docs | Sibling repos | Optional |
| `is_sandbox_mode()` if `ARGUS_SANDBOX=1` | py-argus / node-argus | Optional |
| Skip proxy wiring warnings when `ARGUS_SANDBOX=1` | py-argus / node-argus | Optional |
| **No change** to `load_env()` / `loadEnv()` IPC contract | SDKs | **Required** |

When the CLI wraps a process it sets:

```bash
ARGUS_SANDBOX=1
ARGUS_SANDBOX_SESSION=sess_...
SSL_CERT_FILE=~/.argus/ca-bundle.pem   # (+ REQUESTS_CA_BUNDLE, NODE_EXTRA_CA_CERTS)
```

SDKs can read these env vars but **do not need to** for HTTP capture to work — OS redirect + MITM handles traffic.

### Library mode remains

Users who cannot use sandbox (CI without sudo, WSL, certificate pinning, pinned CA bundles) keep existing SDK wiring docs and `load_env()` flow unchanged.

### `apply_proxy_to_environ`

In sandbox mode the CLI injects CA env vars; **`HTTP_PROXY` / `HTTPS_PROXY` are not required** (capture is OS-level). SDK `apply_proxy_to_environ` helpers remain for **library mode only** — do not call them automatically when `ARGUS_SANDBOX=1`.

---

## 16. Hot Reload and Process Trees

### uvicorn `--reload` behavior

```
argus run uvicorn app:main --reload
        │
        ▼
  reloader process (parent)
        │
        └── spawns worker (child) ← outbound HTTP happens here
                │
                └── on file change: new worker PID
```

### Requirements

1. **Do not capture single PID only** — session must track **process tree**.
2. After spawn, CLI calls `sandbox_register_pids` with root PID.
3. Redirector intercept spec: **cgroup-based** (Linux eBPF) or **process tree walk** (Windows/macOS) OR register PIDs on `exec` events if mitmproxy_rs exposes them.
4. On reload: new worker PID → register before outbound calls (redirector may capture by parent cgroup / name prefix `uvicorn` as backup).

### Acceptance test

```bash
argus run uvicorn app:main --reload --port 8000
# Edit source file → reload
# Outbound API call still intercepted without re-approval
```

---

## 17. Known Limitations

Document prominently in user-facing docs:

| Limitation | Cause |
|------------|-------|
| Certificate pinning | App rejects Argus CA |
| gRPC, PostgreSQL, Redis | Not HTTP; out of scope |
| WSL2 | eBPF disabled |
| Linux kernel <6.8 | eBPF redirector unsupported |
| Docker default bridge network | Separate network namespace |
| QUIC / HTTP3 | May bypass TCP redirect |
| Hardcoded CA bundles (some Rust binaries) | Ignores `SSL_CERT_FILE` |
| `no_proxy` destinations | localhost etc. not captured (by design) |

---

## 18. Milestone Plan

### Overview timeline

| Milestone | Focus | Target | Duration |
|-----------|-------|--------|----------|
| **M0** | Spike & protocol design | Week 1–2 | 2 weeks |
| **M1** | Argus transparent acceptor + session IPC | Week 3–5 | 3 weeks |
| **M2** | `argus run` on Linux | Week 6–9 | 4 weeks |
| **M3** | `argus run` on Windows | Week 10–13 | 4 weeks |
| **M4** | `argus run` on macOS | Week 14–20 | 6–7 weeks |
| **M5** | Polish, docs, `status`/`sessions` | Week 21–23 | 2–3 weeks |

**Total estimate:** ~5–6 months (one senior Rust engineer, macOS parallel effort may need Swift help).

---

### M0 — Spike & Design Validation

**Goal:** Prove mitmproxy_rs integrates with Argus rewrite path; freeze protocols.

**Tasks:**

- [ ] Run `mitmproxy --mode local:curl` on Linux, Windows, macOS dev machines
- [ ] Read mitmproxy_rs `local_redirector.rs`, `mitmproxy_ipc.proto`
- [ ] Prototype: transparent TCP → Argus `handle_mitm_request` (manual test harness, no CLI)
- [ ] Confirm shared-port sniff strategy on 9000–9100 (no separate port pool)
- [ ] Finalize IPC v4 JSON schema (`sandbox_create`, etc.)
- [ ] Create `argus/cli/` crate skeleton with clap
- [ ] Draft `argus/scripts/install/` standalone installer scripts (§14.4)

**Deliverables:**

- Spike report (internal) with chosen hook points
- DB migration draft for `sandbox_sessions`
- Approved IPC v4 spec (section 10 of this doc)

**Acceptance criteria:**

- [ ] Single curl request through transparent path gets placeholder rewritten on allowed host
- [ ] Explicit CONNECT proxy still works (no regression)

---

### M1 — Argus Core: Transparent Acceptor + Session IPC

**Goal:** Argus desktop accepts redirected streams without mitmproxy CLI.

**Tasks:**

- [ ] Migration `003_sandbox.sql`
- [ ] Implement `proxy/transparent.rs`
- [ ] Implement `proxy/session.rs`
- [ ] Extend `ipc/handler.rs` for v4 messages
- [ ] Wire transparent path to existing bucket `proxy_port` listener (protocol sniff)
- [ ] Extend audit events
- [ ] Unit tests: session auth, PID lookup, host deny
- [ ] Integration test: mock redirector sends TLS to transparent port

**Deliverables:**

- Transparent acceptor in Argus desktop
- Sandbox session CRUD over IPC
- Tests in `argus/src-tauri/tests/`

**Acceptance criteria:**

- [ ] Mock client → transparent port → rewrite → upstream (test API key)
- [ ] Unregistered PID → denied + audit
- [ ] Session revoke → subsequent connections denied
- [ ] Library mode CONNECT unchanged

---

### M2 — Linux: `argus run` + mitmproxy-linux

**Goal:** Ship first usable `argus run` on native Linux.

**Tasks:**

- [ ] Add `argus/cli/` with `run`, `status` (minimal)
- [ ] Integrate `mitmproxy-linux` redirector (`argus/redirector-linux/`)
- [ ] Implement full sidecar lifecycle (§8)
- [ ] sudo/polkit UX for BPF load
- [ ] PID registration + uvicorn reload test
- [ ] **Bundled install:** Linux desktop installer ships CLI + redirector on PATH (§14)
- [ ] **Standalone:** publish `install-cli.sh`, `install-sandbox.sh`, release tarballs
- [ ] Test matrix: Ubuntu 22.04+, Fedora 39+, kernel 6.8+

**Deliverables:**

- `argus run` working on Linux amd64
- `docs/run-mode.md`, `docs/install-sidecars.md` (user guide)
- `argus/tests/sandbox/run_smoke.sh`
- Desktop `.deb`/AppImage includes CLI + redirector

**Acceptance criteria:**

- [ ] `argus run curl https://api.example.com` with placeholder in header → rewritten
- [ ] `argus run uvicorn ... --reload` survives reload
- [ ] `--dry-run` prints plan without exec
- [ ] Fresh Argus install: `argus --version` works without extra steps (M2+)
- [ ] Graceful error when Argus not running / proxy disabled

---

### M3 — Windows: `argus run` + mitmproxy-windows

**Goal:** Windows sandbox with elevated redirector helper.

**Tasks:**

- [ ] Integrate `mitmproxy-windows` (`argus/redirector-windows/`)
- [ ] Bundle WinDivert.dll + pre-signed WinDivert64.sys
- [ ] UAC elevation flow (redirector subprocess via `runas`; CLI stays unprivileged)
- [ ] Sign all shipped binaries with Windows cert
- [ ] Named pipe redirector IPC to Argus
- [ ] PID tree registration on Windows
- [ ] **Bundled install:** NSIS ships `bin\argus.exe` CLI + `lib\argus\` redirector + WinDivert
- [ ] **Standalone:** `install-cli.ps1`, `install-sandbox.ps1` on GitHub releases

**Deliverables:**

- `argus run` on Windows x64
- Signed NSIS installer with full sidecar stack
- Standalone zip assets for CLI-only / sandbox-only refresh

**Acceptance criteria:**

- [ ] Same smoke tests as M2 on Windows 10/11
- [ ] SmartScreen no warning (with EV cert) or documented OV reputation build
- [ ] Admin/UAC prompt once per run when not already elevated (redirector only)

---

### M4 — macOS: `argus run` + Network Extension

**Goal:** macOS sandbox via forked mitmproxy-macos sysex.

**Tasks:**

- [ ] Fork `mitmproxy-macos` → `argus/macos-redirector/`
- [ ] Rebrand bundle IDs, signing identities
- [ ] Manual Developer ID re-sign CI script (entitlements `-systemextension` suffix)
- [ ] Embed sysex in Argus.app; activate via Tauri on first `argus run`
- [ ] Notarize app + extension + CLI together
- [ ] Unix socket IPC to Argus transparent acceptor
- [ ] **Bundled install:** DMG/pkg ships CLI symlink + embedded sysex
- [ ] Document System Extension approval UX

**Deliverables:**

- Notarized Argus.dmg with extension
- `argus run` on macOS 13+ (Apple Silicon + Intel)

**Acceptance criteria:**

- [ ] Fresh machine: install → approve extension → `argus run curl ...` works
- [ ] Extension upgrade replaces old version cleanly (mitmproxy_rs 0.11.5+ behavior)
- [ ] Same uvicorn reload test

---

### M5 — Polish & Supporting Commands

**Goal:** Production-ready v0.3 release.

**Tasks:**

- [ ] `argus sessions`, `argus sessions revoke`
- [ ] `argus status` (rich output)
- [ ] `--traffic` terminal summary
- [ ] `argus doctor` preflight command (sidecar version match, redirector on disk)
- [ ] UI: active run sessions in Approvals/Audit
- [ ] `argus/docs/architecture.md` §11.6 `argus run`
- [ ] `argus/docs/run-mode.md`, `argus/docs/install-sidecars.md`
- [ ] Optional: link from py-argus / node-argus READMEs to Argus docs (§15)
- [ ] Optional npm wrapper `@useargus/cli` (§14.4)
- [ ] Performance pass: connection teardown (mitmproxy macOS cleanup bug class)

**Deliverables:**

- Argus v0.3.0 release — **full installer includes CLI + sandbox on all platforms**
- `argus/docs/run-mode.md`, `argus/docs/install-sidecars.md`
- Published curl/PowerShell/npm install paths

**Acceptance criteria:**

- [ ] All M2–M4 acceptance tests in CI where possible
- [ ] No critical leaks of real secrets in child env for proxy-enabled mappings
- [ ] Audit trail shows session + command preview

---

### Post-v0.3 backlog (v0.4+)

- [ ] `--isolate` filesystem sandbox (Landlock / sandbox-exec / Windows job objects)
- [ ] nftables fallback for Linux <6.8
- [ ] DNS query logging via redirector UDP path
- [ ] `argus traffic` TUI
- [ ] CI recipe without sudo (fail with clear message)

---

## 19. Testing Strategy

### Unit tests (Rust)

- Session create/revoke/expire
- PID registry membership
- Transparent vs CONNECT routing (first-byte sniff)
- Rewrite passthrough (existing + transparent path)

### Integration tests

| Test | Platform |
|------|----------|
| Mock redirector → transparent port | All (CI) |
| `argus run curl` smoke | Linux CI (kernel 6.8 runner) |
| `argus run python httpx` without wiring | Linux CI |
| uvicorn reload | Manual + nightly |
| Windows full path | Manual / self-hosted runner |
| macOS full path | Manual / macOS runner |

### Regression

- Existing library-mode tests under `py-argus/` / `node-argus/` must pass unchanged
- **`argus/tests/sandbox/`** — primary `argus run` smoke tests (live in `argus/`)
- Explicit CONNECT proxy tests unchanged

### Security tests

- Host not in allowlist → 403
- Expired session → deny
- Revoked session → deny
- Wrong bucket token in session → deny
- Placeholder never replaced for denied host

---

## 20. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| mitmproxy_rs API not embeddable without fork | High | M0 spike; fork and maintain minimal patch set |
| macOS NE signing complexity | High | Budget M4 separately; hire Swift/macOS CI expertise |
| AGPL + MIT dependency | Medium | Use as library only; legal review if copying code |
| Linux sudo UX friction | Medium | Document polkit rule; optional setcap on redirector (careful) |
| uvicorn reload PID race | Medium | Register PIDs early; cgroup intercept where available |
| Argus proxy self-capture loop (Linux) | High | Pin mitmproxy_rs with tgid fix; exclude Argus PIDs |
| WinDivert blocked by enterprise policy | Medium | Fall back to library mode; document |
| Certificate pinning in target apps | Low | Document limitation; no false promise |

---

## 21. References

### mitmproxy / local capture

- [mitmproxy_rs repository](https://github.com/mitmproxy/mitmproxy_rs)
- [Proxy modes (local capture limitations)](https://docs.mitmproxy.org/stable/concepts/modes/)
- [Linux local capture announcement](https://www.mitmproxy.org/posts/local-capture/linux/)
- [Windows local capture announcement](https://www.mitmproxy.org/posts/local-capture/windows/)
- [macOS local capture announcement](https://www.mitmproxy.org/posts/local-capture/macos/)
- [mitmproxy_rs macOS redirector README](https://github.com/mitmproxy/mitmproxy_rs/tree/main/mitmproxy-macos/redirector)
- [Linux eBPF tgid bug #7787](https://github.com/mitmproxy/mitmproxy/issues/7787)

### Libraries

- [Aya eBPF Rust crate](https://github.com/aya-rs/aya)
- [windivert Rust crate](https://docs.rs/windivert)
- [WinDivert documentation](https://reqrypt.org/windivert-doc.html)
- [trans_proxy (reference only)](https://github.com/madeye/trans_proxy)
- [tun2proxy (reference only)](https://github.com/tun2proxy/tun2proxy)

### Apple / Windows platform

- [TN3134 Network Extension deployment](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment)
- [Exporting Developer ID Network Extension (Apple forum)](https://developer.apple.com/forums/thread/737894)
- [Notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Tauri macOS code signing](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri Windows code signing](https://v2.tauri.app/distribute/sign/windows/)
- [Windows driver attestation signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-attestation)

### Tauri + System Extension

- [Tauri system extension issue #9586](https://github.com/tauri-apps/tauri/issues/9586)
- [tauri-macos-xcode](https://github.com/Choochmeque/tauri-macos-xcode)

### Argus existing docs (internal)

- `argus/docs/architecture.md` §11.5 — Per-bucket HTTP MITM proxy
- `argus/docs/security.md` — Proxy threat model
- `argus/src-tauri/src/proxy/` — Current implementation
- `py-argus/README.md` — Library mode
- `node-argus/README.md` — Library mode

---

## Appendix A — Decision Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Repo scope | All sandbox code under `argus/` | Single product tree; no core changes in sibling folders |
| Default install | CLI + sandbox bundled with desktop | Users expect `argus run` after one install |
| Standalone install | curl / PowerShell / optional npm | Refresh sidecars without full GUI reinstall; CI headless |
| CLI verb | `argus run` | Matches docker/cargo/npm; shorter than `sandbox run` |
| Capture library | mitmproxy_rs | Only battle-tested cross-platform local capture in Rust ecosystem |
| Architecture | Sidecar CLI + desktop IPC | TTY, signals, privilege separation |
| MITM engine | Keep Argus `proxy/` | Bucket auth, rewrite, audit already implemented |
| Transparent auth | Sandbox session + PID registry | No Proxy-Authorization in transparent mode |
| macOS capture | Fork mitmproxy-macos sysex | Cannot build NE in pure Rust/Tauri |
| WinDivert driver | Official pre-signed binary | Avoid Microsoft attestation signing |
| SDK changes | Minimal; docs in `argus/docs/` | Sandbox needs no SDK code; optional helpers + README links only |
| First platform | Linux | Fastest validation; mitmproxy 11.1 Linux capture |

---

## Appendix B — Quick Start for Implementers

```bash
# After M2 (Linux):
export ARGUS_BUCKET_ID=...
export ARGUS_BUCKET_TOKEN=...
argus run uvicorn app:main --reload

# After M1 (dev test without full redirector):
# Use mock transparent client connecting to Argus transparent port

# Spike (M0):
mitmproxy --mode local:curl
mitmdump --mode local:python --flow-detail 1
```

---

*End of plan. Use this document as the source of truth for v0.3 `argus run` implementation. Update milestones as spikes reveal mitmproxy_rs integration constraints.*
