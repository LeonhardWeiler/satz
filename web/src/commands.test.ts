import { expect, test } from 'vitest'
import { ACTIONS } from './commands'


test('every shortcut is canonical and belongs to one action', () => {
  const keys = ACTIONS.map((a) => a.keys).filter(Boolean)
  expect(keys.filter((k) => !/^(Ctrl )?(Alt )?(Shift )?\S+$/.test(k))).toEqual([])
  expect(keys.filter((k, i) => keys.indexOf(k) !== i)).toEqual([])
})
