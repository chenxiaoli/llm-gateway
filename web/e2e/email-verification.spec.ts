import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Phase 4 e2e: signup -> email verification -> login.
 *
 * PREREQUISITE: backend running on :8080 with `[email] transport = "file"`
 * in config.toml so verification emails land in `./dev-emails` as .eml
 * files we can scrape for tokens. With the default `transport = "noop"`
 * these tests will fail at the "read latest email" step — the noop mailer
 * writes nothing to disk.
 *
 * Route reference:
 *   /register -> /check-email -> /verify-email/:token -> /login -> /onboarding
 *
 * Selector convention note: Login/Register forms render
 * `<label className="label"><span>...</span></label>` without `htmlFor`/`id`
 * association, so `getByLabel` doesn't resolve. Every form input is reached
 * via `getByPlaceholder(...)` (matching the existing app.spec.ts pattern).
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

test('signup -> verify email -> login -> onboarding', async ({ page }) => {
  const runTag = String(Date.now());
  const username = `alice_${runTag}`;
  const email = `alice+${runTag}@example.com`;
  const password = 'Passw0rd!';

  // 1. Register — lands at /check-email.
  await page.goto('/register');
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Email').fill(email);
  await page.getByPlaceholder('Password').fill(password);
  await page.getByPlaceholder('Confirm Password').fill(password);
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page).toHaveURL(/\/check-email/);

  // 2. Read the verification token from the .eml file.
  const body = await readLatestEmailTo(email);
  const match = body.match(/\/verify-email\/([A-Za-z0-9_-]+)/);
  expect(match, 'verification email must contain a /verify-email/<token> link').not.toBeNull();
  const token = match![1];

  // 3. Visit the verification link — VerifyEmail page shows the success panel.
  await page.goto(`/verify-email/${token}`);
  await expect(page.getByText('Email verified')).toBeVisible({ timeout: 10000 });

  // 4. Log in with the credentials — should succeed now, land at onboarding
  //    (brand-new user has zero org memberships).
  await page.goto('/login');
  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Password').fill(password);
  await page.getByRole('button', { name: 'Sign In' }).click();
  await expect(page).toHaveURL(/\/onboarding$/);
});
