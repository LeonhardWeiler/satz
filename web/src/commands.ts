import { align, ALIGNS, BOOLEANS, combinable, combine } from './align'
import type { Editor } from './editor'
import type { Node, Props } from './model'
import { TABLE_ACTIONS } from './cells'
import { DATE, format, insert, PAGE_NUMBER } from './textEdit'

export type Action = {
  title: string
  keys: string
  group: string
  run: (editor: Editor) => void
  can?: (editor: Editor) => boolean
  typing?: 'any' | 'text'
  native?: true
}

export type Panel = 'left' | 'right' | 'ui' | 'rulers'

const noop = () => {}

export const ui: {
  start(): void
  open(): void
  save(as: boolean): void
  place(): void
  export(): void
  find(): void
  dialog(d: 'palette' | 'help' | 'settings' | 'history'): void
  hide(panel: Panel): void
  focus(back: boolean): void
  fit(): void
  fitSelection(): void
  zoomTo(zoom: number): void
  zoomBy(factor: number): void
} = { start: noop, open: noop, save: noop, place: noop, export: noop, find: noop, dialog: noop, hide: noop, focus: noop, fit: noop, fitSelection: noop, zoomTo: noop, zoomBy: noop }

export function addPage(editor: Editor) {
  const [id] = editor.apply({ type: 'addPage', after: editor.master ? editor.snapshot.pages.at(-1)!.id : editor.pageId })
  if (id) editor.showPage(id)
}

const selected = (editor: Editor) => editor.selection.length > 0
const copy = (editor: Editor) => void navigator.clipboard.writeText(editor.copy(editor.selection)).catch((e: unknown) => editor.fail(`Could not copy: ${(e as Error).message}.`))
const each = (editor: Editor, f: (n: Node) => unknown) => editor.batch(() => editor.selected().forEach(f))
const toggle = (editor: Editor, prop: 'hidden' | 'locked') => {
  const on = editor.selected().some((n) => !n[prop])
  each(editor, (n) => editor.apply({ type: 'set', id: n.id, [prop]: on }))
}
export const siblings = (editor: Editor) => (editor.nodes.get(editor.selection[0] ?? '')?.parent?.children ?? editor.page.children).map((n) => n.id)

function history(editor: Editor, type: 'undo' | 'redo') {
  editor.endTyping()
  editor.finishPen(false)
  editor.apply({ type })
}

function jump(editor: Editor, to: (k: number, n: number) => number) {
  const { spreads } = editor.snapshot
  const k = spreads.findIndex((s) => s.includes(editor.pageId))
  const [id] = spreads[Math.max(0, Math.min(spreads.length - 1, to(k, spreads.length)))] ?? []
  if (id) editor.showPage(id)
}

function remove(editor: Editor) {
  if (!editor.guide) return editor.apply({ type: 'delete', ids: editor.selection })
  const { id, axis, index } = editor.guide
  const { guides } = editor.sheets.find((s) => s.id === id)!
  editor.apply({ type: 'setGuides', id, guides: { ...guides, [axis]: guides[axis].toSpliced(index, 1) } })
  editor.set({ guide: null })
}

const STYLE = ['fills', 'strokes', 'strokeWeight', 'strokeAlign', 'join', 'cap', 'lineStyle', 'opacity', 'blend', 'effects'] as const
let copied: Props = {}

