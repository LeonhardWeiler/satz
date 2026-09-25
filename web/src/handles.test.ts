import { expect, test } from 'vitest'
import { handleAt, portAt, portsOf, resized } from './handles'

const view = { x: 100, y: 50, zoom: 2 }
/** A box across the spine of a spread: from x -20 on the left page to 30 on the right. */
const box = { x: -20, y: 10, w: 50, h: 40 }

test('corners win over edges, edges are grabbed a little outside, the middle is not', () => {
  const at = (x: number, y: number) => handleAt(view, x, y, { box })
  expect(at(60, 70)).toBe('nw')
  expect(at(163, 147)).toBe('se')
  expect(at(110, 67)).toBe('n')
  expect(at(57, 100)).toBe('w')
  expect(at(163, 100)).toBe('e')
  expect(at(110, 100)).toBeUndefined()
  expect(at(50, 100)).toBeUndefined()
})

test('a box without height has no top or bottom handles', () => {
  expect(handleAt(view, 60, 70, { box: { ...box, h: 0 } })).toBe('w')
})

test('the ends of a line are grabbed within half a handle', () => {
  const line: [{ x: number; y: number }, { x: number; y: number }] = [{ x: -10, y: 0 }, { x: 10, y: 0 }]
  expect(handleAt(view, 84, 50, { line })).toBe('end0')
  expect(handleAt(view, 124, 53, { line })).toBe('end1')
  expect(handleAt(view, 100, 50, { line })).toBeUndefined()
})

test('the ports sit inside the frame edges, and closer to the middle of a low frame', () => {
  const ports = portsOf(view, box)
  expect(ports).toEqual({ in: { x: 60, y: 86 }, out: { x: 160, y: 134 } })
  expect(portAt(ports, 165, 130)).toBe('out')
  expect(portAt(ports, 60, 86)).toBe('in')
  expect(portAt(ports, 110, 110)).toBeUndefined()
  expect(portsOf(view, { ...box, h: 4 }).in.y).toBe(74)
})

test('dragging a handle scales the frames in the box on both pages of the spread', () => {
  const left = { x: -20, y: 10, w: 10, h: 10 }
  const right = { x: 20, y: 40, w: 10, h: 10 }
  const keys = { altKey: false, shiftKey: false }
  expect(resized(box, 'e', { x: 50, y: 0 }, keys, [left, right])).toEqual([
    { x: -20, y: 10, w: 20, h: 10 },
    { x: 60, y: 40, w: 20, h: 10 },
  ])
  expect(resized(box, 'w', { x: 60, y: 0 }, keys, [left])).toEqual([{ x: 38, y: 10, w: 2, h: 10 }])
})

test('Alt resizes about the centre and Shift keeps the proportions', () => {
  const frame = [box]
  expect(resized(box, 'e', { x: 10, y: 0 }, { altKey: true, shiftKey: false }, frame)).toEqual([{ x: -30, y: 10, w: 70, h: 40 }])
  expect(resized(box, 'se', { x: 50, y: 0 }, { altKey: false, shiftKey: true }, frame)).toEqual([{ x: -20, y: 10, w: 100, h: 80 }])
  expect(resized(box, 's', { x: 0, y: 40 }, { altKey: false, shiftKey: true }, frame)).toEqual([{ x: -45, y: 10, w: 100, h: 80 }])
})
