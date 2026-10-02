import { expect, test, open } from './util'

test.use({ serviceWorkers: 'allow' })

test('satz installs and opens again offline', async ({ page, context }) => {
  await open(page)
  await page.evaluate(() => navigator.serviceWorker.ready)
  await expect(page.locator('link[rel=manifest]')).toHaveAttribute('href', '/satz/manifest.webmanifest')
  await context.setOffline(true)
  await page.reload()
  await expect(page.getByRole('tree', { name: 'Layers' })).toBeVisible()
})
