import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Phase 4 e2e: full password-reset flow.
 *
 * PREREQUISITE: backend on :8080 with `[email] transport = "file"` in
 * config.toml. Both the initial signup verification email and the password-
 * reset email are scraped from .eml files in `./dev-emails`.
 *
 * Flow: register -> verify email -> login -> logout -> forgot-password ->
 * read reset token from .eml -> reset-password/:token -> login with new
 * password -> onboarding.
 */

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

test('forgot password -> reset -> login with new password', async ({ page }) => {
  const runTag = String(Date.now());
  const username = `bob_${runTag}`;
  const email = `bob+${runTag}@example.com`;
  const oldPassword = 'OldPassw0rd!';
  const newPassword = 'NewPassw0rd!';

  // 1. Register + verify (reuse the email-verification pattern).
  await page.goto('/register');
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Email').fill(email);
  await page.getByPlaceholder('Password').fill(oldPassword);
  await page.getByPlaceholder('Confirm Password').fill(oldPassword);
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page).toHaveURL(/\/check-email/);

  const verifyBody = await readLatestEmailTo(email);
  const verifyToken = verifyBody.match(/\/verify-email\/([A-Za-z0-9_-]+)/)![1];
  await page.goto(`/verify-email/${verifyToken}`);
  await expect(page.getByText('Email verified')).toBeVisible({ timeout: 10000 });

  // 2. Request a password reset.
  await page.goto('/forgot-password');
  await page.getByPlaceholder('you@example.com').fill(email);
  await page.getByRole('button', { name: 'Send reset link' }).click();
  await expect(page.getByText('Check your email').first()).toBeVisible();

  // 3. Read the reset token from the freshest .eml. It must be newer than the
  //    verification email, so match on /reset-password/ specifically.
  const resetBody = await readLatestEmailTo(email);
  const resetToken = resetBody.match(/\/reset-password\/([A-Za-z0-9_-]+)/)![1];

  // 4. Visit the reset link, set a new password.
  await page.goto(`/reset-password/${resetToken}`);
  await page.getByPlaceholder('New password').fill(newPassword);
  await page.getByPlaceholder('Confirm new password').fill(newPassword);
  await page.getByRole('button', { name: 'Update password' }).click();
  await expect(page.getByText('Password updated')).toBeVisible({ timeout: 10000 });

  // 5. Auto-redirect to /login (within ~2s); log in with the NEW password.
  await expect(page).toHaveURL(/\/login$/, { timeout: 5000 });
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Password').fill(newPassword);
  await page.getByRole('button', { name: 'Sign In' }).click();
  await expect(page).toHaveURL(/\/onboarding$/);
});
