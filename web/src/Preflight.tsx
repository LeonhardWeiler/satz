import { useEditor, type Editor } from './editor'
import type { Issue } from './model'

const problem = (i: Issue) =>
  i.problem === 'missingFont' ? `Missing font ${i.font}`
  : i.problem === 'overset' ? 'Overset text'
  : i.problem === 'shortOfBleed' ? 'Short of the bleed'
  : i.problem === 'lowPpi' ? `Image at ${Math.round(i.ppi)} ppi`
  : 'RGB color'

export function Preflight({ editor }: { editor: Editor }) {
  const issues = useEditor(editor, (e) => e.snapshot.preflight)
  const selection = useEditor(editor, (e) => e.selection)

  const show = (i: Issue) => {
    editor.showPage(i.page)
    editor.set({ selection: [i.layer] })
  }

  return (
    <section className="panel preflight" aria-label="Preflight">
      <header className="panel-header">
        <h2>Preflight</h2>
      </header>
      {issues.length === 0 ? (
        <p className="preflight-none">No issues</p>
      ) : (
        <ul className="preflight-list">
          {issues.map((i) => (
            <li key={`${i.layer} ${problem(i)}`}>
              <button type="button" aria-current={selection.includes(i.layer)} onClick={() => show(i)}>
                <span>{i.name}</span>
                <span className="preflight-problem">{problem(i)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
