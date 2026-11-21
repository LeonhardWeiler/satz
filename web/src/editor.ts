import { useSyncExternalStore, type ReactNode } from 'react'
import type { Engine } from './engine/engine'
import { textSizing, type Command, type Container, type Modes, type Node, type Page, type Palette, type Resizing, type Scope, type Sheet, type Snapshot, type TextNode, type Typeface, type Version } from './model'
import type { Previewed } from './exportWorker'
import type { Handle } from './file'
import { penPath, type Anchor } from './pen'
import { spin } from './handles'
import type { Box } from './renderer'
import { index, type Entry } from './select'
import { contours, toPath, type At, type Contour } from './vector'
import type { Editing } from './textEdit'

export type Shape = 'rect' | 'line' | 'ellipse' | 'polygon' | 'star'
export type Tool = 'move' | 'frame' | 'text' | 'pen' | 'eyedropper' | 'table' | Shape
export type Pen = { id: string; anchors: Anchor[] }

export { MM } from './types'
import { MM } from './types'
export const CLIP = 'application/x-satz-layers'
const FRAMED = ['create', 'placeImage', 'setFrame']
const GRID: Record<string, number> = { x: 10, y: 10, w: 100, h: 100 }
const FRACTIONS = ['opacity', 'sweep', 'inner', 'ratio']

export const timed = 'satzPerf' in globalThis ? (name: string, start: number) => void performance.measure(name, { start }) : () => {}

function rounded(cmd: Command, was?: Node): Command {
  const cents = (v: number) => Math.round(v * 100) / 100
  const out: Record<string, unknown> = { ...cmd }
  for (const [k, v] of Object.entries(cmd)) {
    const per = FRAMED.includes(cmd.type) ? GRID[k] : undefined
    if (per && was?.[k as keyof Node] === v) continue
    if (typeof v === 'number' && k !== 'by') out[k] = per ? (Math.round((v / MM) * per) / per) * MM : FRACTIONS.includes(k) ? Math.round(v * 1e4) / 1e4 : cents(v)
    else if (Array.isArray(v) && v.every((n) => typeof n === 'number')) out[k] = v.map(cents)
  }
  return out as Command
}

export const UNTITLED: { name: string; handle: Handle | null } = { name: 'Untitled.satz', handle: null }

export const scopeOf = ({ swatches, collections, variables }: Palette, modes: Modes = {}): Scope => ({
  swatches,
  collections,
  variables,
  modes,
})

const verbs: Record<Command['type'], string> = {
  create: 'create the layer',
  placeImage: 'place the image',
  setFrame: 'move or resize the layer',
  setText: 'change the text',
  editText: 'edit the text',
  format: 'format the text',
  scaleText: 'scale the text',
  addTextStyle: 'add the text style',
  setTextStyle: 'change the text style',
  deleteTextStyle: 'delete the text style',
  toc: 'build the table of contents',
  anchor: 'anchor the layer',
  unanchor: 'release the anchored layer',
  tableInsert: 'insert the row or column',
  tableDelete: 'delete the row or column',
  mergeCells: 'merge the cells',
  splitCell: 'split the cell',
  sizeTable: 'size the table',
  set: 'change the layer',
  setPath: 'change the path',
  fitImage: 'fit the image',
  flip: 'flip the layer',
  flatten: 'flatten the layer',
  boolean: 'combine the shapes',
  fillArea: 'fill the area',
  delete: 'delete the layers',
  group: 'group the layers',
  ungroup: 'ungroup the layers',
  mask: 'mask the layers',
  autoLayout: 'add auto layout',
  move: 'move the layers',
  order: 'reorder the layers',
  duplicate: 'duplicate the layers',
  copy: 'copy the layers',
  setDocument: 'change the document',
  relink: 'relink the image',
  embed: 'embed the images',
  paste: 'paste the copied layers',
  addPage: 'add a page',
  setSection: 'change the section',
  duplicatePage: 'duplicate the page',
  setPage: 'change the page',
  setGrids: 'change the grids',
  setGuides: 'change the guides',
  deletePage: 'delete the page',
  movePages: 'move the pages',
  addMaster: 'add a master',
  setMaster: 'rename the master',
  deleteMaster: 'delete the master',
  useMaster: 'apply the master',
  override: 'override the master layer',
  resetToMaster: 'reset to the master',
  thread: 'thread the text frames',
  threadBefore: 'thread the text frames',
  unthread: 'unthread the text frame',
  addSwatch: 'add the swatch',
  setSwatch: 'change the swatch',
  deleteSwatch: 'delete the swatch',
  addCollection: 'add the collection',
  setCollection: 'rename the collection',
  deleteCollection: 'delete the collection',
  addMode: 'add the mode',
  setMode: 'rename the mode',
  deleteMode: 'delete the mode',
  addVariable: 'add the variable',
  setVariable: 'change the variable',
  deleteVariable: 'delete the variable',
  bind: 'bind the variable',
  useMode: 'switch the mode',
  restore: 'restore the version',
  undo: 'undo',
  redo: 'redo',
  beginUndoGroup: 'start an undo step',
  endUndoGroup: 'end an undo step',
}

