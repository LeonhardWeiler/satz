import { opened, png, rest, select, shot, start, test } from './lib'
import { place } from '../e2e/util'

const P = '05 Properties'
const both = (page, n: string, what: string) => shot(page, P, n + ' ' + what, `Properties: ${what}`, ['right'])

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
  await both(page, '15', 'two shapes (mixed values)')
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
  await shot(page, P, '30 fit image menu', 'Fit image menu', [page.getByRole('menu')], 16)
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Adjust image' }).click()
  await shot(page, P, '31 adjust image', 'Adjust image popover', [page.getByRole('dialog').last()], 16)
  await page.locator('.menu-backdrop').click({ position: { x: 5, y: 5 } })
  await page.getByRole('button', { name: 'Crop' }).click()
  await rest(page)
  await shot(page, '04 Canvas', '40 crop mode', 'Image crop mode', [page.getByLabel('Page canvas')], 0)
  await both(page, '21', 'image in crop mode')
  await page.keyboard.press('Escape')
  await sec(page, 'Layout').getByRole('button', { name: 'Keep aspect ratio' }).click()
  await part(page, '23', 'keep aspect ratio', 'Layout')
})

const sec = (page, name: string) => page.getByRole('complementary', { name: 'Properties' }).getByRole('region', { name, exact: true })
const part = (page, n: string, what: string, name: string) => shot(page, P, `${n} ${what}`, `Properties: ${what}`, [sec(page, name)], 8)

test('property states', async ({ page }) => {
  await start(page, { h: 2200 })
  await select(page, 'Sun')
  await sec(page, 'Fill').getByRole('button', { name: 'Fill color' }).first().click()
  await page.getByRole('dialog', { name: 'Fill color' }).getByRole('radio', { name: 'Linear' }).click()
  await page.keyboard.press('Escape')
  await rest(page)
  await part(page, '50', 'linear gradient', 'Fill')
  await select(page, 'Rectangle…')
  await sec(page, 'Fill').getByRole('button', { name: 'Add fill' }).click()
  await sec(page, 'Fill').getByRole('button', { name: 'Add fill' }).click()
  await rest(page)
  await part(page, '51', 'several fills', 'Fill')
  await select(page, 'Sun')
  await sec(page, 'Effects').getByRole('button', { name: /^Hide / }).first().click()
  await rest(page)
  await part(page, '52', 'hidden effect', 'Effects')
  await sec(page, 'Effects').getByRole('button', { name: 'Add effect' }).click()
  await sec(page, 'Effects').getByRole('combobox', { name: 'Effect type' }).last().click()
  await page.getByRole('menu').getByRole('menuitemradio').last().click()
  await rest(page)
  await part(page, '53', 'two effects', 'Effects')
  await select(page, 'Satz sets type…')
  await sec(page, 'Stroke').getByRole('button', { name: 'Add stroke' }).click()
  await rest(page)
  await part(page, '54', 'text stroke', 'Stroke')
  await sec(page, 'Text').getByRole('button', { name: 'Type options' }).click()
  await page.getByRole('dialog', { name: 'Type options' }).getByRole('tab', { name: 'Paragraph' }).click()
  await page.getByRole('dialog', { name: 'Type options' }).getByRole('checkbox', { name: 'Shading' }).check()
  await page.keyboard.press('Escape')
  await rest(page)
  await part(page, '55', 'text shading', 'Text')
  await sec(page, 'Text frame').getByRole('button', { name: 'Frame options' }).click()
  await sec(page, 'Text frame').getByRole('textbox', { name: 'Columns' }).fill('2')
  await page.keyboard.press('Enter')
  await sec(page, 'Text frame').getByRole('checkbox', { name: 'Column rule' }).check()
  await sec(page, 'Text frame').getByRole('textbox', { name: 'Baseline grid in pt', exact: true }).fill('12')
  await page.keyboard.press('Enter')
  await sec(page, 'Text frame').getByRole('radio', { name: 'Auto height' }).click()
  await rest(page)
  await part(page, '56', 'text frame: insets, rule, baseline grid, auto height', 'Text frame')
  await shot(page, '04 Canvas', '38 baseline grid and column rule', 'Text frame with 2 columns, a column rule and a baseline grid')
  await select(page, 'Arrow')
  await opened(page, '06 Popovers & menus', '15 line end select', 'Line end select (arrows)', sec(page, 'Stroke').getByRole('combobox', { name: 'End point' }))
  await select(page, 'Frame')
  await page.keyboard.press('Shift+a')
  const al = sec(page, 'Auto layout')
  await al.getByRole('radio', { name: 'Vertical layout' }).click()
  await al.getByRole('combobox', { name: 'Spacing mode' }).click()
  await page.getByRole('menu').getByRole('menuitemradio', { name: 'Space between' }).click()
  const each = al.getByRole('button', { name: 'Padding of each side' })
  if ((await each.getAttribute('aria-pressed')) !== 'true') await each.click()
  await rest(page)
  await part(page, '57', 'auto layout: vertical, space between, padding per side', 'Auto layout')
  await shot(page, '04 Canvas', '52 vertical auto layout', 'Vertical auto layout frame with space between', [{ x: 300, y: 60, width: 1000, height: 940 }], 0)
})

test('field states', async ({ page }) => {
  await start(page, { h: 1400 })
  await select(page, 'Star')
  const layout = sec(page, 'Layout')
  const x = layout.getByRole('textbox', { name: /^X\b/ })
  await x.focus()
  await part(page, '60', 'field focused', 'Layout')
  await x.fill('abc')
  await page.keyboard.press('Enter')
  await part(page, '61', 'field with invalid input', 'Layout')
  const label = layout.locator('.field-label.scrubs').first()
  const b = (await label.boundingBox())!
  await page.mouse.move(b.x + 4, b.y + 4)
  await page.mouse.down()
  await page.mouse.move(b.x + 30, b.y + 4, { steps: 4 })
  await part(page, '62', 'field scrubbing', 'Layout')
  await page.mouse.up()
  await page.keyboard.press('Control+z')
  await page.keyboard.press('Tab')
  await page.keyboard.press('Tab')
  await shot(page, P, '63 keyboard focus', 'Keyboard focus ring', ['right'])
})
