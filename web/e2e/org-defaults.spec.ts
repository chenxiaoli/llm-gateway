import { test, expect, request } from '@playwright/test';

/**
 * Phase 5 E2E: org-default rate-limit enforcement.
 *
 * Exercises the full chain end-to-end:
 *   UI login → management API (set org default, create key) →
 *   proxy (/v1/chat/completions) → in-memory rate limiter →
 *   429 + Retry-After on the (N+1)th request.
 *
 * Reuses the admin's current org (the default org admin owns). Sets the
 * org default back to null in `afterAll` so re-runs start clean —
 * leaving `default_rate_limit_rpm = 3` on the default org would
 * 429-storm every subsequent request in the org.
 *
 * Admin credentials are env-overridable (E2E_ADMIN_USER / E2E_ADMIN_PASS)
 * and default to the seed convention `admin` / `admin123456` used across
 * `invitations.spec.ts` / `app.spec.ts`. The admin must be an owner/admin
 * of the default org (needed to set org defaults + create keys).
 *
 * Backend on :8080 per `playwright.config.ts` `baseURL`. The `baseURL`
 * points at the backend, which serves a built `dist/` — but that build
 * redirects `/login` → `/` (SPA fallback quirk in the served bundle), so
 * the UI login step targets the Vite dev server on :5173 (which proxies
 * `/api` + `/v1` to :8080 per `vite.config.ts`). Raw API request contexts
 * hit :8080 directly. No existing e2e makes raw API calls through
 * Playwright's request layer, so we hardcode the absolute URLs here.
 */

const ADMIN_USER = process.env.E2E_ADMIN_USER ?? 'admin';
const ADMIN_PASS = process.env.E2E_ADMIN_PASS ?? 'admin123456';
const RUN_TAG = process.env.E2E_RUN_TAG ?? String(Date.now());

const BACKEND = 'http://localhost:8080';
const DEV_SERVER = 'http://localhost:5173';
const ORG_DEFAULT_RPM = 3;

test('org default rate limit is enforced on proxy requests', async ({ browser }) => {
  // Fresh context with no storage — avoids lingering auth (refresh-token
  // cookie from a prior run redirects /login to the landing page).
  const context = await browser.newContext();
  const page = await context.newPage();

  // --- 1. Login via UI so localStorage is seeded with the JWT + currentOrg. ---
  // Target the Vite dev server (:5173) — the backend's served dist build
  // redirects /login → /, but the dev server renders the SPA correctly.
  await page.goto(`${DEV_SERVER}/login`);
  await page.getByPlaceholder('Username').fill(ADMIN_USER);
  await page.getByPlaceholder('Password').fill(ADMIN_PASS);
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL('**/dashboard');

  // --- 2. Pull JWT from localStorage; derive the org slug from the URL. ---
  // The app persists only the raw tokens in localStorage (keys
  // `llm_gateway_admin_token` / `llm_gateway_refresh_token` per
  // `web/src/api/client.ts`); the zustand auth store's `currentOrg` lives
  // in memory only, so the slug is read from the post-login dashboard URL
  // (`/{slug}/dashboard`).
  const token = await page.evaluate(
    () => localStorage.getItem('llm_gateway_admin_token') ?? null,
  );
  expect(token, 'JWT must be present after UI login').toBeTruthy();
  const slug = (await page.url()).match(/\/([^/]+)\/dashboard$/)?.[1];
  expect(slug, 'must land on an org-scoped dashboard').toBeTruthy();

  // --- 3. Management API context (JWT-authed). ---
  const apiContext = await request.newContext({
    baseURL: BACKEND,
    extraHTTPHeaders: { authorization: `Bearer ${token}` },
  });

  // Snapshot the pre-test defaults so afterAll can restore them exactly.
  const beforeResp = await apiContext.get(`/api/v1/${slug}/defaults`);
  expect(beforeResp.ok()).toBeTruthy();
  const beforeDefaults = await beforeResp.json();

  try {
    // --- 4. Create a key with no per-key rate_limit (org default applies). ---
    const keyResp = await apiContext.post(`/api/v1/${slug}/keys`, {
      data: {
        name: `e2e-org-default-${RUN_TAG}`,
        rate_limit: null,
        budget_monthly: null,
      },
    });
    expect(keyResp.ok()).toBeTruthy();
    const keyBody = await keyResp.json();
    // CreateKeyResponse field is `key` (plaintext), NOT `plaintext`.
    expect(keyBody.key).toBeTruthy();
    const apiKey: string = keyBody.key;
    const keyId: string = keyBody.id;

    // --- 5. Set org default_rate_limit_rpm. ---
    const putResp = await apiContext.put(`/api/v1/${slug}/defaults`, {
      data: { default_rate_limit_rpm: ORG_DEFAULT_RPM, default_budget_monthly_usd: null },
    });
    expect(putResp.ok()).toBeTruthy();

    // --- 6. Fire N requests — none should be 429. ---
    const proxyCtx = await request.newContext({
      baseURL: BACKEND,
      // Proxy path authenticates with the API key directly, not the JWT.
      extraHTTPHeaders: { authorization: `Bearer ${apiKey}` },
    });

    for (let i = 0; i < ORG_DEFAULT_RPM; i++) {
      const r = await proxyCtx.post('/v1/chat/completions', {
        data: { model: 'gpt-test', messages: [{ role: 'user', content: 'hi' }] },
      });
      // Don't assert success — upstream provider is unreachable in this env.
      // Enforcement happens before upstream routing, so a 429 here would mean
      // the limiter tripped early (a bug).
      expect(r.status(), `request ${i + 1} should not be rate-limited`).not.toBe(429);
    }

    // --- 7. The (N+1)th request must be 429 with a positive Retry-After. ---
    const over = await proxyCtx.post('/v1/chat/completions', {
      data: { model: 'gpt-test', messages: [{ role: 'user', content: 'hi' }] },
    });
    expect(over.status(), 'N+1th request must be rate-limited').toBe(429);
    const retryAfter = over.headers()['retry-after'];
    expect(retryAfter, 'Retry-After header must be present').toBeTruthy();
    expect(Number(retryAfter), 'Retry-After must parse to a positive integer').toBeGreaterThan(0);

    // --- 8. Cleanup the test key. ---
    const delResp = await apiContext.delete(`/api/v1/${slug}/keys/${keyId}`);
    expect(delResp.ok()).toBeTruthy();
  } finally {
    // --- 9. ALWAYS restore org defaults, even on assertion failure. ---
    // Leaving default_rate_limit_rpm = 3 on the default org would 429-storm
    // every later request in the org (this test reuses the admin's org).
    await apiContext.put(`/api/v1/${slug}/defaults`, {
      data: {
        default_rate_limit_rpm: beforeDefaults.default_rate_limit_rpm ?? null,
        default_budget_monthly_usd: beforeDefaults.default_budget_monthly_usd ?? null,
      },
    });
    await apiContext.dispose();
    await context.close();
  }
});
