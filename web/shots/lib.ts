import { expect, test as base, type Locator, type Page } from '@playwright/test'
import { appendFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { drawn, png, screen, drag } from '../e2e/util'

export { expect, png, screen, drag, drawn }
export const OUT = '/tmp/satz-shots'
export const test = base.extend({})
test.use({ deviceScaleFactor: 2 })

type Rect = { x: number; y: number; width: number; height: number }
type Target = Locator | Rect | 'right' | 'left'

export async function start(page: Page, { w = 1600, h = 1000, doc = 'Sample' as string | null } = {}) {
  page.on('filechooser', () => {})
  await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await page.setViewportSize({ width: w, height: h })
  await page.goto('')
  if (doc) {
    await page.getByRole('dialog', { name: 'New document' }).getByRole('button', { name: doc }).click()
    await expect(page.getByRole('dialog', { name: 'New document' })).toHaveCount(0)
    await rest(page)
    await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
  }
}

export async function example(page: Page, name: string, size: { w?: number; h?: number } = {}) {
  await start(page, size)
  const chooser = page.waitForEvent('filechooser')
  await page.keyboard.press('Control+o')
  await (await chooser).setFiles(join(import.meta.dirname, '../../examples', name))
  await expect(page).toHaveTitle(`${name} - Satz`)
  await rest(page)
}

/** Moves the pointer where it shows no tooltip and no hover. */
export async function rest(page: Page) {
  const v = page.viewportSize()!
  await page.mouse.move(v.width - 60, v.height - 4)
}

export const layers = (page: Page) => page.getByRole('tree', { name: 'Layers' })
export const props = (page: Page) => page.getByRole('complementary', { name: 'Properties' })
export const section = (page: Page, name: string) => props(page).getByRole('region', { name, exact: true })

/** Opens `trigger`, saves it with the menu or popover it opens, closes it with Escape. */
export async function opened(page: Page, cat: string, name: string, caption: string, trigger: Locator, popup: Locator = page.getByRole('menu').last(), extra: Target[] = []) {
  await trigger.click()
  await expect(popup).toBeVisible()
  await shot(page, cat, name, caption, [trigger, popup, ...extra], 16)
  await page.keyboard.press('Escape')
  await expect(popup).toHaveCount(0)
}

export async function select(page: Page, ...names: string[]) {
  for (const [i, n] of names.entries()) {
    const row = layers(page).locator('.layer-name').filter({ hasText: new RegExp(`^${n.endsWith('…') ? n.slice(0, -1) : n + '$'}`) }).first()
    await row.click({ modifiers: i ? ['Shift'] : [] })
  }
  await rest(page)
}

async function rectOf(page: Page, t: Target): Promise<Rect | null> {
  if (t === 'right' || t === 'left')
    return page.evaluate((side) => {
      const el = document.querySelector('.' + side)!
      const r = el.getBoundingClientRect()
      let bottom = r.top
      for (const c of el.querySelectorAll('*')) {
        const b = c.getBoundingClientRect()
        if (!c.children.length && b.width && b.height && b.height < r.height / 2 && getComputedStyle(c).visibility !== 'hidden') bottom = Math.max(bottom, b.bottom)
      }
      return { x: r.left, y: r.top, width: r.width, height: Math.min(r.height, bottom - r.top + 12) }
    }, t)
  if ('x' in t) return t
  return t.boundingBox()
}

/** Saves a screenshot of the whole window or of the union of `targets`, padded by `pad`, as `<cat>/<nn> <name>.png`. */
export async function shot(page: Page, cat: string, name: string, caption: string, targets: Target[] = [], pad = 12) {
  await drawn(page)
  await page.waitForTimeout(150)
  const v = page.viewportSize()!
  let clip: Rect | undefined
  if (targets.length) {
    const rs = (await Promise.all(targets.map((t) => rectOf(page, t)))).filter((r): r is Rect => !!r)
    const p = targets.some((t) => t === 'right' || t === 'left') && targets.length === 1 ? 0 : pad
    const x0 = Math.max(0, Math.min(...rs.map((r) => r.x)) - p)
    const y0 = Math.max(0, Math.min(...rs.map((r) => r.y)) - p)
    const x1 = Math.min(v.width, Math.max(...rs.map((r) => r.x + r.width)) + p)
    const y1 = Math.min(v.height, Math.max(...rs.map((r) => r.y + r.height)) + p)
    clip = { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
  }
  mkdirSync(join(OUT, cat), { recursive: true })
  const file = join(OUT, cat, `${name}.png`)
  await page.screenshot({ path: file, clip })
  appendFileSync(join(OUT, 'manifest.jsonl'), JSON.stringify({ cat, name, caption, file }) + '\n')
}
