import process from 'node:process'
import { chromium } from '@playwright/test'
const url = process.argv[2]
const nets = [
  ['ungedrosselt', null],


]
const browser = await chromium.launch({ env: Object.fromEntries(Object.entries(process.env).filter(([k]) => k !== 'WAYLAND_DISPLAY' && k !== 'DISPLAY')), args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] })
for (const [name, net] of nets) {
  const res = []
  for (let run = 0; run < 3; run++) {
    const ctx = await browser.newContext({ viewport: { width: 1400, height: 1000 } })
    const page = await ctx.newPage()
    const cdp = await ctx.newCDPSession(page)
    await cdp.send('Network.enable')
    if (net) await cdp.send('Network.emulateNetworkConditions', { offline: false, ...net })
    const times = []
    for (let i = 0; i < 2; i++) {
      const t0 = Date.now()
      await page.goto(url, { waitUntil: 'commit' })
      await page.waitForSelector('.loading', { timeout: 60000 }).catch(() => {})
      const loading = Date.now() - t0
      await page.waitForFunction(() => { const z = document.querySelector('[aria-label="Zoom"]'); return z && z.textContent !== '0%' }, null, { timeout: 60000 })
      await page.evaluate(() => new Promise((d) => requestAnimationFrame(() => requestAnimationFrame(d))))
      times.push(loading, Date.now() - t0)
    }
    res.push(times)
    await ctx.close()
  }
  const med = (i) => res.map((r) => r[i]).sort((a, b) => a - b)[1]
  console.log(name.padEnd(20), 'kalt: Ladeanzeige', med(0), 'ms, 1. Bild', med(1), 'ms | warm: 1. Bild', med(3), 'ms')
}
await browser.close()
