import { type Locator, type Page } from '@playwright/test'
import { expect, test, addVariable, colors, open, screen, choose } from './util'

const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

test('collections, modes and variables are made with the plus and edited in a popover', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const edit = await addVariable(page, 'Color')
  await expect(panel.getByRole('textbox', { name: 'Collection name' })).toHaveValue('Collection 1')
  const name = edit.getByRole('textbox', { name: 'Variable name' })
  await expect(name).toHaveValue('Color 1')
  await name.fill('Brand')
  await name.press('Enter')
  const brand = page.getByRole('dialog', { name: 'Edit Brand' })
  await brand.getByRole('button', { name: 'Add mode' }).click()
  await expect(brand.getByRole('textbox', { name: 'Mode name' })).toHaveCount(2)
  await expect(brand.getByRole('textbox', { name: 'Mode name' }).last()).toHaveValue('Mode 2')

  await brand.getByRole('button', { name: 'Brand in Mode 2' }).click()
  const picker = page.getByRole('dialog', { name: 'Brand in Mode 2' })
  await picker.getByRole('textbox', { name: 'Hex' }).fill('ff0000')
  await picker.getByRole('textbox', { name: 'Hex' }).press('Enter')
  await page.keyboard.press('Escape')
  await expect(picker).toBeHidden()
  await expect(brand).toBeVisible()
  await expect(brand.getByRole('button', { name: 'Brand in Mode 2' }).locator('.chip')).toHaveCSS('--color', '#ff0000ff')
  await page.keyboard.press('Escape')
  await expect(brand).toBeHidden()

  const number = await addVariable(page, 'Number')
  const size = number.getByRole('textbox', { name: 'Number 1 in Mode 1' })
  await size.fill('4')
  await size.press('Enter')
  await expect(size).toHaveValue('4')
  await expect(number.getByRole('textbox', { name: 'Number 1 in Mode 2' })).toHaveValue('0')
  await page.keyboard.press('Escape')
  await expect(number).toBeHidden()
  await page.keyboard.press('Control+z')
  await panel.getByRole('button', { name: /^Number 1/ }).click()
  await expect(size).toHaveValue('0')
  await number.getByRole('button', { name: 'Delete variable' }).click()
  await expect(panel.getByRole('button', { name: /^Number 1/ })).toHaveCount(0)
  await expect(panel.getByRole('button', { name: /^Brand/ })).toHaveCount(1)
})

test('a font variable sets the font of a text in the mode of its page', async ({ page }) => {
  await open(page)
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('region', { name: 'Fonts' }).getByRole('button', { name: 'Add font' }).click()
  await (await chooser).setFiles(new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const edit = await addVariable(page, 'Font')
  await edit.getByRole('button', { name: 'Add mode' }).click()
  await choose(edit.getByRole('combobox', { name: 'Font 1 in Mode 2' }), 'DM Mono Regular')
  await page.keyboard.press('Escape')
  await expect(edit).toBeHidden()
  await choose(panel.getByRole('combobox', { name: 'Collection 1 mode' }), 'Collection 1: Mode 2')

  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  await panel.getByRole('button', { name: 'Apply variable to Font', exact: true }).click()
  await page.getByRole('listbox', { name: 'Font variables' }).getByRole('option', { name: 'Font 1' }).click()
  await expect(panel.getByRole('button', { name: 'Font: Font 1' })).toBeVisible()
  await panel.getByRole('button', { name: 'Detach variable from Font', exact: true }).click()
  await expect(panel.getByRole('combobox', { name: 'Font' })).toHaveText('DM Mono Regular')
})

async function theme(page: Page) {
  const dialog = await addVariable(page, 'Color')
  await dialog.getByRole('button', { name: 'Add mode' }).click()
  for (const [mode, hex] of [['Mode 1', 'ff0000'], ['Mode 2', '0000ff']]) {
    await dialog.getByRole('button', { name: `Color 1 in ${mode}` }).click()
    const hexField = page.getByRole('dialog', { name: `Color 1 in ${mode}` }).getByRole('textbox', { name: 'Hex' })
    await hexField.fill(hex)
    await hexField.press('Enter')
    await page.keyboard.press('Escape')
  }
  await page.keyboard.press('Escape')
  await expect(dialog).toBeHidden()
}

test('a fill bound to a colour variable follows the mode of its frame and page', async ({ page }) => {
  await open(page)
  await theme(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const bind = async (layer: Locator) => {
    await layer.click()
    await panel.getByRole('button', { name: 'Fill color' }).click()
    const picker = page.getByRole('dialog', { name: 'Fill color' })
    await picker.getByRole('tab', { name: 'Swatches' }).click()
    await picker.getByRole('option', { name: 'Color 1' }).click()
    await page.keyboard.press('Escape')
  }
  const rects = layers.getByRole('button', { name: 'Rectangle', exact: true })
  await bind(rects.last())
  await bind(rects.first())
  await expect(panel.getByText('Color 1')).toBeVisible()
  /** Colours of the top rectangle and of the one in the frame, from one screenshot. */
  const shown = async () => colors(page, [await screen(page, 74, 7), await screen(page, 100, 185)])
  const expectColors = async (top: number[], inFrame: number[]) => {
    const [a, b] = await shown()
    expect(near(a, top) && near(b, inFrame), `${a} ${b}`).toBe(true)
  }
  const [red, blue] = [[255, 0, 0], [0, 0, 255]]
  await expectColors(red, red)

  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await choose(panel.getByRole('combobox', { name: 'Collection 1 mode' }), 'Collection 1: Mode 2')
  await expectColors(blue, blue)

  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  const frameMode = panel.getByRole('combobox', { name: 'Collection 1 mode' })
  await expect(frameMode).toHaveText('Collection 1: auto (Mode 2)')
  await choose(frameMode, 'Collection 1: Mode 1')
  await expectColors(blue, red)
})

test('a number variable binds to width and detaches with the value of the current mode', async ({ page }) => {
  await open(page)
  const dialog = await addVariable(page, 'Number')
  await dialog.getByRole('button', { name: 'Add mode' }).click()
  for (const [mode, v] of [['Mode 1', '10'], ['Mode 2', '20']]) {
    await dialog.getByRole('textbox', { name: `Number 1 in ${mode}` }).fill(v)
    await dialog.getByRole('textbox', { name: `Number 1 in ${mode}` }).press('Enter')
  }
  await page.keyboard.press('Escape')

  const panel = page.getByRole('complementary', { name: 'Properties' })
  await choose(panel.getByRole('combobox', { name: 'Collection 1 mode' }), 'Collection 1: Mode 2')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await panel.getByRole('button', { name: 'Apply variable to W in mm' }).click()
  await page.getByRole('listbox', { name: 'Number variables' }).getByRole('option', { name: 'Number 1' }).click()
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveCount(0)
  await expect(panel.getByRole('button', { name: 'W in mm: Number 1' })).toBeVisible()
  await panel.getByRole('button', { name: 'Detach variable from W in mm' }).click()
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveValue('20')
})
