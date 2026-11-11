import { neutral, solid, type ColorMode } from './color'
import { Check, Field, Group, More, NameInput, Section, Segmented, Select, SIDE_NAMES, SidesToggle } from './controls'
import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { bounds, ends, MM, scopeOf, useEditor, type Editor } from './editor'
import { Icon, KindIcon } from './icons'
import { FORMATS, ORIENTATIONS, formatOf } from './Start'
import { pickProfile } from './file'
import type { Bindable as Prop, Blend, Constraint, Command, DateFormat, Grid, LineStyle, Node, Page, Props, Section as Numbers, TextNode, Wrap } from './model'
import { isOpen, radiusOf, rounded } from './model'
import { AlignBar, rowOf, setGap } from './align'
import { AutoLayout, flows, Sizing } from './AutoLayout'
import { EffectList, PaintList } from './Paints'
import { sameOf, TextFrameSection, TextSection } from './Text'
import { Bindable, ModeSelects } from './Variables'

/** The rotation of `n`, for a group that of all its layers, or null if they differ. */
const turnOf = (n: Node): number | null => {
  if (n.kind !== 'group') return n.rotation
  const turns = n.children.map(turnOf)
  return turns.every((t) => t === turns[0]) ? turns[0] : null
}
const WRAPS: Record<Wrap, string> = { none: 'No text wrap', around: 'Wrap around', jump: 'Jump over' }
const BLENDS: Record<Blend, string> = {
  normal: 'Normal', darken: 'Darken', multiply: 'Multiply', colorBurn: 'Color burn',
  lighten: 'Lighten', screen: 'Screen', colorDodge: 'Color dodge',
  overlay: 'Overlay', softLight: 'Soft light', hardLight: 'Hard light',
  difference: 'Difference', exclusion: 'Exclusion',
  hue: 'Hue', saturation: 'Saturation', color: 'Color', luminosity: 'Luminosity',
}
const BLEND_GROUPS: Blend[] = ['normal', 'colorBurn', 'colorDodge', 'hardLight', 'exclusion']
const DASHES: Record<LineStyle, string> = { solid: 'M1 4h22', dashed: 'M1 4h5M10 4h5M19 4h4', dotted: 'M2 4h.01M7 4h.01M12 4h.01M17 4h.01M22 4h.01', wavy: 'M1 4q2.75-3 5.5 0t5.5 0 5.5 0 5.5 0', zigzag: 'M1 5.5l2.75-3 2.75 3 2.75-3 2.75 3 2.75-3 2.75 3 2.75-3 2.75 3' }
const LINE_STYLES = Object.fromEntries(
  Object.entries(DASHES).map(([k, d]) => [
    k,
    <>
      <svg className="dash" viewBox="0 0 24 8" aria-hidden="true">
        <path d={d} />
      </svg>
      <span className="dash-name">{k[0].toUpperCase() + k.slice(1)}</span>
    </>,
  ]),
) as Record<LineStyle, ReactNode>
const end = (d: string, name: string) => (
  <>
    <svg className="dash" viewBox="0 0 24 8" aria-hidden="true">
      <path d={d} />
    </svg>
    {name}
  </>
)
export const ARROWS = {
  start: { none: end('M1 4h22', 'None'), arrow: end('M23 4H2M5 1 2 4l3 3', 'Arrow') },
  end: { none: end('M1 4h22', 'None'), arrow: end('M1 4h22M19 1l3 3-3 3', 'Arrow') },
}