export class Editor {
  readonly snapshot: Snapshot

  readonly nodes: Map<string, Entry>
  readonly selection: string[] = []
  readonly tool: Tool = 'move'
  readonly renaming: string | null = null
  readonly hover: string | null = null
  readonly guide: { id: string; axis: 'x' | 'y'; index: number } | null = null
  readonly pen: Pen | null = null
  readonly dragging: boolean = false
  readonly pointer: Point | undefined
  readonly editing: Editing | null = null
  readonly found: { story: string; at: number; end: number }[] = []
  readonly threading: { id: string; before: boolean } | null = null
  readonly placing: { hash: string; name: string; w: number; h: number; url: string }[] = []
  readonly pageId: string
  readonly back: string | null = null
  readonly overview: string[] | null = null
  readonly preflight: boolean = false
  readonly grids: boolean = true
  readonly vector: { id: string; mode: 'move' | 'add' | 'delete' | 'fill'; at: At[] } | null = null
  readonly cropping: string | null = null
  readonly inks = { on: 2 ** 31 - 1, over: true, gamut: true }
  readonly previewed: Previewed | null = null
  readonly version: Version | null = null
  readonly pointerInk: number | null = null
  readonly file = UNTITLED
  readonly project: string | null = null
  readonly home: boolean = false
  readonly dirty: boolean = false
  readonly status: ReactNode = ''
  readonly statusError: boolean = false
  readonly statusAction: { label: string; run: () => void } | null = null
  readonly autosaves: boolean = true
  readonly autosaveFailed: boolean = false
  private typing = false
  private quiet: ReturnType<typeof setTimeout> | undefined
  readonly copied: string | null = null
  readonly clip: string | null = null
  private copies: { ids: string[]; from: { x: number; y: number } } | null = null
  readonly groups: number = 0
  private savedAt: string
  private listeners = new Set<() => void>()
  private pointed = new Set<() => void>()

  private get w() {
    return this as { -readonly [K in keyof Editor]: Editor[K] }
  }

  constructor(readonly engine: Engine) {
    this.snapshot = this.read()
    this.pageId = this.snapshot.pages[0].id
    this.nodes = this.index()
    this.savedAt = engine.version()
  }

  load(bytes: Uint8Array, file = UNTITLED, dirty = false, updates: Uint8Array[] = []) {
    this.engine.load(bytes, updates)
    Object.assign(this, { selection: [], tool: 'move', renaming: null, hover: null, guide: null, pen: null, editing: null, threading: null, overview: null, cropping: null, vector: null, copied: null, clip: null, copies: null })
    this.set({ placing: [] })
    this.typing = false
    this.w.groups = 0
    this.w.snapshot = this.read()
    this.w.pageId = this.snapshot.pages[0].id
    this.w.nodes = this.index()
    this.saved(file, dirty ? '' : this.engine.version())
  }

