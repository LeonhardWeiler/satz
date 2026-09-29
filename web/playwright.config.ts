import { defineConfig, devices } from '@playwright/test'

const env = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => k !== 'WAYLAND_DISPLAY' && k !== 'DISPLAY'),
) as Record<string, string>

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  workers: 2,
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: { baseURL: 'http://localhost:4173/satz/', contextOptions: { reducedMotion: 'reduce' } },
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
    command: 'pnpm preview --port 4173 --strictPort',
    url: 'http://localhost:4173/satz/',
  },
})
