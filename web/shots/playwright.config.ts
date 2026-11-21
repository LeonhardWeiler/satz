import { defineConfig } from '@playwright/test'
import base from '../playwright.config'

export default defineConfig({
  ...base,
  testDir: '.',
  retries: 0,
  timeout: 120_000,
  expect: { timeout: 45_000 },
  reporter: 'list',
  projects: (['dark', 'light'] as const).flatMap((scheme) => {
    const use = { ...base.projects![0].use, colorScheme: scheme, deviceScaleFactor: 2 }
    return [
      { name: scheme, use, testIgnore: 'audit.spec.ts' },
      { name: `${scheme} audit`, use, testMatch: 'audit.spec.ts', timeout: 180_000 },
    ]
  }),
  webServer: { ...base.webServer!, cwd: '..' },
})
