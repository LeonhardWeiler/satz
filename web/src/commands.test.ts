import { expect, test } from 'vitest'
import { ACTIONS } from './commands'


test('every shortcut is canonical and belongs to one action', () => {
  const keys = ACTIONS.map((a) => a.keys).filter(Boolean)
  expect(keys.filter((k) => !/^(Ctrl )?(Alt )?(Shift )?\S+$/.test(k))).toEqual([])
  expect(keys.filter((k, i) => keys.indexOf(k) !== i)).toEqual([])
})

test('titles are sentence case without a period and single letters only pick tools', () => {
  expect(ACTIONS.map((a) => a.title).filter((t) => /\.$|^[a-z]| [A-Z][a-z]/.test(t))).toEqual([])
  expect(ACTIONS.filter((a) => /^[A-Z]$/.test(a.keys) && a.group !== 'Tools').map((a) => a.title)).toEqual([])
})
