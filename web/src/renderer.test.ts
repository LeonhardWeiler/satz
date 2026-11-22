import { readFileSync } from 'node:fs'
import CanvasKitInit from '../canvaskit/canvaskit.js'
import { expect, test } from 'vite-plus/test'
import { Engine, initSync } from './engine/engine'
import type { Command, Snapshot } from './model'
import { Renderer } from './renderer'

const firstPage = (engine: Engine) => {
  const [page] = (JSON.parse(engine.snapshot()) as Snapshot).pages
  if (!page) throw new Error('no page')
  return page
}

test('layers with shadows are recorded again only when their content changes', async () => {
  const ck = await CanvasKitInit()
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const engine = Engine.sample()
  let recorded = 0
  Object.assign(ck, {
    PictureRecorder: new Proxy(ck.PictureRecorder, {
      construct: (target, args) => {
        recorded++
        return Reflect.construct(target, args) as object
      },
    }),
  })
  const renderer = new Renderer(ck, engine, () => {})
  const surface = ck.MakeSurface(300, 400)!
  const draw = () => {
    recorded = 0
    const page = firstPage(engine)
    renderer.draw(surface.getCanvas(), [page], [page], { x: 0, y: 0, zoom: 0.5 }, 1, null)
    return recorded
  }
  const shapes = () => {
    const group = firstPage(engine).children[3]
    if (group?.kind !== 'group') throw new Error('no group')
    return group.children
  }
  const apply = (cmd: Command) => engine.apply(cmd)

  expect(draw()).toBeGreaterThan(0)
  expect(draw()).toBe(0)
  const [sun, star, triangle] = shapes()
  if (!sun || !star || !triangle) throw new Error('no shapes')
  apply({ type: 'setFrame', id: sun.id, x: sun.x + 10, y: sun.y, w: sun.w, h: sun.h })
  expect(draw()).toBe(2)
  expect(draw()).toBe(0)

  const [group] = apply({ type: 'group', ids: [star.id, triangle.id], frame: false })
  if (!group) throw new Error('not grouped')
  apply({ type: 'set', id: group, effects: [{ type: 'dropShadow', x: 0, y: 2, radius: 4, color: 0x00000040, visible: true }] })
  draw()
  expect(draw()).toBe(0)
  apply({ type: 'set', id: triangle.id, opacity: 0.5 })
  expect(draw()).toBe(1)
})

test('text in several sizes shares one typeface per font, freed with the renderer', async () => {
  const ck = await CanvasKitInit()
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const engine = Engine.sample()
  const typefaces: { delete: () => void }[] = []
  let freed = 0
  const make = ck.Typeface.MakeTypefaceFromData.bind(ck.Typeface)
  ck.Typeface.MakeTypefaceFromData = (data) => {
    const t = make(data)!
    const free = t.delete.bind(t)
    t.delete = () => (freed++, free())
    typefaces.push(t)
    return t
  }
  const renderer = new Renderer(ck, engine, () => {})
  const surface = ck.MakeSurface(300, 400)!
  const page = firstPage(engine)
  const text = page.children.find((n) => n.kind === 'text')!
  for (const size of [12, 24]) {
    engine.apply({ type: 'format', id: text.id, range: null, size })
    renderer.draw(surface.getCanvas(), [page], [page], { x: 0, y: 0, zoom: 0.5 }, 1, null)
  }
  expect(typefaces).toHaveLength(1)
  renderer.delete()
  expect(freed).toBe(1)
  ck.Typeface.MakeTypefaceFromData = make
})

test('text in a missing font is highlighted pink', async () => {
  const ck = await CanvasKitInit()
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const engine = Engine.sample()
  const renderer = new Renderer(ck, engine, () => {})
  const surface = ck.MakeSurface(300, 400)!
  const page = firstPage(engine)
  const text = page.children.find((n) => n.kind === 'text')!
  const pink = () => {
    renderer.draw(surface.getCanvas(), [page], [page], { x: 0, y: 0, zoom: 0.5 }, 1, null)
    const px = surface.getCanvas().readPixels(0, 0, { width: 300, height: 400, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB })!
    let n = 0
    for (let i = 0; i < px.length; i += 4) {
      const [r = 0, g = 0, b = 0] = px.subarray(i, i + 3)
      if (r > 220 && g < 150 && b > 120) n++
    }
    return n
  }
  const before = pink()
  engine.apply({ type: 'format', id: text.id, range: null, font: { name: 'Gone Sans', family: '', style: '', hash: '0123456789abcdef' } })
  expect(pink()).toBeGreaterThan(before + 500)
  renderer.delete()
})
