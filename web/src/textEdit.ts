import type { Editor } from './editor'
import type { Span, TextNode, TextProps, Typeface } from './model'
import { tab } from './cells'
import { eachOf } from './Text'
import type { TextAlign, TextDecoration } from './types'

export const PAGE_NUMBER = '\u0018'
export const DATE = '\u0019'

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

function before(text: string, i: number, word: boolean) {
  if (word) {
    let at = 0
    for (const s of words.segment(text)) if (s.isWordLike && s.index < i) at = s.index
    return at
  }
  return i > 0 ? graphemes.segment(text).containing(i - 1)!.index : 0
}

function after(text: string, i: number, word: boolean) {
  if (word) {
    for (const s of words.segment(text)) if (s.isWordLike && s.index + s.segment.length > i) return s.index + s.segment.length
    return text.length
  }
  const s = graphemes.segment(text).containing(i)
  return s ? s.index + s.segment.length : text.length
}

export function wordAt(text: string, i: number): [number, number] {
  for (const s of words.segment(text)) if (s.index <= i && i < s.index + s.segment.length) return [s.index, s.index + s.segment.length]
  return [i, i]
}

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

const lorem = (n: number) => `L${Array.from({ length: n }, (_, i) => LOREM[i % LOREM.length]).join(' ').slice(1)}.`

function move(editor: Editor, to: number, extend: boolean) {
  const e = editor.editing!
  select(editor, extend ? e.anchor : to, to)
}


export function within(spans: Span[], r: [number, number]) {
  let at = 0
  return spans.filter((s) => (at += s.len) > r[0] && at - s.len < r[1])
}

function face(fonts: Typeface[], f: Typeface, key: string) {
  const bold = /bold|black|heavy/i.test(f.style) !== (key === 'b')
  const italic = /italic|oblique/i.test(f.style) !== (key === 'i')
  const style = [bold && 'Bold', italic && 'Italic'].filter(Boolean).join(' ') || 'Regular'
  return [fonts.find((o) => o.family === f.family && o.style.toLowerCase() === style.toLowerCase()), `${f.family} ${style}`] as const
}

export type Format = { textAlign: TextAlign } | { textDecoration: TextDecoration } | { face: 'b' | 'i' } | { size: number }

export function format(editor: Editor, f: Format) {
  const ed = editor.editing
  const nodes = (ed ? [editor.nodes.get(ed.id)!.node] : editor.selected()).filter((n): n is TextNode => n.kind === 'text')
  const r = ed && ed.anchor !== ed.focus ? range(ed) : null
  const all = nodes.map((n) => editor.storyOf(n).spans)
  const spans = r ? within(all[0] ?? [], r) : all.flat()
  if (!spans.length) return
  const set = (props: TextProps) => nodes.forEach((n) => editor.apply({ type: 'format', id: n.id, range: r, ...props }))
  editor.endTyping()
  editor.batch(() => {
    if ('textAlign' in f) set(f)
    else if ('textDecoration' in f) set({ textDecoration: spans.every((s) => s.textDecoration === f.textDecoration) ? 'none' : f.textDecoration })
    else if ('face' in f) {
      const font = spans[0]?.font ?? editor.snapshot.fonts[0]
      if (!font) return
      const [to, name] = face(editor.snapshot.fonts, font, f.face)
      if (to) set({ font: to })
      else editor.say(`No ${name} in this document`)
    } else nodes.forEach((n, i) => eachOf(editor, n.id, all[i] ?? [], r)('size', (v) => Math.min(5000, Math.max(1, v + f.size))))
  })
}

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
    const [start = 0, end = 0] = e.key === 'ArrowUp' || e.key === 'ArrowDown' ? [0, text.length] : engine.textLine(ed.id, focus)
    move(editor, e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? start : end, extend)
  } else if (e.key === 'ArrowLeft') move(editor, a !== b && !extend && !word ? a : before(text, focus, word), extend)
  else if (e.key === 'ArrowRight') move(editor, a !== b && !extend && !word ? b : after(text, focus, word), extend)
  else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    const up = e.key === 'ArrowUp'
    const [x = 0, top = 0, bottom = 0] = engine.caret(ed.id, focus)
    let to = engine.textIndex(ed.id, x, up ? top - 0.5 : bottom + 0.5)
    if (engine.textLine(ed.id, to)[0] === engine.textLine(ed.id, focus)[0]) {
      const n = editor.nodes.get(ed.id)?.node
      const frame = n?.kind === 'text' ? n : undefined
      const [start = 0, end = 0] = frame ? engine.frameRange(frame.id) : [0, 0]
      if (up) to = frame?.prev ? Math.max(0, start - 1) : 0
      else to = frame?.next && end < text.length ? end : text.length
    }
    move(editor, to, extend)
  } else if (e.key === 'Home' || e.key === 'End') {
    const [start = 0, end = 0] = mod ? [0, text.length] : engine.textLine(ed.id, focus)
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
  else if (mod && key === 'a') select(editor, 0, text.length)
  else if (e.key === 'Tab') {
    const m = a === b && /\blorem(\d+)?$/i.exec(text.slice(0, a))
    if (!m) {
      tab(editor, ed.id, e.shiftKey)
      return true
    }
    select(editor, a - m[0].length, a)
    insert(editor, lorem(Math.min(Math.max(Number(m[1] ?? 10), 1), 10000)))
  } else return false
  return true
}
