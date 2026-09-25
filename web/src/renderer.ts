import type { Canvas, CanvasKit, Font, Paint, Rect, SkPicture, Typeface } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { close, decode, type Op, type Paint as Fill } from './displayList'

/** Marks the font of a glyph run whose own font is missing, drawn in the bundled one. */
const MISSING = 2 ** 31

type Cache = Map<number, { hash: number; picture: SkPicture }>

export type View = { x: number; y: number; zoom: number }
export type Box = { x: number; y: number; w: number; h: number }
export type Overlay = {
  selection: Box[]
  hover?: Box
  marquee?: Box
  handles?: Box
  /** Ends of a selected line, shown instead of box handles. */
  ends?: { x: number; y: number }[]
  /** Where dragged layers land in an auto layout frame. */
  insert?: [{ x: number; y: number }, { x: number; y: number }]
  pen?: { anchors: { x: number; y: number; hx: number; hy: number }[]; cursor?: { x: number; y: number } } | null
  /** Display lists of the caret or selection in the text being edited, each in the space of a page at `x`. */
  text?: { ops: Uint32Array; x: number }[]
  /** In- and out-ports of a text frame in screen space: empty, threaded, or holding overset text. */
  ports?: { x: number; y: number; state: 'empty' | 'threaded' | 'overset' }[]
  /** Lines in screen space from each frame of a thread to the next. */
  threads?: [{ x: number; y: number }, { x: number; y: number }][]
}

const FIT_PADDING = 64
export const HANDLE = 8
const PORT = 10
/** Path verbs of CanvasKit's path commands. */
const [MOVE, LINE, CLOSE] = [0, 1, 5]

/** A page on the canvas: its trim size and bleed, and the x of its left edge on its spread. */
export type Sheet = { x: number; width: number; height: number; bleed: number }

/** The view that fits the pages of a spread, aligned at the top, into `width` × `height` px. */
export function fitView(sheets: Sheet[], width: number, height: number): View {
  const left = Math.min(...sheets.map((s) => s.x))
  const w = Math.max(...sheets.map((s) => s.x + s.width)) - left
  const h = Math.max(...sheets.map((s) => s.height))
  const bleed = Math.max(...sheets.map((s) => s.bleed))
  const zoom = Math.min((width - 2 * FIT_PADDING) / (w + 2 * bleed), (height - 2 * FIT_PADDING) / (h + 2 * bleed))
  return { x: (width - w * zoom) / 2 - left * zoom, y: (height - h * zoom) / 2, zoom }
}

const BACKGROUND = '#1e1e1e'
const TRIM = '#000000'
const BLEED = '#ff3b30'
const ACCENT = '#0d99ff'
const BLENDS = [
  'SrcOver', 'Multiply', 'Screen', 'Overlay', 'Darken', 'Lighten', 'ColorDodge', 'ColorBurn',
  'HardLight', 'SoftLight', 'Difference', 'Exclusion', 'Hue', 'Saturation', 'Color', 'Luminosity',
] as const
const CAPS = ['Butt', 'Round', 'Square'] as const
const JOINS = ['Miter', 'Round', 'Bevel'] as const

export class Renderer {
  /** Item pictures by item id, and pictures of layers with shadows by layer hash. */
  private pictures: Cache = new Map()
  private layers: Cache = new Map()
  private typefaces = new Map<number, Typeface>()
  private fonts = new Map<string, Font>()
  /** Paint for display-list content; `chrome` draws the page, guides and overlay. */
  private paint: Paint
  private chrome: Paint

  constructor(
    private ck: CanvasKit,
    private engine: Engine,
  ) {
    this.paint = new ck.Paint()
    this.paint.setAntiAlias(true)
    this.chrome = this.paint.copy()
  }

