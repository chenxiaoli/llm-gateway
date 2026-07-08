import { test, expect, type BrowserContext, type Page } from '@playwright/test';

/**
 * Phase 3 happy-path E2E: wizard-gated signup + single-use invitations.
 *
 * These tests assume a running backend on :8080 (per playwright.config.ts
 * `baseURL`) and a pre-seeded admin user — same convention as `app.spec.ts`:
 *
 *   username: admin
 *   password: admin123456
 *
 * Brand-new users created here (e.g. `alice`, `bob`, `carol`) must have
 * unique-ish names per run so re-runs against a shared DB don't collide on
 * the username-unique constraint. We suffix with a timestamp.
 *
 * Route reference (post Phase 2.1 URL migration):
 *   /login, /register, /onboarding, /accept-invite
 *   /{org_slug}/dashboard, /{org_slug}/admin/invitations
 */

const ADMIN_USER = 'admin';
const ADMIN_PASS = 'admin123456';

// Per-run suffix keeps these tests re-runnable against a shared seed DB
// without tripping the username-unique constraint. `process.env.E2E_RUN_TAG`
// can override the default (a wall-clock timestamp) when callers want
// reproducible names.
declare const process: { env: Record<string, string | undefined> };
const RUN_TAG = process.env.E2E_RUN_TAG ?? String(Date.now());
const orgName = `Acme ${RUN_TAG}`;
const orgSlug = `acme-${RUN_TAG}`;

/**
 * Log in via the new `/login` route (Phase 2.1 URL). Resolves once the
 * dashboard URL is reached.
 */
async function loginAs(page: Page, username: string, password: string) {
  await page.goto('/login');
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Password').fill(password);
  await page.getByRole('button', { name: 'Sign In' }).click();
}

test.describe('Phase 3: wizard + invitations', () => {
  test('signup → wizard → create org → land in dashboard', async ({ page }) => {
    const username = `alice_${RUN_TAG}`;

    // 1. Register — lands at /onboarding (limbo state, no org yet).
    await page.goto('/register');
    await page.getByPlaceholder('Username').fill(username);
    await page.getByPlaceholder('Password').fill('password123');
    await page.getByPlaceholder('Confirm password').fill('password123');
    await page.getByRole('button', { name: 'Register' }).click();

    await expect(page).toHaveURL(/\/onboarding$/);

    // 2. Create an org via the wizard.
    await page.getByPlaceholder('e.g., Acme Inc.').fill(orgName);
    await page.getByPlaceholder('e.g., acme').fill(orgSlug);
    await page.getByRole('button', { name: 'Create', exact: true }).click();

    // 3. Should land at the new org's dashboard.
    await expect(page).toHaveURL(new RegExp(`/${orgSlug}/dashboard$`));
  });

  test('admin mints invite → second user signs up via link → both in same org', async ({ browser }) => {
    // ---- Admin session: log in, generate an invitation, capture its URL ----
    const adminCtx: BrowserContext = await browser.newContext();
    const adminPage: Page = await adminCtx.newPage();

    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    // After login the admin lands at their current org's dashboard.
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug, 'admin should land at an org-scoped dashboard').toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await expect(adminPage.getByRole('heading', { name: 'Invitations' })).toBeVisible();

    // Generate button is in the "Generate invite link" card.
    await adminPage.getByRole('button', { name: 'Generate', exact: true }).click();

    // The pending invitation row renders the link inside a <code> element
    // followed by a copy button. Grab its text content.
    const linkCell = adminPage.locator('code').filter({ hasText: /\/accept-invite\?token=/ }).first();
    await expect(linkCell).toBeVisible({ timeout: 10000 });
    const inviteUrl = (await linkCell.innerText()).trim();
    expect(inviteUrl, 'invite URL should target /accept-invite').toContain('/accept-invite?token=');

    // ---- Invitee session: open invite, sign up to accept ----
    const inviteeCtx: BrowserContext = await browser.newContext();
    const inviteePage: Page = await inviteeCtx.newPage();

    await inviteePage.goto(inviteUrl);

    // Preview card: "Join <org>" heading, "Sign up to accept" CTA.
    await expect(inviteePage.getByRole('heading', { name: /Join / })).toBeVisible();
    await inviteePage.getByRole('button', { name: 'Sign up to accept' }).click();

    // Should land on /register with the invite token forwarded.
    await expect(inviteePage).toHaveURL(/\/register/);

    const inviteeUsername = `bob_${RUN_TAG}`;
    await inviteePage.getByPlaceholder('Username').fill(inviteeUsername);
    await inviteePage.getByPlaceholder('Password').fill('password123');
    await inviteePage.getByPlaceholder('Confirm password').fill('password123');
    await inviteePage.getByRole('button', { name: 'Register' }).click();

    // Accepting via the register form auto-joins the org and lands in its
    // dashboard.
    await expect(inviteePage).toHaveURL(new RegExp(`/${adminSlug}/dashboard$`));

    await adminCtx.close();
    await inviteeCtx.close();
  });

  test('logged-in user accepts invite via /accept-invite', async ({ browser }) => {
    // Admin mints a fresh invite.
    const adminCtx = await browser.newContext();
    const adminPage = await adminCtx.newPage();
    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug).toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await adminPage.getByRole('button', { name: 'Generate', exact: true }).click();
    const linkCell = adminPage.locator('code').filter({ hasText: /\/accept-invite\?token=/ }).first();
    await expect(linkCell).toBeVisible({ timeout: 10000 });
    const inviteUrl = (await linkCell.innerText()).trim();

    // Third user signs up *first* (no invite), lands in their own org via
    // the wizard, then visits the invite URL while logged in and accepts.
    const userCtx = await browser.newContext();
    const userPage = await userCtx.newPage();
    const username = `carol_${RUN_TAG}`;

    await userPage.goto('/register');
    await userPage.getByPlaceholder('Username').fill(username);
    await userPage.getByPlaceholder('Password').fill('password123');
    await userPage.getByPlaceholder('Confirm password').fill('password123');
    await userPage.getByRole('button', { name: 'Register' }).click();
    await expect(userPage).toHaveURL(/\/onboarding$/);

    // Create her own throwaway org so she's no longer in limbo.
    await userPage.getByPlaceholder('e.g., Acme Inc.').fill(`Carol ${RUN_TAG}`);
    await userPage.getByPlaceholder('e.g., acme').fill(`carol-${RUN_TAG}`);
    await userPage.getByRole('button', { name: 'Create', exact: true }).click();
    await expect(userPage).toHaveURL(new RegExp(`/carol-${RUN_TAG}/dashboard$`));

    // Now visit the invite while logged in — should see Accept / Decline.
    await userPage.goto(inviteUrl);
    await expect(userPage.getByRole('heading', { name: /Join / })).toBeVisible();
    await expect(userPage.getByRole('button', { name: 'Accept', exact: true })).toBeVisible();

    await userPage.getByRole('button', { name: 'Accept', exact: true }).click();

    // After accepting, current_org switches to the admin's org.
    await expect(userPage).toHaveURL(new RegExp(`/${adminSlug}/dashboard$`));

    await adminCtx.close();
    await userCtx.close();
  });
});
