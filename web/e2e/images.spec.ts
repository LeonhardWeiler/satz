import { expect, test } from '@playwright/test'
import { autosaved, colors, open, place, png, screen } from './util'

const red = (width: number, height: number) => png('red.png', width, height, () => [255, 0, 0])

const isRed = ([r, g, b]: number[]) => r > 240 && g < 15 && b < 15

test('a placed image draws at 300 ppi, preflight reports it enlarged, and it stays after a reload', async ({ page }) => {
  await open(page)
  await place(page, red(600, 300))
  const layers = page.getByRole('tree', { name: 'Layers' })
  await expect(layers.getByRole('treeitem', { name: 'red.png', selected: true })).toBeVisible()
  const properties = page.getByRole('complementary', { name: 'Properties' })
  await expect(properties.getByText('Image · 300 ppi')).toBeVisible()
  const width = properties.getByRole('textbox', { name: 'W in mm' })
  await expect(width).toHaveValue('50.8')
  const middle = await screen(page, 148 / 2, 210 / 2)
  expect((await colors(page, [middle])).map(isRed)).toEqual([true])

  await width.fill('101.6')
  await width.press('Enter')
  await expect(page.getByRole('region', { name: 'Preflight' }).getByRole('button', { name: /red\.png.*Image at 150 ppi/ })).toBeVisible()

  await autosaved(page)
  await page.reload()
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toBeVisible()
  expect((await colors(page, [middle])).map(isRed)).toEqual([true])
})

test('a file that is not an image is not placed and says why', async ({ page }) => {
  await open(page)
  await place(page, { name: 'notes.png', mimeType: 'image/png', buffer: Buffer.from('notes') })
  await expect(page.getByText('Could not place notes.png: not a PNG or JPEG image. Choose a PNG or JPEG file.')).toBeVisible()
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem', { name: 'notes.png' })).toHaveCount(0)
})
