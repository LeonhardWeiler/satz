import { useEditor, type Editor, type Tool } from './editor'
import { Icon } from './icons'

const TOOLS: { tool: Tool; label: string; key: string }[] = [
  { tool: 'move', label: 'Move', key: 'V' },
  { tool: 'frame', label: 'Frame', key: 'F' },
  { tool: 'rect', label: 'Rectangle', key: 'R' },
  { tool: 'text', label: 'Text', key: 'T' },
]

export function Toolbar({ editor }: { editor: Editor }) {
  const active = useEditor(editor, (e) => e.tool)
  return (
    <div className="toolbar" role="toolbar" aria-label="Tools">
      {TOOLS.map(({ tool, label, key }) => (
        <button
          key={tool}
          type="button"
          className="tool"
          aria-pressed={active === tool}
          aria-label={label}
          title={`${label} (${key})`}
          onClick={() => editor.set({ tool })}
        >
          <Icon name={tool} />
        </button>
      ))}
    </div>
  )
}
