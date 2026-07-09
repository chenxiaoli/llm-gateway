import { test, expect, type BrowserContext, type Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Phase 3/4 happy-path E2E: wizard-gated signup + single-use invitations.
 *
 * Phase 4 update: invitations are now email-bound. Each Generate requires a
 * recipient_email, invitees must sign up WITH that email, and a verify-email
 * step is inserted before any login (Phase 4's verification gate would
 * otherwise 403 the login with `email_not_verified`).
 *
 * These tests assume a running backend on :8080 (per playwright.config.ts
 * `baseURL`) with `[email] transport = "file"` so verification emails land in
 * `./dev-emails` as .eml files we can scrape, and a pre-seeded admin user —
 * same convention as `app.spec.ts`:
 *
 *   username: admin
 *   password: admin123456
 *
 * Brand-new users created here (e.g. `alice`, `bob`, `carol`) must have
 * unique-ish names per run so re-runs against a shared DB don't collide on
 * the username-unique constraint. We suffix with a timestamp.
 *
 * Route reference (post Phase 2.1 URL migration):
 *   /login, /register, /check-email, /verify-email/:token, /onboarding,
 *   /accept-invite, /{org_slug}/dashboard, /{org_slug}/admin/invitations
 */

const ADMIN_USER = 'admin';
const ADMIN_PASS = 'admin123456';
const DEV_EMAIL_DIR = path.resolve(__dirname, '../../dev-emails');

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

/**
 * Read the most recent .eml file in ./dev-emails whose body contains `to`.
 * Used to scrape the email-verification token after a fresh signup.
 */
async function readLatestEmailTo(to: string): Promise<string> {
  const files = fs
    .readdirSync(DEV_EMAIL_DIR)
    .map((name) => ({
      name,
      mtime: fs.statSync(path.join(DEV_EMAIL_DIR, name)).mtimeMs,
    }))
    .sort((a, b) => b.mtime - a.mtime);
  for (const f of files) {
    const content = fs.readFileSync(path.join(DEV_EMAIL_DIR, f.name), 'utf8');
    if (content.includes(to)) return content;
  }
  throw new Error(`no email found in ${DEV_EMAIL_DIR} addressed to ${to}`);
}

/**
 * Phase 4 helper: complete the verify-email step for a freshly-registered
 * user by scraping the token from the .eml addressed to `email` and visiting
 * /verify-email/:token. Resolves once the "Email verified" success panel is
 * visible.
 */
async function verifyEmail(page: Page, email: string) {
  const body = await readLatestEmailTo(email);
  const token = body.match(/\/verify-email\/([A-Za-z0-9_-]+)/)![1];
  await page.goto(`/verify-email/${token}`);
  await expect(page.getByText('Email verified')).toBeVisible({ timeout: 10000 });
}

test.describe('Phase 3: wizard + invitations', () => {
  test('signup -> wizard -> create org -> land in dashboard', async ({ page }) => {
    const username = `alice_${RUN_TAG}`;
    const email = `alice+${RUN_TAG}@example.com`;

    // 1. Register — Phase 4 gates login on email verification, so signup
    //    lands at /check-email (not /onboarding).
    await page.goto('/register');
    await page.getByPlaceholder('Username').fill(username);
    await page.getByPlaceholder('Email').fill(email);
    await page.getByPlaceholder('Password').fill('password123');
    await page.getByPlaceholder('Confirm Password').fill('password123');
    await page.getByRole('button', { name: 'Register' }).click();

    await expect(page).toHaveURL(/\/check-email$/);

    // Verify email, then log in — now lands at /onboarding (limbo state, no org).
    await verifyEmail(page, email);
    await page.goto('/login');
    await page.getByPlaceholder('Username').fill(username);
    await page.getByPlaceholder('Password').fill('password123');
    await page.getByRole('button', { name: 'Sign In' }).click();
    await expect(page).toHaveURL(/\/onboarding$/);

    // 2. Create an org via the wizard.
    await page.getByPlaceholder('e.g., Acme Inc.').fill(orgName);
    await page.getByPlaceholder('e.g., acme').fill(orgSlug);
    await page.getByRole('button', { name: 'Create', exact: true }).click();

    // 3. Should land at the new org's dashboard.
    await expect(page).toHaveURL(new RegExp(`/${orgSlug}/dashboard$`));
  });

  test('admin mints invite -> second user signs up via link -> both in same org', async ({ browser }) => {
    const recipientEmail = `bob+${RUN_TAG}@example.com`;

    // ---- Admin session: log in, generate an invitation, capture its URL ----
    const adminCtx: BrowserContext = await browser.newContext();
    const adminPage: Page = await adminCtx.newPage();

    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    // After login the admin lands at their current org's dashboard.
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug, 'admin should land at an org-scoped dashboard').toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await expect(adminPage.getByRole('heading', { name: 'Invitations' })).toBeVisible();

    // Phase 4: invitation is email-bound — fill recipient_email before Generate.
    await adminPage.getByPlaceholder('alice@example.com').fill(recipientEmail);
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
    // Phase 4: invitee must sign up WITH the recipient email (preview
    // pre-fills it; we refill explicitly for robustness).
    await inviteePage.getByPlaceholder('Email').fill(recipientEmail);
    await inviteePage.getByPlaceholder('Password').fill('password123');
    await inviteePage.getByPlaceholder('Confirm Password').fill('password123');
    await inviteePage.getByRole('button', { name: 'Register' }).click();

    // Phase 4: register-time accept grants membership, but the user still
    // must verify email before login is allowed — lands at /check-email.
    await expect(inviteePage).toHaveURL(/\/check-email$/);
    await verifyEmail(inviteePage, recipientEmail);

    // Log in — should land in the admin's org (already a member).
    await inviteePage.goto('/login');
    await inviteePage.getByPlaceholder('Username').fill(inviteeUsername);
    await inviteePage.getByPlaceholder('Password').fill('password123');
    await inviteePage.getByRole('button', { name: 'Sign In' }).click();
    await expect(inviteePage).toHaveURL(new RegExp(`/${adminSlug}/dashboard$`));

    await adminCtx.close();
    await inviteeCtx.close();
  });

  test('logged-in user accepts invite via /accept-invite', async ({ browser }) => {
    const recipientEmail = `carol+${RUN_TAG}@example.com`;

    // Admin mints a fresh invite bound to carol's email.
    const adminCtx = await browser.newContext();
    const adminPage = await adminCtx.newPage();
    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug).toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await adminPage.getByPlaceholder('alice@example.com').fill(recipientEmail);
    await adminPage.getByRole('button', { name: 'Generate', exact: true }).click();
    const linkCell = adminPage.locator('code').filter({ hasText: /\/accept-invite\?token=/ }).first();
    await expect(linkCell).toBeVisible({ timeout: 10000 });
    const inviteUrl = (await linkCell.innerText()).trim();

    // Third user signs up *first* with the recipient email, creates her own
    // throwaway org, then visits the invite URL while logged in and accepts.
    const userCtx = await browser.newContext();
    const userPage = await userCtx.newPage();
    const username = `carol_${RUN_TAG}`;

    await userPage.goto('/register');
    await userPage.getByPlaceholder('Username').fill(username);
    await userPage.getByPlaceholder('Email').fill(recipientEmail);
    await userPage.getByPlaceholder('Password').fill('password123');
    await userPage.getByPlaceholder('Confirm Password').fill('password123');
    await userPage.getByRole('button', { name: 'Register' }).click();
    await expect(userPage).toHaveURL(/\/check-email$/);
    await verifyEmail(userPage, recipientEmail);

    // Log in, then create her own throwaway org so she's no longer in limbo.
    await userPage.goto('/login');
    await userPage.getByPlaceholder('Username').fill(username);
    await userPage.getByPlaceholder('Password').fill('password123');
    await userPage.getByRole('button', { name: 'Sign In' }).click();
    await expect(userPage).toHaveURL(/\/onboarding$/);

    await userPage.getByPlaceholder('e.g., Acme Inc.').fill(`Carol ${RUN_TAG}`);
    await userPage.getByPlaceholder('e.g., acme').fill(`carol-${RUN_TAG}`);
    await userPage.getByRole('button', { name: 'Create', exact: true }).click();
    await expect(userPage).toHaveURL(new RegExp(`/carol-${RUN_TAG}/dashboard$`));

    // Now visit the invite while logged in — email matches + is verified, so
    // the Accept button is offered.
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
