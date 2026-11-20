import { expect, test } from 'vitest'
import { ACTIONS } from './commands'
import type { Editor } from './editor'


test('every shortcut is canonical and belongs to one action', () => {
  const keys = ACTIONS.map((a) => a.keys).filter(Boolean)
  expect(keys.filter((k) => !/^(Ctrl )?(Alt )?(Shift )?\S+$/.test(k))).toEqual([])
  expect(keys.filter((k, i) => keys.indexOf(k) !== i)).toEqual([])
})

test('titles are sentence case without a period and single letters only pick tools', () => {
  expect(ACTIONS.map((a) => a.title).filter((t) => /\.$|^[a-z]| [A-Z][a-z]/.test(t))).toEqual([])
  expect(ACTIONS.filter((a) => /^[A-Z]$/.test(a.keys) && a.group !== 'Tools').map((a) => a.title)).toEqual([])
})

test('paste style needs a copied style and a selection', () => {
  const action = (title: string) => ACTIONS.find((a) => a.title === title)!
  const node = { id: 'a', fills: [], opacity: 0.5 }
  const editor = (selection: string[]) => ({ selection, selected: () => (selection.length ? [node] : []) }) as unknown as Editor
  action('Copy style').run(editor(['a']))
  expect(action('Paste style').can!(editor(['a']))).toBe(true)
  expect(action('Paste style').can!(editor([]))).toBe(false)
})