export const ACTIONS: Action[] = [
  { title: 'New document', keys: 'Ctrl Alt N', group: 'File', run: () => ui.start(), typing: 'any' },
  { title: 'Open', keys: 'Ctrl O', group: 'File', run: () => ui.open(), typing: 'any' },
  { title: 'Save', keys: 'Ctrl S', group: 'File', run: () => ui.save(false), typing: 'any' },
  { title: 'Save as', keys: 'Ctrl Shift S', group: 'File', run: () => ui.save(true), typing: 'any' },
  { title: 'Place image', keys: 'Ctrl Shift K', group: 'File', run: () => ui.place(), typing: 'any' },
  { title: 'Export', keys: 'Ctrl Shift E', group: 'File', run: () => ui.export(), typing: 'any' },
  { title: 'Preflight', keys: 'Ctrl Alt Y', group: 'File', run: (editor) => editor.togglePreflight(), typing: 'any' },
  { title: 'Page overview', keys: '.', group: 'View', run: (editor) => editor.toggleOverview() },
  { title: 'Show or hide left panel', keys: 'Alt 1', group: 'View', run: () => ui.hide('left'), typing: 'any' },
  { title: 'Show or hide right panel', keys: 'Alt 2', group: 'View', run: () => ui.hide('right'), typing: 'any' },
  { title: 'Show or hide interface', keys: 'Shift W', group: 'View', run: () => ui.hide('ui') },
  { title: 'Focus next area', keys: 'F6', group: 'View', run: () => ui.focus(false), typing: 'any' },
  { title: 'Focus previous area', keys: 'Shift F6', group: 'View', run: () => ui.focus(true), typing: 'any' },
  { title: 'Show or hide layout grids', keys: 'Shift G', group: 'View', run: (editor) => editor.set({ grids: !editor.grids }) },
  { title: 'Show or hide rulers', keys: 'Shift R', group: 'View', run: () => ui.hide('rulers') },
  { title: 'Zoom to fit', keys: 'Shift 1', group: 'View', run: () => ui.fit() },
  { title: 'Zoom to selection', keys: 'Shift 2', group: 'View', can: (editor) => editor.selection.length > 0, run: () => ui.fitSelection() },
  { title: 'Zoom to 100%', keys: 'Ctrl 0', group: 'View', run: () => ui.zoomTo(1) },
  { title: 'Zoom in', keys: 'Ctrl +', group: 'View', run: () => ui.zoomBy(1.25) },
  { title: 'Zoom out', keys: 'Ctrl -', group: 'View', run: () => ui.zoomBy(1 / 1.25) },
  { title: 'Next spread', keys: 'PgDn', group: 'Pages', run: (editor) => jump(editor, (k) => k + 1) },
  { title: 'Previous spread', keys: 'PgUp', group: 'Pages', run: (editor) => jump(editor, (k) => k - 1) },
  { title: 'First spread', keys: 'Home', group: 'Pages', run: (editor) => jump(editor, () => 0) },
  { title: 'Last spread', keys: 'End', group: 'Pages', run: (editor) => jump(editor, (_, n) => n - 1) },
  { title: 'Add page', keys: 'Ctrl Alt P', group: 'Pages', run: addPage, typing: 'any' },
  {
    title: 'Delete selected pages',
    keys: '',
    group: 'Pages',
    run: (editor) => {
      if (editor.master) editor.apply({ type: 'deleteMaster', id: editor.pageId })
      else editor.deletePages(editor.overview?.length ? editor.overview : [editor.pageId])
    },
  },
  {
    title: 'Duplicate page',
    keys: '',
    group: 'Pages',
    run: (editor) => {
      const [id] = editor.apply(editor.master ? { type: 'addMaster', like: editor.pageId, layers: true } : { type: 'duplicatePage', id: editor.pageId })
      if (id) editor.showPage(id)
    },
  },
  {
    title: 'Facing pages on or off',
    keys: '',
    group: 'Pages',
    run: (editor) => editor.apply({ type: 'setDocument', facingPages: !editor.snapshot.facingPages }),
  },
  {
    title: 'New master',
    keys: '',
    group: 'Pages',
    run: (editor) => {
      const [id] = editor.apply({ type: 'addMaster', like: editor.pageId })
      if (id) editor.showPage(id)
      editor.set({ overview: null })
    },
  },
  { title: 'Undo', keys: 'Ctrl Z', group: 'Edit', run: (editor) => history(editor, 'undo'), typing: 'text' },
  { title: 'Redo', keys: 'Ctrl Shift Z', group: 'Edit', run: (editor) => history(editor, 'redo'), typing: 'text' },
  { title: 'Version history', keys: 'Ctrl Alt Z', group: 'Edit', run: () => ui.dialog('history'), typing: 'any' },
  { title: 'Find and replace', keys: 'Ctrl F', group: 'Edit', run: () => ui.find(), typing: 'any' },
  { title: 'Copy', keys: 'Ctrl C', group: 'Edit', can: selected, native: true, run: copy },
  { title: 'Cut', keys: 'Ctrl X', group: 'Edit', can: selected, native: true, run: (editor) => {
      copy(editor)
      editor.apply({ type: 'delete', ids: editor.selection })
    } },
  { title: 'Paste', keys: 'Ctrl V', group: 'Edit', native: true, run: (editor) => editor.paste() },
  { title: 'Duplicate', keys: 'Ctrl D', group: 'Edit', can: selected, run: (editor) => editor.duplicate(editor.selection) },
  { title: 'Delete', keys: 'Del', group: 'Edit', can: (editor) => editor.selection.length > 0 || !!editor.guide, run: remove },
  { title: 'Select all', keys: 'Ctrl A', group: 'Edit', can: (editor) => editor.page.children.length > 0, run: (editor) => editor.set({ selection: siblings(editor) }) },
  { title: 'Select none', keys: '', group: 'Edit', run: (editor) => editor.set({ selection: [] }) },
  { title: 'Group', keys: 'Ctrl G', group: 'Arrange', run: (editor) => editor.set({ selection: editor.apply({ type: 'group', ids: editor.selection, frame: false }) }) },
  { title: 'Frame selection', keys: 'Ctrl Alt G', group: 'Arrange', run: (editor) => editor.set({ selection: editor.apply({ type: 'group', ids: editor.selection, frame: true }) }) },
  { title: 'Ungroup', keys: 'Ctrl Shift G', group: 'Arrange', can: (editor) => editor.selected().some((n) => n.kind === 'group' || n.kind === 'frame'), run: (editor) => editor.set({ selection: editor.apply({ type: 'ungroup', ids: editor.selection }) }) },
  { title: 'Use as mask', keys: 'Ctrl Alt M', group: 'Arrange', run: (editor) => editor.set({ selection: editor.apply({ type: 'mask', ids: editor.selection }) }) },
  { title: 'Flatten', keys: 'Ctrl E', group: 'Arrange', can: (editor) => editor.selected().some((n) => n.kind === 'text' || n.kind === 'shape'), run: (editor) => each(editor, (n) => (n.kind === 'text' || n.kind === 'shape') && editor.apply({ type: 'flatten', id: n.id })) },
  { title: 'Flip horizontal', keys: 'Shift H', group: 'Arrange', run: (editor) => each(editor, (n) => editor.apply({ type: 'flip', id: n.id, vertical: false })) },
  { title: 'Flip vertical', keys: 'Shift V', group: 'Arrange', run: (editor) => each(editor, (n) => editor.apply({ type: 'flip', id: n.id, vertical: true })) },
  { title: 'Add auto layout', keys: 'Shift A', group: 'Arrange', run: (editor) => editor.set({ selection: editor.apply({ type: 'autoLayout', ids: editor.selection }) }) },
  { title: 'Remove auto layout', keys: 'Alt Shift A', group: 'Arrange', run: (editor) => each(editor, (n) => n.direction !== 'none' && editor.apply({ type: 'set', id: n.id, direction: 'none' })) },
  { title: 'Bring forward', keys: 'Ctrl ]', group: 'Arrange', run: (editor) => editor.apply({ type: 'order', ids: editor.selection, to: 'forward' }) },
  { title: 'Bring to front', keys: 'Ctrl Shift ]', group: 'Arrange', run: (editor) => editor.apply({ type: 'order', ids: editor.selection, to: 'front' }) },
  { title: 'Send backward', keys: 'Ctrl [', group: 'Arrange', run: (editor) => editor.apply({ type: 'order', ids: editor.selection, to: 'backward' }) },
  { title: 'Send to back', keys: 'Ctrl Shift [', group: 'Arrange', run: (editor) => editor.apply({ type: 'order', ids: editor.selection, to: 'back' }) },
  { title: 'Hide', keys: 'Ctrl Shift H', group: 'Arrange', run: (editor) => toggle(editor, 'hidden') },
  { title: 'Lock', keys: 'Ctrl Shift L', group: 'Arrange', run: (editor) => toggle(editor, 'locked') },
  {
    title: 'Copy style',
    keys: '',
    group: 'Edit',
    can: (editor) => editor.selection.length > 0,
    run: (editor) => {
      const n = editor.selected()[0] as Props
      copied = Object.fromEntries(STYLE.filter((k) => k in n).map((k) => [k, n[k]]))
    },
  },
  {
    title: 'Paste style',
    keys: '',
    group: 'Edit',
    can: (editor) => selected(editor) && Object.keys(copied).length > 0,
    run: (editor) => editor.batch(() => editor.selected().forEach((n) => editor.apply({ type: 'set', id: n.id, ...copied }))),
  },
  { title: 'Rename', keys: 'F2', group: 'Edit', can: selected, run: (editor) => editor.set({ renaming: editor.selection[0] }) },
  {
    title: 'Reset to master',
    keys: '',
    group: 'Pages',
    can: (editor) => editor.selected().some((n) => n.overrideOf),
    run: (editor) => editor.apply({ type: 'resetToMaster', ids: editor.selected().flatMap((n) => (n.overrideOf ? [n.id] : [])) }),
  },
  ...BOOLEANS.map(([op, title, , keys]): Action => ({ title, keys, group: 'Arrange', run: (editor) => combine(editor, op), can: combinable })),
  ...ALIGNS.map(([how, title, , keys]): Action => ({ title, keys, group: 'Align', run: (editor) => align(editor, how), can: (editor) => editor.selection.length >= (how === 'tidy' ? 2 : how.startsWith('distribute') ? 3 : 1) })),
  {
    title: 'Color mode CMYK or RGB',
    keys: '',
    group: 'File',
    run: (editor) => editor.apply({ type: 'setDocument', colorMode: editor.snapshot.colorMode === 'cmyk' ? 'rgb' : 'cmyk' }),
  },
  { title: 'Bold', keys: 'Ctrl B', group: 'Text', run: (editor) => format(editor, { face: 'b' }), typing: 'text' },
  { title: 'Italic', keys: 'Ctrl I', group: 'Text', run: (editor) => format(editor, { face: 'i' }), typing: 'text' },
  { title: 'Underline', keys: 'Ctrl U', group: 'Text', run: (editor) => format(editor, { textDecoration: 'underline' }), typing: 'text' },
  { title: 'Strikethrough', keys: 'Ctrl Shift X', group: 'Text', run: (editor) => format(editor, { textDecoration: 'strikethrough' }), typing: 'text' },
  { title: 'Align text left', keys: 'Ctrl Alt L', group: 'Text', run: (editor) => format(editor, { textAlign: 'left' }), typing: 'text' },
  { title: 'Align text center', keys: 'Ctrl Alt T', group: 'Text', run: (editor) => format(editor, { textAlign: 'center' }), typing: 'text' },
  { title: 'Align text right', keys: 'Ctrl Alt R', group: 'Text', run: (editor) => format(editor, { textAlign: 'right' }), typing: 'text' },
  { title: 'Justify text', keys: 'Ctrl Alt J', group: 'Text', run: (editor) => format(editor, { textAlign: 'justify' }), typing: 'text' },
  { title: 'Increase font size', keys: 'Ctrl Shift .', group: 'Text', run: (editor) => format(editor, { size: 1 }), typing: 'text' },
  { title: 'Decrease font size', keys: 'Ctrl Shift ,', group: 'Text', run: (editor) => format(editor, { size: -1 }), typing: 'text' },
  { title: 'Auto height', keys: 'Ctrl Alt H', group: 'Text', run: (editor) => editor.resize(editor.selected(), 'autoHeight'), typing: 'text' },
  { title: 'Auto fit', keys: 'Ctrl Alt C', group: 'Text', run: (editor) => editor.resize(editor.selected(), 'autoFit'), typing: 'text' },
  { title: 'Insert page number', keys: 'Ctrl Alt Shift N', group: 'Text', run: (editor) => insert(editor, PAGE_NUMBER), can: (editor) => !!editor.editing, typing: 'text' },
  { title: 'Insert date', keys: '', group: 'Text', run: (editor) => insert(editor, DATE), can: (editor) => !!editor.editing },
  ...TABLE_ACTIONS,
  { title: 'Move tool', keys: 'V', group: 'Tools', run: (editor) => editor.setTool('move') },
  { title: 'Frame tool', keys: 'F', group: 'Tools', run: (editor) => editor.setTool('frame') },
  { title: 'Rectangle tool', keys: 'R', group: 'Tools', run: (editor) => editor.setTool('rect') },
  { title: 'Ellipse tool', keys: 'O', group: 'Tools', run: (editor) => editor.setTool('ellipse') },
  { title: 'Line tool', keys: 'L', group: 'Tools', run: (editor) => editor.setTool('line') },
  { title: 'Polygon tool', keys: 'Y', group: 'Tools', run: (editor) => editor.setTool('polygon') },
  { title: 'Star tool', keys: 'S', group: 'Tools', run: (editor) => editor.setTool('star') },
  { title: 'Pen tool', keys: 'P', group: 'Tools', run: (editor) => editor.setTool('pen') },
  { title: 'Text tool', keys: 'T', group: 'Tools', run: (editor) => editor.setTool('text') },
  { title: 'Eyedropper', keys: 'I', group: 'Tools', run: (editor) => editor.setTool('eyedropper') },
  { title: 'Command palette', keys: 'Ctrl K', group: 'View', run: () => ui.dialog('palette'), typing: 'any' },
  { title: 'Keyboard shortcuts', keys: '?', group: 'View', run: () => ui.dialog('help') },
  { title: 'Settings', keys: 'Ctrl ,', group: 'View', run: () => ui.dialog('settings'), typing: 'any' },
]

