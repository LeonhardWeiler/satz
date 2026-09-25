import type { Editor } from './editor'

/** Stands for the number of the page a text is on, as InDesign's Current Page Number. */
export const PAGE_NUMBER = '\u0018'

/** The text layer being edited in its frame; `anchor` and `focus` are UTF-16 indices. */
export type Editing = { id: string; anchor: number; focus: number }

const words = new Intl.Segmenter(undefined, { granularity: 'word' })

export function textOf(editor: Editor) {
  const n = editor.editing && editor.nodes.get(editor.editing.id)?.node
  return n?.kind === 'text' ? n.text : ''
}

export function range(e: Editing): [number, number] {
  return [Math.min(e.anchor, e.focus), Math.max(e.anchor, e.focus)]
}

/** The code point boundary before `i`, or the start of the word before it. */
function before(text: string, i: number, word: boolean) {
  if (word) {
    let at = 0
    for (const s of words.segment(text)) if (s.isWordLike && s.index < i) at = s.index
    return at
  }
  const low = text.charCodeAt(i - 1)
  return i - (low >= 0xdc00 && low <= 0xdfff && i > 1 ? 2 : 1)
}

/** The code point boundary after `i`, or the end of the word after it. */
function after(text: string, i: number, word: boolean) {
  if (word) {
    for (const s of words.segment(text)) if (s.isWordLike && s.index + s.segment.length > i) return s.index + s.segment.length
    return text.length
  }
  const high = text.charCodeAt(i)
  return i + (high >= 0xd800 && high <= 0xdbff ? 2 : 1)
}

/** The word around `i`, for a double-click. */
export function wordAt(text: string, i: number): [number, number] {
  for (const s of words.segment(text)) if (s.index <= i && i < s.index + s.segment.length) return [s.index, s.index + s.segment.length]
  return [i, i]
}

/** Replaces the selection with `text` inside the typing undo step and puts the caret after it. */
export function insert(editor: Editor, text: string) {
  const e = editor.editing
  if (!e) return
  const [a, b] = range(e)
  if (!text && a === b) return
  editor.beginTyping()
  editor.apply({ type: 'editText', id: e.id, range: [a, b], text })
  editor.set({ editing: { id: e.id, anchor: a + text.length, focus: a + text.length } })
}

export function select(editor: Editor, anchor: number, focus: number) {
  const e = editor.editing
  if (!e) return
  editor.endTyping()
  editor.set({ editing: { id: e.id, anchor, focus } })
}

/** Moves the caret to `to`, or the focus when `extend`. */
function move(editor: Editor, to: number, extend: boolean) {
  const e = editor.editing!
  select(editor, extend ? e.anchor : to, to)
}

/** Keys of a text layer being edited, Figma's with Ctrl for Cmd. Returns true when handled. */
export function handleTextKey(editor: Editor, e: KeyboardEvent): boolean {
  const ed = editor.editing
  if (!ed) return false
  const text = textOf(editor)
  const mod = e.ctrlKey || e.metaKey
  const key = e.key.toLowerCase()
  const [a, b] = range(ed)
  const { focus } = ed
  const extend = e.shiftKey
  const engine = editor.engine
  if (e.key === 'Escape') editor.stopEditing()
  else if (e.key === 'ArrowLeft') move(editor, a !== b && !extend && !mod ? a : Math.max(0, before(text, focus, mod)), extend)
  else if (e.key === 'ArrowRight') move(editor, a !== b && !extend && !mod ? b : Math.min(text.length, after(text, focus, mod)), extend)
  else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    const up = e.key === 'ArrowUp'
    const [x, top, bottom] = engine.caret(ed.id, focus)
    let to = engine.textIndex(ed.id, x, up ? top - 0.5 : bottom + 0.5)
    if (engine.textLine(ed.id, to)[0] === engine.textLine(ed.id, focus)[0]) {
      // Past the first or last line of a frame the caret goes on in the frame before or after.
      const n = editor.nodes.get(ed.id)?.node
      const frame = n?.kind === 'text' ? n : undefined
      if (up) to = frame?.prev ? Math.max(0, frame.start - 1) : 0
      else to = frame?.next && frame.end < text.length ? frame.end : text.length
    }
    move(editor, to, extend)
  } else if (e.key === 'Home' || e.key === 'End') {
    const [start, end] = mod ? [0, text.length] : engine.textLine(ed.id, focus)
    move(editor, e.key === 'Home' ? start : end, extend)
  } else if (e.key === 'Backspace' || e.key === 'Delete') {
    const forward = e.key === 'Delete'
    if (a === b) {
      const to = forward ? Math.min(text.length, after(text, a, mod)) : Math.max(0, before(text, a, mod))
      if (to === a) return true
      editor.set({ editing: { ...ed, anchor: a, focus: to } })
    }
    insert(editor, '')
  } else if (e.key === 'Enter') insert(editor, '\n')
  else if (mod && e.altKey && e.shiftKey && e.code === 'KeyN') insert(editor, PAGE_NUMBER)
  else if (mod && key === 'a') select(editor, 0, text.length)
  else if (mod && (key === 'z' || key === 'y')) {
    editor.endTyping()
    editor.apply({ type: key === 'y' || e.shiftKey ? 'redo' : 'undo' })
  } else if (e.key === 'Tab') return true
  else return false
  return true
}
