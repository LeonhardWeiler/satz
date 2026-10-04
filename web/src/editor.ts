import { useSyncExternalStore } from 'react'
import type { Engine } from './engine/engine'
import { textSizing, type Command, type Container, type Modes, type Node, type Page, type Palette, type Resizing, type Scope, type Snapshot, type TextNode, type Typeface } from './model'
import type { Previewed } from './exportWorker'
import type { Handle } from './file'
import { penPath, type Anchor } from './pen'
import { spin } from './handles'
import type { Box } from './renderer'
import { index, type Entry } from './select'
import { contours, toPath, type At, type Contour } from './vector'
import type { Editing } from './textEdit'

export type Shape = 'rect' | 'line' | 'ellipse' | 'polygon' | 'star'
export type Tool = 'move' | 'frame' | 'text' | 'pen' | 'eyedropper' | Shape
export type Pen = { id: string; anchors: Anchor[] }

export const MM = 72 / 25.4
const FRAMED = ['create', 'placeImage', 'setFrame']
const GRID: Record<string, number> = { x: 10, y: 10, w: 100, h: 100 }

/**
 * Puts x and y on a 0.1 mm grid and w and h on 0.01 mm unless they stay as in `was`, and the
 * other numbers of `cmd` on 2 decimals, except the factor `by`.
 */
function rounded(cmd: Command, was?: Node): Command {
  const cents = (v: number) => Math.round(v * 100) / 100
  const out: Record<string, unknown> = { ...cmd }
  for (const [k, v] of Object.entries(cmd)) {
    const per = FRAMED.includes(cmd.type) ? GRID[k] : undefined
    if (per && was?.[k as keyof Node] === v) continue
    if (typeof v === 'number' && k !== 'by') out[k] = per ? (Math.round((v / MM) * per) / per) * MM : cents(v)
    else if (Array.isArray(v) && v.every((n) => typeof n === 'number')) out[k] = v.map(cents)
  }
  return out as Command
}

const UNTITLED: { name: string; handle: Handle | null } = { name: 'Untitled.satz', handle: null }

export const scopeOf = ({ swatches, collections, variables }: Palette, modes: Modes = {}): Scope => ({
  swatches,
  collections,
  variables,
  modes,
})

export class Editor {
  snapshot: Snapshot
  nodes: Map<string, Entry>
  selection: string[] = []
  tool: Tool = 'move'
  renaming: string | null = null
  /** The layer under the pointer in the layers panel, highlighted on the canvas. */
  hover: string | null = null
  /** The guide picked on the canvas: the page, its axis and index. */
  guide: { id: string; axis: 'x' | 'y'; index: number } | null = null
  /** The path being drawn with the pen tool, inside an open undo group. */
  pen: Pen | null = null
  /** The pointer is down on the canvas, possibly inside a drag's undo group. */
  dragging = false
  /** Where the pointer is over the canvas, in spread coordinates. */
  pointer: Point | undefined
  /** The text layer edited in its frame, which is then the selection. */
  editing: Editing | null = null
  /** The text frame whose out-port was clicked, to thread into the next one clicked. */
  threading: string | null = null
  /** Images to place, the first at the next click on the canvas. */
  placing: { hash: string; name: string; w: number; h: number }[] = []
  /** The current page or master, shown on the canvas in its spread. */
  pageId: string
  /** The page shown before the current master, to return to. */
  back: string | null = null
  /** The pages selected in the page overview, `null` while it is closed. */
  overview: string[] | null = null
  /** The preflight is open: the pages show as they print, with what `inks` marks over them. */
  preflight = false
  /** The layout grids of the pages show and layers snap to them. */
  grids = true
  /** The path being edited on the canvas, what a click on it does and its picked knot. */
  vector: { id: string; mode: 'move' | 'add' | 'delete' | 'fill'; at: At | null } | null = null
  /** The image whose handles crop it and which a drag moves inside its frame. */
  cropping: string | null = null
  /** The plates shown (bit 0 for C to 3 for K, then the spots), and whether ink above the limit and colours out of gamut are marked. */
  inks = { on: 2 ** 31 - 1, over: true, gamut: true }
  /** The inks of the shown pages and of the document, from the worker. */
  previewed: Previewed | null = null
  /** The ink coverage in % under the pointer while the preflight is open. */
  pointerInk: number | null = null
  file = UNTITLED
  /** The document has changed since it was last saved to or opened from its file. */
  dirty = false
  /** The message in the status bar. */
  status = ''
  /** The engine trapped; nothing is saved any more. */
  broken = false
  /** Typing into the edited text is one undo step until the caret moves. */
  private typing = false
  /** The plain text the last copy of layers put on the clipboard; other text there came from outside. */
  copied: string | null = null
  /** The last copies `duplicate` made and where their originals were. */
  private copies: { ids: string[]; from: { x: number; y: number } } | null = null
  /** How many undo groups are open; the engine holds one for all of them. */
  groups = 0
  private savedAt: string
  private listeners = new Set<() => void>()

