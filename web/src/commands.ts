import type { Editor } from './editor'

/** A command by its title, shortcut and group; one without `run` runs by pressing its shortcut, one whose `can` fails is greyed out in the context menu. */
export type Action = { title: string; keys: string; group: string; run?: (editor: Editor) => void; can?: (editor: Editor) => boolean }

const pageOrLast = (editor: Editor) =>
  editor.snapshot.pages.some((p) => p.id === editor.pageId) ? editor.pageId : editor.snapshot.pages.at(-1)!.id

export const ACTIONS: Action[] = [
  { title: 'New document', keys: 'Ctrl N', group: 'File' },
  { title: 'Open', keys: 'Ctrl O', group: 'File' },
  { title: 'Save', keys: 'Ctrl S', group: 'File' },
  { title: 'Save as', keys: 'Ctrl Shift S', group: 'File' },
  { title: 'Place image', keys: 'Ctrl Shift K', group: 'File' },
  { title: 'Export PDF', keys: 'Ctrl Shift E', group: 'File' },
  { title: 'Preflight', keys: 'Ctrl Alt Y', group: 'File' },
  { title: 'Page overview', keys: '.', group: 'View' },
  { title: 'Show or hide left panel', keys: 'Alt 1', group: 'View' },
  { title: 'Show or hide right panel', keys: 'Alt 2', group: 'View' },
  { title: 'Show or hide interface', keys: 'Ctrl \\', group: 'View' },
  { title: 'Show or hide layout grids', keys: 'Shift G', group: 'View' },
  { title: 'Zoom to fit', keys: 'Shift 1', group: 'View' },
  { title: 'Zoom to selection', keys: 'Shift 2', group: 'View' },
  { title: 'Zoom to 100 %', keys: 'Ctrl 0', group: 'View' },
  { title: 'Zoom in', keys: 'Ctrl +', group: 'View' },
  { title: 'Zoom out', keys: 'Ctrl -', group: 'View' },
  { title: 'Next spread', keys: 'PgDn', group: 'Pages' },
  { title: 'Previous spread', keys: 'PgUp', group: 'Pages' },
  { title: 'First spread', keys: 'Home', group: 'Pages' },
  { title: 'Last spread', keys: 'End', group: 'Pages' },
  {
    title: 'Add page',
    keys: '',
    group: 'Pages',
    run: (editor) => {
      const [id] = editor.apply({ type: 'addPage', after: pageOrLast(editor) })
      if (id) editor.showPage(id)
    },
  },
  {
    title: 'Delete selected pages',
    keys: '',
    group: 'Pages',
    run: (editor) => editor.deletePages(editor.overview?.length ? editor.overview : [pageOrLast(editor)]),
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
  { title: 'Copy', keys: 'Ctrl C', group: 'Edit' },
  { title: 'Cut', keys: 'Ctrl X', group: 'Edit' },
  { title: 'Paste', keys: 'Ctrl V', group: 'Edit' },
  { title: 'Duplicate', keys: 'Ctrl D', group: 'Edit' },
  { title: 'Delete', keys: 'Del', group: 'Edit' },
  { title: 'Select all', keys: 'Ctrl A', group: 'Edit' },
  { title: 'Group', keys: 'Ctrl G', group: 'Arrange' },
  { title: 'Frame selection', keys: 'Ctrl Alt G', group: 'Arrange' },
  { title: 'Ungroup', keys: 'Ctrl Shift G', group: 'Arrange', can: (editor) => editor.selected().some((n) => n.kind === 'group' || n.kind === 'frame') },
  { title: 'Use as mask', keys: 'Ctrl Alt M', group: 'Arrange' },
  { title: 'Add auto layout', keys: 'Shift A', group: 'Arrange' },
  { title: 'Remove auto layout', keys: 'Shift Alt A', group: 'Arrange' },
  { title: 'Bring forward', keys: 'Ctrl ]', group: 'Arrange' },
  { title: 'Bring to front', keys: 'Ctrl Shift ]', group: 'Arrange' },
  { title: 'Send backward', keys: 'Ctrl [', group: 'Arrange' },
  { title: 'Send to back', keys: 'Ctrl Shift [', group: 'Arrange' },
  { title: 'Hide selection', keys: 'Ctrl Shift H', group: 'Arrange' },
  { title: 'Lock selection', keys: 'Ctrl Shift L', group: 'Arrange' },
  { title: 'Rename', keys: 'F2', group: 'Edit' },
  { title: 'Move tool', keys: 'V', group: 'Tools' },
  { title: 'Frame tool', keys: 'F', group: 'Tools' },
  { title: 'Rectangle tool', keys: 'R', group: 'Tools' },
  { title: 'Ellipse tool', keys: 'O', group: 'Tools' },
  { title: 'Line tool', keys: 'L', group: 'Tools' },
  { title: 'Pen tool', keys: 'P', group: 'Tools' },
  { title: 'Text tool', keys: 'T', group: 'Tools' },
  { title: 'Command palette', keys: 'Ctrl K', group: 'View' },
  { title: 'Keyboard shortcuts', keys: '?', group: 'View' },
]

/** The titles of the actions the context menu offers with and without a selection, `null` between groups. */
export const MENU = {
  selected: ['Cut', 'Copy', 'Paste', 'Duplicate', 'Delete', null, 'Group', 'Frame selection', 'Ungroup', 'Use as mask', 'Add auto layout', null, 'Bring to front', 'Send to back', null, 'Hide selection', 'Lock selection', 'Rename'],
  none: ['Paste', 'Select all', null, 'Undo', 'Redo', null, 'Zoom to fit', 'Add page', 'Page overview', null, 'Command palette', 'Keyboard shortcuts'],
}

const MAC = /Mac|iP/.test(navigator.platform)
const SYMBOLS: Record<string, string> = { Ctrl: '⌘', Alt: '⌥', Shift: '⇧' }
/** `keys` as the platform shows them: ⌘ ⌥ ⇧ on a Mac. */
export const keyLabel = (keys: string) => (MAC ? keys.split(' ').map((k) => SYMBOLS[k] ?? k).join(' ') : keys)

const KEYS: Record<string, [key: string, code: string]> = {
  PgDn: ['PageDown', 'PageDown'],
  PgUp: ['PageUp', 'PageUp'],
  Home: ['Home', 'Home'],
  End: ['End', 'End'],
  Del: ['Delete', 'Delete'],
  F2: ['F2', 'F2'],
  '.': ['.', 'Period'],
  '?': ['?', 'Slash'],
  '\\': ['\\', 'Backslash'],
  '[': ['[', 'BracketLeft'],
  ']': [']', 'BracketRight'],
  '+': ['+', 'Equal'],
  '-': ['-', 'Minus'],
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

/** 2 when `query` is in `text`, 1 when its letters are in order, else 0. */
export function fuzzy(query: string, text: string) {
  const q = query.toLowerCase()
  const s = text.toLowerCase()
  if (s.includes(q)) return 2
  let i = 0
  for (const c of s) if (c === q[i]) i++
  return i === q.length ? 1 : 0
}
