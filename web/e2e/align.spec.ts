import { expect, test, open } from './util'

test('layers align to each other, a single layer to its frame, and three distribute', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const box = async (name: string) => {
    await layers.getByRole('button', { name, exact: true }).click()
    return Promise.all(['X in mm', 'W in mm'].map(async (f) => Number(await panel.getByRole('textbox', { name: f, exact: true }).inputValue())))
  }
  const x = async (name: string) => (await box(name))[0]
  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await expect(panel.getByRole('button', { name: 'Distribute horizontal spacing' })).toBeDisabled()
  await panel.getByRole('button', { name: 'Align left' }).click()
  expect(await x('Triangle')).toBe(await x('Sun'))

  await page.keyboard.press('Alt+A')
  expect(await x('Sun')).toBe(0)

  await layers.getByRole('button', { name: 'Star', exact: true }).click({ modifiers: ['Control'] })
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await page.keyboard.press('Alt+Shift+H')
  await page.keyboard.press('Control+z')
  await page.keyboard.press('Control+Shift+z')
  const [[sx, sw], [tx, tw], [rx]] = [await box('Sun'), await box('Triangle'), await box('Star')]
  expect(rx - tx - tw).toBeCloseTo(tx - sx - sw, 0)
})

test('booleans combine shapes into the bottommost, from the panel and the keyboard, and undo splits them again', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const shapes = layers.getByRole('button', { name: /^(Sun|Triangle)$/ })
  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await expect(panel.getByRole('toolbar', { name: 'Boolean' })).toBeHidden()
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await panel.getByRole('toolbar', { name: 'Boolean' }).getByRole('button', { name: 'Union' }).click()
  await expect(shapes).toHaveCount(1)
  await expect(panel.getByRole('toolbar', { name: 'Boolean' })).toBeHidden()
  await page.keyboard.press('Control+z')
  await expect(shapes).toHaveCount(2)

  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await layers.getByRole('button', { name: 'Triangle', exact: true }).click({ modifiers: ['Control'] })
  await page.keyboard.press('Control+Alt+x')
  await expect(shapes).toHaveCount(1)
})