  /**
   * Draws the display list of each page of `lists` at its x on the spread, over the
   * pages `sheets`, clipped to their bleed together: a layer across the spine shows on
   * both pages, and the bleed runs around the spread's outer edges.
   */
  draw(canvas: Canvas, lists: { id: string; x: number }[], sheets: Sheet[], view: View, dpr: number, overlay: Overlay) {
    const { ck, chrome: paint } = this
    canvas.clear(ck.parseColorString(BACKGROUND))
    canvas.save()
    canvas.scale(dpr, dpr)
    canvas.translate(view.x, view.y)
    canvas.scale(view.zoom, view.zoom)
    const trims = sheets.map((s) => ck.XYWHRect(s.x, 0, s.width, s.height))
    const bleeds = sheets.map((s) => ck.LTRBRect(s.x - s.bleed, -s.bleed, s.x + s.width + s.bleed, s.height + s.bleed))
    const box = ([l, t, r, b]: Float32Array) => ck.Path.MakeFromCmds([MOVE, l, t, LINE, r, t, LINE, r, b, LINE, l, b, CLOSE])!
    let bleed = box(bleeds[0])
    for (const r of bleeds.slice(1)) {
      const b = box(r)
      const u = ck.Path.MakeFromOp(bleed, b, ck.PathOp.Union)
      b.delete()
      if (u) {
        bleed.delete()
        bleed = u
      }
    }
    const bounds = bleed.getBounds()
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setColor(ck.WHITE)
    for (const r of trims) canvas.drawRect(r, paint)
    canvas.save()
    canvas.clipPath(bleed, ck.ClipOp.Intersect, true)
    const live = new Set<number>()
    for (const { id, x } of lists) {
      const ops = decode(this.engine.displayList(id))
      for (const op of ops) {
        if (op.op === 'beginItem') live.add(op.item)
        else if (op.op === 'pushLayer') live.add(op.hash)
      }
      canvas.save()
      canvas.translate(x, 0)
      const local = ck.LTRBRect(bounds[0] - x, bounds[1], bounds[2] - x, bounds[3])
      this.drawOps(canvas, ops, ops[0]?.op === 'page' ? 1 : 0, ops.length, local)
      canvas.restore()
    }
    canvas.restore()
    for (const { ops, x } of overlay.text ?? []) {
      canvas.save()
      canvas.translate(x, 0)
      for (const op of decode(ops)) this.drawOp(canvas, op)
      canvas.restore()
    }
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setStrokeWidth(0)
    paint.setColor(ck.parseColorString(TRIM))
    for (const r of trims) canvas.drawRect(r, paint)
    paint.setColor(ck.parseColorString(BLEED))
    canvas.drawPath(bleed, paint)
    bleed.delete()
    canvas.restore()
    this.drawOverlay(canvas, view, dpr, overlay)
    for (const cache of [this.pictures, this.layers]) {
      for (const [key, { picture }] of cache) {
        if (live.has(key)) continue
        picture.delete()
        cache.delete(key)
      }
    }
  }

