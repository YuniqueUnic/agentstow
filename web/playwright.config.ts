import { defineConfig } from '@playwright/test';

const port = Number(process.env.PLAYWRIGHT_AGENTSTOW_PORT ?? '8877');
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: 'list',
  timeout: 30_000,
  expect: {
    timeout: 5_000
  },
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure'
  },
  webServer: {
    command: 'node ./tests/e2e/start-agentstow-server.mjs',
    url: baseURL,
    reuseExistingServer: false,
    timeout: 120_000
  }
});
