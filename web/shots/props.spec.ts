import { png, rest, select, shot, start, test } from './lib'
import { place } from '../e2e/util'

const P = '05 Properties'
const Q = '12 Quick edit bar'
const quick = (page) => page.locator('.quick')

async function both(page, n: string, what: string) {
  await shot(page, P, n + ' ' + what, `Properties: ${what}`, ['right'])
  if (await quick(page).isVisible()) await shot(page, Q, n + ' ' + what, `Quick edit bar: ${what}`, [quick(page)], 16)
}

test('sample elements', async ({ page }) => {
  await start(page, { h: 2200 })
  await shot(page, P, '01 nothing selected', 'Properties with nothing selected: document, page, grids, fonts, variables', ['right'])
  const items: [string, string][] = [
    ['Shapes', 'group'],
    ['Masked', 'group with a mask'],
    ['Mask', 'shape used as mask'],
    ['Stripes', 'masked shape'],
    ['Arrow', 'line with arrow end'],
    ['Blurred tile', 'rectangle with layer blur'],
    ['Triangle', 'polygon'],
    ['Star', 'star with shadow'],
    ['Sun', 'ellipse with radial gradient'],
    ['Frame', 'frame'],
    ['Satz sets type…', 'text frame'],
  ]
  let i = 2
  for (const [name, what] of items) {
    await select(page, name)
    await both(page, String(i++).padStart(2, '0'), what)
  }
  await page.getByRole('tree', { name: 'Layers' }).locator('.layer-name').filter({ hasText: /^Rectangle/ }).first().click()
  await rest(page)
  await both(page, '13', 'rectangle inside a frame (constraints)')
  await page.getByRole('tree', { name: 'Layers' }).locator('.layer-name').filter({ hasText: /^Rectangle/ }).last().click()
  await rest(page)
  await both(page, '14', 'rectangle')
  await select(page, 'Star', 'Triangle')
  await both(page, '15', 'two shapes (boolean bar, mixed values)')
  await select(page, 'Star', 'Satz sets type…')
  await both(page, '16', 'shape and text (mixed kinds)')
  await select(page, 'Star')
  await page.keyboard.press('Control+e')
  await rest(page)
  await both(page, '17', 'vector path (flattened star)')
  await select(page, 'Frame')
  await page.keyboard.press('Shift+a')
  await rest(page)
  await both(page, '18', 'frame with auto layout')
  await page.getByRole('tree', { name: 'Layers' }).locator('.layer-name').filter({ hasText: /^Rectangle/ }).first().click()
  await rest(page)
  await both(page, '19', 'child of an auto layout frame')
})

test('image', async ({ page }) => {
  await start(page, { h: 2200 })
  await place(page, png('photo.png', 200, 140, (x, y) => [x, y, 160]), [74, 150])
  await rest(page)
  await both(page, '20', 'image')
  await page.getByRole('button', { name: 'Fit image' }).click()
  await shot(page, Q, '30 fit image menu', 'Fit image menu', [quick(page), page.getByRole('menu')], 16)
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Adjust image' }).click()
  await shot(page, Q, '31 adjust image', 'Adjust image popover', [quick(page), page.getByRole('dialog').last()], 16)
  await page.locator('.menu-backdrop').click({ position: { x: 5, y: 5 } })
  await page.getByRole('button', { name: 'Crop' }).click()
  await rest(page)
  await shot(page, '04 Canvas', '40 crop mode', 'Image crop mode', [page.getByLabel('Page canvas')], 0)
  await both(page, '21', 'image in crop mode')
})
