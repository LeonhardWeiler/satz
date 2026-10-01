import { expect, test, drag, near, open, pixels, screen } from './util'


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
  expect(edge.some((p) => near(p, [56, 174, 224]))).toBe(true)
})

test('a click beside the pages with a tool picks the move tool, a drag from there draws', async ({ page }) => {
  await open(page)
  const tools = page.getByRole('toolbar', { name: 'Tools' })
  const items = page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem')
  const before = await items.count()
  await page.keyboard.press('r')
  const [x, y] = await screen(page, -20, 50)
  await page.mouse.click(x, y)
  await expect(tools.getByRole('button', { name: 'Move', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await expect(items).toHaveCount(before)
  await page.keyboard.press('r')
  await drag(page, [x, y], await screen(page, 10, 70))
  await expect(items).toHaveCount(before + 1)
})

test('a line moves when dragged in the middle and changes one end at a time', async ({ page }) => {
  await open(page)
  const layout = page.getByRole('region', { name: 'Layout' })
  const field = (title: string) => layout.getByTitle(title).getByRole('textbox')
  const [ax, ay] = await screen(page, 30, 86)
  const [bx] = await screen(page, 90, 86)
  await page.keyboard.press('l')
  await page.mouse.move(ax, ay)
  await page.mouse.down()
  await page.mouse.move(bx, ay, { steps: 4 })
  const end = await pixels(page, ax - 6, ay - 6, 12, 12)
  expect(end.some(([r, g, b]) => b > 200 && r < 100 && g > 100)).toBe(true)
  await page.mouse.up()
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

test('a line turns about its middle when dragged beyond an end', async ({ page }) => {
  await open(page)
  const layout = page.getByRole('region', { name: 'Layout' })
  const field = (title: string) => layout.getByTitle(title).getByRole('textbox')
  const [ax, ay] = await screen(page, 30, 86)
  const [bx] = await screen(page, 90, 86)
  await page.keyboard.press('l')
  await drag(page, [ax, ay], [bx, ay])
  const [, by] = await screen(page, 60, 116)
  await drag(page, [bx + 8, ay], [(ax + bx) / 2, by])
  await expect(field('Length in mm')).toHaveValue('60')
  await expect(field('X in mm')).toHaveValue('60')
  await expect(field('Y in mm')).toHaveValue('56')
})

test('property sections space their rows evenly', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const expectEven = async () => {
    const sections = await panel.evaluate((el) =>
      [...el.querySelectorAll('.section')].map((s) => {
        const r = s.getBoundingClientRect()
        const kids = [...s.children].filter((c) => c.getClientRects().length).map((c) => c.getBoundingClientRect())
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

test('ctrl+alt+m on several layers makes a mask group under the highest', async ({ page }) => {
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
  await expect(layers.getByTitle('Masked by Frame')).toHaveCount(1)

  await page.keyboard.press('Control+z')
  await expect(group).toHaveCount(0)
  await expect(masks).toHaveCount(1)
})

test('a selected masked layer is dragged where the mask hides it', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Stripes', exact: true }).click()
  const field = page.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')
  await expect(field).toHaveValue('8')
  const from = await screen(page, 9, 57)
  const [to] = await screen(page, 19, 57)
  await page.keyboard.down('Control')
  await drag(page, from, [to, from[1]])
  await page.keyboard.up('Control')
  await expect(field).toHaveValue('18')
})

test('holding ctrl hovers the deepest layer under the pointer', async ({ page }) => {
  await open(page)
  const [x, y] = await screen(page, 23, 52)
  const accent = async () => (await pixels(page, x - 1, y - 3, 3, 7)).some((p) => near(p, [56, 174, 224]))
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

test('resize cursors turn with the layer', async ({ page }) => {
  await open(page)
  const canvas = page.getByLabel('Page canvas')
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const field = (title: string) => panel.getByTitle(title, { exact: true }).getByRole('textbox')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  const [x, y, w, h] = await Promise.all(['X in mm', 'Y in mm', 'W in mm', 'H in mm'].map(async (t) => Number(await field(t).inputValue())))
  const top = async (degrees: number) => {
    await field('Rotation in °').fill(String(degrees))
    await field('Rotation in °').press('Enter')
    const r = (degrees * Math.PI) / 180
    await page.mouse.move(...(await screen(page, x + w / 2 - (h / 2) * Math.sin(r), y + h / 2 - (h / 2) * Math.cos(r))))
  }
  await top(90)
  await expect(canvas).toHaveCSS('cursor', 'ew-resize')
  await top(30)
  await expect(canvas).toHaveCSS('cursor', /url.*rotate\(-120/)
})

test('typed values round to two decimals and stay within the bounds of their field', async ({ page }) => {
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
  await expect(field('Font size in pt')).toHaveValue('0.1')
  await page.keyboard.press('Control+z')
  await expect(field('Font size in pt')).toHaveValue('14')
  await type('Opacity', '150')
  await expect(field('Opacity')).toHaveValue('100')
  await expect(layers.getByRole('button', { name: 'Rectangle', exact: true })).toHaveCount(2)
})

test('a rectangle typed to height 0 keeps the least size', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const field = (title: string) => panel.getByTitle(title, { exact: true }).getByRole('textbox')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await field('H in mm').fill('0')
  await field('H in mm').press('Enter')
  await expect(field('H in mm')).toHaveValue('0.01')
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

test('side panels are resized at their inner edge and keep their width after a reload', async ({ page }) => {
  await open(page)
  const right = page.getByRole('complementary', { name: 'Properties' })
  const box = (await right.boundingBox())!
  await drag(page, [box.x + 2, box.y + 300], [box.x - 98, box.y + 300])
  await expect.poll(async () => Math.round((await right.boundingBox())!.width)).toBe(Math.round(box.width) + 100)
  await page.reload()
  await expect.poll(async () => Math.round((await right.boundingBox())!.width)).toBe(Math.round(box.width) + 100)
})

test('ctrl and the wheel over the quick edit bar zoom the canvas', async ({ page }) => {
  await open(page)
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 40), await screen(page, 60, 80))
  const box = (await page.getByRole('toolbar', { name: 'Quick edit' }).boundingBox())!
  const zoom = await page.getByLabel('Zoom').textContent()
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.wheel(0, -100)
  await page.keyboard.up('Control')
  await expect(page.getByLabel('Zoom')).not.toHaveText(zoom!)
})

test('the centre of one of several selected layers dragged onto another swaps their places', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 20), await screen(page, 40, 40))
  await page.keyboard.press('r')
  await drag(page, await screen(page, 60, 30), await screen(page, 80, 60))
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle' }).nth(1).click({ modifiers: ['Shift'] })
  await drag(page, await screen(page, 70, 45), await screen(page, 30, 30))
  await page.keyboard.press('Escape')
  await page.mouse.click(...(await screen(page, 65, 35)))
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('60')
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveValue('20')
})

test('a marquee outlines the lines it selects along the line, not around its box', async ({ page }) => {
  await open(page)
  await page.keyboard.press('l')
  await drag(page, await screen(page, 40, 100), await screen(page, 60, 120))
  await page.keyboard.press('Escape')
  await page.mouse.move(...(await screen(page, 5, 92)))
  await page.mouse.down()
  await page.mouse.move(...(await screen(page, 65, 125)), { steps: 4 })
  const accent = async (x: number, y: number) => {
    const [sx, sy] = await screen(page, x, y)
    return (await pixels(page, sx - 2, sy - 2, 5, 5)).some((p) => near(p, [56, 174, 224]))
  }
  expect(await accent(60, 100)).toBe(false)
  await page.mouse.up()
})
