import { test, expect } from '@playwright/test';

// Use Vite dev server (port 5173) which has SPA fallback and latest code.
test.use({ baseURL: 'http://localhost:5173' });

const LANGS = [
  { lang: 'zh', heading: '快速开始', apiKeyTitle: 'API 密钥管理', balanceTitle: '余额充值', notFound: '文档未找到' },
  { lang: 'en', heading: 'Getting Started', apiKeyTitle: 'API Keys', balanceTitle: 'Balance & Top-up', notFound: 'Document not found' },
];

for (const { lang, heading, apiKeyTitle, balanceTitle, notFound } of LANGS) {
  test.describe(`docs page (${lang})`, () => {
    test(`redirects /docs to /docs/${lang}/user/getting-started`, async ({ page }) => {
      await page.goto('/');
      await page.evaluate((l) => localStorage.setItem('i18n-language', l), lang);

      await page.goto('/docs');
      await expect(page).toHaveURL(new RegExp(`/docs/${lang}/user/getting-started`));
    });

    test(`renders getting-started page`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      await expect(page.getByRole('heading', { name: heading })).toBeVisible();

      await expect(page.getByRole('cell', { name: 'baseURL' })).toBeVisible();
      await expect(page.getByRole('cell', { name: 'apiKey' })).toBeVisible();

      await expect(page.getByText('OpenAI SDK', { exact: true })).toBeVisible();
      await expect(page.getByText('Anthropic SDK', { exact: true })).toBeVisible();
      await expect(page.getByText('cURL', { exact: true })).toBeVisible();

      await expect(page.getByRole('cell', { name: '/v1/chat/completions' })).toBeVisible();
      await expect(page.getByRole('cell', { name: '/v1/messages' })).toBeVisible();
    });

    test(`copy buttons work`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const copyButtons = page.getByRole('button', { name: 'Copy' });
      await expect(copyButtons.first()).toBeVisible();
      await copyButtons.first().click();

      await expect(page.getByRole('button', { name: 'Copied' }).first()).toBeVisible({ timeout: 3000 });
    });

    test(`sidebar shows all nav items`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const nav = page.locator('nav[aria-label="Documentation navigation"]');
      await expect(nav.getByRole('link', { name: heading })).toBeVisible();
      await expect(nav.getByRole('link', { name: apiKeyTitle })).toBeVisible();

      const channelLink = lang === 'zh' ? '渠道配置' : 'Channel Configuration';
      await expect(nav.getByRole('link', { name: channelLink })).toBeVisible();
    });

    test(`sidebar highlights active page`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const activeLink = page.locator('a[aria-current="page"]');
      await expect(activeLink).toBeVisible();
      await expect(activeLink).toHaveAttribute('href', `/docs/${lang}/user/getting-started`);
    });

    test(`navigates to another user doc`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const nav = page.locator('nav[aria-label="Documentation navigation"]');
      await nav.getByRole('link', { name: apiKeyTitle }).click();
      await expect(page).toHaveURL(new RegExp(`/docs/${lang}/user/api-keys`));

      const body = page.locator('article');
      await expect(body).toBeVisible();
      const content = await body.textContent();
      expect(content).not.toContain('import ');
      expect(content).not.toContain('export default');
    });

    test(`navigates to admin doc`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const channelLink = lang === 'zh' ? '渠道配置' : 'Channel Configuration';
      const nav = page.locator('nav[aria-label="Documentation navigation"]');
      await nav.getByRole('link', { name: channelLink }).click();
      await expect(page).toHaveURL(new RegExp(`/docs/${lang}/admin/channels`));

      const body = page.locator('article');
      await expect(body).toBeVisible();
    });

    test(`shows 404 for invalid doc slug`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/nonexistent-doc`);

      await expect(page.getByText('404')).toBeVisible();
      await expect(page.getByText(notFound)).toBeVisible();
    });

    test(`page refresh preserves current doc`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);
      const nav = page.locator('nav[aria-label="Documentation navigation"]');
      await nav.getByRole('link', { name: balanceTitle }).click();
      await expect(page).toHaveURL(new RegExp(`/docs/${lang}/user/balance`));

      await page.reload();
      await expect(page).toHaveURL(new RegExp(`/docs/${lang}/user/balance`));

      const body = page.locator('article');
      await expect(body).toBeVisible();
    });

    test(`code blocks contain server URL`, async ({ page }) => {
      await page.goto(`/docs/${lang}/user/getting-started`);

      const origin = new URL(page.url()).origin;

      const codeBlocks = page.locator('pre code');
      const count = await codeBlocks.count();
      expect(count).toBeGreaterThanOrEqual(3);

      const allCode = await codeBlocks.allTextContents();
      const combined = allCode.join('\n');
      expect(combined).toContain(origin);
    });
  });
}

test.describe('docs language switching', () => {
  test('language toggle switches URL and content', async ({ page }) => {
    await page.goto('/docs/zh/user/getting-started');
    await expect(page.getByRole('heading', { name: '快速开始' })).toBeVisible();

    await page.locator('header button[aria-label="Switch to English"]').click();

    await expect(page).toHaveURL(/\/docs\/en\/user\/getting-started/);
    await expect(page.getByRole('heading', { name: 'Getting Started' })).toBeVisible();
  });

  test('theme toggle works on docs page', async ({ page }) => {
    await page.goto('/docs/zh/user/getting-started');

    const html = page.locator('html');
    const initialTheme = await html.getAttribute('data-theme');

    await page.locator('header button[aria-label="Toggle theme"]').click();
    const newTheme = await html.getAttribute('data-theme');

    expect(newTheme).not.toBe(initialTheme);
    expect(['light', 'dark']).toContain(newTheme);
  });
});
