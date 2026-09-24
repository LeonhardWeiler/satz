import { expect, test } from '@playwright/test'
import { drag, open, pixels, screen } from './util'

const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

test('moving a gradient layer leaves the page and the handles alone', async ({ page }) => {
  await open(page)
  const sun = await screen(page, 23, 37)
  await page.keyboard.down('Control')
  await drag(page, sun, [sun[0] + 40, sun[1]])
  await page.keyboard.up('Control')
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })).toHaveText('Sun')

  const [wx, wy] = await screen(page, 140, 85)
  for (const p of await pixels(page, wx, wy, 4, 4)) expect(p).toEqual([255, 255, 255])
  const [tx, ty] = await screen(page, 23, 22)
  const edge = await pixels(page, tx + 40 - 1, ty - 3, 3, 7)
  expect(edge.some((p) => near(p, [13, 153, 255]))).toBe(true)
})
