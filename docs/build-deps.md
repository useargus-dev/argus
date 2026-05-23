# Build dependencies (SQLCipher)

Argus encrypts `~/.argus/argus.db` with **SQLCipher** via `rusqlite` and the **`bundled-sqlcipher`** feature. SQLCipher is compiled into the app; each platform still needs a **crypto library at link time** (OpenSSL or macOS Security).

We intentionally **do not** use `bundled-sqlcipher-vendored-openssl` (that compiles OpenSSL from source and requires **Perl** on Windows).

The same setup applies to **`cargo check`**, **`pnpm tauri dev`**, and **`pnpm tauri build`**.

---

## Cargo.toml

```toml
rusqlite = { version = "0.39", features = ["bundled-sqlcipher"] }
```

---

## Windows

### One-time setup

1. Install [vcpkg](https://vcpkg.io/en/getting-started.html).
2. Install static OpenSSL (matches typical Tauri x64 MSVC builds):

   ```powershell
   vcpkg install openssl:x64-windows-static
   ```

3. Note the install path, e.g. `C:\vcpkg\installed\x64-windows-static`.

### Every build session (PowerShell)

Set before `cargo check`, `pnpm tauri dev`, or `pnpm tauri build`:

```powershell
$env:OPENSSL_DIR = "C:\vcpkg\installed\x64-windows-static"
$env:OPENSSL_LIB_DIR = "$env:OPENSSL_DIR\lib"
$env:OPENSSL_INCLUDE_DIR = "$env:OPENSSL_DIR\include"
```

Adjust `OPENSSL_DIR` if your vcpkg root differs.

### After changing SQLCipher / OpenSSL deps

```powershell
cd src-tauri
cargo clean
cargo check
```

Static OpenSSL is linked into the release `.exe`; **end users do not install OpenSSL**.

---

## macOS

- Feature: `bundled-sqlcipher` only.
- Install **Xcode Command Line Tools**: `xcode-select --install`
- No vcpkg or Homebrew OpenSSL required — the linker uses **Security Framework**.

```bash
cd src-tauri && cargo check
```

---

## Linux

Install build tools and OpenSSL headers:

```bash
# Debian / Ubuntu
sudo apt install build-essential pkg-config libssl-dev

# Fedora
sudo dnf install gcc pkg-config openssl-devel
```

```bash
cd src-tauri && cargo check
```

Release packages may depend on the distro `libssl` at runtime (normal for `.deb` / AppImage).

---

## CI (future)

| Runner | Steps |
|--------|--------|
| `windows-latest` | vcpkg + `openssl:x64-windows-static` + set `OPENSSL_DIR` |
| `macos-latest` | Xcode CLT |
| `ubuntu-latest` | `libssl-dev` |

---

## Troubleshooting

| Error | Fix |
|-------|-----|
| `Command 'perl' not found` | Remove `bundled-sqlcipher-vendored-openssl`; use `bundled-sqlcipher` + vcpkg on Windows |
| `Could not find directory of OpenSSL installation` (Windows) | Set `OPENSSL_DIR` (and lib/include dirs) before building |
| Link errors after feature change | `cargo clean` then rebuild |

---

## What does *not* need OpenSSL

Pure-Rust crates used for secrets (no SQLCipher): `argon2`, `aes-gcm`, `hkdf`, `totp-rs`, etc.