const STROKE_ALIGNS = [
  ['inside', 'Inside', 'strokeInside'],
  ['center', 'Center', 'strokeCenter'],
  ['outside', 'Outside', 'strokeOutside'],
] as const
const JOINS = [
  ['miter', 'Miter join', 'joinMiter'],
  ['round', 'Round join', 'joinRound'],
  ['bevel', 'Bevel join', 'joinBevel'],
] as const
const CAPS = [
  ['none', 'No cap', 'capNone'],
  ['round', 'Round cap', 'capRound'],
  ['square', 'Square cap', 'capSquare'],
] as const
const MODES: Record<ColorMode, string> = { rgb: 'RGB', cmyk: 'CMYK' }
const DATES: Record<DateFormat, string> = { english: 'November 9, 2026', german: '9. November 2026', us: '11/09/2026', dotted: '09.11.2026', iso: '2026-11-09' }
const NUMBERINGS = { arabic: '1, 2, 3', upperRoman: 'I, II, III', lowerRoman: 'i, ii, iii' }
const HORIZONTAL: Record<Constraint, string> = { min: 'Left', max: 'Right', stretch: 'Left & right', center: 'Center', scale: 'Scale' }
const VERTICAL: Record<Constraint, string> = { min: 'Top', max: 'Bottom', stretch: 'Top & bottom', center: 'Center', scale: 'Scale' }
export function Properties({ editor }: { editor: Editor }) {
  const page = useEditor(editor, (e) => e.page)
  const isPage = useEditor(editor, (e) => e.snapshot.pages.includes(e.page))
  const mode = useEditor(editor, (e) => e.snapshot.colorMode)
  const overview = useEditor(editor, (e) => e.overview)
  const cropping = useEditor(editor, (e) => e.cropping)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const selection = useEditor(editor, (e) => e.selection)
  const [layerMore, setLayerMore] = useState(false)
  const nodes = selection.flatMap((id) => editor.nodes.get(id)?.node ?? [])

  const same = <T,>(get: (n: Node) => T | undefined) => {
    const values = nodes.map(get)
    return values.every((v) => v === values[0]) ? (values[0] ?? null) : null
  }
  const sameList = <T,>(get: (n: Node) => T[]) => {
    const values = nodes.map((n) => JSON.stringify(get(n)))
    return values.every((v) => v === values[0]) ? get(nodes[0]) : null
  }
  const each = (cmd: (n: Node) => Command | undefined) => {
    editor.batch(() => {
      for (const n of nodes) {
        const c = cmd(n)
        if (c) editor.apply(c)
      }
    })
  }
  const frame = (key: 'x' | 'y' | 'w' | 'h') => (v: number) => {
    const b = bounds(nodes)
    const by = nodes.every((n) => n.keepRatio) && (key === 'w' || key === 'h') && b[key] ? v / b[key] : 1
    const t = { x: b.x, y: b.y, w: b.w * by, h: b.h * by, [key]: v }
    const sx = b.w ? t.w / b.w : 1
    const sy = b.h ? t.h / b.h : 1
    each((n) => ({ type: 'setFrame', id: n.id, x: t.x + (n.x - b.x) * sx, y: t.y + (n.y - b.y) * sy, w: b.w ? n.w * sx : t.w, h: b.h ? n.h * sy : t.h }))
  }

  const one = nodes.length === 1 ? nodes[0] : undefined
  const row = rowOf(editor)
  const ids = snapshot.pages.map((p) => p.id)
  const picked = overview?.filter((id) => ids.includes(id)) ?? []
  const targets = isPage && picked.length ? picked : [page.id]
  const numbers = targets.map((id) => ids.indexOf(id) + 1)
  const setSection = (s: Partial<Numbers>) => page.section && editor.apply({ type: 'setSection', id: page.id, section: { ...page.section, ...s } })
  const sheets = targets.flatMap((id) => (id === page.id ? page : (snapshot.pages.find((p) => p.id === id) ?? [])))
  const master = sameOf(sheets, (p) => p.master ?? '')
  const setSheets = (props: PageProps) => editor.batch(() => sheets.forEach((p) => editor.apply({ type: 'setPage', id: p.id, ...props(p) })))
  const scope = scopeOf(snapshot, one?.activeModes)
  const parent = one && editor.nodes.get(one.id)?.parent
  const box = nodes.length ? bounds(nodes) : undefined
  const bindable = (prop: Prop, title: string, label: string, field: ReactNode) =>
    one ? (
      <Bindable editor={editor} id={one.id} prop={prop} title={title} label={label}>
        {field}
      </Bindable>
    ) : (
      field
    )
  const set = (props: Props) => each((n) => ({ type: 'set', id: n.id, ...props }))
  const wrapped = nodes.some((n) => n.wrap !== 'none')
  const layerUsed = wrapped || nodes.some((n) => n.blend !== 'normal')
  const open = nodes.length > 0 && nodes.every(isOpen)
  const stroked = nodes.length > 0 && nodes.every((n) => n.kind !== 'group')
  const sided = nodes.length > 0 && nodes.every(rounded)
  const strokes = stroked ? sameList((n) => n.strokes) : null
  const line = one && ends(one)
  const length = line && Math.hypot(line[1].x - line[0].x, line[1].y - line[0].y)
  const angle = line && (Math.atan2(line[0].y - line[1].y, line[1].x - line[0].x) * 180) / Math.PI
  const setLine = (length: number, degrees: number) => {
    if (!one || !line) return
    const [a] = line
    const r = (degrees * Math.PI) / 180
    editor.apply({ type: 'setPath', id: one.id, path: [0, a.x, a.y, 1, a.x + length * Math.cos(r), a.y - length * Math.sin(r)] })
  }

  const texts = nodes.length > 0 && nodes.every((n) => n.kind === 'text')
  const fill = nodes.length > 0 && nodes.every((n) => n.kind !== 'group') && (
    <PaintList
      editor={editor}
      title={texts ? 'Text color' : 'Fill'}
      paints={sameList((n) => n.fills)}
      ppi={one?.ppi}
      image={one?.id}
      images={snapshot.images}
      added={solid(neutral(texts ? 'black' : 'gray', mode))}
      mode={mode}
      scope={scope}
      onChange={(fills) => set({ fills })}>
      {mode === 'cmyk' && !!sameList((n) => n.fills)?.length && (
        <Check label="Overprint fill" value={same((n) => n.overprintFill)} set={(overprintFill) => set({ overprintFill })} />
      )}
    </PaintList>
  )

  return (
    <aside
      className="panel properties"
      aria-label="Properties"
      onPointerDown={editor.gesture}
    >
      <header className="panel-header">
        {one ? <KindIcon node={one} /> : <Icon name={nodes.length ? 'group' : isPage ? 'doc' : 'master'} />}
        <h2>{one && cropping === one.id ? 'Crop' : one ? (one.kind === 'text' ? 'Text frame' : one.name) : nodes.length ? `${nodes.length} layers` : isPage ? 'Document' : page.name}</h2>
        {one && cropping === one.id && <button type="button" className="button" onClick={() => editor.set({ cropping: null })}>Done</button>}
        {one?.overrideOf && (
          <>
            <span className="override-note">Overrides master</span>
            <button type="button" className="button" aria-label="Reset to master" onClick={() => editor.apply({ type: 'resetToMaster', ids: [one.id] })}>
              Reset
            </button>
          </>
        )}
      </header>
      {nodes.length > 1 && (
        <AlignBar editor={editor}>
          {row && (
            <Field
              label="Spacing"
              title={row.axis === 'x' ? 'Horizontal spacing' : 'Vertical spacing'}
              unit="length"
              value={row.gaps.every((g) => Math.abs(g - row.gaps[0]) < 1e-6) ? row.gaps[0] : null}
              onCommit={(v) => setGap(editor, row, v)}
            />
          )}
        </AlignBar>
      )}
      {one?.kind === 'group' && one.children.length > 1 && <AlignBar editor={editor} inside={one.children} />}
      {!box && <DocumentSection editor={editor} />}
      {!box && (
        <Section
          title={isPage ? (targets.length > 1 ? `Pages ${numbers.toSorted((a, b) => a - b).join(', ')}` : `Page ${numbers[0]}`) : 'Master'}
          actions={
            <>
              {isPage && page.detached.length > 0 && (
                <button
                  type="button"
                  className="icon-button"
                  aria-label="Reset overrides"
                  title="Reset overrides"
                  onClick={() => editor.apply({ type: 'resetToMaster', ids: [page.id] })}
                >
                  <Icon name="reset" />
                </button>
              )}
              {isPage &&
                targets.length === 1 &&
                snapshot.pages.some((p) => p.width !== page.width || p.height !== page.height || p.bleed !== page.bleed) && (
                  <button
                    type="button"
                    className="icon-button"
                    aria-label="Apply size to all pages"
                    title="Apply size to all pages"
                    onClick={() => editor.batch(() => snapshot.pages.forEach((p) => editor.apply({ type: 'setPage', id: p.id, width: page.width, height: page.height, bleed: page.bleed })))}
                  >
                    <Icon name="pages" />
                  </button>
                )}
            </>
          }
        >
          <FormatRow sheets={sheets} each={setSheets} />
          <div className="grid">
            {(['width', 'height', 'bleed'] as const).map((k) => (
              <Field
                key={k}
                label={k === 'bleed' ? 'Bleed' : k === 'width' ? 'W' : 'H'}
                value={sameOf(sheets, (p) => p[k])}
                min={k === 'bleed' ? 0 : 1}
                max={k === 'bleed' ? 100 * MM : 14400}
                unit="length"
                onCommit={(v) => setSheets(() => ({ [k]: v }))}
              />
            ))}
            {isPage && (
              <Select
                label="Master"
                prefix="Master"
                value={master}
                options={Object.fromEntries([['', 'None'], ...snapshot.masters.map((m) => [m.id, m.name])])}
                onChange={(m) => editor.batch(() => targets.forEach((id) => editor.apply({ type: 'useMaster', page: id, master: m || null })))}
              />
            )}
            <ModeSelects editor={editor} id={page.id} own={page.modes} inherited={{}} />
          </div>
          {isPage && targets.length === 1 && (
            <More label="Section" used={page.section !== null}>
              <label className="check">
                <input
                  type="checkbox"
                  checked={page.section !== null}
                  onChange={(e) => editor.apply({ type: 'setSection', id: page.id, section: e.currentTarget.checked ? { start: numbers[0], style: 'arabic', prefix: '' } : null })}
                />
                Start section
              </label>
              {page.section && (
                <div className="grid">
                  <Field label="Start" title="Section starts at" unit="" int min={1} max={page.section.style === 'arabic' ? 9999 : 3999} value={page.section.start} onCommit={(start) => setSection({ start })} />
                  <Select label="Numbering" value={page.section.style} options={NUMBERINGS} onChange={(style) => setSection({ style })} />
                  <label className="field">
                    <span className="field-label">Prefix</span>
                    <NameInput label="Prefix" value={page.section.prefix} blank onCommit={(prefix) => setSection({ prefix })} />
                  </label>
                </div>
              )}
            </More>
          )}
        </Section>
      )}
      {!box && <GridSection editor={editor} sheets={sheets} all={isPage && targets.length === 1 ? snapshot.pages : []} />}
      {box && (
        <Section title="Layout">
          <div className="grid">
            <Field label="X" min={-Infinity} value={box.x} unit="length" onCommit={frame('x')} />
            <Field label="Y" min={-Infinity} value={box.y} unit="length" onCommit={frame('y')} />
            {line ? (
              <>
                <Field label="L" title="Length" value={length!} unit="length" onCommit={(v) => setLine(v, angle!)} />
                <Field label={<Icon name="angle" />} title="Angle" min={-Infinity} value={angle!} unit="°" onCommit={(v) => setLine(length!, v)} />
              </>
            ) : (
              <>
                <div className="grid pair">
                  {bindable('w', 'W', 'W', <Field label="W" value={box.w} unit="length" onCommit={frame('w')} />)}
                  {bindable('h', 'H', 'H', <Field label="H" value={box.h} unit="length" onCommit={frame('h')} />)}
                  <button
                    type="button"
                    className="icon-button ratio"
                    aria-label="Keep aspect ratio"
                    title="Keep aspect ratio"
                    aria-pressed={same((n) => n.keepRatio) ?? 'mixed'}
                    onClick={() => set({ keepRatio: !same((n) => n.keepRatio) })}
                  >
                    <Icon name="ratio" />
                  </button>
                </div>
                <Field
                  label={<Icon name="angle" />}
                  title="Rotation"
                  unit="°"
                  min={-Infinity}
                  reset={0}
                  value={same(turnOf)}
                  onCommit={(v) =>
                    editor.batch(() => {
                      for (const n of nodes) {
                        if (n.kind !== 'group') editor.apply({ type: 'set', id: n.id, rotation: ((v % 360) + 360) % 360 })
                        else {
                          const b = editor.shown(n)
                          editor.turn([n], v - (turnOf(n) ?? 0), { x: b.x + b.w / 2, y: b.y + b.h / 2 })
                        }
                      }
                    })
                  }
                />
              </>
            )}
            {sided && (
              bindable(
                'radius',
                'Corner radius',
                'R',
                <Field
                  label="R"
                  title="Corner radius"
                  unit="length"
                  reset={0}
                  value={same((n) => (rounded(n) ? (radiusOf(n) ?? undefined) : 0))}
                  onCommit={(radius) => set({ radius, corners: [] })}
                />,
              )
            )}
          </div>
          {one?.kind === 'frame' && <Check label="Clip content" value={one.clip} set={(clip) => set({ clip })} />}
          {one && <Sizing editor={editor} node={one} set={set} />}
          {one && parent?.kind === 'frame' && parent.direction !== 'none' && (
            <label className="check">
              <input type="checkbox" checked={one.absolute} onChange={(e) => set({ absolute: e.currentTarget.checked })} />
              Absolute position
            </label>
          )}
          {one && parent?.kind === 'frame' && !flows(editor, one) && (
            <Group title="Constraints">
              {(['horizontal', 'vertical'] as const).map((axis) => (
                <Select
                  key={axis}
                  label={`${axis === 'horizontal' ? 'Horizontal' : 'Vertical'} constraint`}
                  prefix={axis === 'horizontal' ? '↔' : '↕'}
                  value={one.constraints[axis]}
                  options={axis === 'horizontal' ? HORIZONTAL : VERTICAL}
                  onChange={(k) => set({ constraints: { ...one.constraints, [axis]: k } })}
                />
              ))}
            </Group>
          )}
        </Section>
      )}
      {one?.kind === 'shape' && (one.shape === 'ellipse' || one.shape === 'polygon' || one.shape === 'star') && (
        <Section title="Shape">
          <div className="grid">
            {(one.shape === 'polygon' || one.shape === 'star') && (
              <Field label="Points" title="Points" unit="" int min={3} max={60} value={one.count} onCommit={(count) => set({ count })} />
            )}
            {one.shape === 'star' && (
              <Field label="Inner radius" title="Inner radius" unit="%" digits={1} min={1} max={100} value={one.ratio * 100} onCommit={(v) => set({ ratio: v / 100 })} />
            )}
            {one.shape === 'ellipse' && (
              <>
                <Field label="Start" title="Arc start" unit="°" min={-Infinity} reset={0} value={one.start} onCommit={(v) => set({ start: ((v % 360) + 360) % 360 })} />
                <Field label="Sweep" title="Arc sweep" unit="°" max={360} reset={360} value={one.sweep * 360} onCommit={(v) => set({ sweep: v / 360 })} />
                <Field label="Inner radius" title="Inner radius" unit="%" digits={1} max={100} reset={0} value={one.inner * 100} onCommit={(v) => set({ inner: v / 100 })} />
              </>
            )}
          </div>
        </Section>
      )}
      {texts && <TextSection editor={editor} nodes={nodes as TextNode[]} />}
      {texts && fill}
      {texts && <TextFrameSection editor={editor} nodes={nodes as TextNode[]} set={set} />}
      {one?.kind === 'frame' && <AutoLayout editor={editor} node={one} set={set} />}
      {box && (
        <Section
          title="Layer"
          actions={
            !layerUsed && (
              <button type="button" className="icon-button" aria-label="Layer options" title="Blend mode and text wrap" aria-expanded={layerMore} onClick={() => setLayerMore(!layerMore)}>
                <Icon name="more" />
              </button>
            )
          }
        >
          <div className="grid">
            {bindable(
              'opacity',
              'Opacity',
              'Opacity',
              <Field label="Opacity" title="Opacity" unit="%" digits={1} max={100} value={same((n) => n.opacity * 100)} onCommit={(v) => set({ opacity: v / 100 })} />,
            )}
            {one?.kind === 'frame' && (
              <ModeSelects editor={editor} id={one.id} own={one.modes} inherited={editor.nodes.get(one.id)?.parent?.activeModes ?? page.modes} />
            )}
          </div>
          {(layerMore || layerUsed) && (
            <>
              <div className="grid">
                <Select label="Blend mode" value={same((n) => n.blend)} options={BLENDS} breaks={BLEND_GROUPS} onChange={(blend) => set({ blend })} />
                {(wrapped || [...editor.nodes.values()].some((e) => e.page?.id === page.id && e.node.kind === 'text' && !selection.includes(e.node.id))) && (
                  <Select label="Text wrap" value={same((n) => n.wrap)} options={WRAPS} onChange={(wrap) => set({ wrap })} />
                )}
                {wrapped && (
                  <Field
                    label="Offset"
                    title="Text wrap offset"
                    unit="length"
                    value={same((n) => (n.wrapOffsetX === n.wrapOffsetY ? n.wrapOffsetX : undefined))}
                    onCommit={(v) => set({ wrapOffsetX: Math.max(0, v), wrapOffsetY: Math.max(0, v) })}
                  />
                )}
              </div>
              {nodes.some((n) => n.wrap !== 'none' && !textBelow(editor, n)) && <p className="empty">Only text frames below it in the layers wrap around it</p>}
            </>
          )}
        </Section>
      )}
      {!texts && fill}
      {stroked && (
        <PaintList
          editor={editor}
          title="Stroke"
          actions={
            sided &&
            !!strokes?.length && (
              <SidesToggle
                what="stroke"
                pressed={same((n) => !!n.strokeSides?.length)}
                onClick={() => set({ strokeSides: same((n) => !!n.strokeSides?.length) ? [] : Array(4).fill(one?.strokeWeight ?? 1) })}
              />
            )
          }
          paints={strokes} added={solid(neutral('black', mode))} mode={mode} scope={scope} onChange={(strokes) => set({ strokes })}>
          {!!strokes?.length && (
            <div className="grid">
              {bindable(
                'strokeWeight',
                'Stroke weight',
                'Weight',
                <Field label="Weight" title="Stroke weight" unit="pt" value={same((n) => n.strokeWeight)} onCommit={(v) => set({ strokeWeight: v })} />,
              )}
              {nodes.every((n) => n.kind !== 'text') ? (
                <Select label="Line style" value={same((n) => n.lineStyle)} options={LINE_STYLES} onChange={(lineStyle) => set({ lineStyle })} />
              ) : (
                <span />
              )}
              {open && (
                <>
                  <Select label="Start point" prefix="Start" value={same((n) => (n.arrowStart ? 'arrow' : 'none'))} options={ARROWS.start} onChange={(v) => set({ arrowStart: v === 'arrow' })} />
                  <Select label="End point" prefix="End" value={same((n) => (n.arrowEnd ? 'arrow' : 'none'))} options={ARROWS.end} onChange={(v) => set({ arrowEnd: v === 'arrow' })} />
                </>
              )}
              {sided && one?.strokeSides?.length === 4 &&
                SIDE_NAMES.map((side, i) => (
                  <Field
                    key={side}
                    label={side}
                    title={`${side} stroke`}
                    unit="pt"
                    value={one.strokeSides![i]}
                    onCommit={(v) => set({ strokeSides: one.strokeSides!.map((w, j) => (j === i ? v : w)) })}
                  />
                ))}
              {mode === 'cmyk' && <Check label="Overprint stroke" value={same((n) => n.overprintStroke)} set={(overprintStroke) => set({ overprintStroke })} />}
            </div>
          )}
          {!!strokes?.length && (
            <More label={open ? 'Cap & join' : 'Align & join'} used={nodes.some((n) => n.join !== 'miter' || (open && n.cap !== 'none'))}>
              <div className="grid">
                {open ? (
                  <Segmented label="Stroke cap" prefix="Cap" value={same((n) => n.cap)} options={CAPS} onChange={(cap) => set({ cap })} />
                ) : (
                  !nodes.some(isOpen) && <Segmented label="Stroke position" prefix="Align" value={same((n) => n.strokeAlign)} options={STROKE_ALIGNS} onChange={(strokeAlign) => set({ strokeAlign })} />
                )}
                <Segmented label="Stroke join" prefix="Join" value={same((n) => n.join)} options={JOINS} onChange={(join) => set({ join })} />
              </div>
            </More>
          )}
        </PaintList>
      )}
      {box && <EffectList effects={sameList((n) => n.effects)} mode={mode} scope={scope} onChange={(effects) => set({ effects })} />}
    </aside>
  )
}