  constructor(readonly engine: Engine) {
    this.snapshot = JSON.parse(engine.snapshot())
    this.pageId = this.snapshot.pages[0].id
    this.nodes = this.index()
    this.savedAt = engine.version()
  }

  /** Replaces the document by the one saved in `bytes`; on an error it stays as it was. */
  load(bytes: Uint8Array, file = UNTITLED, dirty = false) {
    this.engine.load(bytes)
    Object.assign(this, { selection: [], tool: 'move', renaming: null, hover: null, guide: null, pen: null, editing: null, threading: null, placing: [], overview: null, cropping: null })
    this.typing = false
    this.groups = 0
    this.snapshot = JSON.parse(this.engine.snapshot())
    this.pageId = this.snapshot.pages[0].id
    this.nodes = this.index()
    this.saved(file, dirty ? '' : this.engine.version())
  }

  /** The document at `version` is in `file`. */
  saved(file: Editor['file'], version: string) {
    this.file = file
    this.savedAt = version
    this.dirty = this.engine.version() !== version
    this.emit()
  }

  /** The current page or master. */
  get page() {
    const { pages, masters } = this.snapshot
    return pages.find((p) => p.id === this.pageId) ?? masters.find((p) => p.id === this.pageId) ?? pages[0]
  }

  /** The pages of the current page's spread from left to right, or the current master. */
  get spread(): Page[] {
    const { pages, spreads } = this.snapshot
    const ids = spreads.find((s) => s.includes(this.pageId))
    return ids ? ids.map((id) => pages.find((p) => p.id === id)!) : [this.page]
  }

  /**
   * The pages the canvas shows: those of the spread, or with facing pages the left and
   * right page of the current master, which holds the layers of both.
   */
  get sheets(): Page[] {
    const m = this.master
    return m && this.snapshot.facingPages ? [{ ...m, x: -m.width }, m] : this.spread
  }

  /** How far the page of the layer `id` sits right of the spine on its spread. */
  dx(id: string) {
    return this.nodes.get(id)?.page?.x ?? 0
  }

  /** The box of `n` as it shows on the spread, turned by its rotation and its ancestors'. */
  shown(n: Node): Box {
    const [a, b, c, d, e, f] = this.engine.turn(n.id)
    const [cx, cy] = [n.x + n.w / 2, n.y + n.h / 2]
    const [x, y] = [a * cx + c * cy + e + this.dx(n.id), b * cx + d * cy + f]
    return { x: x - n.w / 2, y: y - n.h / 2, w: n.w, h: n.h, rotation: (Math.atan2(-b, a) * 180) / Math.PI }
  }

  /** `p` on the spread in the unturned space of the layer `id` on its page. */
  local(id: string, p: Point): Point {
    const [a, b, c, d, e, f] = this.engine.turn(id)
    const [x, y] = [p.x - this.dx(id) - e, p.y - f]
    return { x: a * x + b * y, y: c * x + d * y }
  }

  /**
   * Turns `nodes` as they were by `degrees` counterclockwise around `c` on the spread:
   * a group through its layers, a line by its ends.
   */
  turn(nodes: Node[], degrees: number, c: Point) {
    for (const n of nodes) {
      if (n.kind === 'group') {
        this.turn(n.children, degrees, c)
        continue
      }
      const at = { x: c.x - this.dx(n.id), y: c.y }
      const line = ends(n)
      if (line) {
        const [a, b] = line.map((p) => spin(p, degrees, at))
        this.apply({ type: 'setPath', id: n.id, path: [0, a.x, a.y, 1, b.x, b.y] })
        continue
      }
      const m = spin({ x: n.x + n.w / 2, y: n.y + n.h / 2 }, degrees, at)
      const rotation = (((n.rotation + degrees) % 360) + 360) % 360
      this.apply({ type: 'set', id: n.id, rotation })
      this.apply({ type: 'setFrame', id: n.id, x: m.x - n.w / 2, y: m.y - n.h / 2, w: n.w, h: n.h })
    }
  }

