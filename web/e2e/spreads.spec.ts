import { expect, test } from '@playwright/test'
import { addPage, colors, current, drag, open, screen } from './util'

const gray = ([r, g, b]: number[]) => [r, g, b].every((c) => Math.abs(c - 0xd9) < 8)

test('a spread shows its pages at the spine, and layers are drawn, selected and moved across it', async ({ page }) => {
  await open(page)
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const title = panel.getByRole('heading', { level: 2 })
  const x = panel.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')
  const at = (i: number, px: number, py: number) => screen(page, px, py, i)
  await addPage(page)
  await addPage(page)
  await expect(current(page, 3)).toHaveAttribute('aria-pressed', 'true')

  await page.keyboard.press('r')
  await drag(page, await at(1, 20, 20), await at(1, 60, 50))
  await expect(rects).toHaveCount(1)
  await page.keyboard.press('r')
  await drag(page, await at(0, 100, 20), await at(0, 140, 50))
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await expect(rects).toHaveCount(1)
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  const [a, b, spine] = await colors(page, [await at(0, 120, 35), await at(1, 40, 35), await at(1, 1, 100)])
  expect([gray(a), gray(b)]).toEqual([true, true])
  expect(spine).toEqual([255, 255, 255])

  await page.mouse.click(...(await at(1, 40, 35)))
  await expect(current(page, 3)).toHaveAttribute('aria-pressed', 'true')
  await expect(title).toHaveText('Rectangle')
  await drag(page, await at(0, 120, 35), await at(0, 160, 35))
  await expect(current(page, 3)).toHaveAttribute('aria-pressed', 'true')
  await expect(rects).toHaveCount(2)
  await expect(x).toHaveValue('-8')
  await page.keyboard.press('Control+z')
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await expect(x).toHaveValue('100')

  await page.keyboard.press('Escape')
  await drag(page, await at(0, 90, -10), await at(1, 70, 60))
  await expect(title).toHaveText('2 layers')
})
