import { useEffect, useRef, useState, type ReactNode } from 'react'
import { ACTIONS, fuzzy, keyLabel, press } from './commands'
import { Chip, ink, NO_SCOPE } from './ColorPicker'
import type { Editor } from './editor'
import { Icon, KindIcon } from './icons'
import type { Fill, Node } from './model'

type Item = { group: string; title: string; keys: string; icon: ReactNode; run: () => void }

const flat = (nodes: Node[]): Node[] => nodes.flatMap((n) => [n, ...('children' in n ? flat(n.children) : [])])

function items(editor: Editor): Item[] {
  const { pages, swatches, textStyles } = editor.snapshot
  const cmd = <Icon name="cmd" />
  return [
    ...ACTIONS.map((a) => ({ group: 'Commands', title: a.title, keys: a.keys, icon: cmd, run: () => (a.run ? a.run(editor) : press(a.keys)) })),
    ...pages.flatMap((p, i) =>
      flat(p.children).map((n) => ({
        group: 'Layers',
        title: n.name,
        keys: `Page ${i + 1}`,
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
      keys: editor.masterOf(p)?.name ?? '',
      icon: <Icon name="doc" />,
      run: () => {
        editor.showPage(p.id)
        editor.set({ overview: null })
      },
    })),
    ...swatches.map((s) => ({
      group: 'Swatches',
      title: `Fill with ${s.name}`,
      keys: ink(s.color),
      icon: <Chip color={s.color} scope={NO_SCOPE} />,
      run: () => fillWith(editor, s.id) || editor.say('Select a layer first'),
    })),
    ...textStyles.map((s) => ({
      group: 'Text styles',
      title: `Style ${s.name}`,
      keys: `${s.size}/${s.lineHeight || 'auto'}`,
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
  const shown = query
    ? all
        .map((x) => ({ x, s: fuzzy(query, x.title) }))
        .filter((m) => m.s)
        .sort((a, b) => b.s - a.s)
        .map((m) => m.x)
        .slice(0, 40)
    : all.filter((x) => x.group === 'Commands').slice(0, 9)
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
              <span>{x.title}</span>
              {x.keys && <kbd>{keyLabel(x.keys)}</kbd>}
            </button>
          </div>
        ))}
        {!shown.length && <p className="pal-empty">Nothing matches. Try a layer name or a command.</p>}
      </div>
    </dialog>
  )
}
