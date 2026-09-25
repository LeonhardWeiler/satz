import { expect, test } from '@playwright/test'
import { drag, open, pixels, screen } from './util'

const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

test('moving a gradient layer leaves the page and the handles alone', async ({ page }) => {
  await open(page)
  const sun = await screen(page, 23, 37)
  await page.keyboard.down('Control')
  await drag(page, sun, [sun[0] + 40, sun[1]])
  await page.keyboard.up('Control')
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })).toHaveText('Sun')

  const [wx, wy] = await screen(page, 140, 85)
  for (const p of await pixels(page, wx, wy, 4, 4)) expect(p).toEqual([255, 255, 255])
  const [tx, ty] = await screen(page, 23, 22)
  const edge = await pixels(page, tx + 40 - 1, ty - 3, 3, 7)
  expect(edge.some((p) => near(p, [13, 153, 255]))).toBe(true)
})

test('a line moves when dragged in the middle and changes one end at a time', async ({ page }) => {
  await open(page)
  const layout = page.getByRole('region', { name: 'Layout' })
  const field = (title: string) => layout.getByTitle(title).getByRole('textbox')
  const [ax, ay] = await screen(page, 30, 86)
  const [bx] = await screen(page, 90, 86)
  await page.keyboard.press('l')
  await drag(page, [ax, ay], [bx, ay])
  await expect(field('Length in mm')).toHaveValue('60')
  await expect(field('Angle in °')).toHaveValue('0')
  const y = await field('Y in mm').inputValue()

  await drag(page, [(ax + bx) / 2, ay], [(ax + bx) / 2, ay + 20])
  await expect(field('Y in mm')).not.toHaveValue(y)
  await expect(field('Length in mm')).toHaveValue('60')
  await expect(field('Angle in °')).toHaveValue('0')
  await expect(field('X in mm')).toHaveValue('30')

  await drag(page, [bx, ay + 20], [bx, ay - 40])
  await expect(field('X in mm')).toHaveValue('30')
  await expect(field('Angle in °')).not.toHaveValue('0')

  await field('Angle in °').fill('90')
  await field('Angle in °').press('Enter')
  await expect(field('Length in mm')).not.toHaveValue('60')
  await expect(field('X in mm')).toHaveValue('30')
})

test('property sections space their rows evenly', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const expectEven = async () => {
    const sections = await panel.evaluate((el) =>
      [...el.querySelectorAll('.section')].map((s) => {
        const r = s.getBoundingClientRect()
        const kids = [...s.children].map((c) => c.getBoundingClientRect())
        return {
          name: s.getAttribute('aria-label'),
          top: kids[0].top - r.top,
          bottom: r.bottom - parseFloat(getComputedStyle(s).borderBottomWidth) - kids.at(-1)!.bottom,
          gaps: kids.slice(1).map((k, i) => k.top - kids[i].bottom),
        }
      }),
    )
    for (const s of sections) {
      expect(s.bottom, s.name!).toBeCloseTo(s.top, 0)
      for (const g of s.gaps) expect(g, s.name!).toBeCloseTo(8, 0)
    }
  }
  await expectEven()
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expectEven()
  await panel.getByRole('button', { name: 'Add stroke' }).click()
  await panel.getByRole('button', { name: 'Add effect' }).click()
  await expectEven()
  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await expectEven()
  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await expectEven()
})

test('ctrl+alt+m on several layers makes a mask group over the lowest', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const masks = layers.getByTitle('Mask', { exact: true })
  const group = layers.getByRole('button', { name: 'Mask group', exact: true })
  await expect(masks).toHaveCount(1)
  await expect(layers.getByTitle('Masked by Mask')).toHaveCount(1)

  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await layers.getByRole('button', { name: 'Frame', exact: true }).click({ modifiers: ['Shift'] })
  await page.keyboard.press('Control+Alt+m')
  await expect(group).toHaveCount(1)
  await expect(masks).toHaveCount(2)
  await expect(layers.getByTitle(/^Masked by Satz sets type/)).toHaveCount(1)
  const [x, y] = await screen(page, 74, 7)
  for (const p of await pixels(page, x, y, 2, 2)) expect(near(p, [0xe8, 0x45, 0x2c])).toBe(true)

  await page.keyboard.press('Control+z')
  await expect(group).toHaveCount(0)
  await expect(masks).toHaveCount(1)
})

test('holding ctrl hovers the deepest layer under the pointer', async ({ page }) => {
  await open(page)
  const [x, y] = await screen(page, 23, 52)
  const accent = async () => (await pixels(page, x - 1, y - 3, 3, 7)).some((p) => near(p, [13, 153, 255]))
  await page.mouse.move(x, y - 20)
  expect(await accent()).toBe(false)
  await page.keyboard.down('Control')
  expect(await accent()).toBe(true)
  await page.keyboard.up('Control')
  expect(await accent()).toBe(false)
})

