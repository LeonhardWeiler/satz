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
    renderer.draw(surface.getCanvas(), { x: 0, y: 0, zoom: 0.5 }, 1, { selection: [] })
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