  saved(file: Editor['file'], version: string) {
    this.w.file = file
    this.savedAt = version
    this.w.dirty = this.engine.version() !== version
    this.emit()
  }

  get page() {
    const { pages, masters } = this.snapshot
    return pages.find((p) => p.id === this.pageId) ?? masters.find((p) => p.id === this.pageId) ?? pages[0]
  }

  get spread(): Page[] {
    const { pages, spreads } = this.snapshot
    const ids = spreads.find((s) => s.includes(this.pageId))
    return ids ? ids.map((id) => pages.find((p) => p.id === id)!) : [this.page]
  }

  get sheets(): Page[] {
    const m = this.master
    return m && this.snapshot.facingPages ? [{ ...m, x: -m.width }, m] : this.spread
  }

  dx(id: string) {
    return this.nodes.get(id)?.page?.x ?? 0
  }

  shown(n: Node): Box {
    const [a, b, c, d, e, f] = this.engine.turn(n.id)
    const [cx, cy] = [n.x + n.w / 2, n.y + n.h / 2]
    const [x, y] = [a * cx + c * cy + e + this.dx(n.id), b * cx + d * cy + f]
    return { x: x - n.w / 2, y: y - n.h / 2, w: n.w, h: n.h, rotation: (Math.atan2(-b, a) * 180) / Math.PI }
  }

  local(id: string, p: Point): Point {
    const [a, b, c, d, e, f] = this.engine.turn(id)
    const [x, y] = [p.x - this.dx(id) - e, p.y - f]
    return { x: a * x + b * y, y: c * x + d * y }
  }

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

  private index() {
    const out = new Map<string, Entry>()
    for (const p of this.spread) index(p.children, undefined, out, p)
    return out
  }

  private settle() {
    const pages = new Set(this.selection.map((id) => this.nodes.get(id)?.page?.id))
    const [only] = pages
    if (pages.size === 1 && only && only !== this.pageId) this.w.pageId = only
  }

  masterOf(page: Page) {
    return this.snapshot.masters.find((m) => m.id === page.master)
  }

  apply(cmd: Command): string[] {
    if (cmd.type === 'undo' || cmd.type === 'redo') {
      this.typing = false
      this.w.groups = 0
    }
    if (this.status) this.say('')
    try {
      const out = this.change(() => this.engine.apply(rounded(cmd, 'id' in cmd ? this.nodes.get(cmd.id)?.node : undefined)))
      const notes = this.engine.notes()
      if (notes.length) this.say(notes.join('. '))
      return out
    } catch (e) {
      this.fail(`Could not ${verbs[cmd.type]}: ${(e as Error).message}`)
      return []
    }
  }

  say = (status: ReactNode, action?: Editor['statusAction'], error = false) => {
    this.w.status = status
    this.w.statusError = error
    this.w.statusAction = action ?? null
    clearTimeout(this.quiet)
    if (status) this.quiet = setTimeout(() => this.say(''), 5000)
    this.emit()
  }

  fail = (status: ReactNode) => this.say(status, null, true)

  beginGroup() {
    if (this.w.groups++ === 0) this.engine.apply({ type: 'beginUndoGroup' })
  }

  endGroup() {
    if (this.groups > 0 && --this.w.groups === 0) this.engine.apply({ type: 'endUndoGroup' })
  }

  batch<T>(f: () => T): T {
    this.beginGroup()
    try {
      return f()
    } finally {
      this.endGroup()
    }
  }

  addFont(bytes: Uint8Array): Typeface {
    return this.change(() => this.engine.addFont(bytes))
  }

  setProfile(icc: Uint8Array | null) {
    this.change(() => this.engine.setProfile(icc ?? undefined))
  }

  removeFont(hash: string) {
    this.change(() => this.engine.removeFont(hash))
  }

