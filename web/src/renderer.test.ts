import { readFileSync } from 'node:fs'
import CanvasKitInit from 'canvaskit-wasm'
import { expect, test } from 'vitest'
import { Engine, initSync } from './engine/engine'
import type { Command, Node, Snapshot } from './model'
import { Renderer } from './renderer'

test('layers with shadows are recorded again only when their content changes', async () => {
  const ck = await CanvasKitInit()
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const engine = new Engine()
  let recorded = 0
  Object.assign(ck, {
    PictureRecorder: new Proxy(ck.PictureRecorder, {
      construct: (target, args) => {
        recorded++
        return Reflect.construct(target, args)
      },
    }),
  })
  const renderer = new Renderer(ck, engine)
  const surface = ck.MakeSurface(300, 400)!
  const draw = () => {
    recorded = 0
    const [page] = (engine.snapshot() as Snapshot).pages
    renderer.draw(surface.getCanvas(), [page], [page], { x: 0, y: 0, zoom: 0.5 }, 1, { selection: [] })
    return recorded
  }
  const shapes = () => ((engine.snapshot() as Snapshot).pages[0].children[3] as Extract<Node, { kind: 'group' }>).children
  const apply = (cmd: Command) => engine.apply(cmd)

  expect(draw()).toBeGreaterThan(0)
  expect(draw()).toBe(0)
  const [sun, star, triangle] = shapes()
  apply({ type: 'setFrame', id: sun.id, x: sun.x + 10, y: sun.y, w: sun.w, h: sun.h })
  expect(draw()).toBe(2)
  expect(draw()).toBe(0)

  const [group] = apply({ type: 'group', ids: [star.id, triangle.id], frame: false })
  apply({ type: 'set', id: group, effects: [{ type: 'dropShadow', x: 0, y: 2, radius: 4, color: 0x00000040, visible: true }] })
  draw()
  expect(draw()).toBe(0)
  apply({ type: 'set', id: triangle.id, opacity: 0.5 })
  expect(draw()).toBe(1)
})

test('text in several sizes shares one typeface per font, freed with the renderer', async () => {
  const ck = await CanvasKitInit()
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const engine = new Engine()
  const typefaces: { delete: () => void }[] = []
  let freed = 0
  const make = ck.Typeface.MakeTypefaceFromData
  ck.Typeface.MakeTypefaceFromData = (data) => {
    const t = make(data)!
    const free = t.delete.bind(t)
    t.delete = () => (freed++, free())
    typefaces.push(t)
    return t
  }
  const renderer = new Renderer(ck, engine)
  const surface = ck.MakeSurface(300, 400)!
  const [page] = (engine.snapshot() as Snapshot).pages
  const text = page.children.find((n) => n.kind === 'text')!
  for (const size of [12, 24]) {
    engine.apply({ type: 'format', id: text.id, range: null, size })
    renderer.draw(surface.getCanvas(), [page], [page], { x: 0, y: 0, zoom: 0.5 }, 1, { selection: [] })
  }
  expect(typefaces).toHaveLength(1)
  renderer.delete()
  expect(freed).toBe(1)
  ck.Typeface.MakeTypefaceFromData = make
})
