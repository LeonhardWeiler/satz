import type { Canvas, CanvasKit, Font, Image, Paint, Rect, RuntimeEffect, SkPicture, Surface, Typeface } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { close, decode, type Op, type Paint as Fill } from './displayList'
import type { Grid, Guides } from './model'
import { gridSpans } from './snap'
import { toPath, type At, type Contour } from './vector'

/** Marks the font of a glyph run whose own font is missing, drawn in the bundled one. */
const MISSING = 2 ** 31
/** Marks an image id as the C, M and Y or K plate, as `image::CMY` and `image::K` do. */
const [PLATE_CMY, PLATE_K] = [2 ** 30, 2 ** 29]

type Cache = Map<number, { hash: number; picture: SkPicture }>

export type View = { x: number; y: number; zoom: number }
/** A box turned `rotation` degrees counterclockwise around its centre. */
export type Box = { x: number; y: number; w: number; h: number; rotation?: number }
type Line = [{ x: number; y: number }, { x: number; y: number }]
export type Overlay = {
  /** The colour of the selection chrome, the theme's accent. */
  accent: string
  /** Boxes, or the ends of lines. */
  selection: (Box | Line)[]
  hover?: Box | Line
  marquee?: Box
  handles?: Box
  /** Corner radius handles in screen space. */
  radii?: { x: number; y: number }[]
  /** Ends of a selected line, shown instead of box handles. */
  ends?: { x: number; y: number }[]
  /** Where dragged layers land in an auto layout frame. */
  insert?: [{ x: number; y: number }, { x: number; y: number }]
  pen?: { anchors: { x: number; y: number; hx: number; hy: number }[]; cursor?: { x: number; y: number } } | null
  /** Display lists of the caret or selection in the text being edited, each in the space of a page at `x`. */
  text?: { ops: Uint32Array; x: number }[]
  /** Ports of a text frame in screen space: threaded, holding overset text, or open to thread on. */
  ports?: readonly { x: number; y: number; state: 'threaded' | 'overset' | 'open' }[]
  /** The knots of the path being edited and the one picked. */
  vector?: { contours: Contour[]; at: At | null }
  /** Show the layout grids and guides of the pages. */
  grids?: boolean
  /** The picked guide: a vertical one at `x` or a horizontal one at `y`. */
  guide?: { x: number } | { y: number }
  /** Lines in screen space from each frame of a thread to the next. */
  threads?: [{ x: number; y: number }, { x: number; y: number }][]
}

const FIT_PADDING = 64
const IMAGE_BYTES = 256 * 2 ** 20
export const HANDLE = 8
const PORT = 10
/** Path verbs of CanvasKit's path commands. */
const [MOVE, LINE, CLOSE] = [0, 1, 5]

/** A page on the canvas: its trim size and bleed, and the x of its left edge on its spread. */
export type Sheet = { x: number; width: number; height: number; bleed: number; grids?: Grid[]; guides?: Guides }
/** RGBA pixels drawn over the page with its bleed, `rect` in the space of the page. */
export type Inked = { width: number; height: number; image: Uint8Array; rect: Box }

/** The view that fits the pages of a spread, aligned at the top, into `width` × `height` px. */
export function fitView(sheets: Sheet[], width: number, height: number): View {
  const left = Math.min(...sheets.map((s) => s.x))
  const w = Math.max(...sheets.map((s) => s.x + s.width)) - left
  const h = Math.max(...sheets.map((s) => s.height))
  const bleed = Math.max(...sheets.map((s) => s.bleed))
  const pad = Math.min(FIT_PADDING, width / 12, height / 12)
  const zoom = Math.min((width - 2 * pad) / (w + 2 * bleed), (height - 2 * pad) / (h + 2 * bleed))
  return { x: (width - w * zoom) / 2 - left * zoom, y: (height - h * zoom) / 2, zoom }
}

