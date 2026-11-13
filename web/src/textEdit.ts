import type { Editor } from './editor'
import { keyOf } from './keys'
import type { Span, TextNode, TextProps, Typeface } from './model'
import { tab } from './Table'
import { eachOf } from './Text'
import type { TextAlign, TextDecoration } from './types'

/** Stands for the number of the page a text is on, as InDesign's Current Page Number. */
export const PAGE_NUMBER = '\u0018'
/** Stands for the date the document is exported on, as InDesign's Output Date. */
export const DATE = '\u0019'

/** The text layer being edited in its frame; `anchor` and `focus` are UTF-16 indices. */
export type Editing = { id: string; anchor: number; focus: number }

const words = new Intl.Segmenter(undefined, { granularity: 'word' })
const graphemes = new Intl.Segmenter()
export const MAC = /Mac|iP/.test(navigator.platform)

export function textOf(editor: Editor) {
  const n = editor.editing && editor.nodes.get(editor.editing.id)?.node
  return n?.kind === 'text' ? editor.storyOf(n).text : ''
}

export function range(e: Editing): [number, number] {
  return [Math.min(e.anchor, e.focus), Math.max(e.anchor, e.focus)]
}

/** The grapheme boundary before `i`, or the start of the word before it. */
function before(text: string, i: number, word: boolean) {
  if (word) {
    let at = 0
    for (const s of words.segment(text)) if (s.isWordLike && s.index < i) at = s.index
    return at
  }
  return i > 0 ? graphemes.segment(text).containing(i - 1)!.index : 0
}

/** The grapheme boundary after `i`, or the end of the word after it. */
function after(text: string, i: number, word: boolean) {
  if (word) {
    for (const s of words.segment(text)) if (s.isWordLike && s.index + s.segment.length > i) return s.index + s.segment.length
    return text.length
  }
  const s = graphemes.segment(text).containing(i)
  return s ? s.index + s.segment.length : text.length
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
  const length = () => editor.storyOf(editor.nodes.get(e.id)!.node as TextNode).text.length
  const before = length()
  editor.apply({ type: 'editText', id: e.id, range: [a, b], text })
  const at = b + length() - before
  editor.set({ editing: { id: e.id, anchor: at, focus: at } })
}

export function select(editor: Editor, anchor: number, focus: number) {
  const e = editor.editing
  if (!e) return
  editor.endTyping()
  editor.set({ editing: { id: e.id, anchor, focus } })
}

const LOREM =
  'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat'.split(' ')

/** `n` words of lorem ipsum as a sentence. */
const lorem = (n: number) => `L${Array.from({ length: n }, (_, i) => LOREM[i % LOREM.length]).join(' ').slice(1)}.`

/** Moves the caret to `to`, or the focus when `extend`. */
function move(editor: Editor, to: number, extend: boolean) {
  const e = editor.editing!
  select(editor, extend ? e.anchor : to, to)
}

const ALIGN: Record<string, TextAlign> = { l: 'left', t: 'center', r: 'right', j: 'justify' }
const DECORATION: Record<string, TextDecoration> = { u: 'underline', x: 'strikethrough' }

/** The runs of `spans` that reach into `r`. */
export function within(spans: Span[], r: [number, number]) {
  let at = 0
  return spans.filter((s) => (at += s.len) > r[0] && at - s.len < r[1])
}

/** The style of the family of `f` that is or is not bold or italic as `key` (b or i) toggles it. */
function face(fonts: Typeface[], f: Typeface, key: string) {
  const bold = /bold|black|heavy/i.test(f.style) !== (key === 'b')
  const italic = /italic|oblique/i.test(f.style) !== (key === 'i')
  const style = [bold && 'Bold', italic && 'Italic'].filter(Boolean).join(' ') || 'Regular'
  return [fonts.find((o) => o.family === f.family && o.style.toLowerCase() === style.toLowerCase()), `${f.family} ${style}`] as const
}

