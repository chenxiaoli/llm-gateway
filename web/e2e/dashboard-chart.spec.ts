import { test, expect } from '@playwright/test';

// Generate a JWT token manually
function makeJWT(sub: string, role: string, secret: string): string {
  const { createHmac } = require('crypto');
  const base64url = (buf: Buffer) => buf.toString('base64url');

  const header = base64url(Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })));
  const now = Math.floor(Date.now() / 1000);
  const payload = base64url(Buffer.from(JSON.stringify({ sub, role, exp: now + 86400, iat: now })));
  const sig = base64url(createHmac('sha256', secret).update(`${header}.${payload}`).digest());
  return `${header}.${payload}.${sig}`;
}

test('dashboard chart shows date labels on XAxis', async ({ page }) => {
  // Set auth token directly
  const token = makeJWT('chenxl', 'admin', 'change-me-in-production');
  await page.goto('/console/login');
  await page.evaluate((t) => {
    localStorage.setItem('llm_gateway_admin_token', t);
    localStorage.setItem('llm_gateway_refresh_token', 'rt_dummy');
  }, token);

  // Navigate to dashboard
  await page.goto('/console/dashboard');
  await page.waitForTimeout(3000); // Wait for data to load

  // Take a screenshot for visual inspection
  await page.screenshot({ path: 'e2e/dashboard-chart.png', fullPage: true });

  // Check for the chart container
  const chartContainer = page.locator('.recharts-responsive-container');
  await expect(chartContainer).toBeVisible({ timeout: 10000 });

  // Check for XAxis tick text elements
  const xTicks = page.locator('.recharts-cartesian-axis-tick-value');
  const tickCount = await xTicks.count();
  console.log(`Found ${tickCount} XAxis tick elements`);

  // Log all tick text content
  for (let i = 0; i < tickCount; i++) {
    const text = await xTicks.nth(i).textContent();
    const fill = await xTicks.nth(i).getAttribute('fill');
    console.log(`  Tick ${i}: text="${text}", fill="${fill}"`);
  }

  // Check the SVG for recharts text elements in general
  const allSvgTexts = page.locator('.recharts-text');
  const svgTextCount = await allSvgTexts.count();
  console.log(`\nTotal recharts text elements: ${svgTextCount}`);
  for (let i = 0; i < svgTextCount; i++) {
    const text = await allSvgTexts.nth(i).textContent();
    const fill = await allSvgTexts.nth(i).getAttribute('fill');
    console.log(`  Text ${i}: "${text}", fill="${fill}"`);
  }

  // Check console errors
  page.on('console', (msg) => {
    if (msg.type() === 'error') console.log('CONSOLE ERROR:', msg.text());
  });

  // Assert at least one tick exists
  expect(tickCount).toBeGreaterThan(0);
});
