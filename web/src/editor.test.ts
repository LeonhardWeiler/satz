import { readFileSync } from 'node:fs'
import { expect, test, vi } from 'vite-plus/test'
import { ACTIONS } from './commands'
import { Editor, onTrap } from './editor'
import { Engine, initSync } from './engine/engine'
import { onKey } from './keys'
import type { Command, EngineError, Snapshot } from './model'

test('the snapshot the editor merges from changes equals the full one', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  let seed = 7
  const rand = (n: number) => {
    seed = (seed * 48271) % 2147483647
    return seed % n
  }
  const pick = <T>(xs: readonly T[]) => {
    const x = xs[rand(xs.length)]
    if (x === undefined) throw new Error('pick from an empty list')
    return x
  }
  const ids = () => [...editor.snapshot.pages, ...editor.snapshot.masters].map((p) => p.id).concat([...editor.nodes.keys()])
  const texts = () => [...editor.nodes.values()].filter((e) => e.node.kind === 'text').map((e) => e.node.id)
  const page = () => pick(editor.snapshot.pages).id
  const steps: (() => Command | undefined)[] = [
    () => ({ type: 'create', parent: page(), kind: pick(['rect', 'text', 'frame'] as const), x: rand(100), y: rand(100), w: 10 + rand(50), h: 10 + rand(50) }),
    () => ({ type: 'setFrame', id: pick(ids()), x: rand(100), y: rand(100), w: 5 + rand(80), h: 5 + rand(80) }),
    () => {
      const t = texts()
      return t.length ? { type: 'editText', id: pick(t), range: [0, rand(3)], text: pick(['', 'a', 'Hallo Welt\n']) } : undefined
    },
    () => ({ type: 'delete', ids: [pick(ids())] }),
    () => ({ type: 'addPage', after: page() }),
    () => ({ type: 'undo' }),
    () => ({ type: 'redo' }),
  ]
  let applied = 0
  for (let i = 0; i < 150; i++) {
    const cmd = pick(steps)()
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

test('a failing command throws its text with the error as cause', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const cause: EngineError = { type: 'notFound', what: 'node', id: '9@9' }
  expect(() => new Engine().apply({ type: 'setText', id: '9@9', text: '' })).toThrow(expect.objectContaining({ message: 'no node 9@9', cause }))
})

test('a trap in the engine calls the trap hook once and an engine error does not', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  let calls = 0
  onTrap(() => calls++)
  expect(() => new Engine().apply({ type: 'setText', id: '9@9', text: '' })).toThrow('no node 9@9')
  expect(calls).toBe(0)
  expect(() => new Engine().trap()).toThrow(WebAssembly.RuntimeError)
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

test('moving the pointer does not notify pointer listeners', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  const listener = vi.fn()
  editor.subscribePointer(listener)
  editor.pointer = { x: 10, y: 20 }
  expect(listener).not.toHaveBeenCalled()
})

test('commands that move or resize the selection leave its locked layers alone', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  vi.stubGlobal('document', { querySelector: () => null })
  vi.stubGlobal('Element', Object)
  vi.stubGlobal('HTMLElement', Object)
  const run = (title: string) => (editor: Editor) => ACTIONS.find((a) => a.title === title)!.run(editor)
  const key = (k: string) => (editor: Editor) => onKey(editor, { key: k, code: k, preventDefault: () => {} } as unknown as KeyboardEvent)
  const commands = {
    nudge: key('ArrowRight'),
    'nudge down': key('ArrowDown'),
    'align left': run('Align left'),
    'align bottom': run('Align bottom'),
    distribute: run('Distribute horizontal spacing'),
    tidy: run('Tidy up'),
    flip: run('Flip horizontal'),
  }
  for (const [name, command] of Object.entries(commands)) {
    const editor = new Editor(Engine.sample())
    const parent = editor.page.id
    const ids = [0, 1, 2, 3].flatMap((i) => editor.apply({ type: 'create', parent, kind: 'rect', x: 10 + i * 40, y: 10 + i * 15, w: 20 + i * 5, h: 20 }))
    editor.apply({ type: 'set', id: ids[1]!, locked: true })
    editor.set({ selection: ids })
    const before = editor.selected()
    command(editor)
    expect(editor.selected()[1], name).toEqual(before[1])
    expect(editor.selected(), name).not.toEqual(before)
  }
  vi.unstubAllGlobals()
})

test('copied text leaves out the placeholders of anchored layers', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.blank(100, 100, 1, false))
  const page = editor.snapshot.pages[0]!.id
  const [text] = editor.apply({ type: 'create', parent: page, kind: 'text', x: 0, y: 0, w: 50, h: 20 })
  const [rect] = editor.apply({ type: 'create', parent: page, kind: 'rect', x: 0, y: 0, w: 5, h: 5 })
  editor.apply({ type: 'editText', id: text!, range: [0, 0], text: 'ab' })
  editor.apply({ type: 'anchor', id: rect!, text: text!, at: 1 })
  editor.set({ editing: { id: text!, anchor: 0, focus: 3 } })
  expect(editor.selectedText()).toBe('ab')
  expect(editor.copy([text!])).toBe('ab')
})

test('a day change shows the new date on the canvas', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.blank(100, 100, 1, false))
  const [text] = editor.apply({ type: 'create', parent: editor.snapshot.pages[0]!.id, kind: 'text', x: 0, y: 0, w: 50, h: 20 })
  editor.apply({ type: 'editText', id: text!, range: [0, 0], text: '\u0019' })
  editor.setToday(new Date(2026, 2, 9, 23, 59))
  editor.saved(editor.file, editor.engine.version())
  expect(editor.nodes.get(text!)?.node.name).toBe('March 9, 2026')
  editor.setToday(new Date(2026, 2, 10, 0, 1))
  expect(editor.nodes.get(text!)?.node.name).toBe('March 10, 2026')
  expect(editor.dirty).toBe(false)
})

test('previewing versions and going back to the master leaves the way back alone', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  editor.apply({ type: 'addPage', after: editor.pageId })
  const [first, second] = editor.snapshot.pages.map((p) => p.id)
  const [master] = editor.apply({ type: 'addMaster', like: first! })
  editor.showPage(first!)
  editor.showPage(master!)
  const [v] = editor.engine.versions()
  editor.preview({ ...v!, page: second! })
  expect(editor.pageId).toBe(second)
  editor.preview(null)
  editor.showPage(master!, editor.back)
  editor.exitMaster()
  expect(editor.pageId).toBe(first)
})