const BLEED = [56, 174, 224, 0.45] as const
const CROP = [128, 131, 139, 1] as const
const GRID = [255, 72, 72, 0.12] as const
const GUIDE = [0, 170, 255, 0.9] as const
const FAR = 1e5
const OVERSET = '#ff6b5e'
const BLENDS = [
  'SrcOver', 'Multiply', 'Screen', 'Overlay', 'Darken', 'Lighten', 'ColorDodge', 'ColorBurn',
  'HardLight', 'SoftLight', 'Difference', 'Exclusion', 'Hue', 'Saturation', 'Color', 'Luminosity',
] as const
/** Shows the inverted C, M and Y plate `cmy` and K plate `k` as their inks print on paper, through the screen colours `lut` of a grid of `n` steps per ink; `on` keeps or drops each ink. */
const INKS = `
uniform shader cmy;
uniform shader k;
uniform shader lut;
uniform float n;
uniform float4 on;
half4 main(float2 p) {
  half4 a = cmy.eval(p);
  if (a.a == 0) return half4(0);
  float3 c = (1 - a.rgb / a.a) * on.rgb * (n - 1);
  float black = (1 - k.eval(p).r / a.a) * on.a * (n - 1);
  float m = min(floor(c.g), n - 2);
  float b = min(floor(black), n - 2);
  float2 at = float2(c.r + 0.5 + n * m, c.b + 0.5 + n * b);
  half4 s = mix(
    mix(lut.eval(at), lut.eval(at + float2(n, 0)), c.g - m),
    mix(lut.eval(at + float2(0, n)), lut.eval(at + float2(n, n)), c.g - m),
    black - b);
  return half4(s.rgb * a.a, a.a);
}`
/**
 * Shows `image` adjusted by [k, m, o] as `Engine.imageSource` gives them, as it is for `plate` 0, or separated
 * through the CMYK `separation` of a grid of `n` steps per channel as the inverted C, M and Y plate 1 or the K plate 2.
 */
const IMAGE = `
uniform shader image;
uniform shader separation;
uniform float3 adjust;
uniform float n;
uniform float plate;
float4 at(float3 i) {
  float2 p = float2(i.r + n * i.g + 0.5, i.b + 0.5);
  return float4(separation.eval(p).rgb, separation.eval(p + float2(0, n)).r);
}
half4 main(float2 p) {
  half4 c = image.eval(p);
  if (c.a == 0) return half4(0);
  float3 v = c.rgb / c.a;
  v = saturate(adjust.x * v - adjust.y * dot(v, float3(0.2126, 0.7152, 0.0722)) + adjust.z);
  if (plate == 0) return half4(v * c.a, c.a);
  float3 q = v * (n - 1);
  float3 i = min(floor(q), n - 2);
  float3 f = q - i;
  float3 a = f.r >= f.g ? (f.r >= f.b ? float3(1, 0, 0) : float3(0, 0, 1)) : (f.g >= f.b ? float3(0, 1, 0) : float3(0, 0, 1));
  float3 b = a + (f.r >= f.g ? (f.g >= f.b ? float3(0, 1, 0) : (f.r >= f.b ? float3(0, 0, 1) : float3(1, 0, 0))) : (f.r >= f.b ? float3(1, 0, 0) : (f.g >= f.b ? float3(0, 0, 1) : float3(0, 1, 0))));
  float x = dot(f, a), y = dot(f, b - a), z = dot(f, 1 - b);
  float4 ink = at(i) * (1 - x) + at(i + a) * (x - y) + at(i + b) * (y - z) + at(i + 1) * z;
  return half4((1 - (plate == 1 ? ink.rgb : ink.aaa)) * c.a, c.a);
}`
const CAPS = ['Butt', 'Round', 'Square'] as const
const JOINS = ['Miter', 'Round', 'Bevel'] as const

export class Renderer {
  /** Item pictures by item id, and pictures of layers with shadows by layer hash, for the page and for its C, M and Y and its K plate. */
  private caches = [0, 1, 2].map(() => ({ pictures: new Map(), layers: new Map() }) as { pictures: Cache; layers: Cache })
  private cache = this.caches[0]
  /** The plates, the shader that shows them and its screen colours, made on first use. */
  private plates: Surface[] = []
  private inksEffect?: RuntimeEffect
  private lut?: Image
  private imageEffect?: RuntimeEffect
  private separation?: Image
  private profile = ''
  private typefaces = new Map<number, Typeface>()
  private fonts = new Map<string, Font>()
  /** Decoded images by the id `source` decodes, least recently shown first; `null` while decoding or when the file does not decode. */
  private images = new Map<number, Image | null>()
  /** Images being decoded. */
  decoding = 0
  /** `Engine.imageSource` by display-list id. */
  private sources = new Map<number, Float64Array>()
  /** Images of the inks over pages by their pixels. */
  private inks = new Map<Uint8Array, Image>()
  /** Items, layers and images drawn since the last `sweep`. */
  private live = new Set<number>()
  private shown = new Set<number>()
  /** Some page was recorded since the last `sweep`. */
  private recorded = false
  /** The pages as `drawScene` last drew them, and what of. */
  private scene?: { surface: Surface; key: unknown[]; ops: Uint32Array[] }
  /** Paint for display-list content; `chrome` draws the page, guides and overlay. */
  private paint: Paint
  private chrome: Paint