function textBelow(editor: Editor, n: Node) {
  const page = editor.nodes.get(n.id)?.page
  for (const [id, e] of editor.nodes) {
    if (id === n.id) return false
    if (e.page === page && e.node.kind === 'text') return true
  }
  return false
}

/** Facing pages, color mode and, in CMYK, the profile of the document, and how dates show. */
function DocumentSection({ editor }: { editor: Editor }) {
  const { facingPages, colorMode, profile, dateFormat } = useEditor(editor, (e) => e.snapshot)
  const color = <Select label="Color mode" value={colorMode} options={MODES} onChange={(colorMode) => editor.apply({ type: 'setDocument', colorMode })} />
  return (
    <section className="section" aria-label="Document">
      <div className="section-body">
        <Check label="Facing pages" value={facingPages} set={(facingPages) => editor.apply({ type: 'setDocument', facingPages })} />
        {colorMode === 'rgb' ? (
          color
        ) : (
          <div className="grid">
            {color}
            <Select<string>
              label="Profile"
              value={profile === null ? 'fogra' : 'own'}
              options={{ fogra: 'FOGRA51', ...(profile !== null && { own: profile || 'Uploaded' }), upload: 'Upload ICC…' }}
              breaks={[profile === null ? 'fogra' : 'own']}
              onChange={(v) => (v === 'upload' ? pickProfile(editor) : v === 'fogra' && editor.setProfile(null))}
            />
          </div>
        )}
        <Select label="Date" value={dateFormat} options={DATES} onChange={(dateFormat) => editor.apply({ type: 'setDocument', dateFormat })} />
      </div>
    </section>
  )
}

