# M0 Spike — Sandbox transparent capture on bucket proxy port

> Internal design note for `argus run` M0/M1. See [sandbox_temp_plan.md](../sandbox_temp_plan.md).

## Port strategy (frozen)

- Each bucket uses **one** loopback listener on `app_buckets.proxy_port` (**9000–9100**).
- **No** separate transparent port pool (9101–9200 removed from plan).
- OS redirector (M2+) targets `127.0.0.1:{bucket.proxy_port}`.

## Protocol sniff (first bytes)

| First bytes | Route | Auth |
|-------------|-------|------|
| `CONNECT ` … | Existing `handle_connect` | `Proxy-Authorization` + grant |
| TLS record `0x16` | `handle_transparent` | Sandbox session + registered PID |
| Other | `501 Not Implemented` | — |

Implementation: read up to 8 bytes, branch; for CONNECT prepend bytes and continue header read; for TLS use `PrefixedStream` into transparent handler.

## mitmproxy_rs integration (M2+)

- **Repository:** https://github.com/mitmproxy/mitmproxy_rs
- **Pinned version:** `39a11ff` on `main` (mitmproxy_rs git; v0.12.9 tag + tun API fix)
- **Crates used by Argus:** `mitmproxy` (core), platform redirector binaries built from `mitmproxy-linux` / `mitmproxy-windows/redirector`
- **Hook:** `Server::init` + `LinuxConf` / `WindowsConf` in `src/packet_sources/` — Argus implements a Rust TCP relay handler (no PyO3) that forwards streams to `127.0.0.1:{proxy_port}`
- **Redirector subprocess:** `mitmproxy-linux-redirector <pipe-dir>` (sudo) or `windows-redirector.exe` (admin); protobuf IPC over Unix datagram / named pipe
- **Intercept spec:** PID-based via `InterceptConf::try_from("{pid}")` or process name prefix
- **tgid fix:** verify [mitmproxy #7787](https://github.com/mitmproxy/mitmproxy/issues/7787) in pinned release; exclude Argus desktop/redirector PIDs from intercept
- **Redirect target:** `127.0.0.1:{proxy_port}` from `sandbox_create` IPC response

## Manual M0 test procedure

1. Enable proxy on a bucket (port e.g. 9001).
2. Library mode: `curl -x http://tok@127.0.0.1:9001 https://allowed-host/...` — must still work.
3. Transparent path: register sandbox session + PID via IPC, open raw TCP to 9001, send TLS ClientHello (`0x16`) — full MITM via `handle_transparent`.

## IPC v4 summary

- `sandbox_create` → `{ session_id, proxy_port, expires_at, env, ca_bundle_path }`
- `sandbox_register_pids` → register child PIDs for session
- `sandbox_revoke` → revoke session

v3 requests without `type` field unchanged.

## Automated tests (M1)

- Unit: `evaluate_transparent_gate` in `proxy/transparent.rs`
- Integration: `src-tauri/tests/transparent_proxy.rs`
- IPC parse: `ipc/protocol.rs` + shared `argus-protocol` crate
