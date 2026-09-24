import { expect, test } from 'vitest'
import { pick } from './select'
import type { Node } from './model'

const rect = (id: string): Node => ({ id, name: id, x: 0, y: 0, w: 1, h: 1, kind: 'rect', fill: 0 })
const group = (id: string, children: Node[]): Node => ({ id, name: id, x: 0, y: 0, w: 1, h: 1, kind: 'group', children })
const frame = (id: string, children: Node[]): Node => ({
  id, name: id, x: 0, y: 0, w: 1, h: 1, kind: 'frame', fill: 0, clip: true, children,
})

// page: frame f [ group g [ a, b ] ], group h [ group i [ c ] ]
const tree = [frame('f', [group('g', [rect('a'), rect('b')])]), group('h', [group('i', [rect('c')])])]

test('a click selects the outermost group, passing through frames', () => {
  expect(pick(tree, ['f', 'g', 'a'], [], 'click')).toBe('g')
  expect(pick(tree, ['h', 'i', 'c'], [], 'click')).toBe('h')
  expect(pick(tree, ['f'], [], 'click')).toBe('f')
  expect(pick(tree, [], ['a'], 'click')).toBeUndefined()
})

test('a click next to a selected layer stays at its depth', () => {
  expect(pick(tree, ['f', 'g', 'b'], ['a'], 'click')).toBe('b')
  expect(pick(tree, ['h', 'i', 'c'], ['a'], 'click')).toBe('h')
})

test('a double click enters the selected group', () => {
  expect(pick(tree, ['h', 'i', 'c'], ['h'], 'double')).toBe('i')
  expect(pick(tree, ['h', 'i', 'c'], ['i'], 'double')).toBe('c')
  expect(pick(tree, ['h', 'i', 'c'], [], 'double')).toBe('h')
})

test('ctrl click selects the deepest layer', () => {
  expect(pick(tree, ['h', 'i', 'c'], [], 'deep')).toBe('c')
})
