import type { Editor } from './editor'

/** A command by its title and shortcut; one without `run` runs by pressing its shortcut. */
export type Action = { title: string; keys: string; tool?: boolean; run?: (editor: Editor) => void }

const pageOrLast = (editor: Editor) =>
  editor.snapshot.pages.some((p) => p.id === editor.pageId) ? editor.pageId : editor.snapshot.pages.at(-1)!.id

export const ACTIONS: Action[] = [
  { title: 'New document', keys: 'Ctrl N' },
  { title: 'Open', keys: 'Ctrl O' },
  { title: 'Save', keys: 'Ctrl S' },
  { title: 'Save as', keys: 'Ctrl Shift S' },
  { title: 'Place image', keys: 'Ctrl Shift K' },
  { title: 'Export PDF', keys: 'Ctrl Shift E' },
  { title: 'Preflight', keys: 'Ctrl Alt Y' },
  { title: 'Page overview', keys: '.' },
  { title: 'Show or hide left panel', keys: 'Alt 1' },
  { title: 'Show or hide right panel', keys: 'Alt 2' },
  { title: 'Show or hide interface', keys: 'Ctrl \\' },
  { title: 'Zoom to fit', keys: 'Shift 1' },
  { title: 'Zoom to selection', keys: 'Shift 2' },
  { title: 'Zoom to 100 %', keys: 'Ctrl 0' },
  { title: 'Zoom in', keys: 'Ctrl +' },
  { title: 'Zoom out', keys: 'Ctrl -' },
  { title: 'Next spread', keys: 'PgDn' },
  { title: 'Previous spread', keys: 'PgUp' },
  { title: 'First spread', keys: 'Home' },
  { title: 'Last spread', keys: 'End' },
  {
    title: 'Add page',
    keys: '',
    run: (editor) => {
      const [id] = editor.apply({ type: 'addPage', after: pageOrLast(editor) })
      if (id) editor.showPage(id)
    },
  },
  {
    title: 'Delete selected pages',
    keys: '',
    run: (editor) => editor.deletePages(editor.overview?.length ? editor.overview : [pageOrLast(editor)]),
  },
  {
    title: 'New master',
    keys: '',
    run: (editor) => {
      const [id] = editor.apply({ type: 'addMaster', like: editor.pageId })
      if (id) editor.showPage(id)
      editor.set({ overview: null })
    },
  },
  { title: 'Undo', keys: 'Ctrl Z' },
  { title: 'Redo', keys: 'Ctrl Shift Z' },
  { title: 'Copy', keys: 'Ctrl C' },
  { title: 'Cut', keys: 'Ctrl X' },
  { title: 'Paste', keys: 'Ctrl V' },
  { title: 'Duplicate', keys: 'Ctrl D' },
  { title: 'Delete', keys: 'Del' },
  { title: 'Select all', keys: 'Ctrl A' },
  { title: 'Group', keys: 'Ctrl G' },
  { title: 'Frame selection', keys: 'Ctrl Alt G' },
  { title: 'Ungroup', keys: 'Ctrl Shift G' },
  { title: 'Use as mask', keys: 'Ctrl Alt M' },
  { title: 'Add auto layout', keys: 'Shift A' },
  { title: 'Remove auto layout', keys: 'Shift Alt A' },
  { title: 'Bring forward', keys: 'Ctrl ]' },
  { title: 'Bring to front', keys: 'Ctrl Shift ]' },
  { title: 'Send backward', keys: 'Ctrl [' },
  { title: 'Send to back', keys: 'Ctrl Shift [' },
  { title: 'Hide selection', keys: 'Ctrl Shift H' },
  { title: 'Lock selection', keys: 'Ctrl Shift L' },
  { title: 'Rename', keys: 'F2' },
  { title: 'Move tool', keys: 'V', tool: true },
  { title: 'Frame tool', keys: 'F', tool: true },
  { title: 'Rectangle tool', keys: 'R', tool: true },
  { title: 'Ellipse tool', keys: 'O', tool: true },
  { title: 'Line tool', keys: 'L', tool: true },
  { title: 'Pen tool', keys: 'P', tool: true },
  { title: 'Text tool', keys: 'T', tool: true },
  { title: 'Command palette', keys: 'Ctrl K' },
  { title: 'Keyboard shortcuts', keys: '?' },
]

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
