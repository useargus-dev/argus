# Argus Documentation

| Document | Purpose |
|---|---|
| [architecture.md](./architecture.md) | System design: **3 auth scopes**, tray + background IPC, **client grants** (bucket + token + uri), SQLCipher, modules |
| [plan.md](./plan.md) | 20 ordered milestones (client libs deferred; tray + v2 IPC in M11–13) |
| [design.md](./design.md) | Bento UI, register/login + **mandatory TOTP or biometric**, elevation modals, client popup, tray |
| [security.md](./security.md) | Threat model, scopes, tokens, mandatory 2FA, tray risks |
| [build-deps.md](./build-deps.md) | SQLCipher / OpenSSL setup (Windows, macOS, Linux) |

## Auth model (quick reference)

| Action | Factors |
|---|---|
| **Register** | Email, username, password + **setup TOTP or biometric** (required) |
| **Sign in (APP)** | Password + TOTP **or** biometric → all pages including Settings |
| **Vault CRUD** | APP + **elevate** (password + TOTP or biometric) |
| **Bucket CRUD** | APP + **elevate** (password + TOTP or biometric) |
| **External app IPC** | Signed-in core + user **popup** on new bucket/uri/token; TTL from bucket or global settings |

## Tray

Closing the main window keeps **active buckets** in the system tray and IPC running until **Sign out**.

Start with **plan.md** Milestone 1.
