# Argus source layout

## Naming rules

| Rule | TS/React | Rust | Python |
|------|----------|------|--------|
| File basename | ≤2 words, kebab-case | ≤2 words, snake_case | ≤2 words, snake_case |
| Function | ≤3 words | ≤3 words | ≤3 words |
| Long names | Use folders for context | Use `mod` folders | Use packages |

Examples: `features/buckets/mapping/detail.tsx` (not `bucket-mapping-detail-panel.tsx`).

## Frontend (`src/`)

```
app/          bootstrap, router, guards
core/         bridge, theme, toast, secrets helpers
shared/       ui primitives, hooks, types, utils, layout shell
features/     domain UI (auth, buckets, secrets, …)
state/        Zustand stores
styles/       global CSS
```

## Backend (`src-tauri/src/`)

```
api/          Tauri command handlers (thin)
infra/        db, persistence
crypto/       encryption, kdf, totp
proxy/        local HTTPS proxy
ipc/          client SDK socket server
register/     account registration flow
sessions/     pending access sessions
util/         helpers (fs, secure, biometry, …)
app/          run, tray (via lib.rs / main.rs)
```

Business logic lives in `api/` handlers and `infra/db/` today; a dedicated `domain/` layer is optional future work.

## SDKs

- `node-argus/src/{env,proxy,ipc}/`
- `py-argus/useargus/{env,proxy,ipc}/` (each subpackage has `__init__.py`)

Public npm/PyPI exports stay stable; internal paths follow the rules above.
