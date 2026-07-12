# Org Create Route — Design

## Problem

A logged-in user who clicks "Create org" in the OrgSwitcher dropdown (`web/src/components/OrgSwitcher.tsx:72`) lands on `/` (Home) because `navigate('/orgs/new')` points at a route that does not exist in `web/src/App.tsx`. The only working org-creation UI today is the onboarding screen, which is intended for limbo users (zero org memberships) and renders the create form alongside the join-invite form.

The backend is fully implemented (`POST /api/v1/orgs` → `crates/api/src/auth.rs:1336`, with tests). The gap is purely a missing frontend route plus a page to host the existing form.

## Goal

Make the OrgSwitcher "Create org" button work for already-logged-in users by adding a dedicated `/orgs/new` route that hosts the org-creation form.

## Non-Goals

- Renaming `OnboardingCreateCard` to `OrgCreateCard` — the existing name is fine; the card's copy is generic.
- Splitting `onboarding.create.*` i18n keys into `orgCreate.*` — the copy reads "Create an org / 1 minute form" which fits both contexts.
- Touching the `/onboarding` flow for limbo users.
- Adding new error paths. The card already handles 409 (slug taken) inline and other errors via toast.
- Changing post-creation behavior. The card already calls `applyAuthResponse` (which sets `currentOrg` to the new org) and navigates to `/{newSlug}/dashboard`.

## Architecture

### Route placement

`/orgs/new` lives **outside** the `/:orgSlug` subtree in `App.tsx`, mirroring how `/onboarding` is mounted. Two reasons:

1. The URL has no org slug (the user is creating one).
2. Putting it under `/:orgSlug` would make the sidebar's nav items point at the *old* org's resources during creation, which is misleading.

