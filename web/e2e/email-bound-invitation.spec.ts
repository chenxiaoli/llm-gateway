import { test, expect, type BrowserContext, type Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Phase 4 e2e: email-bound invitation lifecycle.
 *
 * PREREQUISITE: backend on :8080 with `[email] transport = "file"` in
 * config.toml. The signup verification email needs file transport to
 * scrape the token; the invitation dispatch itself is not email-scraped
 * (the invitation URL is read from the admin UI table).
 *
 * Coverage:
 *   - happy path: admin mints invite bound to bob@example.com -> bob signs up
 *     with that email -> bob verifies -> bob lands in the admin's org
 *     (already a member via the register-time server-side accept).
 *   - negative: admin mints invite bound to alice@example.com -> a different
 *     user (carol) signs up with a different email -> the accept attempt is
 *     rejected with the email_mismatch error.
 *
 * Admin credentials match the seed convention used across app.spec.ts /
 * invitations.spec.ts (username `admin`, password `admin123456`). The admin
 * must exist and have an org before this spec runs.
 */

const ADMIN_USER = 'admin';
const ADMIN_PASS = 'admin123456';
const DEV_EMAIL_DIR = path.resolve(__dirname, '../../dev-emails');

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

async function loginAs(page: Page, username: string, password: string) {
  await page.goto('/login');
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Password').fill(password);
  await page.getByRole('button', { name: 'Sign In' }).click();
}

test.describe('Phase 4 email-bound invitations', () => {
  test('admin mints invite bound to recipient_email -> invitee signs up with that email and joins', async ({ browser }) => {
    const runTag = String(Date.now());
    const recipientEmail = `bob+${runTag}@example.com`;

    // ---- Admin session ----
    const adminCtx: BrowserContext = await browser.newContext();
    const adminPage: Page = await adminCtx.newPage();
    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug, 'admin should land at an org-scoped dashboard').toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await expect(adminPage.getByRole('heading', { name: 'Invitations' })).toBeVisible();

    // Fill the recipient_email (Phase 4 required field) + click Generate.
    // The Generate button is disabled until a valid email is entered.
    await adminPage.getByPlaceholder('alice@example.com').fill(recipientEmail);
    await adminPage.getByRole('button', { name: 'Generate', exact: true }).click();

    // The new pending invitation row includes the recipient email + the
    // invitation URL. The URL goes to /accept-invite?token=...
    const linkCell = adminPage
      .locator('code')
      .filter({ hasText: /\/accept-invite\?token=/ })
      .first();
    await expect(linkCell).toBeVisible({ timeout: 10000 });
    const inviteUrl = (await linkCell.innerText()).trim();
    expect(inviteUrl).toContain('/accept-invite?token=');
    await adminCtx.close();

    // ---- Invitee session ----
    const inviteeCtx: BrowserContext = await browser.newContext();
    const inviteePage: Page = await inviteeCtx.newPage();
    await inviteePage.goto(inviteUrl);
    await expect(inviteePage.getByRole('heading', { name: /Join / })).toBeVisible();
    await inviteePage.getByRole('button', { name: 'Sign up to accept' }).click();
    await expect(inviteePage).toHaveURL(/\/register/);

    const inviteeUsername = `bob_${runTag}`;
    await inviteePage.getByPlaceholder('Username').fill(inviteeUsername);
    // The email field is pre-filled from the invitation preview — refill it
    // explicitly so the test is robust if the preview fetch is slow/missing.
    await inviteePage.getByPlaceholder('Email').fill(recipientEmail);
    await inviteePage.getByPlaceholder('Password').fill('Passw0rd!');
    await inviteePage.getByPlaceholder('Confirm Password').fill('Passw0rd!');
    await inviteePage.getByRole('button', { name: 'Register' }).click();

    // Phase 4: signup lands at /check-email (not the dashboard). The
    // server-side invitation accept already ran in the register tx, so
    // membership is granted but the user must still verify before login.
    await expect(inviteePage).toHaveURL(/\/check-email/);

    // Verify the email.
    const body = await readLatestEmailTo(recipientEmail);
    const verifyToken = body.match(/\/verify-email\/([A-Za-z0-9_-]+)/)![1];
    await inviteePage.goto(`/verify-email/${verifyToken}`);
    await expect(inviteePage.getByText('Email verified')).toBeVisible({ timeout: 10000 });

    // Log in — should land in the admin's org (already a member).
    await inviteePage.goto('/login');
    await inviteePage.getByPlaceholder('Username').fill(inviteeUsername);
    await inviteePage.getByPlaceholder('Password').fill('Passw0rd!');
    await inviteePage.getByRole('button', { name: 'Sign In' }).click();
    await expect(inviteePage).toHaveURL(new RegExp(`/${adminSlug}/dashboard$`));

    await inviteeCtx.close();
  });

  test('rejects accept when signup email does not match recipient_email', async ({ browser }) => {
    const runTag = String(Date.now());
    const boundEmail = `alice+${runTag}@example.com`;

    // Admin mints invite bound to alice+tag.
    const adminCtx: BrowserContext = await browser.newContext();
    const adminPage: Page = await adminCtx.newPage();
    await loginAs(adminPage, ADMIN_USER, ADMIN_PASS);
    const adminSlug = (await adminPage.url()).match(/\/([^/]+)\/dashboard/)?.[1];
    expect(adminSlug).toBeTruthy();

    await adminPage.goto(`/${adminSlug}/admin/invitations`);
    await adminPage.getByPlaceholder('alice@example.com').fill(boundEmail);
    await adminPage.getByRole('button', { name: 'Generate', exact: true }).click();
    const linkCell = adminPage
      .locator('code')
      .filter({ hasText: /\/accept-invite\?token=/ })
      .first();
    await expect(linkCell).toBeVisible({ timeout: 10000 });
    const inviteUrl = (await linkCell.innerText()).trim();
    await adminCtx.close();

    // A different user (carol) signs up with a DIFFERENT email. The register-
    // time accept runs server-side and should reject with email_mismatch.
    const carolCtx: BrowserContext = await browser.newContext();
    const carolPage: Page = await carolCtx.newPage();
    await carolPage.goto(inviteUrl);
    await carolPage.getByRole('button', { name: 'Sign up to accept' }).click();
    await carolPage.getByPlaceholder('Username').fill(`carol_${runTag}`);
    // Overwrite any preview-prefilled email with a mismatching address.
    await carolPage.getByPlaceholder('Email').fill(`carol+${runTag}@example.com`);
    await carolPage.getByPlaceholder('Password').fill('Passw0rd!');
    await carolPage.getByPlaceholder('Confirm Password').fill('Passw0rd!');
    await carolPage.getByRole('button', { name: 'Register' }).click();

    // The accept attempt fails with email_mismatch (i18n key
    // auth.emailMismatch -> "Email does not match the invitation.").
    await expect(carolPage.getByText(/does not match the invitation/i)).toBeVisible({
      timeout: 10000,
    });
    await carolCtx.close();
  });
});
