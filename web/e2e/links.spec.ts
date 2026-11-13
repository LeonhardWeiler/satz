import { expect, test, assets, autosaved, colors, open, png, preflight, screen } from './util'
import type { Page } from '@playwright/test'

const write = (page: Page, name: string, rgb: number[]) =>
  page.evaluate(
    async ([name, bytes]) => {
      const file = await (await navigator.storage.getDirectory()).getFileHandle(name as string, { create: true })
      const w = await file.createWritable()
      await w.write(new Uint8Array(bytes as number[]))
      await w.close()
    },
    [name, [...png(name, 300, 300, () => rgb).buffer]],
  )

const color = async (page: Page) => (await colors(page, [await screen(page, 74, 105)]))[0]

test('a linked image reads its file again after a reload, follows its change, relinks once gone and embeds', async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as { showOpenFilePicker?: unknown; pick?: string }
    w.showOpenFilePicker = async () => [await (await navigator.storage.getDirectory()).getFileHandle(w.pick ?? 'a.png')]
  })
  await open(page)
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('checkbox', { name: 'Link placed images' }).check()
  await write(page, 'a.png', [255, 0, 0])
  await page.getByRole('button', { name: 'Place image' }).click()
  await page.mouse.click(...(await screen(page, 74, 105)))
  const layers = page.getByRole('tree', { name: 'Layers' })
  await expect(layers.getByRole('treeitem', { name: 'a.png' })).toBeVisible()
  const panel = await assets(page)
  const linked = panel.getByRole('region', { name: 'Linked images' })
  await expect(linked.getByText('a.png')).toBeVisible()
  await expect(linked.getByRole('button', { name: 'Embed linked images' })).toBeVisible()
  await expect.poll(() => color(page)).toEqual([255, 0, 0])

  await write(page, 'a.png', [0, 0, 255])
  await autosaved(page)
  await page.reload()
  await expect(layers.getByRole('treeitem', { name: 'a.png' })).toBeVisible()
  await expect.poll(() => color(page)).toEqual([0, 0, 255])

  await page.evaluate(async () => (await navigator.storage.getDirectory()).removeEntry('a.png'))
  await autosaved(page, false)
  await page.reload()
  await expect(layers.getByRole('treeitem', { name: 'a.png' })).toBeVisible()
  await expect((await preflight(page)).getByRole('button', { name: /Missing image a\.png/ })).toBeVisible()
  await write(page, 'b.png', [0, 255, 0])
  await page.evaluate(() => ((window as { pick?: string }).pick = 'b.png'))
  await (await assets(page)).getByRole('button', { name: 'Relink a.png' }).click()
  await expect(linked.getByText('b.png')).toBeVisible()
  await expect.poll(() => color(page)).toEqual([0, 255, 0])

  await linked.getByRole('button', { name: 'Embed linked images' }).click()
  await expect(linked).toBeHidden()
  await page.evaluate(async () => (await navigator.storage.getDirectory()).removeEntry('b.png'))
  await autosaved(page)
  await page.reload()
  await expect(layers.getByRole('treeitem', { name: 'a.png' })).toBeVisible()
  await expect.poll(() => color(page)).toEqual([0, 255, 0])
})
