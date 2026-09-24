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