export const ONLY_WHEN_ABLE = new Set(['Reset to master', 'Table', ...BOOLEANS.map((b) => b[1])])

const GROUP_CAN: Record<string, (editor: Editor) => boolean> = {
  Arrange: (editor) => editor.selection.length > 0,
  Text: (editor) => !!editor.editing || editor.selected().some((n) => n.kind === 'text'),
}

export const able = (editor: Editor, a: Action) => (a.can ?? GROUP_CAN[a.group])?.(editor) ?? true

export const MENU = {
  selected: ['Cut', 'Copy', 'Paste', 'Duplicate', null, 'Group', 'Ungroup', 'Add auto layout', 'Transform', 'Table', null, 'Bring to front', 'Send to back', 'Reset to master', null, 'Rename', 'Hide', 'Lock', null, 'Delete'],
  transform: ['Use as mask', 'Flatten', 'Flip horizontal', 'Flip vertical', 'Tidy up', ...BOOLEANS.map((b) => b[1])],
  table: TABLE_ACTIONS.map((a) => a.title),
  zoom: ['Zoom to fit', 'Zoom to 100%', 'Zoom to selection', null, 'Zoom in', 'Zoom out'],
  none: ['Paste', 'Select all', null, 'Undo', 'Redo', null, 'Zoom to fit', 'Show or hide rulers', 'Show or hide left panel', 'Show or hide right panel', 'Add page', 'Page overview', null, 'Command palette', 'Keyboard shortcuts', 'Settings'],
}

