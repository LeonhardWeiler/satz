import { expect, test } from 'vitest'
import type { Node } from './model'
import { guides, measure, nearest, snap, targets } from './snap'

const box = (x: number, y: number, w: number, h: number) => ({ kind: 'shape', x, y, w, h }) as Node
const sheet = { x: 0, width: 100, height: 200, bleed: 3 }

test('pages and layers give their edges and centres', () => {
  const { x, y } = targets([sheet], [box(10, 20, 30, 40)])
  expect(x.map((l) => l.at)).toEqual([0, 50, 100, 10, 25, 40])
  expect(y.map((l) => l.at)).toEqual([0, 100, 200, 20, 40, 60])
  expect(x[3]).toEqual({ at: 10, from: 20, to: 60 })
})

test('text frames also give their insets, columns and baseline grid', () => {
  const text = {
    ...box(0, 0, 110, 50), kind: 'text',
    insetTop: 5, insetRight: 5, insetBottom: 5, insetLeft: 5, columns: 2, gutter: 10, baselineGrid: 12, baselineStart: 10,
  } as Node
  const { x, y } = targets([], [text])
  expect(x.map((l) => l.at).slice(3)).toEqual([5, 50, 60, 105])
  expect(y.map((l) => l.at).slice(3)).toEqual([5, 45, 15, 27, 39])
  expect(y.at(-1)).toEqual({ at: 39, from: 5, to: 105 })
})

test('the nearest value snaps within the tolerance and nothing beyond it', () => {
  const lines = [{ at: 10, from: 0, to: 1 }, { at: 50, from: 0, to: 1 }]
  expect(snap([13, 52], lines, 5)).toBe(-2)
  expect(snap([14], lines, 5)).toBe(-4)
  expect(snap([16, 44], lines, 5)).toBe(0)
  expect(snap([10], lines, 5)).toBe(0)
})

test('guides run through the values on a line and span it and the box', () => {
  const lines = targets([sheet], [box(10, 150, 30, 20)])
  expect(guides({ x: 10, y: 20, w: 20, h: 10 }, lines, [10, 20, 30], [25])).toEqual([
    { axis: 'x', at: 10, from: 20, to: 170 },
  ])
  expect(guides({ x: 60, y: 90, w: 20, h: 20 }, lines, [60, 70, 80], [90, 100, 110])).toEqual([
    { axis: 'y', at: 100, from: 0, to: 100 },
  ])
})

test('the 2 nearest layers apart from the box are measured', () => {
  const a = box(0, 0, 10, 10)
  const far = box(100, 0, 10, 10)
  const near = box(20, 0, 10, 10)
  const below = box(0, 30, 10, 10)
  const over = box(5, 5, 10, 10)
  expect(nearest(a, [far, over, below, near])).toEqual([near, below])
})

test('distances run across the gap on each axis the boxes are apart on', () => {
  expect(measure({ x: 0, y: 0, w: 10, h: 10 }, { x: 30, y: 5, w: 10, h: 10 })).toEqual([{ x1: 10, y1: 7.5, x2: 30, y2: 7.5, length: 20 }])
  expect(measure({ x: 0, y: 0, w: 10, h: 10 }, { x: 30, y: 40, w: 10, h: 10 })).toEqual([
    { x1: 10, y1: 5, x2: 30, y2: 5, length: 20 },
    { x1: 30, y1: 5, x2: 30, y2: 40, length: 35, dashed: true },
    { x1: 5, y1: 10, x2: 5, y2: 40, length: 30 },
    { x1: 5, y1: 40, x2: 30, y2: 40, length: 25, dashed: true },
  ])
})

test('a box inside another is measured to each of its sides', () => {
  const page = { x: 0, y: 0, w: 100, h: 200 }
  expect(measure({ x: 10, y: 20, w: 30, h: 40 }, page)).toEqual([
    { x1: 0, y1: 40, x2: 10, y2: 40, length: 10 },
    { x1: 40, y1: 40, x2: 100, y2: 40, length: 60 },
    { x1: 25, y1: 0, x2: 25, y2: 20, length: 20 },
    { x1: 25, y1: 60, x2: 25, y2: 200, length: 140 },
  ])
})
