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

test('the arrange bar flattens a shape and adds auto layout', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const arrange = panel.getByRole('toolbar', { name: 'Arrange' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await expect(arrange.getByRole('button', { name: 'Flatten' })).toBeDisabled()
  await arrange.getByRole('button', { name: 'Add auto layout' }).click()
  await expect(panel.getByRole('radio', { name: 'Horizontal layout' })).toBeChecked()

  await layers.getByRole('button', { name: 'Sun', exact: true }).click()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('toolbar', { name: 'Path tools' })).toBeHidden()
  await arrange.getByRole('button', { name: 'Flatten' }).click()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('toolbar', { name: 'Path tools' })).toBeVisible()
})

test('the quick edit bar of a group aligns the layers inside it', async ({ page }) => {
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
