# Argus — UI & Application Design

> Design system, screen specifications, and interaction patterns for the Argus desktop app.  
> **Stack:** React 19 · TypeScript · Tailwind CSS v4 · Zustand · Lucide React · React Hook Form · **boneyard-js** (skeletons)  
> **Layout:** Bento grid dashboard · custom UI component library (no generic component kit)

> **Note:** Screens marked **(planned)** or shown only in wireframes are not implemented yet (e.g. client access popup, rich tray menu). See [README](../README.md) for the current UI.

**Related:** [architecture.md](./architecture.md) · [plan.md](./plan.md) · [security.md](./security.md)

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Information Architecture](#2-information-architecture)
3. [Visual Design System](#3-visual-design-system)
4. [Global Layout](#4-global-layout)
5. [Screen Specifications](#5-screen-specifications)
6. [Authorization & Elevation UX](#8-authorization--elevation-ux)
7. [Client Access Popup](#9-client-access-popup)
8. [System Tray](#10-system-tray)
9. [Approval UX (legacy / CLI)](#11-approval-ux-legacy--cli)
10. [Component Library](#12-component-library)
11. [Forms & Secret Type UX](#13-forms--secret-type-ux)
12. [Security-Sensitive UI Patterns](#14-security-sensitive-ui-patterns)
13. [Bento Layout System](#15-bento-layout-system)
14. [Boneyard Skeletons](#16-boneyard-skeletons)
15. [Responsive & Platform Notes](#17-responsive--platform-notes)
16. [Accessibility](#18-accessibility)
17. [Frontend File Map](#19-frontend-file-map)

---

## 1. Design Principles

| Principle | UI implication |
|---|---|
| **Secrets are scarce** | Never show values by default; reveal is always explicit |
| **Approvals are visible** | Process path + cwd are prominent, not footnotes |
| **Calm, not flashy** | Dark-first security tool aesthetic; no gamification |
| **Local-only trust** | Profile is a local account only — no sync, no cloud sign-in |
| **Bento clarity** | Dashboard uses fixed grid cells; dense info without clutter |
| **Perceived performance** | Boneyard skeletons mirror real layout — no layout shift |
| **Errors are actionable** | "Argus is locked" → button to focus app, not stack traces |

---

## 2. Information Architecture

### Screen map

```
App launch
    │
    ├─ No local account? ──YES──► /register (create local account)
    │                                  │
    │                                  └──► /login (auto once) ──► /dashboard
    │
    └─ Account exists ──► /login
                              Step 1: email/username + password
                              Step 2: TOTP code OR biometric
                              │
                              └──► /dashboard (Scope: APP)
                                       │
                    First visit /vault write ──► Elevate Vault modal
                    First visit /buckets write ──► Elevate Buckets modal
                                       │
              Sidebar (4 items)        ├── /dashboard
                                       ├── /vault
                                       ├── /buckets
                                       └── /settings  → Sign out

  Global:
    • Client access popup (new app: bucket_id + token + uri)
    • Scope elevation modals (vault / buckets)
    • System tray (active buckets when window closed)
    • Expiring → dashboard + vault badges only
```

### Navigation labels

| Route | Label | Icon (Lucide) |
|---|---|---|
| `/dashboard` | Dashboard | `LayoutDashboard` |
| `/vault` | Vault | `KeyRound` |
| `/buckets` | Buckets | `Package` |
| `/settings` | Settings | `Settings` |

### Auth routes (no sidebar)

| Route | Label | When |
|---|---|---|
| `/register` | Create account | First run only (`users` table empty) |
| `/login` | Sign in | Every launch when signed out |

### Removed from navigation (by design)

| Route | Replacement |
|---|---|
| `/expiring` | “Expiring soon” bento tile on Dashboard + filter chip on Vault |
| `/audit` | Rust audit log remains; no dedicated UI in v1 |
| `/approvals` | Dashboard bento tile + sticky approval banner |
| `/lock` | Replaced by `/login` (same unlock semantics in Rust) |

---

## 3. Visual Design System

### 3.1 Theme (Tailwind v4 `@theme`)

Define in `src/styles/globals.css`:

```css
@import "tailwindcss";

@theme {
  /* Surfaces */
  --color-bg: #0c0e12;
  --color-surface: #141820;
  --color-surface-raised: #1c2230;
  --color-border: #2a3344;

  /* Text */
  --color-text: #e8edf4;
  --color-text-muted: #8b95a8;

  /* Accent — guardian eye / trust */
  --color-accent: #3b9eff;
  --color-accent-hover: #5aafff;

  /* Semantic */
  --color-success: #34d399;
  --color-warning: #fbbf24;
  --color-danger: #f87171;

  /* Typography */
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, monospace;
}
```

### 3.2 Color usage

| Token | Use |
|---|---|
| `bg` | App background |
| `surface` | Sidebar, cards |
| `surface-raised` | Modals, banners |
| `accent` | Primary buttons, focus rings |
| `warning` | Expiry badges ≤7d |
| `danger` | Expired, deny, delete confirm |

### 3.3 Typography scale

| Class | Use |
|---|---|
| `text-2xl font-semibold` | Page titles |
| `text-lg font-medium` | Section headers |
| `text-sm` | Table rows, metadata |
| `text-xs text-text-muted` | Timestamps, hints |
| `font-mono text-sm` | Bucket IDs, paths, env labels |

### 3.4 Spacing & radius

- Base unit: **4px** (Tailwind default)
- Card padding: `p-4` or `p-6`
- Border radius: `rounded-lg` (8px) cards, `rounded-md` inputs
- Sidebar width: **240px** fixed

### 3.5 Iconography by secret type

| Type | Icon | Badge color |
|---|---|---|
| `api_key` | `Key` | accent |
| `access_token` | `Ticket` | accent |
| `credential` | `Lock` | muted |
| `recovery_codes` | `Shield` | warning if &lt;3 left |
| `ssh_key` | `Terminal` | muted |
| `certificate` | `FileBadge` | warning near expiry |
| `connection_string` | `Database` | accent |
| `note` | `StickyNote` | muted |

---

## 4. Global Layout

### 4.1 App shell (authenticated)

```
┌────────────────────────────────────────────────────────────────┐
│ ┌──────────┐ ┌───────────────────────────────────────────────┐ │
│ │  👁 Argus │ │  [Approval Banner — conditional]              │ │
│ │          │ ├───────────────────────────────────────────────┤ │
│ │ Dashboard│ │                                               │ │
│ │  Vault   │ │     Main content (bento or list layouts)      │ │
│ │  Buckets │ │                                               │ │
│ │  Settings│ │                                               │ │
│ │          │ │                                               │ │
│ │ ──────── │ │                                               │ │
│ │ [avatar] │ │                                               │ │
│ │ username │ │                                               │ │
│ │ email    │ │                                               │ │
│ └──────────┘ └───────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### 4.2 Sidebar user profile (footer)

Pinned to bottom of sidebar (`components/layout/SidebarProfile.tsx`):

| Element | Source | UI |
|---|---|---|
| Avatar | `users.avatar_url` | 40×40 circle, `object-cover`, fallback initials from username |
| Username | `users.username` | `text-sm font-medium`, truncate |
| Email | `users.email` | `text-xs text-text-muted`, truncate |

- Avatar URL is set in **Settings → Profile** (optional on register; default generated initials).
- Clicking profile row opens Settings → Profile section (optional v1).

### 4.3 Sign out

- **Settings → Sign out** button (primary destructive ghost at bottom of settings page).
- Also optional: sidebar profile long-press menu → Sign out.
- Action: `invoke('sign_out')` → Rust `lock()` + clear frontend stores → navigate `/login`.
- Copy: “Sign out” (not “Log out of cloud”) — makes clear this is local session only.

### 4.3 Empty states

Every list screen needs an empty state with one primary CTA:

| Screen | Empty message | CTA |
|---|---|---|
| Vault | "No secrets yet" | Add Secret |
| Buckets | "No app buckets" | Create Bucket |
| Dashboard | "Welcome — add your first secret" | Go to Vault |
| Approvals tile | "No pending requests" | — |

---

## 5. Screen Specifications

### 5.1 Register — first run (`/register`)

**Wizard steps:**

| Step | Content |
|---|---|
| 1 — Account | Email, username, password, confirm |
| 2 — **Second factor (required)** | Choose **one:** Setup TOTP (QR + verify 6 digits) **or** Enable biometric (Windows Hello / Touch ID) |
| 3 — Done | “Account secured” → `/dashboard` |

**Rule:** Cannot finish register without completing step 2. Linux shows TOTP only (biometric disabled with explanation).

Optional avatar URL on step 1.

---

### 5.2 Login (`/login`)

**Two-step sign-in (Scope APP):**

| Step | UI |
|---|---|
| 1 | Email or username + password |
| 2a | 6-digit TOTP (if `second_factor_type = totp`) |
| 2b | “Use fingerprint / Windows Hello” button (if biometric) |

`invoke('sign_in', { identifier, password, totpCode? })` or biometric path via Rust.

Blocks all routes (including **Settings**) until complete.

---

### 5.3 Dashboard (`/dashboard`)

**Purpose:** Bento home — vault stats, buckets, expiring count, approvals, recent activity, quick actions.

Default route after sign-in. Full bento spec in [§10 Bento Layout System](#10-bento-layout-system).

---

### 5.4 Vault (`/vault`)

**Authorization:** Secrets require the app to be unlocked (**APP** / **VAULT** scope — equivalent). Idle **app lock** shows full-screen `AppLockModal` (TOTP or biometric). No separate vault elevation step or timer.

**Layout:** List + detail split (or list + slide-over panel).

```
┌──────────────────────────────────────────────────────────────────┐
│  Vault                                    [+ Add Secret]         │
│  🔍 Search secrets...                                            │
│  [All Types ▾] [All Orgs ▾] [Environment ▾] [Expiring] [Clear]  │
├───────────────────────────────┬──────────────────────────────────┤
│  Secret list (scroll)         │  Secret detail panel             │
│                               │                                  │
│  🔑 Supabase DB URL           │  Name, type, org, env, tags      │
│     Acme · prod               │  Description                     │
│                               │  ── Value (type-specific) ──     │
│  🎫 GitHub Token  ⚠ 6d       │  [masked] [Reveal] [Copy]        │
│                               │  Expires: 2025-12-31             │
│  🔐 Adobe Login               │  [Edit] [Archive] [Delete]       │
│                               │                                  │
└───────────────────────────────┴──────────────────────────────────┘
```

**List row content:**

- Type icon + name (truncate 40 chars)
- Org tag (pill)
- Environment pill (`prod` / `staging` / `dev`)
- Expiry badge if within 30 days

**Detail panel:**

- Value never auto-loaded — fetch `get_secret` on reveal click only
- Clear value from React state on panel close / sign out event

---

### 5.5 App Buckets (`/buckets`)

**Authorization:** Buckets follow app unlock (**APP** / **BUCKETS** — equivalent). Idle app lock applies to tray/IPC admin the same as vault.

**List view:** Cards with name, mapping count, active client grants, tray-active indicator.

**Detail view:**

```
┌─────────────────────────────────────────────────────────────────┐
│  ← Back    Acme Backend                    [Edit] [Delete]       │
│                                                                  │
│  Bucket ID                                                       │
│  ┌──────────────────────────────────────────────┐  [Copy]       │
│  │ 550e8400-e29b-41d4-a716-446655440000          │               │
│  └──────────────────────────────────────────────┘               │
│  Add to project .env:  ARGUS_BUCKET_ID=<id>                      │
│                                                                  │
│  Access TTL (this bucket)  [1 hour ▾]   ← overrides global default │
  Refresh TTL (optional)    [none ▾]                               │
  [✓] Active in system tray                                        │
  ARGUS_BUCKET_TOKEN (masked) ••••••••  [Reveal] [Regenerate]      │
│                                                                  │
│  ── Mappings ─────────────────────────────────────────────────  │
│  ENV LABEL              SECRET                                   │
│  DATABASE_URL      ←   [Supabase Database URL    ▾]  [✕]        │
│  STRIPE_SECRET     ←   [Stripe Live Key            ▾]  [✕]        │
│  [+ Add mapping]                                                 │
│                                                                  │
│  ── Active approvals for this bucket ──────────────────────────  │
│  uvicorn  ~/projects/acme-backend  expires 2h 14m    [Revoke]   │
└─────────────────────────────────────────────────────────────────┘
```

**Mapping row UX:**

- `env_label`: uppercase auto-transform, validate `[A-Z0-9_]+`
- Secret dropdown: searchable, excludes non-injectable types for hint (still allow mapping but show warning badge)

---

### 5.6 Settings (`/settings`)

**Layout:** Vertical sections in a single scroll, bento-style grouped cards.

| Section | Contents |
|---|---|
| **Profile** | Username, email (editable; save profile) |
| **Security** | Auto-lock after (select), lock on screen lock (toggle), active second factor (select when both TOTP and biometric registered) |
| **Authentication methods** | Set up / re-register TOTP (QR + code); set up / re-register biometric (Windows/macOS) |
| **Background** | Run in system tray when window closed |
| **Notifications** | Notify on new client access; secret expiry warning window (days) |
| **About** | Version, license |
| **Session** | **Sign out** (danger card) |

**Sign out flow:**

1. User clicks **Sign out**
2. Confirm dialog
3. `invoke('sign_out')` → zeroize keys → `/login`

**Not shown:** Access control TTLs, vault elevation, or `.env` fallback (library fallback is always enabled when Argus is stopped).

**Requires APP scope** (full sign-in) to open Settings — same as all pages.

---

## 8. Authorization & Elevation UX

### Scope indicator (optional header chip)

| Chip | Meaning |
|---|---|
| `App locked` | Idle timeout — `AppLockModal` (TOTP or biometric) |
| `App unlocked` | Dashboard, vault, settings available |

### `AppLockModal`

Full-screen overlay when idle app lock fires. TOTP or biometric only (password not required until sign-out or app restart). Vault secrets use the same lock — no separate vault modal.

---

## 9. Client Access Popup **(planned)**

When a **new** client (bucket + uri + token) requests secrets:

```
┌──────────────────────────────────────────────────────────────┐
│  🔐 New application request                                   │
│                                                              │
│  Bucket:     Acme Backend                                    │
│  URI:        file:///Users/dev/projects/acme-backend         │
│  Token:      ARGUS_BUCKET_TOKEN (masked)                       │
│  Process:    python (/usr/bin/python3)                       │
│                                                              │
│  Grant access for:  [15m] [1h] [3h] [8h]  (bucket default: 1h)│
│                                                              │
│  [Deny]                              [Allow access]          │
└──────────────────────────────────────────────────────────────┘
```

- Shown as **native notification** + in-app modal if window open
- Tray click on pending badge opens same modal
- Accept → grant until TTL from bucket `access_ttl_minutes` or global default
- Deny → no secrets; audit `CLIENT_DENIED`

Component: `ClientAccessDialog.tsx`. Event: `client-access-requested`.

---

## 10. System Tray

**Shipped:** tray icon, Open, Sign out; window close hides to tray.

**Planned:** pending-request badge, per-bucket submenu.

| Element | Behavior |
|---|---|
| Tray icon | Argus icon; **(planned)** badge if pending client requests |
| Left click | Open main window |
| Menu: Active buckets | Submenu — one line per `is_tray_active` bucket |
| Menu: Pending (N) | Opens client access queue |
| Menu: Sign out | Full sign-out |

**Close main window (X):** hides window, tray remains (if `run_in_background`).

---

## 11. Approval UX (legacy / CLI)

Process-path approvals (`process_path` + `working_dir`) still supported for CLI tools without `client_token`. Same banner pattern; converge UI copy to “Application request”.

---

## 12. Component Library

**Custom components only** — no shadcn/MUI. All live under `components/ui/` and `components/layout/`.

### 12.1 Primitives (`components/ui/`)

| Component | Props / notes |
|---|---|
| `Button` | `primary`, `secondary`, `ghost`, `danger`; sizes `sm` \| `md` |
| `Input` | `error`, `label`, `hint` |
| `PasswordInput` | reveal toggle, optional strength meter |
| `Badge` | `default`, `warning`, `danger`, `success` |
| `BentoCard` | `colSpan`, `rowSpan`, `title`, `action` slot — base bento cell |
| `BentoGrid` | CSS grid wrapper, `gap-4`, responsive breakpoints |
| `Avatar` | `src`, `fallback` initials, sizes `sm` \| `md` \| `lg` |
| `Modal` | focus trap; ESC closes non-destructive only |
| `Select` | searchable secret picker |
| `CopyButton` | toast + 30s clipboard clear |
| `ConfirmDialog` | destructive confirm |
| `StatTile` | number + label + trend (dashboard) |
| `EmptyState` | icon + message + CTA |

### 12.2 Layout (`components/layout/`)

| Component | Location |
|---|---|
| `AppShell` | Sidebar + main + approval banner slot |
| `Sidebar` | Nav + `SidebarProfile` footer |
| `SidebarProfile` | Avatar, username, email (from auth store) |
| `AuthLayout` | Centered card for `/login`, `/register` |

### 12.3 Domain components

| Component | Location |
|---|---|
| `SecretList` | `components/secrets/SecretList.tsx` |
| `SecretDetail` | `components/secrets/SecretDetail.tsx` |
| `SecretForm` | `components/secrets/SecretForm.tsx` |
| `TypeFields/*` | per-type form sections |
| `BucketCard` | bento-style bucket summary card |
| `BucketMappingTable` | `components/buckets/BucketMappingTable.tsx` |
| `ClientAccessDialog` | `components/clients/ClientAccessDialog.tsx` |
| `AppLockModal` | `components/app/AppLockModal.tsx` |
| `DashboardBento` | composes dashboard tiles |
| `RecentActivityList` | compact audit metadata (dashboard only) |

---

## 13. Forms & Secret Type UX

Use **React Hook Form** + Zod schemas mirroring Rust validation.

### 13.1 Common fields (all types)

| Field | Control | Validation |
|---|---|---|
| Name | text | required, max 128 |
| Description | textarea | max 500 |
| Organization | combobox + free text | max 64 |
| Environment | select | `prod` \| `staging` \| `dev` \| `all` |
| Tags | chip input | max 10 tags, 32 chars each |
| Expires | date | required for `access_token` |

### 13.2 Type-specific fields

See architecture Appendix A. Each type gets a `TypeFields/X.tsx` sub-form.

**Credential type notice (inline banner):**

> This secret cannot be injected into applications. Use copy-only in the UI.

**Recovery codes create flow:**

- Paste area: one code per line
- On save: parse → `{ code, used: false }[]`

**SSH key:**

- Private key textarea (masked)
- Public key auto-derived optional (future)
- Passphrase optional

---

## 14. Security-Sensitive UI Patterns

| Pattern | Rule |
|---|---|
| **Reveal** | Toggle local state; auto-hide after 60s idle |
| **Copy** | Show toast "Copied — clipboard clears in 30s"; invoke Rust clipboard clear |
| **Clipboard** | Never copy without explicit click |
| **Screen share** | Optional setting: blur values when window loses focus |
| **DevTools** | Discourage in release; CSP blocks external scripts |
| **Error messages** | No stack traces to user in release builds |
| **Logging** | `console.log` never prints secret values — use lint rule |

---

## 15. Bento Layout System

### 15.1 Grid definition

```css
/* globals.css or BentoGrid.module */
.bento-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-auto-rows: minmax(120px, auto);
  gap: 1rem;
}
```

| Breakpoint | Columns | Notes |
|---|---|---|
| &lt; 768px | 2 | Stack dashboard on small Tauri windows |
| 768–1024px | 3 | — |
| ≥ 1024px | 4 | Default |

### 15.2 Span tokens

| Token | `grid-column` / `grid-row` | Use |
|---|---|---|
| `span-1` | `span 1` | Stat tiles |
| `span-2` | `span 2` | Wide summaries |
| `span-2x2` | `span 2` / `span 2` | Recent activity |

### 15.3 Visual rules

- Every bento cell: `rounded-xl`, `border border-border`, `bg-surface`, `p-4`
- Hover: subtle `border-accent/30` on clickable cells
- No heavy shadows — flat security-tool aesthetic
- Title row: `text-sm font-medium text-text-muted` uppercase tracking-wide

### 15.4 Page usage

| Page | Bento usage |
|---|---|
| Dashboard | Full `BentoGrid` |
| Buckets | List of `BucketCard` in 2-column grid |
| Vault | List + detail (not bento); optional bento on empty state |
| Settings | Grouped `BentoCard` sections (Profile, Security, …) |
| Login / Register | Single centered card (not bento) |

---

## 16. Boneyard Skeletons

**Package:** `boneyard-js` (installed). Pixel-perfect skeletons extracted from real UI.

### 16.1 Vite setup

```ts
// vite.config.ts
import { boneyardPlugin } from 'boneyard-js/vite'

export default defineConfig({
  plugins: [react(), tailwindcss(), boneyardPlugin()],
})
```

```ts
// main.tsx — import once
import './bones/registry'
```

```json
// boneyard.config.json
{
  "breakpoints": [900, 1200],
  "out": "./src/bones",
  "wait": 800,
  "color": "rgba(255,255,255,0.06)",
  "animate": "shimmer"
}
```

### 16.2 Skeleton names (register per page)

| `name` | Page / component |
|---|---|
| `dashboard-vault-summary` | Dashboard vault tile |
| `dashboard-buckets` | Dashboard buckets tile |
| `dashboard-expiring` | Dashboard expiring tile |
| `dashboard-approvals` | Dashboard approvals tile |
| `dashboard-recent-activity` | Dashboard activity list |
| `vault-list` | Vault secret list |
| `vault-detail` | Vault detail panel |
| `bucket-list` | Buckets grid |
| `bucket-detail` | Bucket detail + mappings |
| `settings-profile` | Settings profile card |

### 16.3 Usage pattern

```tsx
import { Skeleton } from 'boneyard-js/react'

function DashboardVaultSummary({ loading, data }) {
  return (
    <Skeleton name="dashboard-vault-summary" loading={loading} animate="shimmer">
      <StatTile value={data.total} label="Secrets" />
    </Skeleton>
  )
}
```

**Dev workflow:**

```bash
pnpm dev   # boneyardPlugin captures on HMR
# Or: npx boneyard-js build --watch
```

**Rules:**

- Wrap the **same DOM structure** as loaded content (use `fixture` prop in dev for empty states).
- `loading={true}` while `invoke()` in flight; never skeleton secrets values.
- Dark mode: pass `darkColor` matching `--color-surface-raised`.

---

## 17. Responsive & Platform Notes

| Constraint | Approach |
|---|---|
| Min window size | 900×600 (set in `tauri.conf.json`) |
| Narrow width | Collapse detail panel to full-screen modal |
| macOS | Native traffic lights; sidebar respects titlebar |
| Windows | WebView2 — test font rendering (Inter loaded locally, not Google CDN) |
| Linux | WebKitGTK — test dark theme |

**Fonts:** Bundle Inter and JetBrains Mono in `public/fonts/` — avoid CDN for CSP and offline use.

---

## 18. Accessibility

| Requirement | Implementation |
|---|---|
| Keyboard nav | Sidebar `roving tabindex` |
| Focus visible | `ring-2 ring-accent` on interactive elements |
| ARIA | `aria-live="polite"` on approval banner |
| Color contrast | WCAG AA minimum on `text` vs `surface` |
| Screen readers | Announce sign-in success, approval result |

---

## 19. Frontend File Map

Target structure (align with [plan.md](./plan.md) Milestone 2):

```
src/
├── app.tsx
├── main.tsx
├── styles/globals.css
├── bones/                  # boneyard output (registry.ts, *.bones.json)
├── boneyard.config.json
├── pages/
│   ├── login.tsx
│   ├── register.tsx
│   ├── dashboard.tsx
│   ├── vault.tsx
│   ├── buckets.tsx
│   └── settings.tsx
├── components/
│   ├── ui/                 # Button, BentoCard, BentoGrid, Avatar, …
│   ├── layout/             # AppShell, Sidebar, SidebarProfile
│   ├── dashboard/
│   ├── secrets/
│   ├── buckets/
│   └── approvals/
├── state/
│   ├── auth.store.ts       # user profile: email, username, avatarUrl
│   └── vault.store.ts
├── hooks/
├── lib/tauri-bridge.ts
└── types/
```

---

## Appendix — Wireframe index

| Screen | Section |
|---|---|
| Register | §5.1 |
| Login | §5.2 |
| Dashboard (bento) | §5.3, §10 |
| Vault | §5.4 |
| Bucket detail | §5.5 |
| Settings + Sign out | §5.6 |
| Sidebar profile | §4.2 |
| Approval banner | §8.1 |
| Boneyard | §11 |

---

*Design for clarity under pressure — approvals happen when the developer is mid-flow.*
