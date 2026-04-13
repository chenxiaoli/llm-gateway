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
    await page.getByText('Create one').click();
    await expect(page).toHaveURL(/\/console\/register/);

    await page.getByPlaceholder('Username').fill('admin');
    await page.getByPlaceholder('Password').fill('admin123456');
    await page.getByPlaceholder('Confirm password').fill('admin123456');
    await page.getByRole('button', { name: 'Register' }).click();

    await expect(page).toHaveURL(/\/console\/dashboard/);
    await expect(page.getByText('admin', { exact: true })).toBeVisible();
  });

  test('login with existing user', async ({ page }) => {
    await page.goto('/console/login');
    await page.getByPlaceholder('Username').fill('admin');
    await page.getByPlaceholder('Password').fill('admin123456');
    await page.getByRole('button', { name: 'Sign In' }).click();

    await expect(page).toHaveURL(/\/console\/dashboard/);
  });
});

test.describe('dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/console/login');
    await page.getByPlaceholder('Username').fill('admin');
    await page.getByPlaceholder('Password').fill('admin123456');
    await page.getByRole('button', { name: 'Sign In' }).click();
    await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('shows dashboard with stat cards', async ({ page }) => {
    await expect(page.getByText("Today's Requests")).toBeVisible();
    await expect(page.getByText("Today's Cost")).toBeVisible();
    await expect(page.getByText('Monthly Cost')).toBeVisible();
    await expect(page.getByText('Active Models')).toBeVisible();
  });

  test('shows version in console footer', async ({ page }) => {
    await expect(page.getByText(/LLM Gateway v\d/)).toBeVisible();
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
    await page.getByPlaceholder('Username').fill('admin');
    await page.getByPlaceholder('Password').fill('admin123456');
    await page.getByRole('button', { name: 'Sign In' }).click();
    await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('shows provider detail page', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);

    // Back button visible
    await expect(page.getByText('Back to Providers')).toBeVisible();

    // Provider card with name and form fields
    await expect(page.getByText('Test Provider', { exact: false })).toBeVisible();
    await expect(page.getByText('OpenAI Base URL')).toBeVisible();
    await expect(page.getByText('Anthropic Base URL')).toBeVisible();
    await expect(page.getByText('Enabled')).toBeVisible();

    // Save and Delete buttons
    await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Delete Provider' })).toBeVisible();

    // Models and Channels sections
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
    await expect(page.getByText('Add Model')).toBeVisible();
    await expect(page.getByText('Model Name')).toBeVisible();

    // Fill model form
    await page.getByPlaceholder('e.g., gpt-4o').fill('gpt-4o');
    // Select billing type - the custom Select uses a native <select>
    await page.locator('select').selectOption('token');
    // Fill prices
    await page.getByPlaceholder('Input ($/1M)').fill('5.00');
    await page.getByPlaceholder('Output ($/1M)').fill('15.00');
    await page.getByRole('button', { name: 'Create' }).click();

    // Model should appear in table
    await expect(page.getByText('gpt-4o')).toBeVisible();
  });

  test('adds a channel to the provider', async ({ page }) => {
    await page.goto(`/console/providers/${providerId}`);

    await page.getByRole('button', { name: 'Add Channel' }).click();

    await expect(page.getByText('Add Channel')).toBeVisible();
    await expect(page.getByText('Name')).toBeVisible();
    await expect(page.getByText('API Key')).toBeVisible();

    // Fill channel form
    await page.getByPlaceholder('e.g., primary').fill('primary');
    await page.getByPlaceholder('Upstream API key').fill('sk-test-key-12345');
    await page.getByPlaceholder('Leave empty to use provider default').fill('https://custom.api.com/v1');
    await page.getByRole('button', { name: 'Create' }).click();

    // Channel should appear in table
    await expect(page.getByText('primary')).toBeVisible();
    await expect(page.getByText('https://custom.api.com/v1')).toBeVisible();
  });
});
