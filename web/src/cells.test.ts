import { readFileSync } from 'node:fs'
import { expect, test } from 'vitest'
import { TABLE_ACTIONS } from './cells'
import { Editor } from './editor'
import { Engine, initSync } from './engine/engine'

test('rows and columns are deleted only while the selection leaves 1 of them', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  const editor = new Editor(Engine.sample())
  const [t] = editor.apply({ type: 'create', parent: editor.snapshot.pages[0].id, kind: 'table', x: 0, y: 0, w: 100, h: 50 })
  const table = editor.nodes.get(t)!.node
  const cells = 'children' in table ? table.children.map((c) => c.id) : []
  const can = (title: string) => TABLE_ACTIONS.find((a) => a.title === title)!.can!(editor)
  editor.set({ selection: cells })
  expect([can('Delete rows'), can('Delete columns')]).toEqual([false, false])
  editor.set({ selection: [cells[0]] })
  expect([can('Delete rows'), can('Delete columns')]).toEqual([true, true])
})
