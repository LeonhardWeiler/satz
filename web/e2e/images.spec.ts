import { expect, test, autosaved, colors, open, place, png, preflight, screen } from './util'

const red = (width: number, height: number) => png('red.png', width, height, () => [255, 0, 0])

const isRed = ([r, g, b]: number[]) => r > 240 && g < 15 && b < 15

test('a placed image draws at 300 ppi, preflight reports it enlarged, and it stays after a reload', async ({ page }) => {
  await open(page)
  await place(page, red(600, 300))
  const layers = page.getByRole('tree', { name: 'Layers' })
  await expect(layers.getByRole('treeitem', { name: 'red.png', selected: true })).toBeVisible()
  const properties = page.getByRole('complementary', { name: 'Properties' })
  await expect(properties.getByText('Image · 300 ppi · RGB', { exact: true })).toBeVisible()
  const width = properties.getByRole('textbox', { name: 'W in mm' })
  await expect(width).toHaveValue('50.8')
  const middle = await screen(page, 148 / 2, 210 / 2)
  expect((await colors(page, [middle])).map(isRed)).toEqual([true])

  await width.fill('101.6')
  await width.press('Enter')
  await expect((await preflight(page)).getByRole('button', { name: /red\.png.*Image at 150 ppi/ })).toBeVisible()

  await autosaved(page)
  await page.reload()
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toBeVisible()
  expect((await colors(page, [middle])).map(isRed)).toEqual([true])
})

test('a chosen image follows the pointer as a frame until a click places it, and escape drops it', async ({ page }) => {
  await open(page)
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await chooser).setFiles(red(600, 300))
  const layers = page.getByRole('tree', { name: 'Layers' })
  const [x, y] = await screen(page, 40, 40)
  await page.mouse.move(x, y)
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toHaveCount(0)
  await page.mouse.click(x, y)
  await expect(layers.getByRole('treeitem', { name: 'red.png', selected: true })).toBeVisible()
  const properties = page.getByRole('complementary', { name: 'Properties' })
  await expect(properties.getByRole('textbox', { name: 'X in mm' })).toHaveValue('14.6')

  const again = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await again).setFiles(red(600, 300))
  await page.keyboard.press('Escape')
  await page.mouse.click(x, y)
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toHaveCount(1)
})

test('a file that is not an image is not placed and says why', async ({ page }) => {
  await open(page)
  await place(page, { name: 'notes.png', mimeType: 'image/png', buffer: Buffer.from('notes') })
  await expect(page.getByText('Could not place notes.png: not a PNG or JPEG image.')).toBeVisible()
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem', { name: 'notes.png' })).toHaveCount(0)
})
