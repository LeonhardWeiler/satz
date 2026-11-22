import { readFileSync } from 'node:fs'
import { expect, test } from 'vite-plus/test'
import { TABLE_ACTIONS } from './cells'
import { Editor, firstPage } from './editor'
import { Engine, initSync } from './engine/engine'
import type { TextNode } from './model'
import { handleTextKey } from './textEdit'

test('rows and columns are deleted only while the selection leaves 1 of them', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  const [t = ''] = editor.apply({ type: 'create', parent: firstPage(editor.snapshot).id, kind: 'table', x: 0, y: 0, w: 100, h: 50 })
  const table = editor.nodes.get(t)!.node
  const cells = 'children' in table ? table.children.map((c) => c.id) : []
  const can = (title: string) => TABLE_ACTIONS.find((a) => a.title === title)!.can!(editor)
  editor.set({ selection: cells })
  expect([can('Delete rows'), can('Delete columns')]).toEqual([false, false])
  editor.set({ selection: cells.slice(0, 1) })
  expect([can('Delete rows'), can('Delete columns')]).toEqual([true, true])
})

test('tab moves to the next cell in a table and types a tab elsewhere', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.blank(100, 100, 1, false))
  const page = editor.page.id
  const press = () => handleTextKey(editor, { key: 'Tab', shiftKey: false } as KeyboardEvent)
  const [text = ''] = editor.apply({ type: 'create', parent: page, kind: 'text', x: 0, y: 0, w: 50, h: 20 })
  editor.apply({ type: 'editText', id: text, range: [0, 0], text: 'ab' })
  editor.set({ selection: [text], editing: { id: text, anchor: 1, focus: 1 } })
  expect(press()).toBe(true)
  expect(editor.storyOf(editor.nodes.get(text)!.node as TextNode).text).toBe('a\tb')
  const [t = ''] = editor.apply({ type: 'create', parent: page, kind: 'table', x: 0, y: 30, w: 100, h: 50 })
  const table = editor.nodes.get(t)!.node
  const [first = '', second] = 'children' in table ? table.children.map((c) => c.id) : []
  editor.set({ selection: [first], editing: { id: first, anchor: 0, focus: 0 } })
  press()
  expect(editor.editing?.id).toBe(second)
})