  loadImage(bytes: Uint8Array<ArrayBuffer>, name: string, link = false) {
    const { hash, width, height } = this.change(() => this.engine.addImage(bytes, link ? name : undefined))
    const [w, h] = [width, height].map((px) => (px * 72) / 300)
    const fit = Math.min(1, this.page.width / w, this.page.height / h)
    this.setTool('move')
    this.set({ placing: [...this.placing, { hash, name, w: w * fit, h: h * fit, url: URL.createObjectURL(new Blob([bytes])) }] })
    return hash
  }

  preview(v: Version | null) {
    this.w.version = v
    this.change(() => this.engine.preview(v?.peer, v?.counter ?? 0))
    if (v?.page) this.showPage(v.page)
  }

  registerImage(bytes: Uint8Array) {
    return this.change(() => this.engine.registerImage(bytes)).hash
  }

  private read(): Snapshot {
    const s = this.engine.changes()
    const old = this.snapshot as Snapshot | undefined
    const sheets = new Map([...(old?.pages ?? []), ...(old?.masters ?? [])].map((p) => [p.id, p]))
    const keep = (p: Sheet | string): Page => {
      if (typeof p === 'string') return sheets.get(p)!
      const layers = new Map(sheets.get(p.id)?.children.map((n) => [n.id, n]))
      return { ...p, children: p.children.map((n) => (typeof n === 'string' ? layers.get(n)! : n)) }
    }
    const stories = new Map(Object.entries(old?.stories ?? {}))
    for (const [k, v] of Object.entries(s.stories)) {
      const o = stories.get(k)
      if (v === null) stories.delete(k)
      else if (!('at' in v)) stories.set(k, v)
      else {
        stories.set(k, {
          text: o!.text.slice(0, v.at) + v.text + o!.text.slice(v.at + v.cut),
          spans: [...o!.spans.slice(0, v.spansAt), ...v.spans, ...o!.spans.slice(v.spansAt + v.spansCut)],
        })
      }
    }
    return { ...s, pages: s.pages.map(keep), masters: s.masters.map(keep), stories: Object.fromEntries(stories) }
  }

  private change<T>(f: () => T): T {
    const at = this.snapshot.pages.findIndex((p) => p.id === this.pageId)
    let start = performance.now()
    try {
      return f()
    } finally {
      timed('apply', start)
      start = performance.now()
      this.w.snapshot = this.read()
      timed('snapshot', start)
      const { pages, masters } = this.snapshot
      if (!pages.some((p) => p.id === this.pageId) && !masters.some((p) => p.id === this.pageId)) {
        this.w.pageId = pages[Math.max(0, Math.min(at, pages.length - 1))].id
      }
      start = performance.now()
      this.w.nodes = this.index()
      timed('index', start)
      this.w.selection = this.selection.filter((id) => this.nodes.has(id))
      const v = this.vector && this.nodes.get(this.vector.id)?.node
      if (v && v.kind !== 'shape') this.w.vector = null
      const n = this.editing && this.nodes.get(this.editing.id)?.node
      if (this.editing) {
        const len = n?.kind === 'text' ? this.storyOf(n).text.length : -1
        const { anchor, focus } = this.editing
        this.w.editing = len < 0 ? null : { ...this.editing, anchor: Math.min(anchor, len), focus: Math.min(focus, len) }
      }
      this.w.dirty = this.engine.version() !== this.savedAt
      this.follow()
      this.settle()
      this.emit()
    }
  }

