import { useRef, useState } from 'react'
import { useEditor, type Editor, type Shape, type Tool } from './editor'
import { Icon, type IconName } from './icons'
import { smooth } from './vector'
import { Popover } from './Popover'
import { Keys } from './controls'
import { keysFor } from './keys'

type Entry = { tool: Tool; label: string; action?: string; icon: IconName }

const SHAPES: (Entry & { tool: Shape; action: string })[] = [
  { tool: 'rect', label: 'Rectangle', action: 'Rectangle tool', icon: 'rect' },
  { tool: 'line', label: 'Line', action: 'Line tool', icon: 'line' },
  { tool: 'ellipse', label: 'Ellipse', action: 'Ellipse tool', icon: 'ellipse' },
  { tool: 'polygon', label: 'Polygon', action: 'Polygon tool', icon: 'polygon' },
  { tool: 'star', label: 'Star', action: 'Star tool', icon: 'star' },
]
const POINTS = [
  { mode: 'move', label: 'Move point', key: 'V', icon: 'move' },
  { mode: 'add', label: 'Add point', key: 'P', icon: 'addPoint' },
  { mode: 'delete', label: 'Delete point', key: '-', icon: 'deletePoint' },
  { mode: 'fill', label: 'Fill area', key: 'B', icon: 'bucket' },
] as const
const MOVE: Entry = { tool: 'move', label: 'Move', action: 'Move tool', icon: 'move' }
const FRAME: Entry = { tool: 'frame', label: 'Frame', action: 'Frame tool', icon: 'frame' }
const PEN: Entry = { tool: 'pen', label: 'Pen', action: 'Pen tool', icon: 'pen' }
const TEXT: Entry = { tool: 'text', label: 'Text', action: 'Text tool', icon: 'text' }
const TABLE: Entry = { tool: 'table', label: 'Table', icon: 'table' }
const EYEDROPPER: Entry = { tool: 'eyedropper', label: 'Eyedropper', action: 'Eyedropper', icon: 'eyedropper' }

function ToolButton({ entry, active, editor }: { entry: Entry; active: Tool; editor: Editor }) {
  const { tool, label, action, icon } = entry
  return (
    <button
      type="button"
      className="tool"
      aria-pressed={active === tool}
      aria-label={label}
      title={label}
      data-action={action}
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
        <button type="button" className="button" onClick={() => editor.set({ vector: null })}>
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
          <Popover anchor={() => group.current!.getBoundingClientRect()} side="top" className="menu shapes" role="menu" aria-label="Shape tools">
            {SHAPES.map(({ tool, label, action, icon }) => (
              <button
                key={tool}
                type="button"
                role="menuitemradio"
                aria-checked={shape.tool === tool}
                className="menu-item"
                onClick={() => {
                  editor.setTool(tool)
                  setOpen(false)
                }}
              >
                <span>{shape.tool === tool && <Icon name="check" />}</span>
                <Icon name={icon} />
                <span>{label}</span>
                <Keys keys={keysFor(action)} />
              </button>
            ))}
          </Popover>
        )}
      </div>
      <ToolButton entry={PEN} active={active} editor={editor} />
      <ToolButton entry={TEXT} active={active} editor={editor} />
      <ToolButton entry={TABLE} active={active} editor={editor} />
      <ToolButton entry={EYEDROPPER} active={active} editor={editor} />
      <button type="button" className="tool" aria-pressed={placing} aria-label="Place image" title="Place image" data-action="Place image" onClick={onPlaceImage}>
        <Icon name="image" />
      </button>
    </div>
  )
}
