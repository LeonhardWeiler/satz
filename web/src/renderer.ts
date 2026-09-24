import type { Canvas, CanvasKit, Font, Paint, SkPicture } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { decode, type Op } from './displayList'

export type View = { x: number; y: number; zoom: number }
export type Box = { x: number; y: number; w: number; h: number }
export type Overlay = { selection: Box[]; hover?: Box; marquee?: Box; handles?: Box }

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

export class Renderer {
  private pictures = new Map<number, { hash: number; picture: SkPicture }>()
  private fonts = new Map<string, Font>()
  private paint: Paint

  constructor(
    private ck: CanvasKit,
    private engine: Engine,
  ) {
    this.paint = new ck.Paint()
    this.paint.setAntiAlias(true)
  }

  draw(canvas: Canvas, view: View, dpr: number, overlay: Overlay) {
    const { ck, paint } = this
    const ops = decode(this.engine.displayList(0))
    canvas.clear(ck.parseColorString(BACKGROUND))
    canvas.save()
    canvas.scale(dpr, dpr)
    canvas.translate(view.x, view.y)
    canvas.scale(view.zoom, view.zoom)
    let bleedBox = ck.LTRBRect(0, 0, 0, 0)
    let trimBox = bleedBox
    for (let i = 0; i < ops.length; i++) {
      const op = ops[i]
      if (op.op === 'page') {
        const b = op.bleed
        trimBox = ck.LTRBRect(0, 0, op.width, op.height)
        bleedBox = ck.LTRBRect(-b, -b, op.width + b, op.height + b)
        paint.setStyle(ck.PaintStyle.Fill)
        paint.setColor(ck.WHITE)
        canvas.drawRect(trimBox, paint)
        canvas.save()
        canvas.clipRect(bleedBox, ck.ClipOp.Intersect, true)
      } else if (op.op === 'beginItem') {
        let end = i
        while (ops[end].op !== 'endItem') end++
        let cached = this.pictures.get(op.item)
        if (cached?.hash !== op.hash) {
          cached?.picture.delete()
          const recorder = new ck.PictureRecorder()
          const rc = recorder.beginRecording(bleedBox)
          for (const o of ops.slice(i + 1, end)) this.drawOp(rc, o)
          cached = { hash: op.hash, picture: recorder.finishRecordingAsPicture() }
          recorder.delete()
          this.pictures.set(op.item, cached)
        }
        canvas.drawPicture(cached.picture)
        i = end
      } else if (op.op === 'pushClip') {
        const path = ck.Path.MakeFromCmds(op.path)!
        canvas.save()
        canvas.clipPath(path, ck.ClipOp.Intersect, true)
        path.delete()
      } else if (op.op === 'popClip') {
        canvas.restore()
      }
    }
    canvas.restore()
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setStrokeWidth(0)
    paint.setColor(ck.parseColorString(TRIM))
    canvas.drawRect(trimBox, paint)
    paint.setColor(ck.parseColorString(BLEED))
    canvas.drawRect(bleedBox, paint)
    canvas.restore()
    this.drawOverlay(canvas, view, dpr, overlay)
  }

  private drawOverlay(canvas: Canvas, view: View, dpr: number, { selection, hover, marquee, handles }: Overlay) {
    const { ck, paint } = this
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

  private drawOp(canvas: Canvas, op: Op) {
    const { ck, paint } = this
    paint.setStyle(ck.PaintStyle.Fill)
    if (op.op === 'fillPath') {
      const path = ck.Path.MakeFromCmds(op.path)
      if (!path) return
      paint.setColor(op.color)
      canvas.drawPath(path, paint)
      path.delete()
    } else if (op.op === 'glyphRun') {
      paint.setColor(op.color)
      canvas.drawGlyphs(op.glyphs, op.positions, 0, 0, this.font(op.font, op.size), paint)
    }
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
  }
}
