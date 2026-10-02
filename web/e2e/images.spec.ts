import { expect, test, autosaved, colors, drag, open, place, png, preflight, screen } from './util'

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

test('a placed image snaps to the page edge', async ({ page }) => {
  await open(page)
  await place(page, red(600, 300), [25.4 + 0.7, 50])
  const x = page.getByRole('complementary', { name: 'Properties' }).getByRole('textbox', { name: 'X in mm' })
  await expect(x).toHaveValue('0')
})

test('a chosen image follows the pointer as a frame until a click places it, and escape drops it', async ({ page }) => {
  await open(page)
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await chooser).setFiles(red(600, 300))
  const button = page.getByRole('button', { name: 'Place image' })
  await expect(button).toHaveAttribute('aria-pressed', 'true')
  const layers = page.getByRole('tree', { name: 'Layers' })
  const [x, y] = await screen(page, 40, 40)
  await page.mouse.move(x, y)
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toHaveCount(0)
  await page.keyboard.down('Control')
  await page.mouse.click(x, y)
  await page.keyboard.up('Control')
  await expect(layers.getByRole('treeitem', { name: 'red.png', selected: true })).toBeVisible()
  const properties = page.getByRole('complementary', { name: 'Properties' })
  await expect(properties.getByRole('textbox', { name: 'X in mm' })).toHaveValue('14.6')

  const again = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await again).setFiles(red(600, 300))
  await expect(button).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('Escape')
  await expect(button).toHaveAttribute('aria-pressed', 'false')
  await page.mouse.click(x, y)
  await expect(layers.getByRole('treeitem', { name: 'red.png' })).toHaveCount(1)
})

test('a file that is not an image is not placed and says why', async ({ page }) => {
  await open(page)
  await place(page, { name: 'notes.png', mimeType: 'image/png', buffer: Buffer.from('notes') })
  await expect(page.getByText('Could not place notes.png: not a PNG or JPEG image.')).toBeVisible()
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem', { name: 'notes.png' })).toHaveCount(0)
})

test('ctrl and the crop button crop an image, a drag moves it in its frame, and the bar adjusts it', async ({ page }) => {
  await open(page)
  await place(page, png('halves.png', 600, 300, (x) => (x < 300 ? [255, 0, 0] : [0, 0, 255])))
  const at = async (x: number) => (await colors(page, [await screen(page, x, 105)]))[0]
  await expect.poll(async () => isRed(await at(68.6))).toBe(true)
  await page.keyboard.down('Control')
  await drag(page, await screen(page, 99.4, 105), await screen(page, 74, 105))
  await page.keyboard.up('Control')
  const properties = page.getByRole('complementary', { name: 'Properties' })
  await expect(properties.getByRole('textbox', { name: 'W in mm' })).toHaveValue('25.4')
  expect(isRed(await at(68.6))).toBe(true)

  const quick = page.getByRole('toolbar', { name: 'Quick edit' })
  await quick.getByTitle('Crop').click()
  await expect(quick.getByTitle('Crop')).toHaveAttribute('aria-pressed', 'true')
  await drag(page, await screen(page, 60, 105), await screen(page, 70, 105))
  await expect(properties.getByRole('textbox', { name: 'X in mm' })).toHaveValue('48.6')
  expect(isRed(await at(52))).toBe(false)
  expect(isRed(await at(62))).toBe(true)
  await page.keyboard.press('Escape')
  await expect(quick.getByTitle('Crop')).toHaveAttribute('aria-pressed', 'false')

  await quick.getByTitle('Adjust image').click()
  const brightness = page.getByRole('dialog', { name: 'Adjust image' }).getByRole('textbox', { name: 'Brightness' })
  await brightness.fill('-100')
  await brightness.press('Enter')
  await expect.poll(async () => Math.max(...(await at(62)))).toBeLessThan(10)
})

test('the fit menu sizes the frame to its image', async ({ page }) => {
  await open(page)
  await place(page, red(600, 300))
  const height = page.getByRole('complementary', { name: 'Properties' }).getByRole('textbox', { name: 'H in mm' })
  await height.fill('50.8')
  await height.press('Enter')
  await page.getByRole('toolbar', { name: 'Quick edit' }).getByTitle('Fit image').click()
  await page.getByRole('menuitem', { name: 'Frame to image' }).click()
  await expect(height).toHaveValue('25.4')
})