  /** The layers of the current spread. */
  private index() {
    const out = new Map<string, Entry>()
    for (const p of this.spread) index(p.children, undefined, out, p)
    return out
  }

  /** Makes the page of the selection current when it is all on another page of the spread. */
  private settle() {
    const pages = new Set(this.selection.map((id) => this.nodes.get(id)?.page?.id))
    const [only] = pages
    if (pages.size === 1 && only && only !== this.pageId) this.pageId = only
  }

  /** The master a page draws under its layers. */
  masterOf(page: Page) {
    return this.snapshot.masters.find((m) => m.id === page.master)
  }

  /** Applies `cmd`; a command the engine rejects shows in the status bar and returns nothing, a trap throws. */
  apply(cmd: Command): string[] {
    if (cmd.type === 'undo' || cmd.type === 'redo') {
      this.typing = false
      this.groups = 0
    }
    try {
      return this.change(() => this.engine.apply(rounded(cmd, 'id' in cmd ? this.nodes.get(cmd.id)?.node : undefined)))
    } catch (e) {
      if (e instanceof WebAssembly.RuntimeError) throw e
      this.say((e as Error).message)
      return []
    }
  }

  say = (status: string) => {
    this.status = status
    this.emit()
  }

  /** Opens an undo group that lasts until `endGroup`; groups opened inside it join it. */
  beginGroup() {
    if (this.groups++ === 0) this.apply({ type: 'beginUndoGroup' })
  }

  endGroup() {
    if (this.groups > 0 && --this.groups === 0) this.apply({ type: 'endUndoGroup' })
  }

  /** Runs `f` as one undo step, or as part of the group already open. */
  batch<T>(f: () => T): T {
    this.beginGroup()
    try {
      return f()
    } finally {
      this.endGroup()
    }
  }

  /** Adds a TrueType or OpenType font, which sets the text in it again. */
  addFont(bytes: Uint8Array): Typeface {
    return this.change(() => this.engine.addFont(bytes))
  }

  /** Makes the ICC profile `icc` the CMYK profile of the document, or FOGRA51 for `null`. */
  setProfile(icc: Uint8Array | null) {
    this.change(() => this.engine.setProfile(icc ?? undefined))
  }

  /** Removes the added font `hash`; text set in it shows as missing. */
  removeFont(hash: string) {
    this.change(() => this.engine.removeFont(hash))
  }

  /** Adds a PNG or JPEG file to the images to place, at 300 ppi or smaller to fit the current page. */
  loadImage(bytes: Uint8Array, name: string) {
    const { hash, width, height } = this.change(() => this.engine.addImage(bytes) as { hash: string; width: number; height: number })
    const [w, h] = [width, height].map((px) => (px * 72) / 300)
    const fit = Math.min(1, this.page.width / w, this.page.height / h)
    this.setTool('move')
    this.set({ placing: [...this.placing, { hash, name, w: w * fit, h: h * fit }] })
  }

  private change<T>(f: () => T): T {
    const at = this.snapshot.pages.findIndex((p) => p.id === this.pageId)
    try {
      return f()
    } finally {
      this.snapshot = JSON.parse(this.engine.snapshot())
      const { pages, masters } = this.snapshot
      if (!pages.some((p) => p.id === this.pageId) && !masters.some((p) => p.id === this.pageId)) {
        this.pageId = pages[Math.max(0, Math.min(at, pages.length - 1))].id
      }
      this.nodes = this.index()
      this.selection = this.selection.filter((id) => this.nodes.has(id))
      const v = this.vector && this.nodes.get(this.vector.id)?.node
      if (v && !(v.kind === 'shape' && v.shape === 'path')) this.vector = null
      const n = this.editing && this.nodes.get(this.editing.id)?.node
      if (this.editing) {
        const len = n?.kind === 'text' ? this.storyOf(n).text.length : -1
        const { anchor, focus } = this.editing
        this.editing = len < 0 ? null : { ...this.editing, anchor: Math.min(anchor, len), focus: Math.min(focus, len) }
      }
      this.dirty = this.engine.version() !== this.savedAt
      this.follow()
      this.settle()
      this.emit()
    }
  }

