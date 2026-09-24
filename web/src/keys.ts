import { MM, type Editor, type Tool } from './editor'

const TOOLS: Record<string, Tool> = {
  v: 'move', f: 'frame', a: 'frame', r: 'rect', o: 'ellipse', l: 'line', p: 'pen', t: 'text',
}
const ORDER = { BracketRight: ['forward', 'front'], BracketLeft: ['backward', 'back'] } as const
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

  if (editor.pen && (e.key === 'Escape' || e.key === 'Enter')) editor.finishPen(false)
  else if (!mod && !e.altKey && !e.shiftKey && TOOLS[key]) editor.setTool(TOOLS[key])
  else if (!mod && !e.altKey && e.shiftKey && key === 'l') editor.setTool('arrow')
  else if (mod && key === 'z') editor.apply({ type: e.shiftKey ? 'redo' : 'undo' })
  else if (mod && key === 'y') editor.apply({ type: 'redo' })
  else if (mod && key === 'a') editor.set({ selection: siblings })
  else if (mod && key === 'v') editor.set({ selection: editor.apply({ type: 'paste', above: ids }) })
  else if (e.key === 'Escape') {
    if (editor.tool !== 'move') editor.setTool('move')
    else editor.set({ selection: one?.parent ? [one.parent.id] : [] })
  } else if (!ids.length) return false
  else if (e.key === 'Delete' || e.key === 'Backspace') editor.apply({ type: 'delete', ids })
  else if (mod && (key === 'c' || key === 'x')) {
    editor.apply({ type: 'copy', ids })
    if (key === 'x') editor.apply({ type: 'delete', ids })
  }  else if (e.key === 'Enter' && !mod) {
    const kids = editor.selected().flatMap((n) => ('children' in n ? n.children.map((c) => c.id) : []))
    if (kids.length) editor.set({ selection: kids })
  } else if (mod && key === 'd') editor.set({ selection: editor.apply({ type: 'duplicate', ids }) })
  else if (mod && key === 'g' && e.shiftKey) editor.set({ selection: editor.apply({ type: 'ungroup', ids }) })
  else if (mod && key === 'g') editor.set({ selection: editor.apply({ type: 'group', ids, frame: e.altKey }) })
  else if (mod && key === 'r') editor.set({ renaming: ids[0] })
  else if (mod && e.code in ORDER) {
    editor.apply({ type: 'order', ids, to: ORDER[e.code as keyof typeof ORDER][e.shiftKey ? 1 : 0] })
  } else if (!mod && ARROWS[e.key]) {
    const step = (e.shiftKey ? 10 : 1) * MM
    const [dx, dy] = ARROWS[e.key]
    editor.apply({ type: 'beginUndoGroup' })
    for (const n of editor.selected()) {
      editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx * step, y: n.y + dy * step, w: n.w, h: n.h })
    }
    editor.apply({ type: 'endUndoGroup' })
  } else return false
  return true
}
