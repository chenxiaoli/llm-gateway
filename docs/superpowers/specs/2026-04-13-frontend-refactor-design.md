# Frontend Refactor: Ant Design to Tailwind + Zustand

**Date:** 2026-04-13
**Status:** Approved

## Overview

Replace Ant Design (antd) with Tailwind CSS, Zustand, and custom UI components. Refresh the visual design to an ultra-clean minimal aesthetic inspired by Linear/Vercel/Raycast. Remove `antd`, `@ant-design/icons`, and `dayjs`. Add `tailwindcss`, `zustand`, `lucide-react`, `@tanstack/react-table`, `react-hook-form`, `zod`, and `sonner`.

## Scope

All 13 pages, 3 components, 8 hooks, 1 context, and 2 entry files in `web/src/`. This is a big-bang replacement — Ant Design is removed entirely in one pass.

## Tech Stack Changes

### Removing
- `antd` (v5.22.0)
- `@ant-design/icons` (v5.5.0)
- `dayjs` (v1.11.0)

### Adding
- `tailwindcss` + `@tailwindcss/vite` — utility-first CSS
- `zustand` — global state (auth only)
- `lucide-react` — icon library (tree-shakeable)
- `@tanstack/react-table` — headless table (replaces Ant Table)
- `react-hook-form` — form handling
- `@hookform/resolvers` + `zod` — form validation
- `sonner` — toast notifications (replaces `antd message`)

### Keeping
- `react-router-dom` — already in place
- `@tanstack/react-query` — server state
- `axios` — HTTP client
- `recharts` — charts

## Design System

### Visual Direction
Ultra-clean minimal. True black backgrounds, subtle borders, ghost buttons, generous whitespace. Inspired by Linear, Vercel, Raycast.

### Color Palette
```
Background:    #000000 / #0a0a0a (true dark)
Surface:       #111111 / #141414 (barely visible elevation)
Borders:       #1e1e1e / #262626 (very subtle)
Text primary:  #ededed
Text secondary:#888888
Text tertiary: #555555
Accent:        #06d6a0 (green)
Status:        green (active), amber (warning), red (error), neutral (disabled)
```

### Typography
- Font: `Outfit`, sans-serif (keep existing)
- Tight letter spacing on headings
- Large bold page titles (text-2xl/text-3xl)
- Small muted labels and metadata (text-xs, text-zinc-500)

### Component Patterns
- **Buttons**: ghost-style by default (transparent bg, subtle border), filled only for primary actions
- **Tables**: no card wrapper, no heavy header bg — clean rows with subtle dividers
- **Forms**: minimal — labels + inputs, no card wrappers
- **Modals**: centered, small padding, backdrop with subtle blur
- **Tags/badges**: tiny, no background fill — colored text or 1px border
- **Page header**: title + optional action button, right-aligned
- **Sidebar**: slim, icon-first with text labels, muted colors

## Component Migration Map

| Ant Design | Replacement |
|---|---|
| `ConfigProvider` + theme | Tailwind config + `globals.css` |
| `Table` | `@tanstack/react-table` + custom UI |
| `Form` + `Input` | `react-hook-form` + Zod + native inputs |
| `Button` | Custom `<Button>` (primary/secondary/ghost/sizes) |
| `Modal` | Custom `<Modal>` (overlay + backdrop blur) |
| `Card` | Plain `<div>` with Tailwind classes |
| `Tag` | Custom `<Badge>` (colored text/border pill) |
| `Spin` | Custom `<LoadingSpinner>` (SVG) |
| `message` (toasts) | `sonner` |
| `Switch` | Custom `<Toggle>` (checkbox-based) |
| `Typography` | Plain HTML + Tailwind |
| `Space` | Tailwind `flex gap-*` |
| `Row`/`Col` | Tailwind `grid`/`flex` |
| `Select` | Custom `<Select>` |
| `DatePicker` | Native `<input type="date">` |
| `Drawer` | Custom `<Drawer>` (slide-in panel) |
| `Popconfirm` | Custom `<ConfirmDialog>` |
| `InputNumber` | Native `<input type="number">` + react-hook-form |
| `Alert` | Custom `<Alert>` (info/warning/error) |
| `Statistic` | Plain text + Tailwind |
| `Divider` | `<hr>` + Tailwind |
| `@ant-design/icons` | `lucide-react` |

### Custom Components to Build
Each ~20-50 lines. Minimal abstraction.

- `Button` — primary/secondary/ghost, sm/md/lg, icon-only support
- `Modal` — overlay + content + close button, backdrop blur
- `Badge` — status indicator, color variants
- `Toggle` — switch component
- `Select` — dropdown select with search
- `Drawer` — slide-in panel from right
- `LoadingSpinner` — simple SVG animation
- `Alert` — info/warning/error variants
- `ConfirmDialog` — small popover with confirm/cancel

