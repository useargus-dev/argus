# Vendored mitmproxy_rs (patched)

Upstream: https://github.com/mitmproxy/mitmproxy_rs

## Patches applied

1. **`src/packet_sources/wireguard.rs`** — `Tunn::new()` now returns `Result`; map error for `anyhow` compatibility.
2. **`argus/Cargo.toml`** — removed nested `[workspace]` so this crate can be a path dependency of the Argus workspace.
3. **`mitmproxy-linux-ebpf*` manifests** — explicit metadata (no workspace inheritance).

Used by `argus-intercept`, `argus-redirector-linux`, and `argus-redirector-windows`.
