import { defineConfig } from '@playwright/test'
import base from '../playwright.config'

export default defineConfig({
  ...base,
  testDir: '.',
  retries: 0,
  timeout: 60_000,
  reporter: 'list',
  projects: (['dark', 'light'] as const).map((scheme) => ({ name: scheme, use: { ...base.projects![0].use, colorScheme: scheme, deviceScaleFactor: 2 } })),
  webServer: { ...base.webServer!, cwd: '..' },
})
