import { able, ACTIONS, siblings, type Action } from './commands'
import { MM, type Editor } from './editor'
import { settings } from './settings'
import { MAC } from './textEdit'
import { remove } from './vector'

const VECTOR_KEYS: Record<string, 'move' | 'add' | 'delete' | 'fill'> = { v: 'move', p: 'add', '-': 'delete', b: 'fill' }
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
}
const KEYS: Record<string, [key: string, code: string]> = {
  PgDn: ['PageDown', 'PageDown'],
  PgUp: ['PageUp', 'PageUp'],
  Home: ['Home', 'Home'],
  End: ['End', 'End'],
  Del: ['Delete', 'Delete'],
  F2: ['F2', 'F2'],
  F6: ['F6', 'F6'],
  '.': ['.', 'Period'],
  ',': [',', 'Comma'],
  '?': ['?', 'Slash'],
  '\\': ['\\', 'Backslash'],
  '[': ['[', 'BracketLeft'],
  ']': [']', 'BracketRight'],
  '+': ['+', 'Equal'],
  '-': ['-', 'Minus'],
}
const SYMBOLS: Record<string, string> = { Ctrl: '⌘', Alt: '⌥', Shift: '⇧' }

export const keyName = (k: string) => (MAC ? (SYMBOLS[k] ?? k) : k)
export const keysOf = (a: Action) => settings.keys[a.title] ?? a.keys
export const keysFor = (title: string) => {
  const a = ACTIONS.find((a) => a.title === title)
  return a ? keysOf(a) : ''
}
export const keyOf = (e: KeyboardEvent) => (e.altKey && /^Key[A-Z]$/.test(e.code) ? e.code.slice(3) : e.key).toLowerCase()

export const ignored = (e: Event) => e.defaultPrevented || document.querySelector('dialog:modal') !== null

export function isTyping(e: Event) {
  return e.target instanceof HTMLElement && e.target.closest('input:not([type=checkbox]), textarea, select, [contenteditable]') !== null
}

export function comboOf(e: KeyboardEvent) {
  if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return null
  const named = (i: 0 | 1, v: string) => Object.entries(KEYS).find(([, names]) => names[i] === v)?.[0]
  const letter = /^[a-z]$/.test(keyOf(e)) && keyOf(e).toUpperCase()
  const digit = /^Digit\d$/.test(e.code) && e.code.at(-1)!
  const code = !letter && !digit && !named(0, e.key) && e.key !== 'Backspace' && named(1, e.code)
  const key = letter || digit || code || (e.key === ' ' ? 'Space' : e.key === 'Backspace' ? 'Del' : (named(0, e.key) ?? e.key))
  const shift = e.shiftKey && (!!code || key.length > 1 || /^[A-Z\d]$/.test(key))
  return [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', shift ? 'Shift' : '', key].filter(Boolean).join(' ')
}

function mode(editor: Editor, e: KeyboardEvent) {
  const bare = !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey
  const key = keyOf(e)
  const done = e.key === 'Escape' || e.key === 'Enter'
  const tool = VECTOR_KEYS[key]
  if (editor.overview && e.key === 'Escape') editor.set({ overview: null })
  else if (editor.preflight && e.key === 'Escape') editor.togglePreflight()
  else if (editor.vector && done) editor.set({ vector: null })
  else if (editor.vector?.at.length && (e.key === 'Delete' || e.key === 'Backspace')) editor.setKnots(remove(editor.knots(), editor.vector.at))
  else if (editor.vector && bare && tool) editor.set({ vector: { ...editor.vector, mode: tool } })
  else if (editor.cropping && done) editor.set({ cropping: null })
  else if (editor.placing.length && e.key === 'Escape') editor.set({ placing: [] })
  else if (editor.threading && e.key === 'Escape') editor.set({ threading: null })
  else if (editor.pen && done) editor.finishPen(false)
  else return false
  return true
}

function context(editor: Editor, e: KeyboardEvent) {
  const mod = e.ctrlKey || e.metaKey
  const ids = editor.selection
  const one = editor.nodes.get(ids[0] ?? '')
  const arrow = ARROWS[e.key]
  if (mod || e.altKey) return false
  if (e.key === 'Escape') {
    if (editor.tool !== 'move') editor.setTool('move')
    else if (!ids.length && editor.master) editor.exitMaster()
    else editor.set({ selection: one?.parent ? [one.parent.id] : [], guide: null })
  } else if (e.key === 'Tab' && (e.target === document.body || (e.target instanceof Element && e.target.classList.contains('canvas')))) {
    const list = siblings(editor).toReversed()
    const next = list[(list.indexOf(ids[0] ?? '') + (e.shiftKey ? -1 : 1) + list.length) % list.length]
    if (!next) return false
    editor.set({ selection: [next] })
  } else if (!ids.length) return false
  else if (e.key === 'Enter') enter(editor, e.shiftKey)
  else if (arrow) {
    const step = (e.shiftKey ? 1 : 0.1) * MM
    const [dx, dy] = arrow
    editor.batch(() => {
      for (const n of editor.changeable()) editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx * step, y: n.y + dy * step, w: n.w, h: n.h })
    })
  } else return false
  return true
}

const allowed = (editor: Editor, a: Action, combo: string, field: boolean, text: boolean) =>
  !a.native &&
  (a.typing === 'any' || (!field && (!text || a.typing === 'text') && (!editor.overview || combo.startsWith('Ctrl') || a.group === 'View'))) &&
  able(editor, a)

export function onKey(editor: Editor, e: KeyboardEvent) {
  if (ignored(e) || e.isComposing || editor.dragging) return
  const target = e.target instanceof Element ? e.target : null
  const text = !!target?.closest('.text-input')
  const field = !text && (isTyping(e) || !!target?.closest(e.key === 'Enter' ? '.menu, button, a[href], summary' : '.menu'))
  const free = !text && !field
  if (free && mode(editor, e)) return e.preventDefault()
  const combo = comboOf(e)
  const a = combo ? ACTIONS.find((a) => keysOf(a) === combo) : undefined
  if (a && combo && allowed(editor, a, combo, field, text)) {
    if (!e.repeat || a.group !== 'Tools') a.run(editor)
    return e.preventDefault()
  }
  if (free && !editor.overview && context(editor, e)) e.preventDefault()
}

export function enter(editor: Editor, up: boolean) {
  const ids = editor.selection
  const one = editor.nodes.get(ids[0] ?? '')
  if (up) {
    if (one?.parent && editor.selected().every((n) => editor.nodes.get(n.id)?.parent === one.parent)) editor.set({ selection: [one.parent.id] })
  } else if (one?.node.kind === 'shape' && ids.length === 1) editor.editPath(one.node.id)
  else if (one?.node.kind === 'text' && ids.length === 1) editor.set({ editing: { id: one.node.id, anchor: 0, focus: editor.storyOf(one.node).text.length } })
  else {
    const kids = editor.selected().flatMap((n) => ('children' in n ? n.children.map((c) => c.id) : []))
    if (kids.length) editor.set({ selection: kids })
  }
}
