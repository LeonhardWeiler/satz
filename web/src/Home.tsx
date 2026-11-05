import { useEffect, useRef, useState, type ReactNode } from 'react'
import { Engine } from './engine/engine'
import type { Editor } from './editor'
import { ask, begin, findGrantedFonts, open, openProject, projects, removeProject, show, type Project } from './file'
import { Icon } from './icons'
import booklet from '../../examples/booklet.satz?url'
import poster from '../../examples/poster.satz?url'

const EXAMPLES = [
  ['Satz booklet', 'A5, 8 pages, master pages', booklet, 'booklet.satz'],
  ['Poster', 'A2, 1 page, CMYK with a spot color', poster, 'poster.satz'],
  ['Sample', 'A5, 1 page with shapes and text', null, 'Untitled.satz'],
] as const

const ago = (at: number) => {
  const m = Math.round((Date.now() - at) / 60000)
  if (m < 1) return 'Edited just now'
  if (m < 60) return `Edited ${m} min ago`
  if (m < 1440) return `Edited ${Math.round(m / 60)} h ago`
  return `Edited ${new Date(at).toLocaleDateString()}`
}

const Thumb = ({ bytes }: { bytes?: Uint8Array }) => (
  <span className="home-thumb">
    {bytes ? <img src={`data:image/png;base64,${btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(''))}`} alt="" /> : <Icon name="doc" />}
  </span>
)

export function Home({ editor, onNew, children }: { editor: Editor; onNew: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [list, setList] = useState<Project[]>([])
  const reload = () => projects().then(setList, () => setList([]))
  useEffect(() => {
    ref.current!.showModal()
    reload()
  }, [])

  const example = async (url: string | null, name: string) => {
    try {
      if (url) {
        const r = await fetch(url)
        if (!r.ok) throw new Error(`${r.status} ${r.statusText}`)
        await begin(editor, new Uint8Array(await r.arrayBuffer()), { name, handle: null })
        findGrantedFonts(editor)
      } else {
        const e = Engine.sample()
        await begin(editor, e.save())
        e.free()
      }
    } catch (e) {
      editor.fail(`Could not open ${name}: ${(e as Error).message}. Create a new document instead.`)
    }
  }
  const remove = async (p: Project) => {
    if (!(await ask(`Delete ${p.name} from this browser?`, 'Delete'))) return
    await removeProject(editor, p.id)
    reload()
  }

  return (
    <dialog
      ref={ref}
      className="home"
      aria-label="Projects"
      onCancel={(e) => {
        e.preventDefault()
        if (editor.project) show(editor, editor.project)
      }}
    >
      <aside className="home-side">
        <div className="brand">
          <span className="mark" aria-hidden="true" />
          <h1>Satz</h1>
        </div>
        <button type="button" className="primary" onClick={onNew}>
          <Icon name="plus" />
          New document
        </button>
        <button type="button" className="button" onClick={() => open(editor)}>
          <Icon name="folder" />
          Open file…
        </button>
        <h3>Examples</h3>
        {EXAMPLES.map(([title, text, url, name]) => (
          <button key={title} type="button" className="example" onClick={() => example(url, name)}>
            <span>
              <Icon name="doc" />
              {title}
            </span>
            <small>{text}</small>
          </button>
        ))}
      </aside>
      <section className="home-main">
        <h2>Recent</h2>
        {list.length === 0 && <p className="home-empty">Documents you create or open appear here.</p>}
        <ul className="home-grid">
          {list.map((p) => (
            <li key={p.id} className="home-card">
              <button type="button" onClick={() => openProject(editor, p.id)}>
                <Thumb bytes={p.thumb} />
                <strong>{p.name.replace(/\.satz$/, '')}</strong>
                <small>{ago(p.at)}</small>
              </button>
              <button type="button" className="icon-button" aria-label={`Delete ${p.name}`} title="Delete" onClick={() => remove(p)}>
                <Icon name="close" />
              </button>
            </li>
          ))}
        </ul>
      </section>
      {children}
    </dialog>
  )
}
