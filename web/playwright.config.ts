import { defineConfig, devices } from '@playwright/test'
import os from 'node:os'

const env = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !['WAYLAND_DISPLAY', 'DISPLAY', 'LD_LIBRARY_PATH'].includes(k)),
) as Record<string, string>

const port = process.env.PORT ?? '4173'
const url = `http://localhost:${port}/`

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  workers: process.env.CI ? 2 : Math.max(1, Math.min(os.availableParallelism() / 2, Math.floor(os.freemem() / 1.5e9))),
  timeout: 60_000,
  expect: { timeout: 15_000 },
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: { baseURL: url, colorScheme: 'dark', serviceWorkers: 'block', trace: 'retain-on-first-failure', contextOptions: { reducedMotion: 'reduce' } },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          env,
          args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
        },
      },
    },
  ],
  webServer: {
    command: `pnpm preview --port ${port} --strictPort`,
    url,
  },
})