/** Figma's type shortcuts on the selection of the text being edited, or else on the selected text layers. Returns true when handled. */
export function formatKey(editor: Editor, e: KeyboardEvent): boolean {
  if (!(e.ctrlKey || e.metaKey)) return false
  const key = keyOf(e)
  const ed = editor.editing
  const nodes = (ed ? [editor.nodes.get(ed.id)!.node] : editor.selected()).filter((n): n is TextNode => n.kind === 'text')
  const r = ed && ed.anchor !== ed.focus ? range(ed) : null
  const all = nodes.map((n) => editor.storyOf(n)?.spans ?? [])
  const spans = r ? within(all[0], r) : all.flat()
  const format = (props: TextProps) => nodes.forEach((n) => editor.apply({ type: 'format', id: n.id, range: r, ...props }))
  let run: (() => void) | undefined
  if (e.altKey && !e.shiftKey && ALIGN[key]) run = () => format({ textAlign: ALIGN[key] })
  else if (!e.altKey && e.shiftKey === (key === 'x') && DECORATION[key]) {
    run = () => format({ textDecoration: spans.every((s) => s.textDecoration === DECORATION[key]) ? 'none' : DECORATION[key] })
  } else if (!e.altKey && !e.shiftKey && (key === 'b' || key === 'i')) {
    run = () => {
      const [to, name] = face(editor.snapshot.fonts, spans[0].font ?? editor.snapshot.fonts[0], key)
      if (to) format({ font: to })
      else editor.say(`No ${name} in this document`)
    }
  } else if (!e.altKey && e.shiftKey && (e.code === 'Period' || e.code === 'Comma')) {
    const by = e.code === 'Period' ? 1 : -1
    run = () => nodes.forEach((n, i) => eachOf(editor, n.id, all[i], r)('size', (v) => Math.min(5000, Math.max(1, v + by))))
  }
  if (!run || !spans.length) return false
  editor.endTyping()
  editor.batch(run)
  return true
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
  const word = MAC ? e.altKey : mod
  if (e.key === 'Escape') editor.stopEditing()
  else if (MAC && e.metaKey && e.key.startsWith('Arrow')) {
    const [start, end] = e.key === 'ArrowUp' || e.key === 'ArrowDown' ? [0, text.length] : engine.textLine(ed.id, focus)
    move(editor, e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? start : end, extend)
  } else if (e.key === 'ArrowLeft') move(editor, a !== b && !extend && !word ? a : before(text, focus, word), extend)
  else if (e.key === 'ArrowRight') move(editor, a !== b && !extend && !word ? b : after(text, focus, word), extend)
  else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    const up = e.key === 'ArrowUp'
    const [x, top, bottom] = engine.caret(ed.id, focus)
    let to = engine.textIndex(ed.id, x, up ? top - 0.5 : bottom + 0.5)
    if (engine.textLine(ed.id, to)[0] === engine.textLine(ed.id, focus)[0]) {
      // Past the first or last line of a frame the caret goes on in the frame before or after.
      const n = editor.nodes.get(ed.id)?.node
      const frame = n?.kind === 'text' ? n : undefined
      const [start, end] = frame ? engine.frameRange(frame.id) : [0, 0]
      if (up) to = frame?.prev ? Math.max(0, start - 1) : 0
      else to = frame?.next && end < text.length ? end : text.length
    }
    move(editor, to, extend)
  } else if (e.key === 'Home' || e.key === 'End') {
    const [start, end] = mod ? [0, text.length] : engine.textLine(ed.id, focus)
    move(editor, e.key === 'Home' ? start : end, extend)
  } else if (e.key === 'Backspace' || e.key === 'Delete') {
    const forward = e.key === 'Delete'
    if (a === b) {
      const to = forward ? after(text, a, word) : before(text, a, word)
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
  } else if (e.key === 'Tab') {
    const m = a === b && /\blorem(\d*)$/i.exec(text.slice(0, a))
    if (!m) {
      tab(editor, ed.id, e.shiftKey)
      return true
    }
    select(editor, a - m[0].length, a)
    insert(editor, lorem(Math.min(Number(m[1] || 10), 10000)))
  } else if (!formatKey(editor, e)) return false
  return true
}
