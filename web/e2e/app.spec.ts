import { test, expect } from '@playwright/test';

test.describe('homepage', () => {
  test('shows LLM Gateway title and features', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('h1')).toHaveText('LLM Gateway');
    await expect(page.getByText('Dual Protocol')).toBeVisible();
    await expect(page.getByText('Multi-Provider')).toBeVisible();
  });

  test('shows version in footer', async ({ page }) => {
    await page.goto('/');
    await page.getByText(/LLM Gateway v\d/).scrollIntoViewIfNeeded();
    await expect(page.getByText(/LLM Gateway v\d/)).toBeVisible();
  });

  test('navigates to login page', async ({ page }) => {
    await page.goto('/');
    await page.getByText('Go to Dashboard').click();
    await expect(page).toHaveURL(/\/console\/login/);
  });
});

test.describe('auth', () => {
  test('register first admin user', async ({ page }) => {
    await page.goto('/console/login');
    await page.getByText('Create an account').click();
    await expect(page).toHaveURL(/\/console\/register/);

    await page.fill('input[id="username"]', 'admin');
    await page.fill('input[id="password"]', 'admin123456');
    await page.fill('input[id="confirm"]', 'admin123456');
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/console\/dashboard/);
    await expect(page.getByText('admin', { exact: true })).toBeVisible();
  });

  test('login with existing user', async ({ page }) => {
    await page.goto('/console/login');
    await page.fill('input[id="username"]', 'admin');
    await page.fill('input[id="password"]', 'admin123456');
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/console\/dashboard/);
  });
});

test.describe('dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/console/login');
    await page.fill('input[id="username"]', 'admin');
    await page.fill('input[id="password"]', 'admin123456');
    await page.click('button[type="submit"]');
    await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('shows dashboard with stat cards', async ({ page }) => {
    await expect(page.getByText("Today's Requests")).toBeVisible();
    await expect(page.getByText("Today's Cost")).toBeVisible();
    await expect(page.getByText('Monthly Cost')).toBeVisible();
    await expect(page.getByText('Active Models')).toBeVisible();
  });

  test('shows version in console footer', async ({ page }) => {
    await expect(page.locator('.ant-layout-footer')).toContainText(/LLM Gateway v\d/);
  });

  test('navigate to providers page', async ({ page }) => {
    await page.getByText('Providers').click();
    await expect(page).toHaveURL(/\/console\/providers/);
  });

  test('navigate to settings page', async ({ page }) => {
    await page.getByText('Settings').click();
    await expect(page).toHaveURL(/\/console\/settings/);
  });

  test('logout returns to login', async ({ page }) => {
    await page.getByText('Logout').click();
    await expect(page).toHaveURL(/\/console\/login/);
  });
});

test.describe('provider detail', () => {
  let providerId: string;

  test.beforeEach(async ({ request, page }) => {
    // Create a provider via API
    const loginResp = await request.post('/api/v1/auth/login', {
      data: { username: 'admin', password: 'admin123456' },
    });
    expect(loginResp.ok()).toBeTruthy();
    const { token } = await loginResp.json();

    const resp = await request.post('/api/v1/providers', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        name: 'Test Provider',
        openai_base_url: 'https://api.openai.com/v1',
      },
    });
    expect(resp.ok()).toBeTruthy();
    const provider = await resp.json();
    providerId = provider.id;
    expect(providerId).toBeDefined();

    // Login via browser
    await page.goto('/console/login');
    await page.fill('input[id="username"]', 'admin');
    await page.fill('input[id="password"]', 'admin123456');
    await page.click('button[type="submit"]');
    await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('shows provider detail page', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);

    // Back button visible
    await expect(page.getByText('Back to Providers')).toBeVisible();

    // Provider card with name and form fields
    await expect(page.getByText('Test Provider', { exact: false })).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Name' })).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'OpenAI Base URL' })).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Anthropic Base URL' })).toBeVisible();
    await expect(page.getByText('Enabled')).toBeVisible();

    // Provider URL pre-filled
    await expect(page.locator('input#name')).toHaveValue('Test Provider');
    await expect(page.locator('input#openai_base_url')).toHaveValue('https://api.openai.com/v1');

    // Save and Delete buttons
    await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Delete Provider' })).toBeVisible();

    // Models and Channels cards
    await expect(page.getByRole('heading', { name: 'Models' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add Model' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Channels' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add Channel' })).toBeVisible();
  });

  test('navigates back to providers list', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);
    await page.getByText('Back to Providers').click();
    await expect(page).toHaveURL(/\/console\/providers\/?$/);
  });

  test('adds a model to the provider', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);

    await page.getByRole('button', { name: 'Add Model' }).click();

    // Modal should be open
    await expect(page.locator('.ant-modal-title')).toHaveText('Add Model');
    await expect(page.getByText('Model Name')).toBeVisible();

    // Scope fills to modal to avoid ID collision with provider form
    await page.locator('.ant-modal input#name').fill('gpt-4o');
    await page.getByLabel('Billing Type').click();
    await page.locator('.ant-select-item-option').getByText('Token-based').click();
    await page.locator('.ant-modal input#input_price').fill('5.00');
    await page.locator('.ant-modal input#output_price').fill('15.00');
    await page.locator('.ant-modal').getByRole('button', { name: 'Create' }).click();

    // Modal should close, model should appear in table
    await expect(page.getByText('gpt-4o')).toBeVisible();
    await expect(page.locator('.ant-tag-blue').filter({ hasText: 'token' })).toBeVisible();
  });

  test('adds a channel to the provider', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);

    await page.getByRole('button', { name: 'Add Channel' }).click();

    await expect(page.locator('.ant-modal-title')).toHaveText('Add Channel');
    await expect(page.locator('.ant-modal').getByText('Name')).toBeVisible();
    await expect(page.locator('.ant-modal').getByText('API Key')).toBeVisible();

    // Scope fills to modal to avoid ID collision with provider form
    await page.locator('.ant-modal input#name').fill('primary');
    await page.locator('.ant-modal input#api_key').fill('sk-test-key-12345');
    await page.locator('.ant-modal input#base_url').fill('https://custom.api.com/v1');
    await page.locator('.ant-modal').getByRole('button', { name: 'Create' }).click();

    // Channel should appear in table
    await expect(page.getByText('primary')).toBeVisible();
    await expect(page.getByText('https://custom.api.com/v1')).toBeVisible();
  });
});
