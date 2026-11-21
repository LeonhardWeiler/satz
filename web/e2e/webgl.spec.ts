import { expect, test, open } from './util'

test('without webgl 2 a notice replaces the canvas', async ({ page }) => {
  await page.addInitScript(() => {
    const proto = HTMLCanvasElement.prototype as unknown as { getContext: (...args: unknown[]) => unknown }
    const get = proto.getContext
    proto.getContext = function (this: unknown, ...args: unknown[]) {
      return args[0] === 'webgl2' ? null : get.apply(this, args)
    }
  })
  await page.goto('')
  await expect(page.getByRole('alert')).toContainText('WebGL 2 is not available')
})

test('the name and a loading bar show while the engine loads', async ({ page }) => {
  let release = () => {}
  const held = new Promise<void>((r) => (release = r))
  await page.route('**/*.wasm', async (route) => {
    await held
    await route.continue()
  })
  await page.goto('')
  await expect(page.getByRole('heading', { name: 'Satz' })).toBeVisible()
  await expect(page.getByRole('progressbar', { name: 'Loading Satz' })).toBeVisible()
  release()
  await expect(page.getByLabel('Page canvas')).toBeVisible()
  await expect(page.getByRole('progressbar')).toHaveCount(0)
})

test('a notice tells when the engine fails to load', { annotation: { type: 'errors' } }, async ({ page }) => {
  await page.route('**/*.wasm', (route) => route.abort())
  await page.goto('')
  await expect(page.getByRole('alert')).toContainText('Satz could not start')
  await expect(page.getByRole('progressbar')).toHaveCount(0)
})

test('a caught panic in the engine stops Satz with a notice whose button reloads it', { annotation: { type: 'errors' } }, async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as { trap?: () => void }
    const streaming = WebAssembly.instantiateStreaming
    WebAssembly.instantiateStreaming = async (...args) => {
      const out = await streaming(...args)
      const exports = out.instance.exports as Record<string, (...a: unknown[]) => unknown>
      const { engine_trap: trap, engine_new: make } = exports
      if (w.trap || !trap || !make) return out
      let engine: unknown
      w.trap = () => trap(engine)
      return { module: out.module, instance: { exports: { ...exports, engine_new: (...a: unknown[]) => { const p = make(...a); engine ??= p; return p } } } as unknown as WebAssembly.Instance }
    }
  })
  await open(page)
  await page.evaluate(() => Promise.resolve().then(() => (window as unknown as { trap: () => void }).trap()).catch(() => {}))
  await expect(page.getByRole('alert')).toContainText('Satz stopped')
  await page.getByRole('button', { name: 'Reload' }).click()
  await expect(page.getByLabel('Page canvas')).toBeVisible()
})

test('each wasm file is fetched once and all but the full engine are preloaded', async ({ page }) => {
  const wasm: string[] = []
  page.on('request', (r) => r.url().endsWith('.wasm') && wasm.push(r.url()))
  const full = page.waitForRequest(/engine_io_bg-[\w-]+\.wasm$/)
  await open(page)
  await full
  const preloaded = await page.$$eval('link[rel=preload][as=fetch]', (links) => links.map((l) => (l as HTMLLinkElement).href))
  expect(preloaded.sort()).toEqual(wasm.filter((u) => !u.includes('engine_io_bg')).sort())
  expect(wasm).toHaveLength(3)
})
