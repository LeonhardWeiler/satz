import { useRef, useState } from 'react'
import { useEditor, type Editor, type Shape, type Tool } from './editor'
import { Icon, type IconName } from './icons'
import { smooth } from './vector'
import { Popover } from './Popover'

type Entry = { tool: Tool; label: string; key: string; icon: IconName }

const SHAPES: (Entry & { tool: Shape })[] = [
  { tool: 'rect', label: 'Rectangle', key: 'R', icon: 'rect' },
  { tool: 'line', label: 'Line', key: 'L', icon: 'line' },
  { tool: 'ellipse', label: 'Ellipse', key: 'O', icon: 'ellipse' },
  { tool: 'polygon', label: 'Polygon', key: '', icon: 'polygon' },
  { tool: 'star', label: 'Star', key: '', icon: 'star' },
]
const POINTS = [
  { mode: 'move', label: 'Move point', key: 'V', icon: 'move' },
  { mode: 'add', label: 'Add point', key: 'P', icon: 'addPoint' },
  { mode: 'delete', label: 'Delete point', key: '-', icon: 'deletePoint' },
  { mode: 'fill', label: 'Fill area', key: 'B', icon: 'bucket' },
] as const
const MOVE: Entry = { tool: 'move', label: 'Move', key: 'V', icon: 'move' }
const FRAME: Entry = { tool: 'frame', label: 'Frame', key: 'F', icon: 'frame' }
const PEN: Entry = { tool: 'pen', label: 'Pen', key: 'P', icon: 'pen' }
const TEXT: Entry = { tool: 'text', label: 'Text', key: 'T', icon: 'text' }
const EYEDROPPER: Entry = { tool: 'eyedropper', label: 'Eyedropper', key: 'I', icon: 'eyedropper' }

function ToolButton({ entry, active, editor }: { entry: Entry; active: Tool; editor: Editor }) {
  const { tool, label, key, icon } = entry
  return (
    <button
      type="button"
      className="tool"
      aria-pressed={active === tool}
      aria-label={label}
      title={key ? `${label} (${key})` : label}
      onClick={() => editor.setTool(tool)}
    >
      <Icon name={icon} />
    </button>
  )
}

export function Toolbar({ editor, onPlaceImage }: { editor: Editor; onPlaceImage: () => void }) {
  const active = useEditor(editor, (e) => e.tool)
  const [last, setLast] = useState<Shape>('rect')
  const [open, setOpen] = useState(false)
  const group = useRef<HTMLDivElement>(null)
  const current = SHAPES.find((s) => s.tool === active)
  if (current && current.tool !== last) setLast(current.tool)
  const shape = current ?? SHAPES.find((s) => s.tool === last)!
  const vector = useEditor(editor, (e) => e.vector)
  const placing = useEditor(editor, (e) => e.placing.length > 0)

  if (vector) {
    return (
      <div className="toolbar" role="toolbar" aria-label="Path tools">
        {POINTS.map(({ mode, label, key, icon }) => (
          <button
            key={mode}
            type="button"
            className="tool"
            aria-pressed={vector.mode === mode}
            aria-label={label}
            title={`${label} (${key})`}
            onClick={() => editor.set({ vector: { ...vector, mode } })}
          >
            <Icon name={icon} />
          </button>
        ))}
        {([['Smooth point', 'smooth', true], ['Corner point', 'corner', false]] as const).map(([label, icon, on]) => (
          <button
            key={icon}
            type="button"
            className="tool"
            aria-label={label}
            title={`${label} (Ctrl click a point)`}
            disabled={!vector.at.length}
            onClick={() => editor.setKnots(vector.at.reduce((cs, at) => smooth(cs, at, on), editor.knots()), vector.at)}
          >
            <Icon name={icon} />
          </button>
        ))}
        <button type="button" className="button" title="Done (Enter)" onClick={() => editor.set({ vector: null })}>
          Done
        </button>
      </div>
    )
  }

  return (
    <div className="toolbar" role="toolbar" aria-label="Tools">
      <ToolButton entry={MOVE} active={active} editor={editor} />
      <ToolButton entry={FRAME} active={active} editor={editor} />
      <div
        ref={group}
        className="tool-group"
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget)) setOpen(false)
        }}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && open) {
            e.stopPropagation()
            setOpen(false)
          }
        }}
      >
        <ToolButton entry={shape} active={active} editor={editor} />
        <button
          type="button"
          className="tool-more"
          aria-label="Shape tools"
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={() => setOpen((o) => !o)}
        >
          <Icon name="chevron" />
        </button>
        {open && (
          <Popover anchor={() => group.current!.getBoundingClientRect()} side="top" className="menu" role="menu" aria-label="Shape tools">
            {SHAPES.map(({ tool, label, key, icon }) => (
              <button
                key={tool}
                type="button"
                role="menuitemradio"
                aria-checked={active === tool}
                className="menu-item"
                onClick={() => {
                  editor.setTool(tool)
                  setOpen(false)
                }}
              >
                <Icon name={icon} />
                <span>{label}</span>
                <kbd>{key}</kbd>
              </button>
            ))}
          </Popover>
        )}
      </div>
      <ToolButton entry={PEN} active={active} editor={editor} />
      <ToolButton entry={TEXT} active={active} editor={editor} />
      <ToolButton entry={EYEDROPPER} active={active} editor={editor} />
      <button type="button" className="tool" aria-pressed={placing} aria-label="Place image" title="Place image (Ctrl+Shift+K)" onClick={onPlaceImage}>
        <Icon name="image" />
      </button>
    </div>
  )
}
