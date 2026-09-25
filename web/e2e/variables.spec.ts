import { expect, test, type Locator, type Page } from '@playwright/test'
import { PNG } from 'pngjs'
import { open, screen } from './util'

const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

test('collections, modes and variables are made and edited in the local variables dialog', async ({ page }) => {
  await open(page)
  await page.getByRole('button', { name: 'Local variables' }).click()
  const dialog = page.getByRole('dialog', { name: 'Local variables' })
  await dialog.getByRole('button', { name: 'Create collection' }).click()
  await expect(dialog.getByRole('textbox', { name: 'Collection name' })).toHaveValue('Collection 1')

  await dialog.getByRole('button', { name: 'Create variable' }).click()
  await page.getByRole('menuitem', { name: 'Color' }).click()
  const name = dialog.getByRole('textbox', { name: 'Variable name' })
  await expect(name).toHaveValue('Color 1')
  await name.fill('Brand')
  await name.press('Enter')
  await dialog.getByRole('button', { name: 'Add mode' }).click()
  await expect(dialog.getByRole('textbox', { name: 'Mode name' })).toHaveCount(2)
  await expect(dialog.getByRole('textbox', { name: 'Mode name' }).last()).toHaveValue('Mode 2')

  await dialog.getByRole('button', { name: 'Brand in Mode 2' }).click()
  const picker = page.getByRole('dialog', { name: 'Brand in Mode 2' })
  await picker.getByRole('textbox', { name: 'Hex' }).fill('ff0000')
  await picker.getByRole('textbox', { name: 'Hex' }).press('Enter')
  await page.keyboard.press('Escape')
  await expect(picker).toBeHidden()
  await expect(dialog).toBeVisible()
  await expect(dialog.getByRole('button', { name: 'Brand in Mode 2' }).locator('.chip')).toHaveCSS('--color', '#ff0000ff')

  await dialog.getByRole('button', { name: 'Create variable' }).click()
  await page.getByRole('menuitem', { name: 'Number' }).click()
  const size = dialog.getByRole('textbox', { name: 'Number 1 in Mode 1' })
  await size.fill('4')
  await size.press('Enter')
  await expect(size).toHaveValue('4')
  await expect(dialog.getByRole('textbox', { name: 'Number 1 in Mode 2' })).toHaveValue('0')

  await page.keyboard.press('Escape')
  await expect(dialog).toBeHidden()
  await page.keyboard.press('Control+z')
  await page.getByRole('button', { name: 'Local variables' }).click()
  await expect(dialog.getByRole('textbox', { name: 'Number 1 in Mode 1' })).toHaveValue('0')
  await dialog.getByRole('button', { name: 'Delete variable Number 1' }).click()
  await expect(dialog.getByRole('textbox', { name: 'Variable name' })).toHaveCount(1)
})

async function theme(page: Page) {
  await page.getByRole('button', { name: 'Local variables' }).click()
  const dialog = page.getByRole('dialog', { name: 'Local variables' })
  await dialog.getByRole('button', { name: 'Create collection' }).click()
  await dialog.getByRole('button', { name: 'Create variable' }).click()
  await page.getByRole('menuitem', { name: 'Color' }).click()
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
  const colors = async () => {
    const points = [await screen(page, 74, 7), await screen(page, 100, 185)]
    await page.waitForTimeout(100)
    const png = PNG.sync.read(await page.screenshot())
    return points.map(([x, y]) => {
      const i = (Math.round(y) * png.width + Math.round(x)) * 4
      return [png.data[i], png.data[i + 1], png.data[i + 2]]
    })
  }
  const expectColors = async (top: number[], inFrame: number[]) => {
    const [a, b] = await colors()
    expect(near(a, top) && near(b, inFrame), `${a} ${b}`).toBe(true)
  }
  const [red, blue] = [[255, 0, 0], [0, 0, 255]]
  await expectColors(red, red)

  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await panel.getByRole('combobox', { name: 'Collection 1 mode' }).selectOption('Mode 2')
  await expectColors(blue, blue)

  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  const frameMode = panel.getByRole('combobox', { name: 'Collection 1 mode' })
  await expect(frameMode.getByRole('option', { selected: true })).toHaveText('Auto (Mode 2)')
  await frameMode.selectOption('Mode 1')
  await expectColors(blue, red)
})

test('a number variable binds to width and detaches with the value of the current mode', async ({ page }) => {
  await open(page)
  await page.getByRole('button', { name: 'Local variables' }).click()
  const dialog = page.getByRole('dialog', { name: 'Local variables' })
  await dialog.getByRole('button', { name: 'Create collection' }).click()
  await dialog.getByRole('button', { name: 'Create variable' }).click()
  await page.getByRole('menuitem', { name: 'Number' }).click()
  await dialog.getByRole('button', { name: 'Add mode' }).click()
  for (const [mode, v] of [['Mode 1', '10'], ['Mode 2', '20']]) {
    await dialog.getByRole('textbox', { name: `Number 1 in ${mode}` }).fill(v)
    await dialog.getByRole('textbox', { name: `Number 1 in ${mode}` }).press('Enter')
  }
  await page.keyboard.press('Escape')

  const panel = page.getByRole('complementary', { name: 'Properties' })
  await panel.getByRole('combobox', { name: 'Collection 1 mode' }).selectOption('Mode 2')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await panel.getByRole('button', { name: 'Apply variable to W in mm' }).click()
  await page.getByRole('listbox', { name: 'Number variables' }).getByRole('option', { name: 'Number 1' }).click()
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveCount(0)
  await expect(panel.getByRole('button', { name: 'W in mm: Number 1' })).toBeVisible()
  await panel.getByRole('button', { name: 'Detach variable from W in mm' }).click()
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveValue('20')
})
