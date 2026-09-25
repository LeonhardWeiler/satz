import { expect, test } from '@playwright/test'
import { MM, open, pixels, screen } from './util'

/** Height in mm of `n` lines of 12 pt text as the H field shows it. */
const lines = (n: number) => String(Math.round(((n * 16.452) / MM) * 100) / 100)

test('a new text is edited in its frame: typing, arrows, backspace, enter, escape and undo', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const height = panel.getByRole('textbox', { name: 'H in mm' })
  await page.keyboard.press('t')
  await page.mouse.click(...(await screen(page, 20, 80)))
  await expect(page.getByRole('textbox', { name: 'Text editor' })).toBeFocused()
  await page.keyboard.type('Hello')
  await expect(layers.getByRole('button', { name: 'Hello', exact: true })).toBeVisible()

  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('Backspace')
  await page.keyboard.type('L')
  await page.keyboard.press('End')
  await page.keyboard.press('Enter')
  await page.keyboard.type('World')
  await expect(layers.getByRole('button', { name: 'HeLlo World', exact: true })).toBeVisible()
  await expect(height).toHaveValue(lines(2))

  await page.keyboard.press('Control+a')
  const [x, y] = await screen(page, 20, 80)
  const shot = await pixels(page, x, y, 60, 30)
  expect(shot.some(([r, g, b]) => b > 230 && r > 150 && r < 225 && g > 200)).toBe(true)

  await page.keyboard.press('Escape')
  await expect(page.getByRole('textbox', { name: 'Text editor' })).toHaveCount(0)
  await expect(panel.getByRole('heading', { level: 2 })).toHaveText('HeLlo World')
  await page.keyboard.press('Control+z')
  await expect(layers.getByRole('button', { name: 'HeLlo', exact: true })).toBeVisible()
  await expect(height).toHaveValue(lines(1))
  await page.keyboard.press('Control+z')
  await expect(layers.getByRole('button', { name: 'Hello', exact: true })).toBeVisible()
})

test('double-clicking a text puts the caret where it was clicked, and ime input and paste insert there', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await page.mouse.dblclick(...(await screen(page, 15.2, 97)))
  const editor = page.getByRole('textbox', { name: 'Text editor' })
  await expect(editor).toBeFocused()
  await page.keyboard.type('X')
  await expect(layers.getByRole('button', { name: /^XSatz sets type/ })).toBeVisible()

  const cdp = await context.newCDPSession(page)
  await cdp.send('Input.imeSetComposition', { text: '´', selectionStart: 1, selectionEnd: 1 })
  await cdp.send('Input.insertText', { text: 'é' })
  await expect(layers.getByRole('button', { name: /^XéSatz sets type/ })).toBeVisible()

  await page.keyboard.press('Shift+ArrowLeft')
  await page.keyboard.press('Shift+ArrowLeft')
  await page.keyboard.press('Control+c')
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('Control+v')
  await expect(layers.getByRole('button', { name: /^XéXéSatz sets type/ })).toBeVisible()

  await page.mouse.click(...(await screen(page, 140, 205)))
  await expect(editor).toHaveCount(0)
})