  set(patch: Partial<Pick<Editor, 'selection' | 'tool' | 'renaming' | 'pen' | 'editing' | 'threading' | 'placing' | 'overview' | 'preflight' | 'grids' | 'guide' | 'vector' | 'cropping' | 'inks' | 'previewed' | 'found' | 'home' | 'project' | 'autosaves' | 'autosaveFailed' | 'clip'>>) {
    const leaves = this.editing && patch.selection && !patch.selection.includes(this.editing.id)
    if (this.vector && ((patch.selection && !patch.selection.includes(this.vector.id)) || (patch.tool && patch.tool !== 'move'))) patch = { vector: null, ...patch }
    if (this.cropping && ((patch.selection && !patch.selection.includes(this.cropping)) || (patch.tool && patch.tool !== 'move'))) patch = { cropping: null, ...patch }
    if (leaves && !('editing' in patch)) this.stopEditing()
    if (patch.editing) patch = { selection: [patch.editing.id], ...patch }
    if (patch.selection?.length) patch = { guide: null, ...patch }
    for (const p of this.placing) if (patch.placing && !patch.placing.includes(p)) URL.revokeObjectURL(p.url)
    Object.assign(this, patch)
    this.follow()
    this.settle()
    this.emit()
  }

  storyOf(n: TextNode) {
    return this.snapshot.stories[n.story]
  }

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

  copy(ids: string[]) {
    this.apply({ type: 'copy', ids })
    const nodes = ids.map((id) => this.nodes.get(id)!.node)
    const copied = [...new Set(nodes.map((n) => (n.kind === 'text' ? this.storyOf(n).text : n.name)))].join('\n')
    this.w.copied = copied
    this.w.clip = btoa(Array.from(this.engine.clip(), (b) => String.fromCharCode(b)).join(''))
    return copied
  }

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

  paste(clip = this.clip) {
    if (clip && clip !== this.clip) {
      try {
        this.engine.takeClip(Uint8Array.from(atob(clip), (c) => c.charCodeAt(0)))
      } catch (e) {
        return this.fail(`Could not paste the copied layers: ${(e as Error).message}.`)
      }
      this.w.clip = clip
    }
    const editing = this.editing
    const ids = this.batch(() => {
      const ids = this.apply({ type: 'paste', above: this.selection, page: this.page.id })
      if (editing) {
        const [a, b] = [Math.min(editing.anchor, editing.focus), Math.max(editing.anchor, editing.focus)]
        if (a < b) this.apply({ type: 'editText', id: editing.id, range: [a, b], text: '' })
        for (const id of [...ids].reverse()) this.apply({ type: 'anchor', id, text: editing.id, at: a })
        this.set({ editing: { id: editing.id, anchor: a + ids.length, focus: a + ids.length } })
        return ids
      }
      const nodes = ids.map((id) => this.nodes.get(id)!.node)
      const b = bounds(nodes.map((n) => ({ ...n, x: n.x + this.dx(n.id) })))
      const p = this.pointer
      if (!nodes.length || !p || (p.x >= b.x && p.x <= b.x + b.w && p.y >= b.y && p.y <= b.y + b.h)) return ids
      const [dx, dy] = [p.x - b.x - b.w / 2, p.y - b.y - b.h / 2]
      for (const n of nodes) this.apply({ type: 'setFrame', id: n.id, x: n.x + dx, y: n.y + dy, w: n.w, h: n.h })
      return ids
    })
    if (!editing) this.set({ selection: ids })
  }

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

  lookup(id: string) {
    for (const page of [...this.snapshot.pages, ...this.snapshot.masters]) {
      const entry = index(page.children, undefined, undefined, page).get(id)
      if (entry) return { ...entry, page }
    }
    return undefined
  }

  private follow() {
    const e = this.editing
    if (!e) return
    let frame: string
    try {
      frame = this.engine.textFrame(e.id, e.focus)
    } catch {
      return
    }
    if (frame === e.id && this.nodes.has(frame)) return
    const found = this.lookup(frame)
    if (!found) return
    this.w.pageId = found.page.id
    this.w.nodes = this.index()
    this.w.editing = { ...e, id: frame }
    this.w.selection = [frame]
  }

  showPage(id: string) {
    if (id === this.pageId) return
    this.finishPen(false)
    this.stopEditing()
    if (this.snapshot.pages.some((p) => p.id === this.pageId)) this.w.back = this.pageId
    this.w.pageId = id
    this.w.nodes = this.index()
    this.set({ selection: [] })
  }

