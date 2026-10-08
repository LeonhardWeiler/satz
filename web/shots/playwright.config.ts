import { defineConfig } from '@playwright/test'
import base from '../playwright.config'

export default defineConfig({
  ...base,
  testDir: '.',
  retries: 0,
  timeout: 60_000,
  reporter: 'list',
  webServer: { ...base.webServer!, cwd: '..' } as never,
})
