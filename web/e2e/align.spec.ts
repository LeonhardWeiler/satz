import { type Page } from '@playwright/test'
import { drag, expect, test, open, screen } from './util'

test('layers align to each other, a single layer to its frame, and three distribute', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const box = async (name: string) => {
    await layers.getByRole('button', { name, exact: true }).click()
    return Promise.all(['X in mm', 'W in mm'].map(async (f) => Number(await panel.getByRole('textbox', { name: f, exact: true }).inputValue())))
  }
  const x = async (name: string) => (await box(name))[0]
  const pick = async () => {
    await layers.getByRole('button', { name: 'Sun', exact: true }).click()
    await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  }
  await pick()
  await expect(panel.getByRole('button', { name: 'Distribute horizontal spacing' })).toHaveCount(0)
  await panel.getByRole('button', { name: 'Align left' }).click()
  expect(await x('Triangle')).toBe(await x('Sun'))
  await pick()
  await panel.getByRole('button', { name: 'Align to the parent frame or page' }).click()
  await panel.getByRole('button', { name: 'Align left' }).click()
  expect(await x('Triangle')).toBe(0)
  await page.keyboard.press('Control+z')
  await layers.getByRole('button', { name: 'Sun', exact: true }).click()

  await page.evaluate(() => document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'å', code: 'KeyA', altKey: true, bubbles: true, cancelable: true })))
  expect(await x('Sun')).toBe(0)

  await layers.getByRole('button', { name: 'Star', exact: true }).click({ modifiers: ['Control'] })
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await page.keyboard.press('Alt+Shift+H')
  await page.keyboard.press('Control+z')
  await page.keyboard.press('Control+Shift+z')
  const [[sx, sw], [tx, tw], [rx]] = [await box('Sun'), await box('Triangle'), await box('Star')]
  expect(rx - tx - tw).toBeCloseTo(tx - sx - sw, 0)
})

const transform = async (page: Page, layer: string, name: string) => {
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: layer, exact: true }).click({ button: 'right' })
  await page.getByRole('menu', { name: 'Actions' }).getByRole('menuitem', { name: 'Transform' }).click()
  return page.getByRole('menu', { name: 'Actions' }).getByRole('menuitem', { name })
}

test('booleans combine shapes into the bottommost, from the context menu and the keyboard, and undo splits them again', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const shapes = layers.getByRole('button', { name: /^(Sun|Triangle)$/ })
  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await (await transform(page, 'Triangle', 'Union')).click()
  await expect(shapes).toHaveCount(1)
  await page.keyboard.press('Control+z')
  await expect(shapes).toHaveCount(2)

  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await page.keyboard.press('Control+Alt+x')
  await expect(shapes).toHaveCount(1)
})

test('the context menu flattens a shape and leaves out booleans for a single layer', async ({ page }) => {
  await open(page)
  await expect(await transform(page, 'Frame', 'Flatten')).toBeDisabled()
  await expect(page.getByRole('menuitem', { name: 'Union' })).toHaveCount(0)
  await page.keyboard.press('Escape')

  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('toolbar', { name: 'Path tools' })).toBeHidden()
  await (await transform(page, 'Sun', 'Flatten')).click()
  await page.evaluate(() => (document.activeElement as HTMLElement).blur())
  await page.keyboard.press('Enter')
  await expect(page.getByRole('toolbar', { name: 'Path tools' })).toBeVisible()
})

test('the align bar of a group aligns the layers inside it', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const x = async (name: string) => {
    await layers.getByRole('button', { name, exact: true }).click()
    return panel.getByRole('textbox', { name: 'X in mm', exact: true }).inputValue()
  }
  const left = await x('Shapes')
  await page.getByRole('toolbar', { name: 'Align inside' }).getByRole('button', { name: 'Align left' }).click()
  await expect(panel.getByRole('textbox', { name: 'X in mm', exact: true })).toHaveValue(left)
  expect(await x('Sun')).toBe(left)
  expect(await x('Star')).toBe(left)
})

test('layers side by side show their gap in the panel and drag it on the canvas, and the panel keeps its spacing row open', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  for (const x of [20, 50]) {
    await page.keyboard.press('r')
    await drag(page, await screen(page, x, 20), await screen(page, x + 20, 40))
  }
  await page.keyboard.down('Shift')
  await page.mouse.click(...(await screen(page, 30, 30)))
  await page.keyboard.up('Shift')
  await expect(panel.getByRole('textbox', { name: 'Horizontal spacing in mm' })).toHaveCount(0)
  await panel.getByRole('button', { name: 'Distribute and spacing' }).click()
  const gap = panel.getByRole('textbox', { name: 'Horizontal spacing in mm' })
  await expect(gap).toHaveValue('10')
  await gap.fill('5')
  await gap.press('Enter')
  await expect(gap).toHaveValue('5')
  await drag(page, await screen(page, 42.5, 30), await screen(page, 47.5, 30))
  await expect(gap).toHaveValue('10')
  await page.keyboard.press('Control+z')
  await expect(gap).toHaveValue('5')

  await page.keyboard.press('Escape')
  await expect(gap).toHaveCount(0)
  await page.mouse.click(...(await screen(page, 30, 30)))
  await page.keyboard.down('Shift')
  await page.mouse.click(...(await screen(page, 55, 30)))
  await page.keyboard.up('Shift')
  await expect(gap).toBeVisible()
})