  /** Images wider or taller than `maxImage` px are kept shrunk to it; `loaded` runs when an image has decoded. */
  constructor(
    private ck: CanvasKit,
    private engine: Engine,
    private loaded: () => void,
    private maxImage = Infinity,
  ) {
    this.paint = new ck.Paint()
    this.paint.setAntiAlias(true)
    this.chrome = this.paint.copy()
  }

  /**
   * Draws the display list of each page of `lists` at its x on the spread, over the
   * pages `sheets`, clipped to their bleed together: a layer across the spine shows on
   * both pages, and the bleed runs around the spread's outer edges, with their `inks`
   * over them; `cmyk` pages blend their inks as they print, of C, M, Y and K those whose bit is set in `on`.
   * With an `overlay` the pages are drawn again only when their display lists, inks or the view change.
   */
  draw(canvas: Canvas, lists: { id: string; x: number; inks?: Inked }[], sheets: Sheet[], view: View, dpr: number, overlay: Overlay | null, cmyk = false, on = 15) {
    const { ck, chrome: paint } = this
    canvas.clear(ck.TRANSPARENT)
    const place = (c: Canvas) => {
      c.save()
      c.scale(dpr, dpr)
      c.translate(view.x, view.y)
      c.scale(view.zoom, view.zoom)
    }
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
    const { live, shown } = this
    const pages = (c: Canvas, plate: number) => {
      this.cache = this.caches[plate]
      this.recorded = true
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.WHITE)
      for (const r of trims) c.drawRect(r, paint)
      c.save()
      c.clipPath(bleed, ck.ClipOp.Intersect, true)
      for (const { id, x } of lists) {
        const ops = decode((plate ? this.engine.plate(id, plate === 2) : this.engine.displayList(id)).slice())
        for (const op of ops) {
          if (op.op === 'image') {
            const id = this.source(op.image)[0] ?? op.image
            if (!this.images.has(id)) this.decode(id)
            shown.add(id).add(op.image)
          } else if (op.op === 'beginItem') live.add(op.item)
          else if (op.op === 'pushLayer') live.add(op.hash)
        }
        c.save()
        c.translate(x, 0)
        const local = ck.LTRBRect(bounds[0] - x, bounds[1], bounds[2] - x, bounds[3])
        this.drawOps(c, ops, ops[0]?.op === 'page' ? 1 : 0, ops.length, local)
        c.restore()
      }
      c.restore()
    }
    const content = (c: Canvas) => {
      if (cmyk) {
        this.drawPlates(c, on, (p, plate) => {
          place(p)
          pages(p, plate)
          p.restore()
        })
      }
      place(c)
      if (!cmyk) pages(c, 0)
      c.clipPath(bleed, ck.ClipOp.Intersect, true)
      for (const { x, inks } of lists) {
        const image = inks && this.inkImage(inks)
        if (!image) continue
        const { x: l, y: t, w, h } = inks.rect
        c.drawImageRectOptions(image, ck.XYWHRect(0, 0, inks.width, inks.height), ck.XYWHRect(x + l, t, w, h), ck.FilterMode.Nearest, ck.MipmapMode.None, null)
      }
      c.restore()
    }
    if (overlay) this.drawScene(canvas, lists, sheets, view, dpr, cmyk, on, content)
    else content(canvas)
    for (const [pixels, image] of this.inks) {
      if (lists.some((l) => l.inks?.image === pixels)) continue
      image.delete()
      this.inks.delete(pixels)
    }
    place(canvas)
    if (overlay?.grids) {
      paint.setColor(ck.Color(...GRID))
      paint.setStrokeWidth(1 / view.zoom)
      for (const s of sheets) {
        const { x, y } = gridSpans(s)
        for (const [spans, across] of [[x, s.height], [y, s.width]] as const) {
          for (const [a, b] of spans) {
            const r = spans === x ? ck.XYWHRect(a, 0, b - a, across) : ck.XYWHRect(s.x, a, across, b - a)
            paint.setStyle(a === b ? ck.PaintStyle.Stroke : ck.PaintStyle.Fill)
            if (a === b) canvas.drawLine(r[0], r[1], r[2], r[3], paint)
            else canvas.drawRect(r, paint)
          }
        }
      }
      const line = (g: { x: number } | { y: number }) => ('x' in g ? canvas.drawLine(g.x, -FAR, g.x, FAR, paint) : canvas.drawLine(-FAR, g.y, FAR, g.y, paint))
      paint.setColor(ck.Color(...GUIDE))
      paint.setStyle(ck.PaintStyle.Stroke)
      for (const s of sheets) {
        for (const x of s.guides?.x ?? []) line({ x: s.x + x })
        for (const y of s.guides?.y ?? []) line({ y })
      }
      if (overlay.guide) {
        paint.setColor(ck.parseColorString(overlay.accent))
        paint.setStrokeWidth(2 / view.zoom)
        line(overlay.guide)
      }
    }
    for (const { ops, x } of overlay?.text ?? []) {
      canvas.save()
      canvas.translate(x, 0)
      const list = decode(ops)
      this.drawOps(canvas, list, 0, list.length, bounds)
      canvas.restore()
    }
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setStrokeWidth(1 / view.zoom)
    paint.setColor(ck.Color(...BLEED))
    const dash = ck.PathEffect.MakeDash([4 / view.zoom, 3 / view.zoom])
    paint.setPathEffect(dash)
    canvas.drawPath(bleed, paint)
    paint.setPathEffect(null)
    dash?.delete()
    paint.setColor(ck.Color(...CROP))
    for (const r of trims) canvas.drawRect(r, paint)
    const l = sheets[0].x
    const r = Math.max(...sheets.map((s) => s.x + s.width))
    const b = Math.max(...sheets.map((s) => s.height))
    const off = Math.max(...sheets.map((s) => s.bleed)) + 4 / view.zoom
    const len = 14 / view.zoom
    for (const [x, y, dx, dy] of [[l, 0, -1, -1], [r, 0, 1, -1], [l, b, -1, 1], [r, b, 1, 1]]) {
      canvas.drawLine(x + dx * off, y, x + dx * (off + len), y, paint)
      canvas.drawLine(x, y + dy * off, x, y + dy * (off + len), paint)
    }
    bleed.delete()
    canvas.restore()
    if (overlay) this.drawOverlay(canvas, view, dpr, overlay)
  }

  /** Forgets the pictures that no `draw` since the last sweep used, and the least recently shown images beyond `IMAGE_BYTES`. */
  sweep() {
    if (!this.recorded) return
    this.recorded = false
    for (const cache of this.caches.flatMap((c) => [c.pictures, c.layers])) {
      for (const [key, { picture }] of cache) {
        if (this.live.has(key)) continue
        picture.delete()
        cache.delete(key)
      }
    }
    for (const id of this.shown) {
      const image = this.images.get(id)
      if (image === undefined) continue
      this.images.delete(id)
      this.images.set(id, image)
    }
    const size = (i: Image | null) => (i ? i.width() * i.height() * 4 : 0)
    let bytes = 0
    for (const image of this.images.values()) bytes += size(image)
    for (const [id, image] of this.images) {
      if (bytes <= IMAGE_BYTES || this.shown.has(id)) break
      bytes -= size(image)
      image?.delete()
      this.images.delete(id)
    }
    for (const id of this.sources.keys()) if (!this.shown.has(id)) this.sources.delete(id)
    this.live.clear()
    this.shown.clear()
  }

  private source(id: number) {
    let s = this.sources.get(id)
    if (!s) this.sources.set(id, (s = this.engine.imageSource(id)))
    return s
  }

  /** Decodes the image `id` of the engine off the main thread, shrunk to `maxImage`. */
  private decode(id: number) {
    const { ck, images } = this
    images.set(id, null)
    this.decoding++
    const done = (bitmap: ImageBitmap | null) => {
      this.decoding--
      if (images.get(id) !== null) return bitmap?.close()
      images.set(id, bitmap && ck.MakeLazyImageFromTextureSource(bitmap, { width: bitmap.width, height: bitmap.height, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB }))
      this.clear()
      this.loaded()
    }
    createImageBitmap(new Blob([this.engine.image(id) as Uint8Array<ArrayBuffer>]))
      .then((full) => {
        const f = this.maxImage / Math.max(full.width, full.height)
        if (f >= 1) return full
        const [resizeWidth, resizeHeight] = [Math.ceil(full.width * f), Math.ceil(full.height * f)]
        return createImageBitmap(full, { resizeWidth, resizeHeight, resizeQuality: 'high' }).finally(() => full.close())
      })
      .then(done, () => done(null))
  }

  /** Draws what `content` draws through a surface that keeps it while the pages, the view and the size of `canvas` stay. */
  private drawScene(canvas: Canvas, lists: { id: string; x: number; inks?: Inked }[], sheets: Sheet[], view: View, dpr: number, cmyk: boolean, on: number, content: (c: Canvas) => void) {
    const { ck } = this
    const [, , width, height] = canvas.getDeviceClipBounds()
    const key = [width, height, dpr, view.x, view.y, view.zoom, cmyk, on, ...sheets.flatMap((s) => [s.x, s.width, s.height, s.bleed]), ...lists.flatMap((l) => [l.x, l.inks?.image])]
    const ops = lists.map((l) => this.engine.displayList(l.id).slice())
    const same = (a: ArrayLike<unknown>, b: ArrayLike<unknown>) => a.length === b.length && Array.prototype.every.call(a, (v, i) => v === b[i])
    let scene = this.scene
    if (!scene || !same(scene.key, key) || scene.ops.length !== ops.length || ops.some((o, i) => !same(o, scene!.ops[i]))) {
      let surface = scene?.surface
      if (surface?.width() !== width || surface.height() !== height) {
        surface?.delete()
        this.scene = undefined
        if (!width || !height) return
        surface = canvas.makeSurface({ width, height, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Premul, colorSpace: ck.ColorSpace.SRGB })!
      }
      surface.getCanvas().clear(ck.TRANSPARENT)
      content(surface.getCanvas())
      scene = this.scene = { surface, key, ops }
    }
    const image = scene.surface.makeImageSnapshot()
    canvas.drawImage(image, 0, 0, null)
    image.delete()
  }

  /** Draws the plates 1 and 2 that `draw` draws, each on a surface of its own, as their inks print. */
  private drawPlates(canvas: Canvas, on: number, draw: (c: Canvas, plate: number) => void) {
    const { ck } = this
    const [, , width, height] = canvas.getDeviceClipBounds()
    if (this.plates[0]?.width() !== width || this.plates[0]?.height() !== height) {
      for (const s of this.plates) s.delete()
      const info = { width, height, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Premul, colorSpace: ck.ColorSpace.SRGB }
      this.plates = [canvas.makeSurface(info)!, canvas.makeSurface(info)!]
    }
    const plates = this.plates.map((s, i) => {
      s.getCanvas().clear(ck.TRANSPARENT)
      draw(s.getCanvas(), i + 1)
      const image = s.makeImageSnapshot()
      const shader = image.makeShaderOptions(ck.TileMode.Clamp, ck.TileMode.Clamp, ck.FilterMode.Nearest, ck.MipmapMode.None)
      image.delete()
      return shader
    })
    if (!this.lut) {
      const pixels = this.engine.lut()
      const size = Math.round(Math.sqrt(pixels.length / 4))
      const info = { width: size, height: size, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB }
      this.lut = ck.MakeImage(info, pixels, size * 4)!
      this.inksEffect = ck.RuntimeEffect.Make(INKS)!
    }
    const table = this.lut.makeShaderOptions(ck.TileMode.Clamp, ck.TileMode.Clamp, ck.FilterMode.Linear, ck.MipmapMode.None)
    const shader = this.inksEffect!.makeShaderWithChildren([Math.round(Math.sqrt(this.lut.width())), ...[0, 1, 2, 3].map((b) => (on >> b) & 1)], [...plates, table])
    const paint = new ck.Paint()
    paint.setShader(shader)
    canvas.drawPaint(paint)
    paint.delete()
    shader.delete()
    table.delete()
    for (const s of plates) s.delete()
  }

  private drawOverlay(
    canvas: Canvas,
    view: View,
    dpr: number,
    { accent: color, selection, hover, marquee, handles, radii, ends, pen, insert, ports, threads, vector }: Overlay,
  ) {
    const { ck, chrome: paint } = this
    const screen = (b: Box) =>
      ck.XYWHRect(
        Math.round(view.x + b.x * view.zoom) + 0.5,
        Math.round(view.y + b.y * view.zoom) + 0.5,
        Math.round(b.w * view.zoom),
        Math.round(b.h * view.zoom),
      )
    const accent = ck.parseColorString(color)
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
    /** Draws `draw` turned as `b` is. */
    const turned = (b: Box, draw: () => void) => {
      canvas.save()
      if (b.rotation) canvas.rotate(-b.rotation, ...at(b.x + b.w / 2, b.y + b.h / 2))
      draw()
      canvas.restore()
    }
    canvas.save()
    canvas.scale(dpr, dpr)
    paint.setStyle(ck.PaintStyle.Stroke)
    paint.setColor(accent)
    paint.setStrokeWidth(1)
    const outline = (b: Box | Line) =>
      Array.isArray(b) ? canvas.drawLine(...at(b[0].x, b[0].y), ...at(b[1].x, b[1].y), paint) : turned(b, () => canvas.drawRect(screen(b), paint))
    selection.forEach(outline)
    if (hover) {
      paint.setStrokeWidth(2)
      outline(hover)
      paint.setStrokeWidth(1)
    }
    if (marquee) {
      const r = ck.XYWHRect(marquee.x + 0.5, marquee.y + 0.5, marquee.w, marquee.h)
      paint.setStyle(ck.PaintStyle.Fill)
      paint.setColor(ck.Color4f(accent[0], accent[1], accent[2], 0.08))
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
    if (vector) {
      const screen = vector.contours.map((c) => ({
        ...c,
        knots: c.knots.map((k) => {
          const [[x, y], [ix, iy], [ox, oy]] = [at(k.x, k.y), at(k.ix, k.iy), at(k.ox, k.oy)]
          return { x, y, ix, iy, ox, oy }
        }),
      }))
      const path = ck.Path.MakeFromCmds(toPath(screen))
      if (path) canvas.drawPath(path, paint)
      path?.delete()
      screen.forEach(({ knots }, c) =>
        knots.forEach((k, i) => {
          const picked = vector.at?.[0] === c && vector.at[1] === i
          if (picked) {
            for (const [x, y] of [[k.ix, k.iy], [k.ox, k.oy]]) {
              if (x === k.x && y === k.y) continue
              paint.setStyle(ck.PaintStyle.Stroke)
              paint.setColor(accent)
              canvas.drawLine(k.x, k.y, x, y, paint)
              canvas.drawCircle(x, y, 3, paint)
            }
          }
          square(k.x, k.y, 7)
          if (picked) {
            paint.setStyle(ck.PaintStyle.Fill)
            canvas.drawRect(ck.XYWHRect(Math.floor(k.x - 3.5) + 0.5, Math.floor(k.y - 3.5) + 0.5, 7, 7), paint)
          }
        }),
      )
      paint.setStyle(ck.PaintStyle.Stroke)
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
      turned(handles, () => {
        const r = screen(handles)
        canvas.drawRect(r, paint)
        for (const x of [r[0], r[2]]) for (const y of [r[1], r[3]]) square(x, y, HANDLE)
        for (const p of radii ?? []) {
          paint.setStyle(ck.PaintStyle.Fill)
          paint.setColor(ck.WHITE)
          canvas.drawCircle(p.x, p.y, HANDLE / 2, paint)
          paint.setStyle(ck.PaintStyle.Stroke)
          paint.setColor(accent)
          canvas.drawCircle(p.x, p.y, HANDLE / 2, paint)
        }
      })
    }
    for (const [a, b] of threads ?? []) canvas.drawLine(a.x, a.y, b.x, b.y, paint)
    for (const p of ports ?? []) {
      square(p.x, p.y, PORT)
      const [x, y] = [Math.floor(p.x) + 0.5, Math.floor(p.y) + 0.5]
      if (p.state === 'overset' || p.state === 'open') {
        if (p.state === 'overset') paint.setColor(ck.parseColorString(OVERSET))
        paint.setStrokeWidth(p.state === 'overset' ? 2 : 1.5)
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
        canvas.drawPicture(this.cached(this.cache.pictures, op.item, op.hash, () => this.record(ops, i + 1, end, bounds)))
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
      } else if (op.op === 'pushTransform') {
        const end = close(ops, i)
        const [a, b, c, d, e, f] = op.transform
        canvas.save()
        canvas.concat([a, c, e, b, d, f, 0, 0, 1])
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
          const picture = this.cached(this.cache.layers, op.hash, op.hash, () => this.record(ops, i + 1, end, bounds))
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
    if (op.op === 'image') return this.drawImage(canvas, op)
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

  private drawImage(canvas: Canvas, { image, transform }: Extract<Op, { op: 'image' }>) {
    const { ck, paint } = this
    const [id, ...adjust] = this.source(image)
    const img = this.images.get(id ?? image)
    if (!img) return
    const [a, b, c, d, e, f] = transform
    canvas.save()
    canvas.concat([a, c, e, b, d, f, 0, 0, 1])
    paint.setStyle(ck.PaintStyle.Fill)
    paint.setColor(ck.BLACK)
    const [w, h] = [img.width(), img.height()]
    if (id === undefined) canvas.drawImageRectOptions(img, ck.XYWHRect(0, 0, w, h), ck.XYWHRect(0, 0, 1, 1), ck.FilterMode.Linear, ck.MipmapMode.Linear, paint)
    else {
      if (!this.separation) {
        const pixels = this.engine.separation()
        const n = Math.round(Math.cbrt(pixels.length / 8))
        const info = { width: n * n, height: 2 * n, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB }
        this.separation = ck.MakeImage(info, pixels, n * n * 4)!
        this.imageEffect = ck.RuntimeEffect.Make(IMAGE)!
      }
      const plate = image & PLATE_CMY ? 1 : image & PLATE_K ? 2 : 0
      const pixels = img.makeShaderOptions(ck.TileMode.Clamp, ck.TileMode.Clamp, ck.FilterMode.Linear, ck.MipmapMode.Linear, [1 / w, 0, 0, 0, 1 / h, 0, 0, 0, 1])
      const table = this.separation.makeShaderOptions(ck.TileMode.Clamp, ck.TileMode.Clamp, ck.FilterMode.Nearest, ck.MipmapMode.None)
      const shader = this.imageEffect!.makeShaderWithChildren([...adjust, this.separation.height() / 2, plate], [pixels, table])
      paint.setShader(shader)
      canvas.drawRect(ck.XYWHRect(0, 0, 1, 1), paint)
      paint.setShader(null)
      for (const s of [shader, pixels, table]) s.delete()
    }
    canvas.restore()
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

  /** Forgets what shows through the CMYK profile when it is no longer `name`. */
  reprofile(name: string) {
    if (name === this.profile) return
    this.profile = name
    for (const x of [this.lut, this.inksEffect, this.separation, this.imageEffect]) x?.delete()
    this.lut = this.inksEffect = this.separation = this.imageEffect = undefined
    for (const image of this.images.values()) image?.delete()
    this.images.clear()
    this.clear()
  }

  /** Forgets the pictures of all items and layers, and the pages last drawn. */
  private clear() {
    this.scene?.surface.delete()
    this.scene = undefined
    for (const cache of this.caches.flatMap((c) => [c.pictures, c.layers])) {
      for (const { picture } of cache.values()) picture.delete()
      cache.clear()
    }
  }

  private inkImage({ width, height, image }: Inked) {
    let i = this.inks.get(image)
    if (!i) {
      const { ck } = this
      const info = { width, height, alphaType: ck.AlphaType.Unpremul, colorType: ck.ColorType.RGBA_8888, colorSpace: ck.ColorSpace.SRGB }
      i = ck.MakeImage(info, image, width * 4) ?? undefined
      if (i) this.inks.set(image, i)
    }
    return i
  }

  delete() {
    this.clear()
    for (const s of this.plates) s.delete()
    for (const x of [this.lut, this.inksEffect, this.separation, this.imageEffect]) x?.delete()
    for (const image of this.inks.values()) image.delete()
    for (const font of this.fonts.values()) font.delete()
    for (const typeface of this.typefaces.values()) typeface.delete()
    for (const image of this.images.values()) image?.delete()
    this.images.clear()
    this.paint.delete()
    this.chrome.delete()
  }
}