The page does its own auth gate inline (same pattern as `/onboarding`'s `OnboardingGate`), so it does not need to be wrapped in `RequireAuth` at the router level.

### Component structure

```
OrgCreate (page)                  ← new file: web/src/pages/OrgCreate.tsx
└── OnboardingCreateCard          ← existing component, used unchanged
    └── posts to /orgs, calls applyAuthResponse, navigates to /{slug}/dashboard
```

The page is a thin shell: auth gate (loading → spinner, no token → `/login`, else render), then a centered card layout matching `/onboarding`'s visual style. No new business logic — the card already does everything.

### Why reuse OnboardingCreateCard unchanged

The card's i18n keys (`onboarding.create.*`) are generic: title "Create an org", subtitle "1 minute form", field labels "Org name" / "Slug", submit "Create", error "That slug is taken, try another". None of these mention onboarding specifically. Reusing the component avoids forking copy and form behavior; if we later want different copy for the logged-in context, we can extract a shared `OrgCreateCard` and feed it different i18n keys at that point.

## Files

**Create:**

- `web/src/pages/OrgCreate.tsx` — thin page wrapper.
- `web/src/pages/OrgCreate.test.tsx` — three tests (see Testing).

**Modify:**

- `web/src/App.tsx` — add `<Route path="/orgs/new" element={<OrgCreate />} />` next to `/onboarding`.

**No changes:**

- `web/src/components/OrgSwitcher.tsx` — the existing `navigate('/orgs/new')` call becomes correct once the route exists.
- `web/src/components/OnboardingCreateCard.tsx` — used unchanged.

## Implementation Sketch

### `web/src/pages/OrgCreate.tsx`

```tsx
import { Navigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { useAuthGate } from '../stores/authStore';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { OnboardingCreateCard } from '../components/OnboardingCreateCard';

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Inline auth gate — same pattern as /onboarding's OnboardingGate. The page
 * lives outside the org-scoped subtree (no org slug in the URL), so it can't
 * rely on the shared RequireAuth wrapper.
 */
function OrgCreateGate({ children }: { children: React.ReactNode }) {
  const status = useAuthGate();
  if (status === 'loading') {
    return (
      <div className="flex h-screen items-center justify-center">
        <LoadingSpinner size="lg" />
      </div>
    );
  }
  if (status === 'login') {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

export default function OrgCreate() {
  const { t } = useTranslation();
  return (
    <OrgCreateGate>
      <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="max-w-md w-full"
        >
          <h1 className="text-2xl font-semibold mb-1">{t('orgCreate.title')}</h1>
          <p className="text-sm text-base-content/50 mb-6">{t('orgCreate.subtitle')}</p>
          <OnboardingCreateCard />
        </motion.div>
      </div>
    </OrgCreateGate>
  );
}
```

**Wait — i18n keys.** I said the page header would reuse `onboarding.create.title` ("Create an org") and `onboarding.create.subtitle` ("1 minute form"). That works for the card-internal copy but the page-level header copy needs its own keys to avoid coupling the page text to the onboarding section. Two options:

- **Option A (chosen):** Add new i18n keys `orgCreate.title` = "Create a new org" and `orgCreate.subtitle` = "You'll switch to the new org after creating it." in both `en.json` and `zh.json`. Decouples page-level copy from the onboarding section.
- **Option B:** Reuse `onboarding.title` / `onboarding.subtitle` — but those say "Set up your workspace / Create a new org or join one you've been invited to", and the join half is irrelevant here.

Going with Option A — small additive change, cleaner copy.

### `web/src/App.tsx` route addition

Next to the existing `/onboarding` route:

```tsx
<Route path="/onboarding" element={<Onboarding />} />
<Route path="/orgs/new" element={<OrgCreate />} />
```

And the matching import at the top:

```tsx
import OrgCreate from './pages/OrgCreate';
```

### i18n additions

`web/src/i18n/en.json` — add a new top-level section (placement: alongside `onboarding`):

```json
"orgCreate": {
  "title": "Create a new org",
  "subtitle": "You'll switch to the new org after creating it."
}
```

`web/src/i18n/zh.json`:

```json
"orgCreate": {
  "title": "创建新组织",
  "subtitle": "创建后会自动切换到新组织。"
}
```

## Testing

### `web/src/pages/OrgCreate.test.tsx` — three tests

Reuse the existing `renderWithProviders` helper and `server` mock setup from `web/src/test/`.

1. **Authenticated user sees the create form** — seed auth store with a logged-in user; render `<OrgCreate />`; assert `screen.getByText('Create a new org')` (page title) and `screen.getByLabelText(/org name/i)` or `screen.getByPlaceholderText('e.g., Acme Inc.')` are visible.

2. **Unauthenticated → /login** — mock `useAuthGate` returning `'login'`; render; assert `Navigate` was called with `/login`. (Pattern: assert the location ends at `/login` via the test router.)

3. **Successful submit navigates to new org's dashboard** — MSW `http.post('*/api/v1/orgs', ...)` returns an `AuthResponse` with a new org (`slug: 'newco'`); fill name + slug; click Create; assert `useNavigate` was called with `/newco/dashboard` and `currentOrg.slug === 'newco'` in the auth store.

### Test infrastructure note

`web/src/test/server.ts` already has handlers for `*/api/v1/auth/config`, `*/api/v1/auth/me`, `*/api/v1/auth/login`, etc. — the existing handlers cover the auth bootstrap. Test 3 will need its own per-test MSW override for `POST /orgs` (the default handler doesn't exist because the endpoint is only hit during create). Pattern is `server.use(http.post('*/api/v1/orgs', () => HttpResponse.json({...})))` inside the test, before rendering.

## Verification

After implementation:

1. **TypeScript check** — `source ~/.nvm/nvm.sh && cd web && npm run build` passes with no type errors.
2. **Unit tests** — `npm test -- --run src/pages/OrgCreate.test.tsx` passes 3/3.
3. **Full suite regression** — `npm test -- --run` still passes (was 35 files / 188 tests; will be 36 / 191 after).
4. **Manual smoke test**:
   - Log in as a user with at least one existing org.
   - Click the OrgSwitcher in the sidebar → "Create org".
   - Confirm the page renders at `/orgs/new` with the create form.
   - Submit with a unique slug → confirm redirect to `/{newSlug}/dashboard` and the sidebar's OrgSwitcher now shows the new org.
   - Submit with a duplicate slug → confirm inline error "That slug is taken, try another".
   - Log out, navigate directly to `/orgs/new` → confirm redirect to `/login`.

## Risks / Edge Cases

- **Stale `currentOrg`**: until `applyAuthResponse` runs, the auth store still has the old org. If the user has the OrgSwitcher open in another tab, that tab won't update — but that's expected single-tab behavior.
- **Direct URL access by limbo user**: a limbo user (zero orgs) hitting `/orgs/new` would see the same form as on `/onboarding` but without the join-invite option. Not broken, just slightly worse UX than the full onboarding screen. The `OnboardingRedirect` component already redirects limbo users to `/onboarding` for any non-allowed path, so they'd be bounced to `/onboarding` before reaching `/orgs/new`. **Verify this is the case** during implementation; if not, add `/orgs/new` to `OnboardingRedirect`'s allowed-paths list.
- **Browser back button**: after success, the user lands at `/{newSlug}/dashboard`. Browser-back would return them to `/orgs/new`, which would then redirect (via auth gate) — but they're now logged in with the new org, so they'd see the create form again. Acceptable; matches the onboarding flow's behavior.

## Out of Scope

- Rename `OnboardingCreateCard` → `OrgCreateCard`.
- Migrate `onboarding.create.*` i18n keys to `orgCreate.*`.
- Modal-in-OrgSwitcher variant.
- Changing the post-creation navigation target.
- Adding org templates / presets to the create form.
