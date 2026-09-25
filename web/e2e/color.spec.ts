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

test('a cmyk document shows and takes cmyk values in the picker', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  await panel.getByRole('combobox', { name: 'Color mode' }).selectOption('CMYK')

  await rects.last().click()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  const picker = page.getByRole('dialog', { name: 'Fill color' })
  await expect(picker.getByRole('textbox', { name: 'Hex' })).toHaveCount(0)
  const field = (name: string) => picker.getByRole('textbox', { name })
  for (const [name, v] of [['Cyan', '100'], ['Magenta', '0'], ['Yellow', '0'], ['Black', '0']]) {
    await field(name).fill(v)
    await field(name).press('Enter')
  }
  const [x, y] = await screen(page, 74, 7)
  expect(near((await pixels(page, x, y, 1, 1))[0], [0, 163, 228])).toBe(true)

  const hue = picker.getByRole('slider', { name: 'Hue' })
  await hue.focus()
  await hue.press('Home')
  expect(Number(await field('Cyan').inputValue())).toBeLessThan(5)
  expect(Number(await field('Magenta').inputValue())).toBeGreaterThan(80)

  await page.keyboard.press('Escape')
  await expect(picker).toBeHidden()
  await page.keyboard.press('Escape')
  await page.keyboard.press('r')
  const [ax, ay] = await screen(page, 20, 90)
  await page.mouse.move(ax, ay)
  await page.mouse.down()
  await page.mouse.move(ax + 60, ay + 40, { steps: 3 })
  await page.mouse.up()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  await expect(field('Black')).toHaveValue('15')
})

test('a spot swatch is made in the swatches panel, bound in the picker, tinted and deleted', async ({ page }) => {
  await open(page)
  const swatches = page.getByRole('region', { name: 'Swatches' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await swatches.getByRole('button', { name: 'Add swatch' }).click()
  const editor = page.getByRole('dialog', { name: 'Edit swatch' })
  await editor.getByRole('textbox', { name: 'Name' }).fill('HKS 43')
  await editor.getByRole('checkbox', { name: 'Spot color' }).check()
  for (const [name, v] of [['Cyan', '100'], ['Magenta', '60'], ['Yellow', '0'], ['Black', '0']]) {
    await editor.getByRole('textbox', { name }).fill(v)
    await editor.getByRole('textbox', { name }).press('Enter')
  }
  await page.keyboard.press('Escape')
  await expect(editor).toBeHidden()
  await expect(swatches.getByRole('option', { name: 'HKS 43' })).toHaveAttribute('title', /Spot color/)

  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  const picker = page.getByRole('dialog', { name: 'Fill color' })
  await picker.getByRole('tab', { name: 'Swatches' }).click()
  await picker.getByRole('option', { name: 'HKS 43' }).click()
  await page.keyboard.press('Escape')
  const [x, y] = await screen(page, 74, 7)
  const [r, , b] = (await pixels(page, x, y, 1, 1))[0]
  expect(b - r).toBeGreaterThan(100)
  await expect(panel.getByText('HKS 43')).toBeVisible()
  await panel.getByRole('textbox', { name: 'Fill tint' }).fill('0')
  await panel.getByRole('textbox', { name: 'Fill tint' }).press('Enter')
  expect(near((await pixels(page, x, y, 1, 1))[0], [255, 255, 255])).toBe(true)
  await panel.getByRole('textbox', { name: 'Fill tint' }).fill('100')
  await panel.getByRole('textbox', { name: 'Fill tint' }).press('Enter')

  await swatches.getByRole('option', { name: 'HKS 43' }).dblclick()
  await editor.getByRole('button', { name: 'Delete swatch' }).click()
  await expect(swatches.getByRole('option')).toHaveCount(0)
  await expect(panel.getByRole('combobox', { name: 'Fill type' })).toHaveValue('solid')
  const [r2, , b2] = (await pixels(page, x, y, 1, 1))[0]
  expect(b2 - r2).toBeGreaterThan(100)
})

test('arrow keys in the saturation area do not move the layer', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  const x = page.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')
  const before = await x.inputValue()
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Fill color' }).click()
  const area = page.getByRole('dialog', { name: 'Fill color' }).getByRole('slider', { name: 'Saturation and brightness' })
  await area.focus()
  await area.press('ArrowRight')
  await expect(x).toHaveValue(before)
})

test('a new swatch never repeats the name of an existing one', async ({ page }) => {
  await open(page)
  const swatches = page.getByRole('region', { name: 'Swatches' })
  const editor = page.getByRole('dialog', { name: 'Edit swatch' })
  const add = async () => {
    await swatches.getByRole('button', { name: 'Add swatch' }).click()
    await page.keyboard.press('Escape')
  }
  await add()
  await add()
  await swatches.getByRole('option', { name: 'Swatch 1' }).dblclick()
  await editor.getByRole('button', { name: 'Delete swatch' }).click()
  await add()
  await expect(swatches.getByRole('option')).toHaveText(['Swatch 2', 'Swatch 3'])
})
