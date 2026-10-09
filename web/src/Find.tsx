import { useEffect, useMemo, useRef, useState } from 'react'
import { Select } from './controls'
import { useEditor, type Editor } from './editor'
import type { Node, Snapshot } from './model'
import { Icon } from './icons'

type Match = Editor['found'][number]

/** Every match of `find` in the stories in page order, in whole words or case if asked, in text of the text style `style` if given. */
function matches(snapshot: Snapshot, find: string, matchCase: boolean, words: boolean, style: string): Match[] {
  if (!find) return []
  const escaped = find.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(words ? `(?<![\\p{L}\\p{N}_])${escaped}(?![\\p{L}\\p{N}_])` : escaped, `gu${matchCase ? '' : 'i'}`)
  const heads: string[] = []
  const walk = (nodes: Node[]): void =>
    nodes.forEach((n) => {
      if (n.kind === 'text' && n.story === n.id) heads.push(n.id)
      if ('children' in n) walk(n.children)
    })
  for (const p of [...snapshot.pages, ...snapshot.masters]) walk(p.children)
  return heads.flatMap((story) => {
    const { text, spans } = snapshot.stories[story]
    return [...text.matchAll(re)].flatMap((m) => {
      let at = 0
      const span = spans.find((s) => (at += s.len) > m.index)
      return !style || span?.textStyle === style ? [{ story, at: m.index, end: m.index + m[0].length }] : []
    })
  })
}

/** Ctrl+F: finds text in every story and replaces it. */
export function Find({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const input = useRef<HTMLInputElement>(null)
  const [find, setFind] = useState('')
  const [replace, setReplace] = useState('')
  const [matchCase, setMatchCase] = useState(false)
  const [words, setWords] = useState(false)
  const [style, setStyle] = useState('')
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const editing = useEditor(editor, (e) => e.editing)
  const all = useMemo(() => matches(snapshot, find, matchCase, words, style), [snapshot, find, matchCase, words, style])
  useEffect(() => {
    editor.set({ found: all })
    return () => editor.set({ found: [] })
  }, [editor, all])
  const story = editing && editor.nodes.get(editing.id)?.node
  const head = story?.kind === 'text' ? story.story : null
  const lo = editing ? Math.min(editing.anchor, editing.focus) : 0
  const hi = editing ? Math.max(editing.anchor, editing.focus) : 0
  const current = all.findIndex((m) => m.story === head && m.at === lo && m.end === hi)
  const show = (m: Match) => editor.set({ editing: { id: m.story, anchor: m.at, focus: m.end } })
  const next = (d: 1 | -1) => {
    if (!all.length) return
    const after = all.findIndex((m) => (m.story === head ? m.at >= hi : false))
    const i = current >= 0 ? current + d : d > 0 ? Math.max(0, after) : (after > 0 ? after : all.length) - 1
    show(all[(i + all.length) % all.length])
  }
  const replaceOne = () => {
    if (current < 0) return next(1)
    const m = all[current]
    editor.apply({ type: 'editText', id: m.story, range: [m.at, m.end], text: replace })
    const rest = matches(editor.snapshot, find, matchCase, words, style)
    const following = rest.find((r) => r.story === m.story && r.at >= m.at + replace.length) ?? rest.find((r) => r.story !== m.story) ?? rest[0]
    if (following) show(following)
  }
  const replaceAll = () => {
    editor.batch(() => {
      for (const m of all.toReversed()) editor.apply({ type: 'editText', id: m.story, range: [m.at, m.end], text: replace })
    })
    input.current!.focus()
    editor.say(`Replaced ${all.length} ${all.length === 1 ? 'match' : 'matches'}`)
  }
  return (
    <section
      className="find"
      role="dialog"
      aria-label="Find and replace"
      onKeyDown={(e) => {
        if (e.key !== 'Escape') return
        e.preventDefault()
        e.stopPropagation()
        onClose()
      }}
    >
      <div className="find-row">
        <label className="field">
          <input
            ref={input}
            aria-label="Find"
            placeholder="Find"
            autoFocus
            autoComplete="off"
            aria-invalid={!!find && !all.length}
            value={find}
            onChange={(e) => setFind(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && next(e.shiftKey ? -1 : 1)}
          />
        </label>
        <span className="find-acts">
        <span className="find-count" aria-live="polite">
          {find && (current >= 0 ? `${current + 1} of ${all.length}` : `${all.length} found`)}
        </span>
        <button type="button" className="icon-button" aria-label="Previous match" title="Previous match (Shift Enter)" disabled={!all.length} onClick={() => next(-1)}>
          <Icon name="arrowUp" />
        </button>
        <button type="button" className="icon-button" aria-label="Next match" title="Next match (Enter)" disabled={!all.length} onClick={() => next(1)}>
          <Icon name="arrowDown" />
        </button>
        <button type="button" className="icon-button" aria-label="Close" onClick={onClose}>
          <Icon name="close" />
        </button>
        </span>
      </div>
      <div className="find-row">
        <label className="field">
          <input aria-label="Replace with" placeholder="Replace with" autoComplete="off" value={replace} onChange={(e) => setReplace(e.currentTarget.value)} onKeyDown={(e) => e.key === 'Enter' && replaceOne()} />
        </label>
        <span className="find-acts">
          <button type="button" className="button" disabled={!all.length} onClick={replaceOne}>
            Replace
          </button>
          <button type="button" className="button" disabled={!all.length} onClick={replaceAll}>
            Replace all
          </button>
        </span>
      </div>
      <div className="find-row find-options">
        <label className="check">
          <input type="checkbox" checked={matchCase} onChange={(e) => setMatchCase(e.currentTarget.checked)} />
          Match case
        </label>
        <label className="check">
          <input type="checkbox" checked={words} onChange={(e) => setWords(e.currentTarget.checked)} />
          Whole words
        </label>
        <Select label="In text style" prefix="Style" value={style} options={Object.fromEntries([['', 'any'], ...snapshot.textStyles.map((s) => [s.id, s.name])])} onChange={setStyle} />
      </div>
    </section>
  )
}
