import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.e2e.ts',
  timeout: 30_000,
  fullyParallel: false,
  use: {
    baseURL: 'http://127.0.0.1:4173',
    headless: true,
    launchOptions: {
      executablePath: '/usr/bin/chromium',
    },
  },
  webServer: [
    {
      command: 'cargo run',
      url: 'http://127.0.0.1:8080/api/tasks',
      cwd: '../backend',
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'pnpm dev --host 127.0.0.1 --port 4173',
      url: 'http://127.0.0.1:4173',
      cwd: '.',
      reuseExistingServer: true,
      timeout: 120_000,
    },
  ],
})