const GRIDS: Record<Grid['kind'], string> = { columns: 'Columns', rows: 'Rows', grid: 'Grid' }

/** The layout grids of `sheets`, and a button that gives them to `all` when they differ. */
function GridSection({ editor, sheets, all }: { editor: Editor; sheets: Page[]; all: Page[] }) {
  const key = (p: Page) => JSON.stringify(p.grids)
  const grids = sameOf(sheets, key) === null ? null : sheets[0].grids
  const setGrids = (grids: Grid[], to = sheets) => editor.batch(() => to.forEach((p) => editor.apply({ type: 'setGrids', id: p.id, grids })))
  const fresh: Grid = { kind: 'columns', count: 2, gutter: 5 * MM, margin: 15 * MM, size: 5 * MM }
  const [presets, setPresets] = useState<DOMRect | null>(null)
  return (
    <Section
      title="Layout grids"
      onAdd={(e) => setPresets(e.currentTarget.getBoundingClientRect())}
      actions={
        <>
          {grids && all.some((p) => key(p) !== key(sheets[0])) && (
            <button type="button" className="icon-button" aria-label="Apply grids to all pages" title="Apply grids to all pages" onClick={() => setGrids(grids, all)}>
              <Icon name="pages" />
            </button>
          )}
          {presets &&
            createPortal(
              <ContextMenu
                anchor={() => presets}
                side="bottom"
                label="Add layout grid"
                onClose={() => setPresets(null)}
                items={GRID_PRESETS.map(([name, kinds]) => [name, () => setGrids([...(grids ?? []), ...kinds.map(([kind, count]) => ({ ...fresh, kind, count, gutter: (count > 6 ? 4 : 5) * MM }))]), true])}
              />,
              document.body,
            )}
        </>
      }
    >
      {!grids && <p className="empty" title="Adding one replaces them on all pages">Mixed</p>}
      {grids && grids.length > 0 && (
        <ul className="rows">
          {grids.map((g, i) => {
            const set = (next: Partial<Grid>) => setGrids(grids.map((h, j) => (j === i ? { ...g, ...next } : h)))
            return (
              <li key={i} className="paint">
                <div className="row">
                  <Select label="Grid type" value={g.kind} options={GRIDS} onChange={(kind) => set({ kind })} />
                  <button type="button" className="icon-button" aria-label="Remove grid" title="Remove grid" onClick={() => setGrids(grids.filter((_, j) => j !== i))}>
                    <Icon name="minus" />
                  </button>
                </div>
                <div className="grid">
                  {g.kind === 'grid' ? (
                    <Field label="Size" title="Cell size" unit="length" min={1} value={g.size} onCommit={(size) => set({ size })} />
                  ) : (
                    <>
                      <Field label={GRIDS[g.kind]} title={GRIDS[g.kind]} unit="" int min={1} max={100} value={g.count} onCommit={(count) => set({ count })} />
                      <Field label="Gutter" title="Gutter" unit="length" value={g.gutter} onCommit={(gutter) => set({ gutter })} />
                      <Field label="Margin" title="Margin" unit="length" value={g.margin} onCommit={(margin) => set({ margin: Math.max(0, margin) })} />
                    </>
                  )}
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </Section>
  )
}

const GRID_PRESETS: [string, [Grid['kind'], number][]][] = [
  ['1 column', [['columns', 1]]],
  ['2 columns', [['columns', 2]]],
  ['3 columns', [['columns', 3]]],
  ['4 columns', [['columns', 4]]],
  ['12 columns', [['columns', 12]]],
  ['3 × 3 modules', [['columns', 3], ['rows', 3]]],
  ['6 × 6 modules', [['columns', 6], ['rows', 6]]],
  ['5 mm grid', [['grid', 1]]],
]

type PageProps = (p: Page) => Partial<Pick<Page, 'width' | 'height' | 'bleed'>> & { scale?: boolean }

/** A format and orientation picker for `sheets`. */
function FormatRow({ sheets, each }: { sheets: Page[]; each: (props: PageProps) => void }) {
  const w = sameOf(sheets, (p) => p.width)
  const h = sameOf(sheets, (p) => p.height)
  const landscape = w !== null && h !== null ? w > h : null
  const format =
    w !== null && h !== null ? formatOf(w, h) : 'Custom'
  const orient = (wide: boolean) => each((p) => (p.width > p.height === wide ? {} : { width: p.height, height: p.width }))
  return (
      <div className="grid">
        <Select
          label="Format"
          value={format}
          options={Object.fromEntries([...FORMATS.map(([n]) => [n, n]), ...(format === 'Custom' ? [['Custom', 'Custom']] : [])])}
          onChange={(n) => {
            const [, a, b] = FORMATS.find(([f]) => f === n) ?? []
            if (!a || !b) return
            const [width, height] = landscape ? [b, a] : [a, b]
            each(() => ({ width: width * MM, height: height * MM, scale: true }))
          }}
        />
        <Segmented
          label="Orientation"
          value={landscape === null ? null : landscape ? 'landscape' : 'portrait'}
          options={ORIENTATIONS}
          onChange={(o) => orient(o === 'landscape')}
        />
      </div>
  )
}
