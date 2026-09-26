import { useRef, useState } from 'react'
import { useEditor, type Editor, type Shape, type Tool } from './editor'
import { Icon, type IconName } from './icons'
import { Popover } from './Popover'

type Entry = { tool: Tool; label: string; key: string; icon: IconName }

const SHAPES: (Entry & { tool: Shape })[] = [
  { tool: 'rect', label: 'Rectangle', key: 'R', icon: 'rect' },
  { tool: 'line', label: 'Line', key: 'L', icon: 'line' },
  { tool: 'arrow', label: 'Arrow', key: 'Shift+L', icon: 'arrow' },
  { tool: 'ellipse', label: 'Ellipse', key: 'O', icon: 'ellipse' },
  { tool: 'polygon', label: 'Polygon', key: '', icon: 'polygon' },
  { tool: 'star', label: 'Star', key: '', icon: 'star' },
]
const MOVE: Entry = { tool: 'move', label: 'Move', key: 'V', icon: 'move' }
const FRAME: Entry = { tool: 'frame', label: 'Frame', key: 'F', icon: 'frame' }
const PEN: Entry = { tool: 'pen', label: 'Pen', key: 'P', icon: 'pen' }
const TEXT: Entry = { tool: 'text', label: 'Text', key: 'T', icon: 'text' }

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
          <Icon name="chevron" size={12} />
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
                <Icon name={icon} size={16} />
                <span>{label}</span>
                <kbd>{key}</kbd>
              </button>
            ))}
          </Popover>
        )}
      </div>
      <ToolButton entry={PEN} active={active} editor={editor} />
      <ToolButton entry={TEXT} active={active} editor={editor} />
      <button type="button" className="tool" aria-label="Place image" title="Place image (Ctrl+Shift+K)" onClick={onPlaceImage}>
        <Icon name="image" />
      </button>
    </div>
  )
}