  private drawOverlay(
    canvas: Canvas,
    view: View,
    dpr: number,
    { selection, hover, marquee, handles, ends, pen, insert, ports, threads }: Overlay,
  ) {
    const { ck, chrome: paint } = this
    const screen = (b: Box) =>
      ck.XYWHRect(
        Math.round(view.x + b.x * view.zoom) + 0.5,
        Math.round(view.y + b.y * view.zoom) + 0.5,
        Math.round(b.w * view.zoom),
        Math.round(b.h * view.zoom),
      )
    const accent = ck.parseColorString(ACCENT)
    const at = (x: number, y: number) => [view.x + x * view.zoom, view.y + y * view.zoom] as const
    const square = (x: number, y: number, size: number) => {
      const h = ck.XYWHRect(Math.floor(x - size / 2) + 0.5, Math.floor(y - size / 2) + 0.5, size, size)
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.WHITE)
      canvas.drawRect(h, paint)
      paint.setStyle(ck.PaintStyle.Stroke)
      paint.setColor(accent)
      canvas.drawRect(h, paint)
    }
    canvas.save()
    canvas.scale(dpr, dpr)
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setColor(accent)
    paint.setStrokeWidth(1)
    for (const b of selection) canvas.drawRect(screen(b), paint)
    if (hover) {
      paint.setStrokeWidth(2)
      canvas.drawRect(screen(hover), paint)
      paint.setStrokeWidth(1)
    }
    if (marquee) {
      const r = ck.XYWHRect(marquee.x + 0.5, marquee.y + 0.5, marquee.w, marquee.h)
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.Color(13, 153, 255, 0.1))
      canvas.drawRect(r, paint)
      paint.setStyle(ck.PaintStyle.Stroke)
      paint.setColor(accent)
      canvas.drawRect(r, paint)
    }
    if (pen) {
      const last = pen.anchors.at(-1)!
      if (pen.cursor) canvas.drawLine(...at(last.x, last.y), ...at(pen.cursor.x, pen.cursor.y), paint)
      if (last.hx || last.hy) {
        canvas.drawLine(...at(last.x - last.hx, last.y - last.hy), ...at(last.x + last.hx, last.y + last.hy), paint)
      }
      for (const a of pen.anchors) square(...at(a.x, a.y), 7)
    }
    if (ends) {
      const [a, b] = ends
      canvas.drawLine(...at(a.x, a.y), ...at(b.x, b.y), paint)
      for (const e of ends) square(...at(e.x, e.y), HANDLE)
    }
    if (insert) {
      paint.setStrokeWidth(2)
      canvas.drawLine(...at(insert[0].x, insert[0].y), ...at(insert[1].x, insert[1].y), paint)
      paint.setStrokeWidth(1)
    }
    if (handles) {
      const r = screen(handles)
      canvas.drawRect(r, paint)
      for (const x of [r[0], r[2]]) for (const y of [r[1], r[3]]) square(x, y, HANDLE)
    }
    for (const [a, b] of threads ?? []) canvas.drawLine(a.x, a.y, b.x, b.y, paint)
    for (const p of ports ?? []) {
      square(p.x, p.y, PORT)
      const [x, y] = [Math.floor(p.x) + 0.5, Math.floor(p.y) + 0.5]
      if (p.state === 'overset') {
        paint.setColor(ck.parseColorString(BLEED))
        paint.setStrokeWidth(2)
        canvas.drawLine(x - 3, y, x + 3, y, paint)
        canvas.drawLine(x, y - 3, x, y + 3, paint)
        paint.setStrokeWidth(1)
        paint.setColor(accent)
      } else if (p.state === 'threaded') {
        const path = ck.Path.MakeFromCmds([MOVE, x - 2, y - 3, LINE, x + 3, y, LINE, x - 2, y + 3, CLOSE])
        paint.setStyle(ck.PaintStyle.Fill)
        if (path) canvas.drawPath(path, paint)
        paint.setStyle(ck.PaintStyle.Stroke)
        path?.delete()
      }
    }
    canvas.restore()
  }

  private drawOps(canvas: Canvas, ops: Op[], from: number, to: number, bounds: Rect) {
    const { ck } = this
    for (let i = from; i < to; i++) {
      const op = ops[i]
      if (op.op === 'beginItem') {
        const end = close(ops, i)
        canvas.drawPicture(this.cached(this.pictures, op.item, op.hash, () => this.record(ops, i + 1, end, bounds)))
        i = end
      } else if (op.op === 'pushClip') {
        const end = close(ops, i)
        const path = ck.Path.MakeFromCmds(op.path)!
        canvas.save()
        canvas.clipPath(path, op.invert ? ck.ClipOp.Difference : ck.ClipOp.Intersect, true)
        path.delete()
        this.drawOps(canvas, ops, i + 1, end, bounds)
        canvas.restore()
        i = end
      } else if (op.op === 'pushLayer') {
        const end = close(ops, i)
        const layer = new ck.Paint()
        layer.setAlphaf(op.opacity)
        layer.setBlendMode(ck.BlendMode[BLENDS[op.blend]])
        const blur = op.blur > 0 ? ck.ImageFilter.MakeBlur(op.blur, op.blur, ck.TileMode.Decal, null) : null
        if (!op.shadows.length) {
          layer.setImageFilter(blur)
          canvas.saveLayer(layer)
          this.drawOps(canvas, ops, i + 1, end, bounds)
        } else {
          const picture = this.cached(this.layers, op.hash, op.hash, () => this.record(ops, i + 1, end, bounds))
          canvas.saveLayer(layer)
          for (const s of op.shadows) {
            const p = new ck.Paint()
            const f = ck.ImageFilter.MakeDropShadowOnly(s.offset[0], s.offset[1], s.blur, s.blur, s.color, null)
            p.setImageFilter(f)
            canvas.saveLayer(p)
            canvas.drawPicture(picture)
            canvas.restore()
            f.delete()
            p.delete()
          }
          const p = new ck.Paint()
          p.setImageFilter(blur)
          canvas.saveLayer(p)
          canvas.drawPicture(picture)
          canvas.restore()
          p.delete()
        }
        canvas.restore()
        blur?.delete()
        layer.delete()
        i = end
      } else if (op.op === 'beginMask') {
        const mid = close(ops, i)
        const end = close(ops, mid)
        const mask = new ck.Paint()
        mask.setBlendMode(ck.BlendMode.DstIn)
        canvas.saveLayer()
        this.drawOps(canvas, ops, mid + 1, end, bounds)
        canvas.saveLayer(mask)
        this.drawOps(canvas, ops, i + 1, mid, bounds)
        canvas.restore()
        canvas.restore()
        mask.delete()
        i = end
      } else this.drawOp(canvas, op)
    }
  }

  /** The picture cached under `key`, recorded again when `hash` changes. */
  private cached(cache: Cache, key: number, hash: number, record: () => SkPicture) {
    let entry = cache.get(key)
    if (entry?.hash !== hash) {
      entry?.picture.delete()
      entry = { hash, picture: record() }
      cache.set(key, entry)
    }
    return entry.picture
  }

  private record(ops: Op[], from: number, to: number, bounds: Rect) {
    const recorder = new this.ck.PictureRecorder()
    this.drawOps(recorder.beginRecording(bounds), ops, from, to, bounds)
    const picture = recorder.finishRecordingAsPicture()
    recorder.delete()
    return picture
  }

  private drawOp(canvas: Canvas, op: Op) {
    if (op.op !== 'fillPath' && op.op !== 'strokePath' && op.op !== 'glyphRun') return
    const { ck, paint } = this
    if (op.op === 'glyphRun' && op.font >= MISSING) {
      const font = this.font(op.font, op.size)
      const widths = font.getGlyphWidths(op.glyphs)
      const { ascent, descent } = font.getMetrics()
      const [x, y] = op.positions
      const last = op.glyphs.length - 1
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.Color(255, 64, 160, 0.6))
      canvas.drawRect(ck.LTRBRect(x, y + ascent, op.positions[2 * last] + widths[last], y + descent), paint)
    }
    const shader = this.setPaint(op.paint)
    if (op.op === 'glyphRun') canvas.drawGlyphs(op.glyphs, op.positions, 0, 0, this.font(op.font, op.size), paint)
    else {
      const path = ck.Path.MakeFromCmds(op.path)
      if (op.op === 'strokePath') {
        paint.setStyle(ck.PaintStyle.Stroke)
        paint.setStrokeWidth(op.width)
        paint.setStrokeCap(ck.StrokeCap[CAPS[op.cap]])
        paint.setStrokeJoin(ck.StrokeJoin[JOINS[op.join]])
        paint.setStrokeMiter(4)
      }
      if (path) canvas.drawPath(path, paint)
      path?.delete()
    }
    paint.setShader(null)
    shader?.delete()
  }

  /** Sets fill style and paint; returns the shader to delete after drawing and detaching. */
  private setPaint(p: Fill) {
    const { ck, paint } = this
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setShader(null)
    if (p.type === 'solid' || p.stops.length < 2) {
      paint.setColor(p.type === 'solid' ? p.color : (p.stops[0]?.color ?? ck.TRANSPARENT))
      return null
    }
    const [a, b, c, d, e, f] = p.transform
    const matrix = [a, c, e, b, d, f, 0, 0, 1]
    const colors = p.stops.map((s) => s.color)
    const pos = p.stops.map((s) => s.at)
    const shader =
      p.type === 'linear'
        ? ck.Shader.MakeLinearGradient([0, 0], [1, 0], colors, pos, ck.TileMode.Clamp, matrix)
        : ck.Shader.MakeRadialGradient([0, 0], 1, colors, pos, ck.TileMode.Clamp, matrix)
    paint.setColor(ck.BLACK)
    paint.setShader(shader)
    return shader
  }

  private font(id: number, size: number) {
    const key = `${id}/${size}`
    let font = this.fonts.get(key)
    if (!font) {
      let typeface = this.typefaces.get(id)
      if (!typeface) {
        typeface = this.ck.Typeface.MakeTypefaceFromData(this.engine.font(id).slice().buffer)!
        this.typefaces.set(id, typeface)
      }
      font = new this.ck.Font(typeface, size)
      font.setHinting(this.ck.FontHinting.None)
      font.setSubpixel(true)
      font.setLinearMetrics(true)
      font.setEdging(this.ck.FontEdging.AntiAlias)
      this.fonts.set(key, font)
    }
    return font
  }

  delete() {
    for (const { picture } of [...this.pictures.values(), ...this.layers.values()]) picture.delete()
    for (const font of this.fonts.values()) font.delete()
    for (const typeface of this.typefaces.values()) typeface.delete()
    this.paint.delete()
    this.chrome.delete()
  }
}
