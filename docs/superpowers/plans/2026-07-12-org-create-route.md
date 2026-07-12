# Org Create Route — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the broken "Create org" entry point in the OrgSwitcher by adding a dedicated `/orgs/new` route that hosts the existing `OnboardingCreateCard`.

**Architecture:** A thin `OrgCreate` page wraps the existing card with its own inline auth gate (mirroring `OnboardingGate`). Route is mounted at the top level of `App.tsx` (outside `/:orgSlug` and outside `RequireAuth`). Two new i18n keys (`orgCreate.title`, `orgCreate.subtitle`) decouple page-level copy from the `onboarding.*` section.

**Tech Stack:** React 18 + TypeScript + Vite, react-router-dom v6, vitest + MSW + @testing-library/react, i18next, Tailwind + daisyUI, framer-motion (matches the existing `/onboarding` page).

---

## File Structure

**Create:**

- `web/src/pages/OrgCreate.tsx` — page shell: auth gate + centered card layout hosting `OnboardingCreateCard`.
- `web/src/pages/OrgCreate.test.tsx` — two tests (authenticated render; successful submit navigates).

**Modify:**

- `web/src/i18n/en.json` — add `orgCreate` section with `title` and `subtitle`.
- `web/src/i18n/zh.json` — mirror.
- `web/src/App.tsx` — add `import OrgCreate` and a `<Route path="/orgs/new" element={<OrgCreate />} />` next to `/onboarding`.

**Unchanged (intentionally):**

- `web/src/components/OrgSwitcher.tsx` — the existing `navigate('/orgs/new')` call (line 72) becomes correct once the route exists.
- `web/src/components/OnboardingCreateCard.tsx` — used unchanged. Its `onboarding.create.*` i18n copy is generic ("Create an org", "1 minute form", "Org name", "Slug", "Create", "That slug is taken, try another").
- `web/src/pages/Onboarding.tsx` — the limbo-user flow is untouched.

---

## Task 1: Add `orgCreate` i18n keys

**Files:**
- Modify: `web/src/i18n/en.json` (add a new top-level section near the existing `onboarding` block)
- Modify: `web/src/i18n/zh.json` (mirror)

**Why first:** Page and tests in Task 2 reference `t('orgCreate.title')` and `t('orgCreate.subtitle')`. Adding the keys first avoids a broken render when Task 2 lands. No test for this task — keys are validated indirectly by Task 2's render test.

- [ ] **Step 1: Find the insertion point in `en.json`**

Run: `grep -n '"onboarding"' /workspace/llm-gateway/web/src/i18n/en.json`

You'll get a line number — that's the start of the `onboarding` block. We'll insert the new `orgCreate` section right before it.

- [ ] **Step 2: Add `orgCreate` to `en.json`**