test('the cursor follows keyboard edits without a pointer move', async ({ page }) => {
  await open(page)
  const canvas = page.getByLabel('Page canvas')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await page.mouse.move(...(await screen(page, 74, 77)))
  await expect(canvas).toHaveCSS('cursor', 'ns-resize')
  await page.keyboard.press('Delete')
  await expect(canvas).not.toHaveCSS('cursor', 'ns-resize')
})

test('typed values round to two decimals and out-of-range values are rejected', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const field = (title: string) => panel.getByTitle(title, { exact: true }).getByRole('textbox')
  const rects = layers.getByRole('button', { name: 'Rectangle', exact: true })
  const type = async (title: string, value: string) => {
    await field(title).fill(value)
    await field(title).press('Enter')
  }

  await rects.first().click()
  await type('X in mm', '10.004')
  await rects.last().click()
  await type('X in mm', '10')
  await rects.first().click({ modifiers: ['Shift'] })
  await expect(field('X in mm')).toHaveValue('10')

  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await type('Font size in pt', '0.05')
  await expect(field('Font size in pt')).toHaveValue('14')
  await type('Opacity', '150')
  await expect(field('Opacity')).toHaveValue('100')
  await page.keyboard.press('Control+z')
  await expect(field('Font size in pt')).toHaveValue('14')
  await expect(layers.getByRole('button', { name: 'Rectangle', exact: true })).toHaveCount(2)
})

test('undo finishes an open pen path and waits for a drag to end', async ({ page }) => {
  const errors: Error[] = []
  page.on('pageerror', (e) => errors.push(e))
  await open(page)
  const vectors = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Vector', exact: true })
  const x = page.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')

  await page.keyboard.press('p')
  await page.mouse.click(...(await screen(page, 30, 86)))
  await page.mouse.click(...(await screen(page, 60, 88)))
  await page.keyboard.press('Control+z')
  await page.mouse.click(...(await screen(page, 90, 86)))
  await expect(vectors).toHaveCount(0)

  await page.keyboard.press('v')
  const [rx, ry] = await screen(page, 74, 7)
  await page.mouse.move(rx, ry)
  await page.mouse.down()
  await expect(x).toHaveValue('-3')
  await page.mouse.move(rx + 30, ry, { steps: 3 })
  await page.keyboard.press('Control+z')
  await page.mouse.move(rx + 60, ry, { steps: 3 })
  await page.mouse.up()
  await expect(x).not.toHaveValue('-3')
  await page.keyboard.press('Control+z')
  await expect(x).toHaveValue('-3')
  expect(errors).toEqual([])
})

test('mixed values fit their fields', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const rects = layers.getByRole('button', { name: 'Rectangle', exact: true })
  await rects.first().click()
  await rects.last().click({ modifiers: ['Shift'] })
  const layout = page.getByRole('region', { name: 'Layout' })
  for (const title of ['X in mm', 'W in mm']) {
    const input = layout.getByTitle(title).getByRole('textbox')
    await expect(input).toHaveValue('Mixed')
    expect(await input.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true)
  }
})

test('dropping a group onto its own child changes nothing', async ({ page }) => {
  const errors: Error[] = []
  page.on('pageerror', (e) => errors.push(e))
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: 'Sun' }).click()
  await page.keyboard.press('Control+g')
  const group = layers.getByRole('button', { name: 'Group', exact: true })
  const sun = layers.getByRole('button', { name: 'Sun' })
  const item = layers.getByRole('treeitem', { name: 'Sun' })
  const level = (await item.getAttribute('aria-level'))!
  await group.dragTo(sun)
  await expect(item).toHaveAttribute('aria-level', level)
  await expect(layers.locator('[data-drop]')).toHaveCount(0)
  expect(errors).toEqual([])
})

test('undo restores the text content and blur does not redo it', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const text = page.getByRole('textbox', { name: 'Text content' })
  const old = await text.inputValue()
  await text.fill('Changed')
  await text.blur()
  await page.keyboard.press('Control+z')
  await expect(text).toHaveValue(old)
  await text.focus()
  await text.blur()
  await expect(text).toHaveValue(old)
})

test('clicking a panel while drawing with the pen keeps the path one undo step', async ({ page }) => {
  await open(page)
  const items = page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem')
  const count = await items.count()
  await page.keyboard.press('p')
  await page.mouse.click(...(await screen(page, 120, 100)))
  await page.mouse.click(...(await screen(page, 140, 120)))
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 }).click()
  await page.mouse.click(...(await screen(page, 120, 140)))
  await expect(items).toHaveCount(count + 1)
  await page.keyboard.press('Control+z')
  await expect(items).toHaveCount(count)
})

test('space activates a focused layer button', async ({ page }) => {
  await open(page)
  const sun = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Sun' })
  await sun.focus()
  await page.keyboard.press('Space')
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })).toHaveText('Sun')
})
