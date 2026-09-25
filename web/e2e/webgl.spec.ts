import { expect, test } from '@playwright/test'

test('without webgl 2 a notice replaces the canvas', async ({ page }) => {
  await page.addInitScript(() => {
    const proto = HTMLCanvasElement.prototype as unknown as Record<string, (...args: unknown[]) => unknown>
    const get = proto.getContext
    proto.getContext = function (this: unknown, ...args: unknown[]) {
      return args[0] === 'webgl2' ? null : get.apply(this, args)
    }
  })
  await page.goto('')
  await expect(page.getByRole('alert')).toContainText('WebGL 2 is not available')
})

test('the name and a loading bar show while the engine loads', async ({ page }) => {
  let release = () => {}
  const held = new Promise<void>((r) => (release = r))
  await page.route('**/*.wasm', async (route) => {
    await held
    await route.continue()
  })
  await page.goto('')
  await expect(page.getByRole('heading', { name: 'Satz' })).toBeVisible()
  await expect(page.getByRole('progressbar', { name: 'Loading Satz' })).toBeVisible()
  release()
  await expect(page.getByLabel('Page canvas')).toBeVisible()
  await expect(page.getByRole('progressbar')).toHaveCount(0)
})

test('a notice tells when the engine fails to load', async ({ page }) => {
  await page.route('**/*.wasm', (route) => route.abort())
  await page.goto('')
  await expect(page.getByRole('alert')).toContainText('Satz could not start')
  await expect(page.getByRole('progressbar')).toHaveCount(0)
})
