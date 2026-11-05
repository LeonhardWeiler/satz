import { useEffect, useRef, useState, type ReactNode } from 'react'
import { Engine } from './engine/engine'
import type { Editor } from './editor'
import { ask, begin, findGrantedFonts, open, openProject, projects, removeProject, show, type Project } from './file'
import { Icon } from './icons'
import booklet from '../../examples/booklet.satz?url'
import poster from '../../examples/poster.satz?url'

const EXAMPLES = [
  ['Satz booklet', 'A5, 8 pages, master pages', booklet, 'booklet.satz'],
  ['Poster', 'A2, CMYK with a spot color', poster, 'poster.satz'],
  ['Sample', 'A5, 1 page with shapes and text', null, 'Untitled.satz'],
] as const

const ago = (at: number) => {
  const m = Math.round((Date.now() - at) / 60000)
  if (m < 1) return 'Edited just now'
  if (m < 60) return `Edited ${m} min ago`
  if (m < 1440) return `Edited ${Math.round(m / 60)} h ago`
  return `Edited ${new Date(at).toLocaleDateString()}`
}

/** The first page of `e` as a PNG about 240 px on its long side. */
const firstPage = (e: Engine) => {
  const p = JSON.parse(e.snapshot()).pages[0]
  return e.png(p.id, (240 * 72) / Math.max(p.width, p.height))
}

const Thumb = ({ bytes }: { bytes?: Uint8Array }) => (
  <span className="home-thumb">
    {bytes ? <img src={`data:image/png;base64,${btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(''))}`} alt="" /> : <Icon name="doc" />}
  </span>
)

export function Home({ editor, onNew, children }: { editor: Editor; onNew: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [list, setList] = useState<Project[]>([])
  const [thumbs, setThumbs] = useState<(Uint8Array | undefined)[]>([])
  const reload = () => projects().then(setList, () => setList([]))
  useEffect(() => {
    ref.current!.showModal()
    ref.current!.focus()
    reload()
    Promise.all(
      EXAMPLES.map(async ([, , url]) => {
        const e = url ? new Engine() : Engine.sample()
        try {
          if (url) e.load(new Uint8Array(await (await fetch(url)).arrayBuffer()))
          return firstPage(e)
        } catch {
          return undefined
        } finally {
          e.free()
        }
      }),
    ).then(setThumbs)
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
      tabIndex={-1}
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
          Open or import…
        </button>
      </aside>
      <section className="home-main">
        <h2>{list.length ? 'Recent' : 'Start from an example'}</h2>
        {list.length > 0 && (
          <ul className="home-grid" aria-label="Recent">
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
        )}
        {list.length > 0 && <h3>Examples</h3>}
        <ul className="home-grid" aria-label="Examples">
          {EXAMPLES.map(([title, text, url, name], i) => (
            <li key={title} className="home-card">
              <button type="button" onClick={() => example(url, name)}>
                <Thumb bytes={thumbs[i]} />
                <strong>{title}</strong>
                <small>{text}</small>
              </button>
            </li>
          ))}
        </ul>
      </section>
      {children}
    </dialog>
  )
}
