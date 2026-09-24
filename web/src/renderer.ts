import type { Canvas, CanvasKit, Font, Paint, Rect, SkPicture } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { close, decode, type Op, type Paint as Fill } from './displayList'

export type View = { x: number; y: number; zoom: number }
export type Box = { x: number; y: number; w: number; h: number }
export type Overlay = {
  selection: Box[]
  hover?: Box
  marquee?: Box
  handles?: Box
  pen?: { anchors: { x: number; y: number; hx: number; hy: number }[]; cursor?: { x: number; y: number } } | null
}

const FIT_PADDING = 64
export const HANDLE = 8

export function fitView(
  page: { width: number; height: number; bleed: number },
  width: number,
  height: number,
): View {
  const zoom = Math.min(
    (width - 2 * FIT_PADDING) / (page.width + 2 * page.bleed),
    (height - 2 * FIT_PADDING) / (page.height + 2 * page.bleed),
  )
  return { x: (width - page.width * zoom) / 2, y: (height - page.height * zoom) / 2, zoom }
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
  private pictures = new Map<number, { hash: number; picture: SkPicture }>()
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

  draw(canvas: Canvas, view: View, dpr: number, overlay: Overlay) {
    const { ck, chrome: paint } = this
    const ops = decode(this.engine.displayList(0))
    canvas.clear(ck.parseColorString(BACKGROUND))
    canvas.save()
    canvas.scale(dpr, dpr)
    canvas.translate(view.x, view.y)
    canvas.scale(view.zoom, view.zoom)
    let bleedBox = ck.LTRBRect(0, 0, 0, 0)
    let trimBox = bleedBox
    const page = ops[0]
    if (page?.op === 'page') {
      const b = page.bleed
      trimBox = ck.LTRBRect(0, 0, page.width, page.height)
      bleedBox = ck.LTRBRect(-b, -b, page.width + b, page.height + b)
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.WHITE)
      canvas.drawRect(trimBox, paint)
      canvas.save()
      canvas.clipRect(bleedBox, ck.ClipOp.Intersect, true)
      this.drawOps(canvas, ops, 1, ops.length, bleedBox)
      canvas.restore()
    }
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setStrokeWidth(0)
    paint.setColor(ck.parseColorString(TRIM))
    canvas.drawRect(trimBox, paint)
    paint.setColor(ck.parseColorString(BLEED))
    canvas.drawRect(bleedBox, paint)
    canvas.restore()
    this.drawOverlay(canvas, view, dpr, overlay)
  }

  private drawOverlay(canvas: Canvas, view: View, dpr: number, { selection, hover, marquee, handles, pen }: Overlay) {
    const { ck, chrome: paint } = this
    const screen = (b: Box) =>
      ck.XYWHRect(
        Math.round(view.x + b.x * view.zoom) + 0.5,
        Math.round(view.y + b.y * view.zoom) + 0.5,
        Math.round(b.w * view.zoom),
        Math.round(b.h * view.zoom),
      )
    const accent = ck.parseColorString(ACCENT)
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
      const at = (x: number, y: number) => [view.x + x * view.zoom, view.y + y * view.zoom] as const
      const last = pen.anchors.at(-1)!
      if (pen.cursor) canvas.drawLine(...at(last.x, last.y), ...at(pen.cursor.x, pen.cursor.y), paint)
      if (last.hx || last.hy) {
        canvas.drawLine(...at(last.x - last.hx, last.y - last.hy), ...at(last.x + last.hx, last.y + last.hy), paint)
      }
      for (const a of pen.anchors) {
        const [x, y] = at(a.x, a.y)
        const h = ck.XYWHRect(Math.round(x) - 3.5, Math.round(y) - 3.5, 7, 7)
        paint.setStyle(ck.PaintStyle.Fill)
        paint.setColor(ck.WHITE)
        canvas.drawRect(h, paint)
        paint.setStyle(ck.PaintStyle.Stroke)
        paint.setColor(accent)
        canvas.drawRect(h, paint)
      }
    }
    if (handles) {
      const r = screen(handles)
      canvas.drawRect(r, paint)
      for (const x of [r[0], r[2]]) {
        for (const y of [r[1], r[3]]) {
          const h = ck.XYWHRect(x - HANDLE / 2, y - HANDLE / 2, HANDLE, HANDLE)
          paint.setStyle(ck.PaintStyle.Fill)
          paint.setColor(ck.WHITE)
          canvas.drawRect(h, paint)
          paint.setStyle(ck.PaintStyle.Stroke)
          paint.setColor(accent)
          canvas.drawRect(h, paint)
        }
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
        let cached = this.pictures.get(op.item)
        if (cached?.hash !== op.hash) {
          cached?.picture.delete()
          cached = { hash: op.hash, picture: this.record(ops, i + 1, end, bounds) }
          this.pictures.set(op.item, cached)
        }
        canvas.drawPicture(cached.picture)
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
          const picture = this.record(ops, i + 1, end, bounds)
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
          picture.delete()
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
      const typeface = this.ck.Typeface.MakeTypefaceFromData(this.engine.font(id).slice().buffer)
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
    for (const { picture } of this.pictures.values()) picture.delete()
    for (const font of this.fonts.values()) font.delete()
    this.paint.delete()
    this.chrome.delete()
  }
}