  set(patch: Partial<Pick<Editor, 'selection' | 'tool' | 'renaming' | 'hover' | 'pen' | 'editing' | 'threading' | 'placing' | 'overview' | 'preflight' | 'grids' | 'guide' | 'vector' | 'cropping' | 'inks' | 'previewed' | 'pointerInk'>>) {
    const leaves = this.editing && patch.selection && !patch.selection.includes(this.editing.id)
    if (this.vector && ((patch.selection && !patch.selection.includes(this.vector.id)) || (patch.tool && patch.tool !== 'move'))) patch = { vector: null, ...patch }
    if (this.cropping && ((patch.selection && !patch.selection.includes(this.cropping)) || (patch.tool && patch.tool !== 'move'))) patch = { cropping: null, ...patch }
    if (leaves && !('editing' in patch)) this.stopEditing()
    if (patch.editing) patch = { selection: [patch.editing.id], ...patch }
    if (patch.selection?.length) patch = { guide: null, ...patch }
    Object.assign(this, patch)
    this.follow()
    this.settle()
    this.emit()
  }

  storyOf(n: TextNode) {
    return this.snapshot.stories[n.story]
  }

  /** Resizes the text layers of `nodes` by `mode` where their thread allows; auto fit makes a text of one line auto width and else auto height. */
  resize(nodes: Node[], mode: Resizing | 'autoFit') {
    this.batch(() => {
      for (const n of nodes) {
        if (n.kind !== 'text') continue
        const one = !n.prev && !n.next && this.engine.textLine(n.id, 0)[1] >= this.storyOf(n).text.length
        const m = mode === 'autoFit' ? (one ? 'autoWidth' : 'autoHeight') : mode
        if ((m === 'autoWidth' && (n.prev || n.next)) || (m === 'autoHeight' && n.next)) continue
        this.apply({ type: 'set', id: n.id, sizing: textSizing(n.sizing, m) })
      }
    })
  }

  /** Copies the layers `ids` and returns them as plain text: the text of text layers, the names of the others. */
  copy(ids: string[]) {
    this.apply({ type: 'copy', ids })
    const nodes = ids.map((id) => this.nodes.get(id)!.node)
    this.copied = [...new Set(nodes.map((n) => (n.kind === 'text' ? this.storyOf(n).text : n.name)))].join('\n')
    return this.copied
  }

  /**
   * Duplicates `ids` and selects the copies. Duplicating the last copies again after
   * they were moved moves the new copies as far on, unless `step` is false.
   */
  duplicate(ids: string[], step = true) {
    const at = () => this.nodes.get(ids[0])!.node
    const last = this.copies?.ids.join() === ids.join() && step ? this.copies.from : null
    const d = last ? { x: at().x - last.x, y: at().y - last.y } : { x: 0, y: 0 }
    const from = { x: at().x, y: at().y }
    const copies = this.batch(() => {
      const copies = this.apply({ type: 'duplicate', ids })
      for (const id of d.x || d.y ? copies : []) {
        const n = this.nodes.get(id)!.node
        this.apply({ type: 'setFrame', id, x: n.x + d.x, y: n.y + d.y, w: n.w, h: n.h })
      }
      return copies
    })
    this.copies = { ids: copies, from }
    this.set({ selection: copies })
  }

  /** Pastes the copied layers above the selection, centred under the pointer when it is away from where they were. */
  paste() {
    const ids = this.batch(() => {
      const ids = this.apply({ type: 'paste', above: this.selection, page: this.page.id })
      const nodes = ids.map((id) => this.nodes.get(id)!.node)
      const b = bounds(nodes.map((n) => ({ ...n, x: n.x + this.dx(n.id) })))
      const p = this.pointer
      if (!nodes.length || !p || (p.x >= b.x && p.x <= b.x + b.w && p.y >= b.y && p.y <= b.y + b.h)) return ids
      const [dx, dy] = [p.x - b.x - b.w / 2, p.y - b.y - b.h / 2]
      for (const n of nodes) this.apply({ type: 'setFrame', id: n.id, x: n.x + dx, y: n.y + dy, w: n.w, h: n.h })
      return ids
    })
    this.set({ selection: ids })
  }

