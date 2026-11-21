import { readFileSync } from 'node:fs'
import { expect, test, vi } from 'vite-plus/test'
import { Editor } from './editor'
import { Engine, initSync } from './engine/engine'
import type { Command, Snapshot } from './model'

test('the snapshot the editor merges from changes equals the full one', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  let seed = 7
  const rand = (n: number) => {
    seed = (seed * 48271) % 2147483647
    return seed % n
  }
  const ids = () => [...editor.snapshot.pages, ...editor.snapshot.masters].map((p) => p.id).concat([...editor.nodes.keys()])
  const texts = () => [...editor.nodes.values()].filter((e) => e.node.kind === 'text').map((e) => e.node.id)
  const page = () => editor.snapshot.pages[rand(editor.snapshot.pages.length)].id
  const steps: (() => Command | undefined)[] = [
    () => ({ type: 'create', parent: page(), kind: ['rect', 'text', 'frame'][rand(3)], x: rand(100), y: rand(100), w: 10 + rand(50), h: 10 + rand(50) }) as Command,
    () => {
      const id = ids()[rand(ids().length)]
      return { type: 'setFrame', id, x: rand(100), y: rand(100), w: 5 + rand(80), h: 5 + rand(80) }
    },
    () => {
      const t = texts()
      return t.length ? { type: 'editText', id: t[rand(t.length)], range: [0, rand(3)], text: ['', 'a', 'Hallo Welt\n'][rand(3)] } : undefined
    },
    () => ({ type: 'delete', ids: [ids()[rand(ids().length)]] }),
    () => ({ type: 'addPage', after: page() }),
    () => ({ type: 'undo' }),
    () => ({ type: 'redo' }),
  ]
  let applied = 0
  for (let i = 0; i < 150; i++) {
    const cmd = steps[rand(steps.length)]()
    if (!cmd) continue
    try {
      editor.apply(cmd)
    } catch {
      continue
    }
    applied++
    expect(editor.snapshot, JSON.stringify(cmd)).toEqual(JSON.parse(editor.engine.snapshot()) as Snapshot)
  }
  expect(applied).toBeGreaterThan(100)
})

test('a project that fails to open leaves the open one as it was', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  const { hash } = editor.engine.addImage(readFileSync(new URL('../public/icon-192.png', import.meta.url)), undefined)
  const version = editor.engine.version()
  const other = Engine.blank(100, 100, 1, false).updates(new Uint8Array())
  expect(() => editor.load(other, undefined, false, [new Uint8Array([1, 2, 3])])).toThrow()
  expect(editor.engine.version()).toBe(version)
  expect(editor.engine.imageFile(hash)).toBeDefined()
})

test('a panic in the engine calls the panic hook once', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  let calls = 0
  Engine.onPanic(() => calls++)
  expect(() => new Engine().trap()).toThrow('unreachable')
  expect(calls).toBe(1)
})

test('an image that leaves placing revokes its object URL', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  const png = readFileSync(new URL('../public/icon-192.png', import.meta.url))
  const live = new Set<string>()
  let n = 0
  const create = vi.spyOn(URL, 'createObjectURL').mockImplementation(() => {
    const url = `blob:${n++}`
    live.add(url)
    return url
  })
  const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation((url) => void live.delete(url))
  const place = () => editor.loadImage(new Uint8Array(png), 'icon.png')
  const other = editor.engine.updates(new Uint8Array())
  for (const leave of [() => editor.set({ placing: editor.placing.slice(1) }), () => editor.set({ placing: [] }), () => editor.setTool('rect'), place, () => editor.load(other)]) {
    place()
    expect(live.size).toBe(1)
    leave()
    expect(live.size).toBe(editor.placing.length)
    editor.set({ placing: [] })
  }
  create.mockRestore()
  revoke.mockRestore()
})
