import { useEffect, useRef, useState, type ReactNode } from 'react'
import { ACTIONS, keysOf, press } from './commands'
import { Chip, ink, NO_SCOPE } from './ColorPicker'
import type { Editor } from './editor'
import { Keys } from './controls'
import { Icon, KindIcon } from './icons'
import type { Fill, Node } from './model'

type Item = { group: string; title: string; keys?: string; hint?: string; icon?: ReactNode; run: () => void }

/** How well `query` matches `text`: 3 at its start or a word's, 2 inside, 1 as letters in order, 0 not; and the matched letters. */
function match(query: string, text: string): [number, number[]] {
  const q = query.toLowerCase()
  const s = text.toLowerCase()
  const at = s.indexOf(q)
  if (at >= 0) return [at === 0 || s[at - 1] === ' ' ? 3 : 2, [...q].map((_, k) => at + k)]
  const hits: number[] = []
  for (let k = 0; k < s.length && hits.length < q.length; k++) if (s[k] === q[hits.length]) hits.push(k)
  return [hits.length === q.length ? 1 : 0, hits]
}

const flat = (nodes: Node[]): Node[] => nodes.flatMap((n) => [n, ...('children' in n ? flat(n.children) : [])])

function items(editor: Editor): Item[] {
  const { pages, swatches, textStyles } = editor.snapshot
  return [
    ...ACTIONS.map((a) => ({ group: 'Commands', title: a.title, keys: keysOf(a), run: () => (a.run ? a.run(editor) : press(a.keys)) })),
    ...pages.flatMap((p, i) =>
      flat(p.children).map((n) => ({
        group: 'Layers',
        title: n.name,
        hint: `Page ${i + 1}`,
        icon: <KindIcon node={n} />,
        run: () => {
          editor.showPage(p.id)
          editor.set({ selection: [n.id], overview: null })
        },
      })),
    ),
    ...pages.map((p, i) => ({
      group: 'Pages',
      title: `Page ${i + 1}`,
      hint: editor.masterOf(p)?.name,
      icon: <Icon name="doc" />,
      run: () => {
        editor.showPage(p.id)
        editor.set({ overview: null })
      },
    })),
    ...swatches.map((s) => ({
      group: 'Swatches',
      title: `Fill with ${s.name}`,
      hint: ink(s.color),
      icon: <Chip color={s.color} scope={NO_SCOPE} />,
      run: () => fillWith(editor, s.id) || editor.say('Select a layer first'),
    })),
    ...textStyles.map((s) => ({
      group: 'Text styles',
      title: `Style ${s.name}`,
      hint: `${s.size}/${s.lineHeight || 'auto'}`,
      icon: <Icon name="text" />,
      run: () => {
        const texts = editor.selected().filter((n) => n.kind === 'text')
        if (!texts.length) return editor.say('Select a text first')
        editor.batch(() => {
          for (const n of texts) editor.apply({ type: 'format', id: n.id, range: null, textStyle: s.id })
        })
      },
    })),
  ]
}

/** Fills the selection with the swatch `id`; false when nothing is selected. */
function fillWith(editor: Editor, id: string) {
  const nodes = editor.selected()
  if (!nodes.length) return false
  const fill: Fill = { type: 'solid', color: { swatch: id, tint: 1, alpha: 1 }, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true }
  editor.batch(() => {
    for (const n of nodes) editor.apply({ type: 'set', id: n.id, fills: [fill] })
  })
  return true
}

/** Ctrl K: finds and runs commands, layers, pages, swatches and text styles. */
export function Palette({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  useEffect(() => {
    ref.current!.querySelector('[aria-selected=true]')?.scrollIntoView({ block: 'nearest' })
  }, [at])

  const all = items(editor)
  const ranked = all
    .map((x) => ({ ...x, m: match(query, x.title) }))
    .filter((x) => x.m[0])
    .sort((a, b) => b.m[0] - a.m[0])
  const groups = [...new Set(ranked.map((x) => x.group))]
  const shown = query ? ranked.sort((a, b) => groups.indexOf(a.group) - groups.indexOf(b.group)).slice(0, 40) : ranked.filter((x) => x.group === 'Commands').slice(0, 9)
  const run = (x?: Item) => {
    if (!x) return
    ref.current!.close()
    x.run()
  }

  return (
    <dialog
      ref={ref}
      className="palette"
      aria-label="Command palette"
      onClose={onClose}
      onClick={(e) => e.target === e.currentTarget && ref.current!.close()}
    >
      <div className="pal-top">
        <Icon name="search" />
        <input
          className="pal-in"
          aria-label="Search"
          placeholder="Search commands, layers, pages, swatches, styles"
          spellCheck={false}
          autoFocus
          value={query}
          onChange={(e) => {
            setQuery(e.currentTarget.value)
            setAt(0)
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown' || e.key === 'ArrowUp') setAt(Math.max(0, Math.min(shown.length - 1, at + (e.key === 'ArrowDown' ? 1 : -1))))
            else if (e.key === 'Enter') run(shown[at])
            else return
            e.preventDefault()
            e.stopPropagation()
          }}
        />
        <kbd>Esc</kbd>
      </div>
      <div className="pal-list" role="listbox" aria-label="Results">
        {shown.map((x, i) => (
          <div key={`${x.group}${i}`} role="presentation">
            {x.group !== shown[i - 1]?.group && <div className="pal-g">{x.group}</div>}
            <button type="button" tabIndex={-1} className="pal-it" role="option" aria-selected={i === at} onClick={() => run(x)}>
              {x.icon}
              <span className="pal-t">{[...x.title].map((c, k) => (x.m[1].includes(k) ? <b key={k}>{c}</b> : c))}</span>
              {x.keys ? <Keys keys={x.keys} /> : x.hint && <small>{x.hint}</small>}
            </button>
          </div>
        ))}
        {!shown.length && <p className="pal-empty">Nothing matches. Try a layer name or a command.</p>}
      </div>
    </dialog>
  )
}