  /** Replaces the text of the one selected text layer with `text` from outside, or places it as a new text layer amid the current page, at most 80 % of its width wide. */
  pasteText(text: string) {
    const one = this.selection.length === 1 && this.nodes.get(this.selection[0])?.node
    if (one && one.kind === 'text') return void this.apply({ type: 'setText', id: one.id, text })
    const page = this.page
    const id = this.batch(() => {
      const [id] = this.apply({ type: 'create', parent: page.id, kind: 'text', x: 0, y: 0, w: 0, h: 0 })
      this.apply({ type: 'setText', id, text })
      const n = () => this.nodes.get(id)!.node
      if (n().w > page.width * 0.8) this.apply({ type: 'setFrame', id, x: 0, y: 0, w: page.width * 0.8, h: n().h })
      this.apply({ type: 'setFrame', id, x: (page.width - n().w) / 2, y: (page.height - n().h) / 2, w: n().w, h: n().h })
      return id
    })
    this.set({ selection: [id] })
  }

  /** The layer `id` on any page or master, and the page it is on. */
  lookup(id: string) {
    for (const page of [...this.snapshot.pages, ...this.snapshot.masters]) {
      const entry = index(page.children, undefined, undefined, page).get(id)
      if (entry) return { ...entry, page }
    }
    return undefined
  }

  /** Moves the edit into the frame of the thread that holds the caret, and to its page. */
  private follow() {
    const e = this.editing
    if (!e) return
    let frame: string
    try {
      frame = this.engine.textFrame(e.id, e.focus)
    } catch {
      return
    }
    if (frame === e.id) return
    const found = this.lookup(frame)
    if (!found) return
    this.pageId = found.page.id
    this.nodes = this.index()
    this.editing = { ...e, id: frame }
    this.selection = [frame]
  }

  /** Shows the page `id` with nothing selected. */
  showPage(id: string) {
    if (id === this.pageId) return
    this.finishPen(false)
    this.stopEditing()
    if (this.snapshot.pages.some((p) => p.id === this.pageId)) this.back = this.pageId
    this.pageId = id
    this.nodes = this.index()
    this.set({ selection: [] })
  }

  /** Deletes the pages `del`, keeping at least 1, and selects the page after them in the overview. */
  deletePages(del: string[]) {
    const ids = this.snapshot.pages.map((p) => p.id)
    if (del.length >= ids.length) return this.say('A document keeps at least 1 page')
    const numbers = del.map((id) => ids.indexOf(id) + 1)
    const next = ids.find((id, i) => !del.includes(id) && i >= Math.min(...numbers) - 1) ?? ids.findLast((id) => !del.includes(id))!
    this.batch(() => {
      for (const id of del) this.apply({ type: 'deletePage', id })
    })
    if (this.overview) this.set({ overview: [next] })
    this.say(`Deleted ${del.length === 1 ? `page ${numbers[0]}` : `${del.length} pages`}. Ctrl Z restores`)
  }

  /** The current master, if a master is shown. */
  get master() {
    return this.snapshot.masters.find((m) => m.id === this.pageId)
  }

  /** Opens the page overview with the current page selected, or closes it. */
  toggleOverview() {
    this.set({ overview: this.overview ? null : this.master ? [] : [this.pageId] })
  }

  togglePreflight() {
    this.set({ preflight: !this.preflight, pointerInk: null })
  }

  /** Leaves the current master for the page shown before it. */
  exitMaster() {
    const { pages } = this.snapshot
    this.showPage(pages.find((p) => p.id === this.back)?.id ?? pages[0].id)
  }

  /** Edits the points of the shape `id`, which becomes a path. */
  editPath(id: string) {
    const n = this.nodes.get(id)?.node
    if (n?.kind !== 'shape') return
    if (n.shape !== 'path' || n.rotation) this.apply({ type: 'flatten', id })
    this.set({ selection: [id], vector: { id, mode: 'move', at: null } })
  }

  /** The knots of the path being edited, in the space of its page. */
  knots(): Contour[] {
    const n = this.vector && this.nodes.get(this.vector.id)?.node
    if (n?.kind !== 'shape' || n.shape !== 'path') return []
    const [x, y] = [(u: number) => n.x + u * n.w, (v: number) => n.y + v * n.h]
    return contours(n.path).map(({ knots, closed }) => ({
      closed,
      knots: knots.map((k) => ({ x: x(k.x), y: y(k.y), ix: x(k.ix), iy: y(k.iy), ox: x(k.ox), oy: y(k.oy) })),
    }))
  }

