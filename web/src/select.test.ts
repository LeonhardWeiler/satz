import { expect, test } from 'vitest'
import { pick, span } from './select'
import type { Layout, Node, Style } from './model'

const style: Style = {
  fills: [], strokes: [], strokeWeight: 1, strokeAlign: 'inside', join: 'miter', cap: 'none', lineStyle: 'solid',
  arrowStart: false, arrowEnd: false, opacity: 1, blend: 'normal', effects: [], mask: false, overprintFill: false, overprintStroke: false,
  constraints: { horizontal: 'min', vertical: 'min' },
}
const layout: Layout = {
  direction: 'none', gap: 0, paddingTop: 0, paddingRight: 0, paddingBottom: 0, paddingLeft: 0,
  alignMain: 'start', alignCross: 'start', sizing: { horizontal: 'fixed', vertical: 'fixed' }, absolute: false,
}
const base = (id: string) => ({ id, name: id, x: 0, y: 0, w: 1, h: 1, modes: {}, activeModes: {}, bindings: {}, hidden: false, locked: false, keepRatio: false, rotation: 0, wrap: 'none' as const, wrapOffsetX: 0, wrapOffsetY: 0, bounds: [0, 0, 1, 1] as [number, number, number, number], ...style, ...layout })
const rect = (id: string): Node => ({ ...base(id), kind: 'shape', shape: 'rect', radius: 0 })
const group = (id: string, children: Node[]): Node => ({ ...base(id), kind: 'group', children })
const frame = (id: string, children: Node[]): Node => ({ ...base(id), kind: 'frame', clip: true, children })

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

test('shift click fills up to a layer before or after the selection and drops those after one among it', () => {
  const list = ['0', '1', '2', '3', '4', '5', '6']
  const sorted = (sel: string[] | null) => sel?.toSorted()
  expect(sorted(span(list, ['1', '2', '3', '4'], '6'))).toEqual(['1', '2', '3', '4', '5', '6'])
  expect(sorted(span(list, ['1', '2', '3', '4', '5', '6'], '2'))).toEqual(['1', '2'])
  expect(sorted(span(list, ['1', '2'], '0'))).toEqual(['0', '1', '2'])
  expect(sorted(span(list, ['1', '2', '3', '4'], '2'))).toEqual(['1', '2'])
  expect(sorted(span(list, ['x', '1', '3'], '5'))).toEqual(['1', '3', '4', '5', 'x'])
  expect(span(list, ['3'], '5')?.at(-1)).toBe('5')
  expect(span(list, ['x'], '5')).toBeNull()
})
