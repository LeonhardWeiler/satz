import { Fragment, useEffect, useRef, useState, type ReactNode } from 'react'
import { able, ACTIONS, ONLY_WHEN_ABLE } from './commands'
import { keysOf } from './keys'
import { Chip, ink, NO_SCOPE } from './ColorPicker'
import type { Editor } from './editor'
import { Keys } from './controls'
import { Icon, KindIcon } from './icons'
import { sizeOf } from './Text'
import { pageName, type Fill, type Node } from './model'

type Item = { id: string; group: string; title: string; keys?: string; hint?: string; icon?: ReactNode; off?: boolean; run: () => void }

function match(query: string, text: string): [number, number[]] {
  const q = query.toLowerCase()
  const s = text.toLowerCase()
  const at = s.indexOf(q)
  if (at >= 0) return [at === 0 || s[at - 1] === ' ' ? 3 : 2, [...q].map((_, k) => at + k)]
  const hits: number[] = []
  for (let k = 0; k < s.length && hits.length < q.length; k++) if (s[k] === q[hits.length]) hits.push(k)
  return [hits.length === q.length && q.length > 2 ? 1 : 0, hits]
}

function runs(text: string, hits: number[]) {
  const out: [number, string, boolean][] = []
  ;[...text].forEach((c, k) => {
    const hit = hits.includes(k)
    const last = out.at(-1)
    if (last?.[2] === hit) last[1] += c
    else out.push([k, c, hit])
  })
  return out
}

const flat = (nodes: Node[]): Node[] => nodes.flatMap((n) => [n, ...('children' in n ? flat(n.children) : [])])

function items(editor: Editor): Item[] {
  const { pages, swatches, textStyles, colorMode } = editor.snapshot
  return [
    ...ACTIONS.filter((a) => !ONLY_WHEN_ABLE.has(a.title) || able(editor, a)).map((a) => ({ id: a.title, group: 'Commands', title: a.title, keys: keysOf(a), off: !able(editor, a), run: () => a.run(editor) })),
    ...pages.flatMap((p) =>
      flat(p.children).map((n) => ({
        id: n.id,
        group: 'Layers',
        title: n.name,
        hint: pageName(editor.snapshot, p.id),
        icon: <KindIcon node={n} />,
        run: () => {
          editor.showPage(p.id)
          editor.set({ selection: [n.id], overview: null })
        },
      })),
    ),
    ...pages.map((p) => ({
      id: p.id,
      group: 'Pages',
      title: pageName(editor.snapshot, p.id)!,
      hint: editor.masterOf(p)?.name,
      icon: <Icon name="doc" />,
      run: () => {
        editor.showPage(p.id)
        editor.set({ overview: null })
      },
    })),
    ...swatches.map((s) => ({
      id: s.id,
      group: 'Swatches',
      title: `Fill with ${s.name}`,
      hint: ink(s.color, colorMode),
      icon: <Chip color={s.color} scope={NO_SCOPE} />,
      run: () => fillWith(editor, s.id) || editor.fail('Select a layer first'),
    })),
    ...textStyles.map((s) => ({
      id: s.id,
      group: 'Text styles',
      title: `Style ${s.name}`,
      hint: sizeOf(s),
      icon: <Icon name="text" />,
      run: () => {
        const texts = editor.selected().filter((n) => n.kind === 'text')
        if (!texts.length) return editor.fail('Select a text first')
        editor.batch(() => {
          for (const n of texts) editor.apply({ type: 'format', id: n.id, range: null, textStyle: s.id })
        })
      },
    })),
  ]
}

function fillWith(editor: Editor, id: string) {
  const nodes = editor.selected()
  if (!nodes.length) return false
  const fill: Fill = { type: 'solid', color: { swatch: id, tint: 1, alpha: 1 }, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true }
  editor.batch(() => {
    for (const n of nodes) editor.apply({ type: 'set', id: n.id, fills: [fill] })
  })
  return true
}

export function Palette({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  useEffect(() => {
    ref.current!.showModal()
  }, [])

  const all = items(editor)
  const ranked = all
    .map((x) => ({ ...x, m: match(query, x.title) }))
    .filter((x) => x.m[0])
    .sort((a, b) => Number(!!a.off) - Number(!!b.off) || b.m[0] - a.m[0])
  const groups = [...new Set(ranked.map((x) => x.group))]
  const shown = query ? ranked.sort((a, b) => groups.indexOf(a.group) - groups.indexOf(b.group)).slice(0, 40) : ranked.filter((x) => x.group === 'Commands').slice(0, 9)
  const run = (x?: Item) => {
    if (!x || x.off) return
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
            ref.current!.querySelector('.pal-list')!.scrollTop = 0
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
              const next = Math.max(0, Math.min(shown.length - 1, at + (e.key === 'ArrowDown' ? 1 : -1)))
              setAt(next)
              ref.current!.querySelectorAll('[role=option]')[next]?.scrollIntoView({ block: 'nearest' })
            } else if (e.key === 'Enter') run(shown[at])
            else return
            e.preventDefault()
            e.stopPropagation()
          }}
        />
        <Keys keys="Esc" />
      </div>
      <div className="pal-list" role="listbox" aria-label="Results">
        {shown.map((x, i) => (
          <div key={`${x.group} ${x.id}`} role="presentation">
            {x.group !== shown[i - 1]?.group && <div className="pal-g">{x.group}</div>}
            <button type="button" tabIndex={-1} className="pal-it" role="option" aria-selected={i === at} disabled={x.off} onClick={() => run(x)}>
              {x.icon}
              <span className="pal-t">{runs(x.title, x.m[1]).map(([k, t, hit]) => (hit ? <b key={k}>{t}</b> : <Fragment key={k}>{t}</Fragment>))}</span>
              {x.keys ? <Keys keys={x.keys} /> : x.hint && <small>{x.hint}</small>}
            </button>
          </div>
        ))}
        {!shown.length && <p className="pal-empty">Nothing matches. Try a layer name or a command.</p>}
      </div>
    </dialog>
  )
}
