import { example, opened, props, rest, section, select, shot, start, test } from './lib'

const C = '06 Popovers & menus'
const T = '07 Text'
const Q = '12 Quick edit bar'
const P = '05 Properties'

test('extras sample', async ({ page }) => {
  await start(page, { h: 1400 })
  await select(page, 'Satz sets type…')
  await props(page).getByRole('button', { name: 'Insert character' }).click()
  await page.getByRole('menu').getByRole('menuitem', { name: /All characters/ }).click()
  await shot(page, T, '12 all characters', 'All characters (glyphs) dialog')
  await page.getByRole('dialog', { name: 'All characters' }).getByRole('textbox', { name: 'Search characters' }).fill('arrow')
  await shot(page, T, '13 all characters, search', 'Glyphs dialog searched', [page.getByRole('dialog', { name: 'All characters' })], 8)
  await page.keyboard.press('Escape')
  await select(page, 'Rectangle')
  await opened(page, Q, '32 quick fill color menu', 'Quick edit fill colour menu', page.locator('.quick').getByRole('button', { name: 'Fill color' }), page.getByRole('menu').last())
  await section(page, 'Fill').getByRole('button', { name: 'Hide fill' }).click()
  await rest(page)
  await shot(page, C, '50 hidden fill', 'Fill hidden with the eye', [section(page, 'Fill')], 8)
  await page.keyboard.press('Control+z')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await section(page, 'Variables').getByRole('button', { name: 'Add variable' }).click()
  await page.getByRole('menu', { name: 'Create variable' }).getByRole('menuitem', { name: 'Number' }).click()
  await page.keyboard.press('Escape')
  await select(page, 'Rectangle')
  await props(page).locator('.bind-button').first().click()
  await page.getByRole('listbox').last().getByRole('option').first().click()
  await rest(page)
  await shot(page, C, '51 field bound to variable', 'A field bound to a number variable (pill and detach)', ['right'])
})

test('extras poster', async ({ page }) => {
  await example(page, 'poster.satz', { h: 1600 })
  await page.getByRole('tree', { name: 'Layers' }).locator('.layer-name').filter({ hasText: /^Earthrise$/ }).first().click()
  await rest(page)
  await shot(page, P, '22 image in a CMYK document', 'Image fill of an RGB image in a CMYK document (RGB → CMYK)', ['right'])
  await shot(page, Q, '22 image in a CMYK document', 'Quick edit bar: image', [page.locator('.quick')], 16)
})

test('extras master override', async ({ page }) => {
  await example(page, 'booklet.satz', { h: 1200 })
  await page.keyboard.press('PageDown')
  await rest(page)
  const c = (await page.getByLabel('Page canvas').boundingBox())!
  await page.mouse.click(c.x + c.width * 0.3, c.y + c.height * 0.1, { modifiers: ['Control', 'Shift'] })
  await rest(page)
  await shot(page, P, '23 overridden master layer', 'A master layer overridden on a page (Reset to master)', ['right'])
  await shot(page, '04 Canvas', '71 overridden master layer', 'Master layer overridden on a page')
})
