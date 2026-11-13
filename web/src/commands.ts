import { align, ALIGNS, BOOLEANS, combinable, combine } from './align'
import type { Editor } from './editor'
import type { Props } from './model'
import { settings } from './settings'
import { keyOf } from './keys'
import { DATE, insert, MAC, PAGE_NUMBER } from './textEdit'

/** A command by its title, shortcut and group; one without `run` runs by pressing its shortcut, one whose `can` fails is greyed out in the context menu. */
export type Action = { title: string; keys: string; group: string; run?: (editor: Editor) => void; can?: (editor: Editor) => boolean }

const pageOrLast = (editor: Editor) =>
  editor.snapshot.pages.some((p) => p.id === editor.pageId) ? editor.pageId : editor.snapshot.pages.at(-1)!.id

export function addPage(editor: Editor) {
  const [id] = editor.apply({ type: 'addPage', after: pageOrLast(editor) })
  if (id) editor.showPage(id)
}

const STYLE = ['fills', 'strokes', 'strokeWeight', 'strokeAlign', 'join', 'cap', 'lineStyle', 'opacity', 'blend', 'effects'] as const
let copied: Props = {}

export const ACTIONS: Action[] = [
  { title: 'New document', keys: 'Ctrl Alt N', group: 'File' },
  { title: 'Open', keys: 'Ctrl O', group: 'File' },
  { title: 'Save', keys: 'Ctrl S', group: 'File' },
  { title: 'Save as', keys: 'Ctrl Shift S', group: 'File' },
  { title: 'Place image', keys: 'Ctrl Shift K', group: 'File' },
  { title: 'Export', keys: 'Ctrl Shift E', group: 'File' },
  { title: 'Preflight', keys: 'Ctrl Alt Y', group: 'File' },
  { title: 'Page overview', keys: '.', group: 'View' },
  { title: 'Show or hide left panel', keys: 'Alt 1', group: 'View' },
  { title: 'Show or hide right panel', keys: 'Alt 2', group: 'View' },
  { title: 'Show or hide interface', keys: 'Shift W', group: 'View' },
  { title: 'Show or hide layout grids', keys: 'Shift G', group: 'View' },
  { title: 'Show or hide rulers', keys: 'Shift R', group: 'View' },
  { title: 'Zoom to fit', keys: 'Shift 1', group: 'View' },
  { title: 'Zoom to selection', keys: 'Shift 2', group: 'View' },
  { title: 'Zoom to 100%', keys: 'Ctrl 0', group: 'View' },
  { title: 'Zoom in', keys: 'Ctrl +', group: 'View' },
  { title: 'Zoom out', keys: 'Ctrl -', group: 'View' },
  { title: 'Next spread', keys: 'PgDn', group: 'Pages' },
  { title: 'Previous spread', keys: 'PgUp', group: 'Pages' },
  { title: 'First spread', keys: 'Home', group: 'Pages' },
  { title: 'Last spread', keys: 'End', group: 'Pages' },
  { title: 'Add page', keys: 'Ctrl Alt P', group: 'Pages', run: addPage },
  {
    title: 'Delete selected pages',
    keys: '',
    group: 'Pages',
    run: (editor) => editor.deletePages(editor.overview?.length ? editor.overview : [pageOrLast(editor)]),
  },
  {
    title: 'Duplicate page',
    keys: '',
    group: 'Pages',
    run: (editor) => {
      const [id] = editor.apply({ type: 'duplicatePage', id: pageOrLast(editor) })
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
  { title: 'Undo', keys: 'Ctrl Z', group: 'Edit' },
  { title: 'Redo', keys: 'Ctrl Shift Z', group: 'Edit' },
  { title: 'Version history', keys: 'Ctrl Alt Z', group: 'Edit' },
  { title: 'Find and replace', keys: 'Ctrl F', group: 'Edit' },
  { title: 'Copy', keys: 'Ctrl C', group: 'Edit' },
  { title: 'Cut', keys: 'Ctrl X', group: 'Edit' },
  { title: 'Paste', keys: 'Ctrl V', group: 'Edit' },
  { title: 'Duplicate', keys: 'Ctrl D', group: 'Edit' },
  { title: 'Delete', keys: 'Del', group: 'Edit' },
  { title: 'Select all', keys: 'Ctrl A', group: 'Edit' },
  { title: 'Select none', keys: '', group: 'Edit', run: (editor) => editor.set({ selection: [] }) },
  { title: 'Group', keys: 'Ctrl G', group: 'Arrange' },
  { title: 'Frame selection', keys: 'Ctrl Alt G', group: 'Arrange' },
  { title: 'Ungroup', keys: 'Ctrl Shift G', group: 'Arrange', can: (editor) => editor.selected().some((n) => n.kind === 'group' || n.kind === 'frame') },
  { title: 'Use as mask', keys: 'Ctrl Alt M', group: 'Arrange' },
  { title: 'Flatten', keys: 'Ctrl E', group: 'Arrange', can: (editor) => editor.selected().some((n) => n.kind === 'text' || n.kind === 'shape') },
  { title: 'Flip horizontal', keys: 'Shift H', group: 'Arrange' },
  { title: 'Flip vertical', keys: 'Shift V', group: 'Arrange' },
  { title: 'Add auto layout', keys: 'Shift A', group: 'Arrange' },
  { title: 'Remove auto layout', keys: 'Shift Alt A', group: 'Arrange' },
  { title: 'Bring forward', keys: 'Ctrl ]', group: 'Arrange' },
  { title: 'Bring to front', keys: 'Ctrl Shift ]', group: 'Arrange' },
  { title: 'Send backward', keys: 'Ctrl [', group: 'Arrange' },
  { title: 'Send to back', keys: 'Ctrl Shift [', group: 'Arrange' },
  { title: 'Hide', keys: 'Ctrl Shift H', group: 'Arrange' },
  { title: 'Lock', keys: 'Ctrl Shift L', group: 'Arrange' },
  {
    title: 'Copy style',
    keys: '',
    group: 'Edit',
    run: (editor) => {
      const n = editor.selected()[0] as Props
      copied = Object.fromEntries(STYLE.filter((k) => k in n).map((k) => [k, n[k]]))
    },
  },
  {
    title: 'Paste style',
    keys: '',
    group: 'Edit',
    can: () => Object.keys(copied).length > 0,
    run: (editor) => editor.batch(() => editor.selected().forEach((n) => editor.apply({ type: 'set', id: n.id, ...copied }))),
  },
  { title: 'Rename', keys: 'F2', group: 'Edit' },
  {
    title: 'Reset to master',
    keys: '',
    group: 'Pages',
    can: (editor) => editor.selected().some((n) => n.overrideOf),
    run: (editor) => editor.apply({ type: 'resetToMaster', ids: editor.selected().flatMap((n) => (n.overrideOf ? [n.id] : [])) }),
  },
  ...BOOLEANS.map(([op, title, , keys]): Action => ({ title, keys: keys.replaceAll('+', ' '), group: 'Arrange', run: (editor) => combine(editor, op), can: combinable })),
  ...ALIGNS.map(([how, title, , keys]): Action => ({ title, keys: keys.replaceAll('+', ' '), group: 'Align', run: (editor) => align(editor, how), can: how === 'tidy' ? (editor) => editor.selection.length > 1 : undefined })),
  {
    title: 'Color mode CMYK or RGB',
    keys: '',
    group: 'File',
    run: (editor) => editor.apply({ type: 'setDocument', colorMode: editor.snapshot.colorMode === 'cmyk' ? 'rgb' : 'cmyk' }),
  },
  { title: 'Auto height', keys: 'Ctrl Alt H', group: 'Text' },
  { title: 'Auto fit', keys: 'Ctrl Alt C', group: 'Text' },
  { title: 'Insert page number', keys: 'Ctrl Alt Shift N', group: 'Text', run: (editor) => editor.editing && insert(editor, PAGE_NUMBER) },
  { title: 'Insert date', keys: '', group: 'Text', run: (editor) => editor.editing && insert(editor, DATE) },
  { title: 'Move tool', keys: 'V', group: 'Tools' },
  { title: 'Frame tool', keys: 'F', group: 'Tools' },
  { title: 'Rectangle tool', keys: 'R', group: 'Tools' },
  { title: 'Ellipse tool', keys: 'O', group: 'Tools' },
  { title: 'Line tool', keys: 'L', group: 'Tools' },
  { title: 'Polygon tool', keys: 'Y', group: 'Tools' },
  { title: 'Star tool', keys: 'S', group: 'Tools' },
  { title: 'Pen tool', keys: 'P', group: 'Tools' },
  { title: 'Text tool', keys: 'T', group: 'Tools' },
  { title: 'Eyedropper', keys: 'I', group: 'Tools' },
  { title: 'Command palette', keys: 'Ctrl K', group: 'View' },
  { title: 'Keyboard shortcuts', keys: '?', group: 'View' },
  { title: 'Settings', keys: 'Ctrl ,', group: 'View' },
]

/** The context menu leaves these out instead of greying them out. */
export const ONLY_WHEN_ABLE = new Set(['Reset to master', ...BOOLEANS.map((b) => b[1])])

/** The titles of the actions the context menu offers with and without a selection, `null` between groups. */
export const MENU = {
  selected: ['Cut', 'Copy', 'Paste', 'Duplicate', null, 'Group', 'Ungroup', 'Add auto layout', 'Transform', null, 'Bring to front', 'Send to back', 'Reset to master', null, 'Delete'],
  transform: ['Use as mask', 'Flatten', 'Flip horizontal', 'Flip vertical', 'Tidy up', ...BOOLEANS.map((b) => b[1])],
  none: ['Paste', 'Select all', null, 'Undo', 'Redo', null, 'Zoom to fit', 'Show or hide rulers', 'Show or hide left panel', 'Show or hide right panel', 'Add page', 'Page overview', null, 'Command palette', 'Keyboard shortcuts', 'Settings'],
}

const SYMBOLS: Record<string, string> = { Ctrl: '⌘', Alt: '⌥', Shift: '⇧' }
/** The key `k` as the platform names it: ⌘ ⌥ ⇧ on a Mac. */
export const keyName = (k: string) => (MAC ? (SYMBOLS[k] ?? k) : k)

const KEYS: Record<string, [key: string, code: string]> = {
  PgDn: ['PageDown', 'PageDown'],
  PgUp: ['PageUp', 'PageUp'],
  Home: ['Home', 'Home'],
  End: ['End', 'End'],
  Del: ['Delete', 'Delete'],
  F2: ['F2', 'F2'],
  '.': ['.', 'Period'],
  ',': [',', 'Comma'],
  '?': ['?', 'Slash'],
  '\\': ['\\', 'Backslash'],
  '[': ['[', 'BracketLeft'],
  ']': [']', 'BracketRight'],
  '+': ['+', 'Equal'],
  '-': ['-', 'Minus'],
}

/** The keys `a` runs on, as the settings remap them. */
export const keysOf = (a: Action) => settings.keys[a.title] ?? a.keys

/** The shortcut `e` presses as `keys` names it, null for a lone modifier. */
export function comboOf(e: KeyboardEvent) {
  if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return null
  const named = Object.entries(KEYS).find(([, [key]]) => key === e.key)?.[0]
  const key = /^[a-z]$/.test(keyOf(e)) ? keyOf(e).toUpperCase() : /^Digit\d$/.test(e.code) ? e.code.at(-1)! : e.key === ' ' ? 'Space' : (named ?? e.key)
  const shift = e.shiftKey && (key.length > 1 || /^[A-Z\d]$/.test(key))
  return [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', shift ? 'Shift' : '', key].filter(Boolean).join(' ')
}

/** Presses the shortcut `keys` on the document as if typed. */
export function press(keys: string) {
  const parts = keys.split(' ')
  const last = parts.at(-1)!
  const shiftKey = parts.includes('Shift')
  const [key, code] =
    KEYS[last] ?? (/^\d$/.test(last) ? [last, `Digit${last}`] : [shiftKey ? last : last.toLowerCase(), `Key${last}`])
  document.body.dispatchEvent(
    new KeyboardEvent('keydown', { key, code, shiftKey, ctrlKey: parts.includes('Ctrl'), altKey: parts.includes('Alt'), bubbles: true, cancelable: true }),
  )
}