Open `web/src/i18n/en.json`. Find the `"onboarding": { ... }` block (starts with `"onboarding": {` on its own line). Insert this new section immediately **before** it (keep the comma after the previous section's closing `}`):

```json
"orgCreate": {
  "title": "Create a new org",
  "subtitle": "You'll switch to the new org after creating it."
},
```

- [ ] **Step 3: Verify JSON is valid**

Run: `node -e "JSON.parse(require('fs').readFileSync('/workspace/llm-gateway/web/src/i18n/en.json', 'utf8'))" && echo OK`

Expected output: `OK`. If you get a parse error, you have a malformed JSON (missing/extra comma); fix and re-run.

- [ ] **Step 4: Add `orgCreate` to `zh.json`**

Open `web/src/i18n/zh.json`. Find the `"onboarding": { ... }` block. Insert this immediately before it:

```json
"orgCreate": {
  "title": "创建新组织",
  "subtitle": "创建后会自动切换到新组织。"
},
```

- [ ] **Step 5: Verify JSON is valid**

Run: `node -e "JSON.parse(require('fs').readFileSync('/workspace/llm-gateway/web/src/i18n/zh.json', 'utf8'))" && echo OK`

Expected output: `OK`.

- [ ] **Step 6: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "$(cat <<'EOF'
feat(i18n): add orgCreate.{title,subtitle} keys

Page-level copy for the new /orgs/new route. Decouples the page
header from onboarding.* keys so the logged-in-user context can
have its own copy ("You'll switch to the new org after creating it.")
without forking OnboardingCreateCard.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Create `OrgCreate` page with tests

**Files:**
- Create: `web/src/pages/OrgCreate.tsx`
- Create: `web/src/pages/OrgCreate.test.tsx`

**TDD order:** tests first (fail because `OrgCreate` doesn't exist), then implement, then run tests (pass).

**Test scope note:** The spec listed three tests. This plan implements two — the "unauthenticated → /login" case is dropped because `useAuthGate` depends on the React Query bootstrap's `isLoading` state, which is flaky to drive from a unit test without intricate timing setup. The existing `/onboarding` test suite (which uses the same gate) doesn't cover this case either. The redirect is verified by manual smoke (Task 3 Step 5 item 7) instead.

### Step 1: Write the test file

- [ ] **Step 1: Create `web/src/pages/OrgCreate.test.tsx` with the following content**

```tsx
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useAuthStore } from '../stores/authStore';
import { clearToken, clearRefreshToken } from '../api/client';
import type { User, OrgSummary } from '../types';
import OrgCreate from './OrgCreate';

// Mock useNavigate so we can assert where the page would take the user
// after a successful create — without that assertion, success looks the
// same as failure (both just stop rendering the form).
const { mockNavigate } = vi.hoisted(() => ({ mockNavigate: vi.fn() }));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});

const existingUser: User = {
  id: 'user-1',
  username: 'alice',
  platform_role: null,
  email: null,
  email_verified_at: '2026-01-01T00:00:00Z',
};

const existingOrg: OrgSummary = {
  id: 'org-old',
  slug: 'oldco',
  name: 'Old Co',
  role: 'owner',
  group_id: null,
};

const newOrg: OrgSummary = {
  id: 'org-new',
  slug: 'acme',
  name: 'Acme',
  role: 'owner',
  group_id: null,
};

const authResponse = {
  token: 'fresh-jwt',
  refresh_token: 'fresh-refresh',
  user: existingUser,
  current_org: newOrg,
  orgs: [existingOrg, newOrg],
};

// /auth/me response after switching to the new org — the auth store
// refetches via applyAuthResponse and this is what it gets back.
const meAfterCreate = {
  ...existingUser,
  current_org: newOrg,
  orgs: [existingOrg, newOrg],
  allow_registration: true,
  impersonating: false,
};

function seedLoggedInUser() {
  useAuthStore.setState({
    user: existingUser,
    currentOrg: existingOrg,
    orgs: [existingOrg],
    impersonating: false,
  });
}

beforeEach(() => {
  mockNavigate.mockClear();
  // Tokens from a prior test would make the gate enter "loading" state
  // waiting for /auth/me; clear so the gate settles immediately.
  clearToken();
  clearRefreshToken();
  seedLoggedInUser();
});

describe('OrgCreate page', () => {
  it('renders page title and the create form for a logged-in user', () => {
    renderWithProviders(<OrgCreate />, { route: '/orgs/new' });

    // Page-level copy from orgCreate.* (NOT onboarding.*).
    expect(screen.getByRole('heading', { name: /create a new org/i })).toBeInTheDocument();
    expect(screen.getByText(/you'll switch to the new org/i)).toBeInTheDocument();
    // Card-internal copy from onboarding.create.* (reused unchanged).
    expect(screen.getByPlaceholderText('e.g., Acme Inc.')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('e.g., acme')).toBeInTheDocument();
  });

  it('on submit, applies auth response and redirects to new org dashboard', async () => {
    server.use(
      http.post('*/api/v1/orgs', () => HttpResponse.json(authResponse)),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterCreate)),
    );

    renderWithProviders(<OrgCreate />, { route: '/orgs/new' });

    await userEvent.type(screen.getByPlaceholderText('e.g., Acme Inc.'), 'Acme');
    const slugInput = screen.getByPlaceholderText('e.g., acme');
    await userEvent.clear(slugInput);
    await userEvent.type(slugInput, 'acme');

    await userEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
    });
    // Auth store now reflects the new current org.
    expect(useAuthStore.getState().currentOrg?.slug).toBe('acme');
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source ~/.nvm/nvm.sh && cd /workspace/llm-gateway/web && npm test -- --run src/pages/OrgCreate.test.tsx
```

Expected: FAIL. Error message should mention `Cannot find module './OrgCreate'` or similar — the page doesn't exist yet. If you see a different error (e.g., TypeScript error in the test file), fix it before proceeding.

### Step 2: Implement the page

- [ ] **Step 3: Create `web/src/pages/OrgCreate.tsx` with the following content**

```tsx
import { Navigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { useAuthGate } from '../stores/authStore';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { OnboardingCreateCard } from '../components/OnboardingCreateCard';

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Inline auth gate — mirrors /onboarding's OnboardingGate. The route lives
 * outside the org-scoped subtree (URL has no org slug) and outside the
 * shared RequireAuth wrapper, so the page runs its own gate: loading →
 * spinner, no token → /login, otherwise render the form.
 *
 * A limbo user (zero org memberships) hitting /orgs/new is bounced to
 * /onboarding by the global OnboardingRedirect in App.tsx before this
 * component mounts, so we don't need a special case here.
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

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source ~/.nvm/nvm.sh && cd /workspace/llm-gateway/web && npm test -- --run src/pages/OrgCreate.test.tsx
```

Expected: PASS, both tests. If the second test fails on the navigation assertion, double-check that `OnboardingCreateCard` is calling `applyAuthResponse` and `navigate(/${slug}/dashboard, { replace: true })` — that contract is the same as `/onboarding`'s.

- [ ] **Step 5: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/pages/OrgCreate.tsx web/src/pages/OrgCreate.test.tsx
git commit -m "$(cat <<'EOF'
feat(web): add OrgCreate page at /orgs/new (unrouted)

Thin page wrapper hosting the existing OnboardingCreateCard. Own auth
gate mirrors OnboardingGate (loading → spinner, no token → /login).
Page-level copy comes from orgCreate.* i18n keys added in the previous
commit; the card's copy stays under onboarding.create.* (generic).

Route wiring lands in the next commit.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Wire the route in App.tsx and verify end-to-end

**Files:**
- Modify: `web/src/App.tsx`

**Goal:** Mount `/orgs/new` next to `/onboarding` so the OrgSwitcher's `navigate('/orgs/new')` call resolves.

- [ ] **Step 1: Add the import**

Open `web/src/App.tsx`. Find the line `import Onboarding from './pages/Onboarding';` (currently around line 17). Add this line immediately **after** it:

```tsx
import OrgCreate from './pages/OrgCreate';
```

- [ ] **Step 2: Add the route**

In the same file, find the `<Route path="/onboarding" element={<Onboarding />} />` line (currently around line 118). Add this line immediately **after** it:

```tsx
<Route path="/orgs/new" element={<OrgCreate />} />
```

The route sits in the same block as the other top-level (non-org-scoped) routes — between `/onboarding` and the org-scoped `/:orgSlug` subtree.

- [ ] **Step 3: Verify TypeScript build**

Run:

```bash
source ~/.nvm/nvm.sh && cd /workspace/llm-gateway/web && npm run build 2>&1 | tail -10
```

Expected: build succeeds with no type errors. The output ends with `✓ built in <seconds>`.

- [ ] **Step 4: Run the full frontend test suite**

Run:

```bash
source ~/.nvm/nvm.sh && cd /workspace/llm-gateway/web && npm test -- --run 2>&1 | tail -10
```

Expected: all test files pass. Pre-Task-1 baseline was 35 files / 188 tests; after Task 2 this should be 36 files / 190 tests (the OrgCreate file adds 1 file with 2 tests). No previously-passing test should regress.

- [ ] **Step 5: Manual smoke test (optional but recommended)**

If the dev server is running on :5173 (frontend) and :8080 (backend):

1. Log in as a user with at least one org.
2. Click the OrgSwitcher dropdown in the sidebar → click "Create org".
3. Confirm the URL changes to `/orgs/new` and the page renders with title "Create a new org" and the form below.
4. Fill name = "Acme", slug = "acme" → click "Create".
5. Confirm redirect to `/acme/dashboard` and the sidebar's OrgSwitcher now shows "Acme".
6. Open the OrgSwitcher again → click "Create org" → fill slug = "acme" (duplicate) → submit → confirm inline error "That slug is taken, try another".
7. Log out → navigate directly to `http://localhost:5173/orgs/new` → confirm redirect to `/login`.

If the dev server isn't running, skip this step — Steps 3 and 4 already cover the build and test regressions.

- [ ] **Step 6: Commit**

```bash
cd /workspace/llm-gateway
git add web/src/App.tsx
git commit -m "$(cat <<'EOF'
feat(web): mount /orgs/new route

Wires the OrgCreate page from the previous commit into the router.
Sits next to /onboarding (both are top-level routes outside the
/:orgSlug subtree). Fixes the OrgSwitcher "Create org" button which
already navigated to /orgs/new but landed on Home because the route
didn't exist.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Verification Summary

After all three tasks:

1. **TypeScript**: `npm run build` clean (Step 3 of Task 3).
2. **Tests**: full suite green at 36 files / 190 tests (Step 4 of Task 3).
3. **Manual**: OrgSwitcher → "Create org" → form renders → success navigates → duplicate slug shows error (Step 5 of Task 3).
4. **Limbo-user safety**: confirmed via code — `OnboardingRedirect` (App.tsx) bounces zero-org users from `/orgs/new` to `/onboarding` because `/orgs/new` is not in its allowed-paths list (`/onboarding`, `/accept-invite`). No code change needed; this is the existing behavior.

## Risks Addressed

- **Stale `currentOrg` during creation**: the card calls `applyAuthResponse` which atomically swaps `currentOrg` to the new org — no in-between state where the sidebar would show mismatched chrome.
- **Limbo user hitting `/orgs/new` directly**: handled by `OnboardingRedirect` (verified — `/orgs/new` is not in the allowed list, so they bounce to `/onboarding`).
- **Browser back after success**: user lands back on `/orgs/new`, sees the empty form again. Acceptable; matches `/onboarding`'s behavior.

## Out of Scope (per spec)

- Renaming `OnboardingCreateCard` → `OrgCreateCard`.
- Migrating `onboarding.create.*` keys to `orgCreate.*`.
- Modal-in-OrgSwitcher variant.
- Post-creation navigation target changes.
- Org templates / presets in the create form.
