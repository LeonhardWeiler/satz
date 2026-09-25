import { expect, test, type Locator, type Page } from '@playwright/test'
import { open, pixels, screen } from './util'

const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

async function expectInside(page: Page, el: Locator) {
  const box = (await el.boundingBox())!
  const { width, height } = page.viewportSize()!
  expect(box.x).toBeGreaterThanOrEqual(0)
  expect(box.y).toBeGreaterThanOrEqual(0)
  expect(box.x + box.width).toBeLessThanOrEqual(width)
  expect(box.y + box.height).toBeLessThanOrEqual(height)
}

test('the colour picker and the shape menu stay inside the window', async ({ page }) => {
  await open(page)
  await page.setViewportSize({ width: 1400, height: 640 })
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Sun' }).click()
  await page.getByRole('button', { name: 'Shadow color' }).click()
  const picker = page.getByRole('dialog', { name: 'Shadow color' })
  await expect(picker).toBeVisible()
  await expectInside(page, picker)
  await page.keyboard.press('Escape')
  await expect(picker).toBeHidden()
  await expect(page.getByRole('button', { name: 'Shadow color' })).toBeFocused()

  await page.setViewportSize({ width: 1000, height: 200 })
  await page.getByRole('button', { name: 'Shape tools' }).click()
  await expectInside(page, page.getByRole('menu', { name: 'Shape tools' }))
})

test('hex, saturation, hue and alpha change the fill', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  const picker = page.getByRole('dialog', { name: 'Fill color' })
  const [x, y] = await screen(page, 74, 7)
  const color = async () => (await pixels(page, x, y, 1, 1))[0]

  await picker.getByRole('textbox', { name: 'Hex' }).fill('00ff00')
  await picker.getByRole('textbox', { name: 'Hex' }).press('Enter')
  expect(near(await color(), [0, 255, 0])).toBe(true)

  const area = (await picker.getByRole('slider', { name: 'Saturation and brightness' }).boundingBox())!
  await page.mouse.click(area.x + 1, area.y + area.height - 1)
  expect(near(await color(), [0, 0, 0])).toBe(true)
  await page.mouse.click(area.x + area.width - 1, area.y + 1)
  expect(near(await color(), [0, 255, 0])).toBe(true)

  await picker.getByRole('textbox', { name: 'Hex' }).fill('0f0')
  await picker.getByRole('textbox', { name: 'Hex' }).press('Enter')
  const hue = picker.getByRole('slider', { name: 'Hue' })
  await hue.focus()
  await hue.press('Home')
  expect(near(await color(), [255, 0, 0])).toBe(true)
  await expect(picker.getByRole('textbox', { name: 'Hex' })).toHaveValue('FF0000')

  await picker.getByRole('slider', { name: 'Opacity' }).fill('40')
  await expect(panel.getByTitle('Fill opacity').getByRole('textbox')).toHaveValue('40')

  await page.mouse.click(10, 10)
  await expect(picker).toBeHidden()
  await page.keyboard.press('Control+z')
  await expect(panel.getByTitle('Fill opacity').getByRole('textbox')).toHaveValue('100')
})
