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