  deletePages(del: string[]) {
    const ids = this.snapshot.pages.map((p) => p.id)
    del = del.filter((id) => ids.includes(id))
    if (!del.length) return
    if (del.length >= ids.length) return this.fail('A document keeps at least 1 page')
    const numbers = del.map((id) => ids.indexOf(id) + 1)
    const next = ids.find((id, i) => !del.includes(id) && i >= Math.min(...numbers) - 1) ?? ids.findLast((id) => !del.includes(id))!
    this.batch(() => {
      for (const id of del) this.apply({ type: 'deletePage', id })
    })
    if (this.overview) this.set({ overview: [next] })
    this.say(`Deleted ${del.length === 1 ? `page ${numbers[0]}` : `${del.length} pages`}`, { label: 'Undo', run: () => this.apply({ type: 'undo' }) })
  }

  get master() {
    return this.snapshot.masters.find((m) => m.id === this.pageId)
  }

  toggleOverview() {
    this.set({ overview: this.overview ? null : [this.pageId] })
  }

  togglePreflight() {
    this.w.pointerInk = null
    this.set({ preflight: !this.preflight })
  }

  exitMaster() {
    const { pages } = this.snapshot
    this.showPage(pages.find((p) => p.id === this.back)?.id ?? pages[0].id)
  }

  editPath(id: string) {
    if (this.nodes.get(id)?.node.kind !== 'shape') return
    this.set({ selection: [id], vector: { id, mode: 'move', at: [] } })
  }

  knots(): Contour[] {
    const n = this.vector && this.nodes.get(this.vector.id)?.node
    return n?.kind === 'shape' ? contours(this.engine.pathOf(n.id)) : []
  }

  setKnots(cs: Contour[], at: At[] = []) {
    const v = this.vector!
    if (!cs.length) this.apply({ type: 'delete', ids: [v.id] })
    else {
      this.changePath({ type: 'setPath', id: v.id, path: toPath(cs) })
      this.set({ vector: { ...v, at } })
    }
  }

  changePath(cmd: Command) {
    this.batch(() => {
      this.apply({ type: 'flatten', id: this.vector!.id })
      this.apply(cmd)
    })
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

  stopEditing() {
    const e = this.editing
    if (!e) return
    this.endTyping()
    this.w.editing = null
    const n = this.nodes.get(e.id)?.node
    if (n?.kind === 'text' && !this.storyOf(n).text && !n.prev && !n.next && !n.cell) this.apply({ type: 'delete', ids: [e.id] })
    this.emit()
  }

  finishPen(closed: boolean) {
    const pen = this.pen
    if (!pen) return
    this.w.pen = null
    if (pen.anchors.length < 2) this.apply({ type: 'delete', ids: [pen.id] })
    else this.apply({ type: 'setPath', id: pen.id, path: penPath(pen.anchors, closed) })
    this.endGroup()
    this.w.status = ''
    this.set({ tool: 'move', selection: pen.anchors.length < 2 ? [] : [pen.id] })
  }

  gesture = () => {
    this.finishPen(false)
    this.endTyping()
    this.beginGroup()
    window.addEventListener('pointerup', () => this.endGroup(), { once: true })
  }

  selected(): Node[] {
    return this.selection.flatMap((id) => this.nodes.get(id)?.node ?? [])
  }

  point(patch: Partial<Pick<Editor, 'hover' | 'pointerInk' | 'pointer' | 'dragging'>>) {
    Object.assign(this, patch)
    for (const l of this.pointed) l()
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  subscribePointer = (listener: () => void) => {
    this.pointed.add(listener)
    return () => this.pointed.delete(listener)
  }

  private emit() {
    for (const l of this.listeners) l()
  }
}

export function useEditor<T>(editor: Editor, read: (e: Editor) => T): T {
  return useSyncExternalStore(editor.subscribe, () => read(editor))
}

export type Point = { x: number; y: number }

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