  /** Gives the path being edited the knots `cs` and picks `at`; without knots the path goes. */
  setKnots(cs: Contour[], at: At | null = null) {
    const v = this.vector!
    if (!cs.length) this.apply({ type: 'delete', ids: [v.id] })
    else {
      this.apply({ type: 'setPath', id: v.id, path: toPath(cs) })
      this.set({ vector: { ...v, at } })
    }
  }

  setTool(tool: Tool) {
    this.finishPen(false)
    this.stopEditing()
    this.set({ tool, threading: null, placing: [] })
  }

  beginTyping() {
    if (this.typing) return
    this.typing = true
    this.beginGroup()
  }

  endTyping() {
    if (!this.typing) return
    this.typing = false
    this.endGroup()
  }

  /** Leaves the edited text selected, or removes it when it is empty, as Figma does. */
  stopEditing() {
    const e = this.editing
    if (!e) return
    this.endTyping()
    this.editing = null
    const n = this.nodes.get(e.id)?.node
    if (n?.kind === 'text' && !this.storyOf(n).text && !n.prev && !n.next) this.apply({ type: 'delete', ids: [e.id] })
    this.emit()
  }

  /** Ends the pen path; a path with fewer than two anchors is removed. */
  finishPen(closed: boolean) {
    const pen = this.pen
    if (!pen) return
    this.pen = null
    if (pen.anchors.length < 2) this.apply({ type: 'delete', ids: [pen.id] })
    else this.apply({ type: 'setPath', id: pen.id, path: penPath(pen.anchors, closed) })
    this.endGroup()
    this.set({ tool: 'move', selection: pen.anchors.length < 2 ? [] : [pen.id] })
  }

  /** Makes the pointer gesture that starts now one undo step. */
  gesture = () => {
    this.finishPen(false)
    this.endTyping()
    this.beginGroup()
    window.addEventListener('pointerup', () => this.endGroup(), { once: true })
  }

  selected(): Node[] {
    return this.selection.flatMap((id) => this.nodes.get(id)?.node ?? [])
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit() {
    for (const l of this.listeners) l()
  }
}

export function useEditor<T>(editor: Editor, read: (e: Editor) => T): T {
  return useSyncExternalStore(editor.subscribe, () => read(editor))
}

export type Point = { x: number; y: number }

/** Start and end in pt of a line, i.e. a two-point path. */
export function ends(n: Node): [Point, Point] | undefined {
  if (n.kind !== 'shape' || n.shape !== 'path' || n.path.length !== 6) return undefined
  const [, u0, v0, , u1, v1] = n.path
  return [
    { x: n.x + u0 * n.w, y: n.y + v0 * n.h },
    { x: n.x + u1 * n.w, y: n.y + v1 * n.h },
  ]
}

export function bounds(nodes: { x: number; y: number; w: number; h: number }[]) {
  const x = Math.min(...nodes.map((n) => n.x))
  const y = Math.min(...nodes.map((n) => n.y))
  const w = Math.max(...nodes.map((n) => n.x + n.w)) - x
  const h = Math.max(...nodes.map((n) => n.y + n.h)) - y
  return { x, y, w, h }
}

/**
 * Where layers `ids` dragged to `p` land among the other children of the auto layout
 * frame `parent`: the index for a move and the insertion line.
 */
export function insertion(parent: Container, ids: string[], p: Point) {
  const horizontal = parent.kind === 'frame' && parent.direction === 'horizontal'
  const others = parent.children.filter((n) => !ids.includes(n.id))
  const flow = others.filter((n) => !n.absolute)
  const main = (n: Node) => (horizontal ? n.x + n.w / 2 : n.y + n.h / 2)
  const next = flow.find((n) => main(n) > (horizontal ? p.x : p.y))
  const index = next ? others.indexOf(next) : others.length
  const prev = flow[(next ? flow.indexOf(next) : flow.length) - 1]
  const at = next && prev ? (horizontal ? (prev.x + prev.w + next.x) / 2 : (prev.y + prev.h + next.y) / 2)
    : next ? (horizontal ? next.x : next.y)
    : prev ? (horizontal ? prev.x + prev.w : prev.y + prev.h)
    : horizontal ? parent.x : parent.y
  const line: [Point, Point] = horizontal
    ? [{ x: at, y: parent.y }, { x: at, y: parent.y + parent.h }]
    : [{ x: parent.x, y: at }, { x: parent.x + parent.w, y: at }]
  return { index, line }
}
