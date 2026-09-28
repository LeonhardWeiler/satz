import { MM, type Editor, type Tool } from './editor'

const TOOLS: Record<string, Tool> = {
  v: 'move', f: 'frame', a: 'frame', r: 'rect', o: 'ellipse', l: 'line', p: 'pen', t: 'text',
}
const ORDER = { BracketRight: ['forward', 'front'], BracketLeft: ['backward', 'back'] } as const
const JUMPS: Record<string, (k: number, n: number) => number> = {
  PageUp: (k) => k - 1,
  PageDown: (k) => k + 1,
  Home: () => 0,
  End: (_, n) => n - 1,
}
const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
}

/** Figma UI3 shortcuts with Ctrl for Cmd. Returns true when the key was handled. */
export function handleKey(editor: Editor, e: KeyboardEvent): boolean {
  const mod = e.ctrlKey || e.metaKey
  const key = e.key.toLowerCase()
  const ids = editor.selection
  const one = editor.nodes.get(ids[0])
  const siblings = (one?.parent?.children ?? editor.page.children).map((n) => n.id)
  const free = e.target === document.body || (e.target as Element).classList?.contains('canvas')
  const toggle = (prop: 'hidden' | 'locked') => {
    const on = editor.selected().some((n) => !n[prop])
    editor.batch(() => {
      for (const n of editor.selected()) editor.apply({ type: 'set', id: n.id, [prop]: on })
    })
  }

  if (editor.dragging) return false
  if (editor.threading && e.key === 'Escape') editor.set({ threading: null })
  else if (editor.pen && (e.key === 'Escape' || e.key === 'Enter')) editor.finishPen(false)
  else if (!mod && !e.altKey && !e.shiftKey && TOOLS[key]) editor.setTool(TOOLS[key])
  else if (!mod && !e.altKey && e.shiftKey && key === 'l') editor.setTool('arrow')
  else if (mod && (key === 'z' || key === 'y')) {
    editor.finishPen(false)
    editor.apply({ type: key === 'y' || e.shiftKey ? 'redo' : 'undo' })
  }
  else if (mod && key === 'a') editor.set({ selection: siblings })
  else if (mod && key === 'v') editor.set({ selection: editor.apply({ type: 'paste', above: ids, page: editor.page.id }) })
  else if (e.key === 'Escape') {
    if (editor.tool !== 'move') editor.setTool('move')
    else if (!ids.length && editor.master) editor.exitMaster()
    else editor.set({ selection: one?.parent ? [one.parent.id] : [] })
  } else if (!mod && e.key in JUMPS) {
    const { spreads } = editor.snapshot
    const k = spreads.findIndex((s) => s.includes(editor.pageId))
    const to = spreads[Math.max(0, Math.min(spreads.length - 1, JUMPS[e.key](k, spreads.length)))]
    editor.showPage(to[0])
  } else if (e.key === 'Tab' && !mod && !e.altKey && free && siblings.length) {
    const list = siblings.toReversed()
    const i = list.indexOf(ids[0])
    editor.set({ selection: [list[(i + (e.shiftKey ? -1 : 1) + list.length) % list.length]] })
  } else if (!ids.length) return false
  else if (e.key === 'Delete' || e.key === 'Backspace') editor.apply({ type: 'delete', ids })
  else if (mod && (key === 'c' || key === 'x')) {
    editor.apply({ type: 'copy', ids })
    if (key === 'x') editor.apply({ type: 'delete', ids })
  } else if (e.key === 'Enter' && e.shiftKey && !mod) {
    if (one?.parent && editor.selected().every((n) => editor.nodes.get(n.id)?.parent === one.parent)) {
      editor.set({ selection: [one.parent.id] })
    }
  } else if (e.key === 'Enter' && !mod && one?.node.kind === 'text' && ids.length === 1) {
    editor.set({ editing: { id: one.node.id, anchor: 0, focus: editor.storyOf(one.node).text.length } })
  } else if (e.key === 'Enter' && !mod) {
    const kids = editor.selected().flatMap((n) => ('children' in n ? n.children.map((c) => c.id) : []))
    if (kids.length) editor.set({ selection: kids })
  } else if (mod && key === 'd') editor.set({ selection: editor.apply({ type: 'duplicate', ids }) })
  else if (mod && key === 'g' && e.shiftKey) editor.set({ selection: editor.apply({ type: 'ungroup', ids }) })
  else if (mod && key === 'g') editor.set({ selection: editor.apply({ type: 'group', ids, frame: e.altKey }) })
  else if ((mod && key === 'r') || e.key === 'F2') editor.set({ renaming: ids[0] })
  else if (mod && e.shiftKey && key === 'h') toggle('hidden')
  else if (mod && e.shiftKey && key === 'l') toggle('locked')
  else if (!mod && e.shiftKey && e.altKey && key === 'a') {
    editor.batch(() => {
      for (const n of editor.selected()) if (n.direction !== 'none') editor.apply({ type: 'set', id: n.id, direction: 'none' })
    })
  } else if (!mod && e.shiftKey && key === 'a') editor.set({ selection: editor.apply({ type: 'autoLayout', ids }) })
  else if (mod && e.altKey && key === 'm') editor.set({ selection: editor.apply({ type: 'mask', ids }) })
  else if (mod && e.code in ORDER) {
    editor.apply({ type: 'order', ids, to: ORDER[e.code as keyof typeof ORDER][e.shiftKey ? 1 : 0] })
  } else if (!mod && ARROWS[e.key]) {
    const step = (e.shiftKey ? 10 : 1) * MM
    const [dx, dy] = ARROWS[e.key]
    editor.batch(() => {
      for (const n of editor.selected()) {
        editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx * step, y: n.y + dy * step, w: n.w, h: n.h })
      }
    })
  } else return false
  return true
}