## State Architecture

### Zustand Store (replaces AuthContext)

Single store for auth. Only auth needs global client-side state.

```typescript
// stores/authStore.ts
interface AuthState {
  user: User | null;
  isLoading: boolean;
  login: (input: LoginRequest) => Promise<void>;
  register: (input: RegisterRequest) => Promise<void>;
  logout: () => void;
  setUser: (user: User) => void;
}
```

The store integrates with React Query — `login()` calls the API, stores the token, then refetches the `me` query. Server data stays in React Query.

### State Layers
- **Zustand**: global client state (auth only)
- **React Query**: server state (keys, providers, logs, usage)
- **React useState**: local UI state (form inputs, modal open/close, selected rows)

No additional stores needed. Add Zustand stores only when state crosses page boundaries.

## Project Structure

```
web/src/
├── api/              # Unchanged — API client + endpoints
├── components/
│   ├── ui/           # NEW — shared UI primitives
│   │   ├── Button.tsx
│   │   ├── Modal.tsx
│   │   ├── Badge.tsx
│   │   ├── Toggle.tsx
│   │   ├── Select.tsx
│   │   ├── Drawer.tsx
│   │   ├── LoadingSpinner.tsx
│   │   ├── Alert.tsx
│   │   └── ConfirmDialog.tsx
│   ├── Layout.tsx    # Rewritten — Tailwind sidebar
│   └── JsonViewer.tsx # Kept as-is
├── hooks/            # Unchanged — React Query data hooks
├── stores/           # NEW — replaces contexts/
│   └── authStore.ts
├── pages/            # All rewritten with Tailwind
│   ├── Home.tsx
│   ├── Login.tsx
│   ├── Register.tsx
│   ├── Dashboard.tsx
│   ├── Keys.tsx
│   ├── KeyDetail.tsx
│   ├── Providers.tsx
│   ├── ProviderDetail.tsx
│   ├── Users.tsx
│   ├── Settings.tsx
│   ├── Usage.tsx
│   └── Logs.tsx
├── lib/              # NEW — utilities
│   └── cn.ts         # clsx + twMerge helper
├── test/             # Updated — remove Ant wrappers
│   └── render.tsx
├── types/            # Unchanged
├── styles/
│   └── global.css    # Rewritten — Tailwind directives + base styles
├── App.tsx           # Updated — remove Ant, use Zustand
└── main.tsx          # Updated — remove ConfigProvider
```

### Deleted
- `contexts/AuthContext.tsx` (replaced by `stores/authStore.ts`)
- `components/StatCard.tsx` (replaced by inline Tailwind in Dashboard)

## Migration Order

1. **Setup** — Install deps, configure Tailwind, create `tailwind.config.ts`, set up `cn.ts`
2. **UI primitives** — Build all custom components in `components/ui/`
3. **Zustand store** — Create `authStore.ts`, wire into `main.tsx`/`App.tsx`
4. **Layout + Sidebar** — Rewrite `Layout.tsx` with Tailwind
5. **Auth pages** — Login, Register
6. **Dashboard** — Tests table pattern
7. **Data pages** — Keys, KeyDetail, Providers, ProviderDetail
8. **Admin pages** — Users, Settings, Usage, Logs
9. **Home page** — Landing/redirect
10. **Cleanup** — Remove Ant deps, update tests, verify E2E

Run `npm run build` and `npm test` at each step to catch errors early.

## Files Changed

| File | Action |
|---|---|
| `web/package.json` | Update deps |
| `web/tailwind.config.ts` | NEW |
| `web/postcss.config.js` | NEW (if needed) |
| `web/vite.config.ts` | Add Tailwind plugin |
| `web/src/styles/global.css` | Rewrite |
| `web/src/main.tsx` | Remove ConfigProvider, add Sonner Toaster |
| `web/src/App.tsx` | Remove Ant Spin, use Zustand auth |
| `web/src/lib/cn.ts` | NEW |
| `web/src/stores/authStore.ts` | NEW (replaces contexts/AuthContext.tsx) |
| `web/src/contexts/AuthContext.tsx` | DELETE |
| `web/src/components/StatCard.tsx` | DELETE |
| `web/src/components/Layout.tsx` | Rewrite |
| `web/src/components/ui/*.tsx` | NEW (9 files) |
| `web/src/pages/*.tsx` | Rewrite (13 files) |
| `web/src/hooks/*.ts` | Remove `antd message`, use `sonner toast` |
| `web/src/test/render.tsx` | Update providers |
| `web/src/pages/*.test.tsx` | Update selectors and mocks |
