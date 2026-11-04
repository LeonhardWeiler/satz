import { join } from 'node:path'
import { writeFileSync } from 'node:fs'
import { test, expect, open, screen, drawn, showPage } from './util'
import type { Page } from '@playwright/test'

// PERF=1 ./check e2e e2e/perf.spec.ts after `cargo test --release -p engine heavy_documents -- --ignored`.
test.skip(!process.env.PERF, 'a benchmark, run with PERF=1')
test.setTimeout(600_000)
test.describe.configure({ mode: 'serial' })
test.use({ trace: 'off' })

const DIR = join(import.meta.dirname, '../../target/perf')

/** Median ms of `runs` calls of `f` until the canvas has drawn, in total and per phase the app times; with PROFILE=name a CPU profile of them in target/perf. */
async function ms(page: Page, name: string, runs: number, f: (i: number) => Promise<void>) {
  const cdp = process.env.PROFILE === name ? await page.context().newCDPSession(page) : null
  if (cdp) {
    await cdp.send('Profiler.enable')
    await cdp.send('Profiler.setSamplingInterval', { interval: 200 })
    await cdp.send('Profiler.start')
  }
  const t: Record<string, number[]> = { total: [], apply: [], snapshot: [], index: [], paint: [] }
  for (let i = 0; i < runs; i++) {
    await page.evaluate(() => performance.clearMeasures())
    const start = performance.now()
    await f(i)
    await drawn(page)
    t.total.push(performance.now() - start)
    const phases = await page.evaluate(() => performance.getEntriesByType('measure').map((m) => [m.name, m.duration] as const))
    for (const k of ['apply', 'snapshot', 'index', 'paint']) t[k].push(phases.filter((p) => p[0] === k).reduce((s, p) => s + p[1], 0))
  }
  if (cdp) writeFileSync(join(DIR, `${name}.cpuprofile`), JSON.stringify((await cdp.send('Profiler.stop')).profile))
  return Object.fromEntries(Object.entries(t).map(([k, v]) => [k, Math.round(v.sort((a, b) => a - b)[runs >> 1] * 10) / 10]))
}

for (const n of (process.env.PAGES ?? '1,8,32').split(',').map(Number)) {
  test(`heavy ${n}`, async ({ page }) => {
    await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
    await page.addInitScript(() => Object.assign(window, { satzPerf: true }))
    await open(page, 2000)
    const chooser = page.waitForEvent('filechooser')
    await page.keyboard.press('Control+o')
    await (await chooser).setFiles(join(DIR, `heavy-${n}.satz`))
    await expect(page).toHaveTitle(`heavy-${n}.satz - Satz`)
    const at = n === 1 ? 1 : Math.floor(n / 2)
    if (n > 1) await showPage(page, at)
    await page.mouse.move(1, 1)
    await drawn(page)
    const i = n === 1 ? undefined : at % 2 === 0 ? 0 : 1
    const shape = await screen(page, 18, 28, i)
    const out: Record<string, Record<string, number>> = {}
    out.select = await ms(page, 'select', 7, async (k) => {
      await page.mouse.click(...(k % 2 ? shape : await screen(page, 5, 5, i)))
    })
    await page.mouse.click(...shape)
    out.nudge = await ms(page, 'nudge', 11, async (k) => {
      await page.keyboard.press(k % 2 ? 'ArrowLeft' : 'ArrowRight')
    })
    out.duplicate = await ms(page, 'duplicate', 7, async () => {
      await page.keyboard.press('Control+d')
    })
    out.undo = await ms(page, 'undo', 7, async () => {
      await page.keyboard.press('Control+z')
    })
    await page.mouse.move(...shape)
    await page.mouse.down()
    out.dragStep = await ms(page, 'dragStep', 21, async (k) => {
      await page.mouse.move(shape[0] + 40 + (k % 2) * 40, shape[1] + 30)
    })
    await page.mouse.up()
    await page.keyboard.press('Control+z')
    const story = await screen(page, 140, 230, i)
    await page.mouse.dblclick(...story)
    out.keystroke = await ms(page, 'keystroke', 15, async () => {
      await page.keyboard.type('x')
    })
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    out.pageSwitch = await ms(page, 'pageSwitch', 5, async (k) => {
      await page.keyboard.press(k % 2 ? 'PageUp' : 'PageDown')
    })
    console.log(`heavy ${n}`, JSON.stringify(out))
    writeFileSync(join(DIR, `result-${n}.json`), JSON.stringify(out))
  })
}
