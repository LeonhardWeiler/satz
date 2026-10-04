import { neutral, solid, type ColorMode } from './color'
import { Check, Field, NameInput, Section, Segmented, Select } from './controls'
import { type ReactNode } from 'react'
import { bounds, ends, MM, scopeOf, useEditor, type Editor } from './editor'
import { Icon, KindIcon, type IconName } from './icons'
import { FORMATS, ORIENTATIONS } from './Start'
import { addFonts, canFindFonts, findFonts, pickProfile, readLocalFonts, removeFont, useLocalFonts } from './file'
import type { Bindable as Prop, Blend, Constraint, Command, Grid, LineStyle, Node, Page, Props, Section as Numbers, Wrap } from './model'
import { isOpen, radiusOf } from './model'
import { AlignBar, BooleanBar, combinable } from './align'
import { ACTIONS, keyLabel, keysOf, press } from './commands'
import { AutoLayout, flows, Sizing } from './AutoLayout'
import { EffectList, PaintList } from './Paints'
import { TextFrameSection, TextSection, TextStyles } from './Text'
import { Bindable, ModeSelects, Variables } from './Variables'

const ARRANGE: [string, IconName][] = [
  ['Flatten', 'flatten'],
  ['Flip horizontal', 'flipHorizontal'],
  ['Flip vertical', 'flipVertical'],
  ['Add auto layout', 'autoLayout'],
]
const WRAPS: Record<Wrap, string> = { none: 'No text wrap', around: 'Wrap around', jump: 'Jump over' }
const BLENDS: Record<Blend, string> = {
  normal: 'Normal', multiply: 'Multiply', screen: 'Screen', overlay: 'Overlay', darken: 'Darken',
  lighten: 'Lighten', colorDodge: 'Color dodge', colorBurn: 'Color burn', hardLight: 'Hard light',
  softLight: 'Soft light', difference: 'Difference', exclusion: 'Exclusion', hue: 'Hue',
  saturation: 'Saturation', color: 'Color', luminosity: 'Luminosity',
}
const LINE_STYLES: Record<LineStyle, string> = { solid: 'Solid', dashed: 'Dashed', dotted: 'Dotted', wavy: 'Wavy', zigzag: 'Zigzag' }

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
const STARTS = [
  ['none', 'No start arrow', 'plainLine'],
  ['arrow', 'Start arrow', 'arrowLeft'],
] as const
const ENDS = [
  ['none', 'No end arrow', 'plainLine'],
  ['arrow', 'End arrow', 'arrowRight'],
] as const
const MODES: Record<ColorMode, string> = { rgb: 'RGB', cmyk: 'CMYK' }
const NUMBERINGS = { arabic: '1, 2, 3', upperRoman: 'I, II, III', lowerRoman: 'i, ii, iii' }
const SPREADS = { single: 'Single pages', facing: 'Facing pages' }
const HORIZONTAL: Record<Constraint, string> = { min: 'Left', max: 'Right', stretch: 'Left & right', center: 'Center', scale: 'Scale' }
const VERTICAL: Record<Constraint, string> = { min: 'Top', max: 'Bottom', stretch: 'Top & bottom', center: 'Center', scale: 'Scale' }
export function Properties({
  editor,
  say,
}: {
  editor: Editor
  say: (message: string) => void
}) {
  const page = useEditor(editor, (e) => e.page)
  const isPage = useEditor(editor, (e) => e.snapshot.pages.includes(e.page))
  const mode = useEditor(editor, (e) => e.snapshot.colorMode)
  const overview = useEditor(editor, (e) => e.overview)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const { fonts, missingFonts } = snapshot
  const locals = useLocalFonts()
  const selection = useEditor(editor, (e) => e.selection)
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
  const frame = (key: 'x' | 'y' | 'w' | 'h') => (v: number) =>
    each((n) => {
      const by = n.keepRatio && (key === 'w' || key === 'h') && n[key] ? v / n[key] : 1
      return { type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w * by, h: n.h * by, [key]: v }
    })

  const one = nodes.length === 1 ? nodes[0] : undefined
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
  const open = nodes.length > 0 && nodes.every(isOpen)
  const stroked = nodes.length > 0 && nodes.every((n) => n.kind !== 'group')
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

  return (
    <aside
      className="panel properties"
      aria-label="Properties"
      onPointerDown={editor.gesture}
    >
      <header className="panel-header">
        {one ? <KindIcon node={one} /> : <Icon name={nodes.length ? 'group' : isPage ? 'doc' : 'master'} />}
        <h2>{one ? one.name : nodes.length ? `${nodes.length} layers` : isPage ? 'Page' : page.name}</h2>
      </header>
      {box && <AlignBar editor={editor} />}
      {box && (
        <div role="toolbar" aria-label="Arrange" className="align">
          {ARRANGE.map(([title, icon]) => {
            const a = ACTIONS.find((a) => a.title === title)!
            return (
              <button
                key={title}
                type="button"
                className="icon-button"
                aria-label={title}
                title={`${title} (${keyLabel(keysOf(a))})`}
                disabled={!(a.can?.(editor) ?? true)}
                onClick={() => press(a.keys)}
              >
                <Icon name={icon} />
              </button>
            )
          })}
        </div>
      )}
      {combinable(editor) && <BooleanBar editor={editor} />}
      {!box && <DocumentSection editor={editor} say={say} />}
      {!box && (
        <Section
          title={isPage ? (targets.length > 1 ? `Pages ${numbers.join(', ')}` : `Page ${numbers[0]}`) : 'Master'}
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
          <FormatRow page sheets={sheets} each={setSheets} />
          <div className="grid">
            {(['width', 'height', 'bleed'] as const).map((k) => (
              <Field
                key={k}
                label={k === 'bleed' ? 'Bleed' : k === 'width' ? 'W' : 'H'}
                value={sameOf(sheets, (p) => p[k])}
                unit="length"
                onCommit={(v) => setSheets(() => ({ [k]: v }))}
              />
            ))}
            {isPage && (
              <Select
                label="Master"
                value={master}
                options={Object.fromEntries([['', 'No master'], ...snapshot.masters.map((m) => [m.id, m.name])])}
                onChange={(m) => editor.batch(() => targets.forEach((id) => editor.apply({ type: 'useMaster', page: id, master: m || null })))}
              />
            )}
            <ModeSelects editor={editor} id={page.id} own={page.modes} inherited={{}} />
          </div>
          {isPage && targets.length === 1 && (
            <label className="check">
              <input
                type="checkbox"
                checked={page.section !== null}
                onChange={(e) => editor.apply({ type: 'setSection', id: page.id, section: e.currentTarget.checked ? { start: numbers[0], style: 'arabic', prefix: '' } : null })}
              />
              Start section
            </label>
          )}
          {isPage && targets.length === 1 && page.section && (
            <div className="grid">
              <Field label="Start" title="Section starts at" unit="" int min={1} value={page.section.start} onCommit={(start) => setSection({ start })} />
              <Select label="Numbering" value={page.section.style} options={NUMBERINGS} onChange={(style) => setSection({ style })} />
              <label className="field">
                <span className="field-label">Prefix</span>
                <NameInput label="Prefix" value={page.section.prefix} blank onCommit={(prefix) => setSection({ prefix })} />
              </label>
            </div>
          )}
        </Section>
      )}
      {!box && <GridSection editor={editor} sheets={sheets} all={isPage && targets.length === 1 ? snapshot.pages : []} />}
      {!box && (
        <Section title="Fonts" onAdd={() => addFonts(editor, say)}>
          <ul className="fonts">
            {fonts.map((f, i) => (
              <li key={f.hash}>
                <span>{f.name}</span>
                {i > 0 && (
                  <button type="button" className="icon-button" aria-label={`Remove ${f.name}`} title="Remove font" onClick={() => removeFont(editor, f.hash).catch(() => {})}>
                    <Icon name="minus" />
                  </button>
                )}
              </li>
            ))}
            {missingFonts.map(({ font }) => (
              <li key={font.hash} className="missing" title="Missing: its text is set in Source Serif 4">
                {font.name}
              </li>
            ))}
          </ul>
          {canFindFonts && !locals.length && (
            <button type="button" className="button" onClick={() => readLocalFonts(say)}>
              List the fonts of this computer
            </button>
          )}
          {canFindFonts && missingFonts.length > 0 && (
            <button type="button" className="button" onClick={() => findFonts(editor, say)}>
              Find missing fonts on this computer
            </button>
          )}
        </Section>
      )}
      {!box && (
        <Variables editor={editor} />
      )}
      {!box && <TextStyles editor={editor} />}
      {box && (
        <Section
          title="Layout"
          actions={
            <>
              {one?.overrideOf && (
                <button type="button" className="icon-button" aria-label="Reset to master" title="Reset to master" onClick={() => editor.apply({ type: 'resetToMaster', ids: [one.id] })}>
                  <Icon name="reset" />
                </button>
              )}
              {!line && (
                <button
                  type="button"
                  className="icon-button"
                  aria-label="Keep aspect ratio"
                  title="Keep aspect ratio"
                  aria-pressed={same((n) => n.keepRatio) ?? 'mixed'}
                  onClick={() => set({ keepRatio: !same((n) => n.keepRatio) })}
                >
                  <Icon name="ratio" />
                </button>
              )}
            </>
          }
        >
          <div className="grid">
            <Field label="X" min={-Infinity} value={nodes.length > 1 ? same((n) => n.x) : box.x} unit="length" onCommit={frame('x')} />
            <Field label="Y" min={-Infinity} value={nodes.length > 1 ? same((n) => n.y) : box.y} unit="length" onCommit={frame('y')} />
            {line ? (
              <>
                <Field label="L" title="Length" value={length!} unit="length" onCommit={(v) => setLine(v, angle!)} />
                <Field label="∠" title="Angle" min={-Infinity} value={angle!} unit="°" onCommit={(v) => setLine(length!, v)} />
              </>
            ) : (
              <>
                {bindable('w', 'W', 'W', <Field label="W" value={same((n) => n.w)} unit="length" onCommit={frame('w')} />)}
                {bindable('h', 'H', 'H', <Field label="H" value={same((n) => n.h)} unit="length" onCommit={frame('h')} />)}
              </>
            )}
            {!line && nodes.every((n) => n.kind !== 'group') && (
              <Field label="∠" title="Rotation" unit="°" min={-Infinity} reset={0} value={same((n) => n.rotation)} onCommit={(v) => set({ rotation: ((v % 360) + 360) % 360 })} />
            )}
            {one?.kind === 'frame' && (
              <label className="check">
                <input type="checkbox" checked={one.clip} onChange={(e) => set({ clip: e.currentTarget.checked })} />
                Clip content
              </label>
            )}
            {nodes.length > 0 && nodes.every((n) => n.kind === 'shape' && n.shape === 'rect') && (
              bindable(
                'radius',
                'Corner radius',
                'R',
                <Field
                  label="R"
                  title="Corner radius"
                  unit="length"
                  reset={0}
                  value={same((n) => ('radius' in n ? (radiusOf(n) ?? undefined) : 0))}
                  onCommit={(radius) => set({ radius, corners: [] })}
                />,
              )
            )}
            {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
              <Field label="N" title="Count" unit="" int min={3} max={60} value={one.count} onCommit={(count) => set({ count })} />
            )}
            {one?.kind === 'shape' && one.shape === 'ellipse' && (
              <>
                <Field label="Start" title="Arc start" unit="°" min={-Infinity} reset={0} value={one.start} onCommit={(v) => set({ start: ((v % 360) + 360) % 360 })} />
                <Field label="Sweep" title="Arc sweep" unit="%" max={100} reset={100} value={one.sweep * 100} onCommit={(v) => set({ sweep: v / 100 })} />
                <Field label="Ratio" title="Inner radius" unit="%" max={100} reset={0} value={one.inner * 100} onCommit={(v) => set({ inner: v / 100 })} />
              </>
            )}
            {one?.kind === 'shape' && one.shape === 'star' && (
              <Field label="Ratio" title="Star ratio" unit="%" min={1} max={100} value={one.ratio * 100} onCommit={(v) => set({ ratio: v / 100 })} />
            )}
          </div>
          {one && <Sizing editor={editor} node={one} set={set} />}
          {one && parent?.kind === 'frame' && parent.direction !== 'none' && (
            <label className="check">
              <input type="checkbox" checked={one.absolute} onChange={(e) => set({ absolute: e.currentTarget.checked })} />
              Absolute position
            </label>
          )}
          {one && parent?.kind === 'frame' && !flows(editor, one) && (
            <div className="grid">
              {(['horizontal', 'vertical'] as const).map((axis) => (
                <Select
                  key={axis}
                  label={`${axis === 'horizontal' ? 'Horizontal' : 'Vertical'} constraint`}
                  value={one.constraints[axis]}
                  options={axis === 'horizontal' ? HORIZONTAL : VERTICAL}
                  onChange={(k) => set({ constraints: { ...one.constraints, [axis]: k } })}
                />
              ))}
            </div>
          )}
        </Section>
      )}
      {one?.kind === 'frame' && <AutoLayout editor={editor} node={one} set={set} />}
      {box && (
        <Section title="Layer">
          <div className="grid">
            {bindable(
              'opacity',
              'Opacity',
              '',
              <Field label="" title="Opacity" unit="%" max={100} value={same((n) => n.opacity * 100)} onCommit={(v) => set({ opacity: v / 100 })} />,
            )}
            <Select label="Blend mode" value={same((n) => n.blend)} options={BLENDS} onChange={(blend) => set({ blend })} />
            <Select label="Text wrap" value={same((n) => n.wrap)} options={WRAPS} onChange={(wrap) => set({ wrap })} />
            {nodes.some((n) => n.wrap !== 'none') && (
              <Field label="" title="Text wrap offset" unit="length" value={same((n) => n.wrapOffset)} onCommit={(v) => set({ wrapOffset: Math.max(0, v) })} />
            )}
            {one?.kind === 'frame' && (
              <ModeSelects editor={editor} id={one.id} own={one.modes} inherited={editor.nodes.get(one.id)?.parent?.activeModes ?? page.modes} />
            )}
          </div>
          {one ? (
            <label className="check">
              <input type="checkbox" checked={one.mask} onChange={(e) => set({ mask: e.currentTarget.checked })} />
              Use as mask
            </label>
          ) : (
          <button
            type="button"
            className="button"
            title="Use as mask (Ctrl+Alt+M)"
            onClick={() => editor.set({ selection: editor.apply({ type: 'mask', ids: selection }) })}
          >
            Use as mask
          </button>
          )}
        </Section>
      )}
      {nodes.length > 0 && nodes.every((n) => n.kind !== 'group') && (
        <PaintList
          title="Fill"
          paints={sameList((n) => n.fills)}
          ppi={one?.ppi}
          images={snapshot.images}
          added={solid(neutral(nodes.every((n) => n.kind === 'text') ? 'black' : 'gray', mode))}
          mode={mode}
          scope={scope}
          onChange={(fills) => set({ fills })}>
          {sameList((n) => n.fills)?.length !== 0 && (
            <Check label="Overprint fill" value={same((n) => n.overprintFill)} set={(overprintFill) => set({ overprintFill })} />
          )}
        </PaintList>
      )}
      {stroked && (
        <PaintList title="Stroke" paints={strokes} added={solid(neutral('black', mode))} mode={mode} scope={scope} onChange={(strokes) => set({ strokes })}>
          {strokes?.length !== 0 && (
            <div className="grid">
              {bindable(
                'strokeWeight',
                'Stroke weight',
                '',
                <Field label="" title="Stroke weight" unit="pt" value={same((n) => n.strokeWeight)} onCommit={(v) => set({ strokeWeight: v })} />,
              )}
              {nodes.every((n) => n.kind !== 'text') && (
                <Select label="Line style" value={same((n) => n.lineStyle)} options={LINE_STYLES} onChange={(lineStyle) => set({ lineStyle })} />
              )}
              {!nodes.some(isOpen) && <Segmented label="Stroke position" value={same((n) => n.strokeAlign)} options={STROKE_ALIGNS} onChange={(strokeAlign) => set({ strokeAlign })} />}
              {open && (
                <>
                  <Segmented label="Stroke cap" value={same((n) => n.cap)} options={CAPS} onChange={(cap) => set({ cap })} />
                  <Segmented label="Start point" value={same((n) => (n.arrowStart ? 'arrow' : 'none'))} options={STARTS} onChange={(v) => set({ arrowStart: v === 'arrow' })} />
                  <Segmented label="End point" value={same((n) => (n.arrowEnd ? 'arrow' : 'none'))} options={ENDS} onChange={(v) => set({ arrowEnd: v === 'arrow' })} />
                </>
              )}
              <Segmented label="Stroke join" value={same((n) => n.join)} options={JOINS} onChange={(join) => set({ join })} />
              <Check label="Overprint stroke" value={same((n) => n.overprintStroke)} set={(overprintStroke) => set({ overprintStroke })} />
            </div>
          )}
        </PaintList>
      )}
      {box && <EffectList effects={sameList((n) => n.effects)} mode={mode} scope={scope} onChange={(effects) => set({ effects })} />}
      {one?.kind === 'text' && <TextSection editor={editor} node={one} />}
      {one?.kind === 'text' && <TextFrameSection editor={editor} node={one} set={set} />}
    </aside>
  )
}

const sameOf = <T, U>(items: T[], get: (t: T) => U): U | null => {
  const values = items.map(get)
  return values.every((v) => v === values[0]) ? values[0] : null
}
const near = (a: number, b: number) => Math.abs(a - b) < 0.5

/** Format, orientation, size, bleed and number of all pages, facing pages, colour mode and raster resolution. */
function DocumentSection({ editor, say }: { editor: Editor; say: (message: string) => void }) {
  const { pages, masters, facingPages, colorMode, rasterPpi, profile } = useEditor(editor, (e) => e.snapshot)
  const sheets = [...pages, ...masters]
  const w = sameOf(pages, (p) => p.width)
  const h = sameOf(pages, (p) => p.height)
  const each = (props: PageProps) => editor.batch(() => sheets.forEach((p) => editor.apply({ type: 'setPage', id: p.id, ...props(p) })))
  const count = (n: number) =>
    editor.batch(() => {
      for (let i = pages.length; i < n; i++) editor.apply({ type: 'addPage', after: editor.snapshot.pages.at(-1)!.id })
      for (const p of pages.slice(Math.max(1, n)).reverse()) editor.apply({ type: 'deletePage', id: p.id })
    })
  return (
    <Section title="Document">
      <FormatRow sheets={pages} each={each} />
      <div className="grid">
        <Field label="W" title="Width of all pages" unit="length" value={w} onCommit={(width) => each(() => ({ width }))} />
        <Field label="H" title="Height of all pages" unit="length" value={h} onCommit={(height) => each(() => ({ height }))} />
        <Field label="Bleed" title="Bleed of all pages" unit="length" value={sameOf(pages, (p) => p.bleed)} onCommit={(bleed) => each(() => ({ bleed }))} />
        <Field label="Pages" title="Pages" unit="" int min={1} value={pages.length} onCommit={count} />
        <Select label="Spreads" value={facingPages ? 'facing' : 'single'} options={SPREADS} onChange={(v) => editor.apply({ type: 'setDocument', facingPages: v === 'facing' })} />
        <Field label="Raster" reset={300} value={rasterPpi} unit="ppi" min={72} max={1200} onCommit={(v) => editor.apply({ type: 'setDocument', rasterPpi: v })} />
        <Select label="Color mode" value={colorMode} options={MODES} onChange={(colorMode) => editor.apply({ type: 'setDocument', colorMode })} />
        {colorMode === 'cmyk' ? (
          <Select<string>
            label="Profile"
            value={profile === null ? 'fogra' : 'own'}
            options={{ fogra: 'FOGRA51', ...(profile !== null && { own: profile || 'Uploaded' }), upload: 'Upload ICC…' }}
            onChange={(v) => (v === 'upload' ? pickProfile(editor, say) : v === 'fogra' && editor.setProfile(null))}
          />
        ) : (
          <div className="kv" title="Output profile">
            <span>Profile</span>
            <strong>sRGB</strong>
          </div>
        )}
      </div>
    </Section>
  )
}

const GRIDS: Record<Grid['kind'], string> = { columns: 'Columns', rows: 'Rows', grid: 'Grid' }

/** The layout grids of `sheets`, and a button that gives them to `all` when they differ. */
function GridSection({ editor, sheets, all }: { editor: Editor; sheets: Page[]; all: Page[] }) {
  const key = (p: Page) => JSON.stringify(p.grids)
  const grids = sameOf(sheets, key) === null ? null : sheets[0].grids
  const setGrids = (grids: Grid[], to = sheets) => editor.batch(() => to.forEach((p) => editor.apply({ type: 'setGrids', id: p.id, grids })))
  const fresh: Grid = { kind: 'columns', count: 2, gutter: 5 * MM, margin: 15 * MM, size: 5 * MM }
  return (
    <Section
      title="Layout grids"
      onAdd={() => setGrids([...(grids ?? []), fresh])}
      actions={
        grids &&
        all.some((p) => key(p) !== key(sheets[0])) && (
          <button type="button" className="icon-button" aria-label="Apply grids to all pages" title="Apply grids to all pages" onClick={() => setGrids(grids, all)}>
            <Icon name="pages" />
          </button>
        )
      }
    >
      {!grids && <p className="empty">Click + to replace mixed grids</p>}
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
                    <Field label="Size" title="Cell size" unit="length" value={g.size} onCommit={(size) => size > 0 && set({ size })} />
                  ) : (
                    <>
                      <Field label="N" title={GRIDS[g.kind]} unit="" int min={1} value={g.count} onCommit={(count) => set({ count })} />
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

type PageProps = (p: Page) => Partial<Pick<Page, 'width' | 'height' | 'bleed'>> & { scale?: boolean }

/** A format and orientation picker for `sheets`. */
function FormatRow({ page, sheets, each }: { page?: boolean; sheets: Page[]; each: (props: PageProps) => void }) {
  const w = sameOf(sheets, (p) => p.width)
  const h = sameOf(sheets, (p) => p.height)
  const landscape = w !== null && h !== null ? w > h : null
  const format =
    w !== null && h !== null ? (FORMATS.find(([, a, b]) => near(Math.min(w, h), a * MM) && near(Math.max(w, h), b * MM))?.[0] ?? 'Custom') : 'Custom'
  const orient = (wide: boolean) => each((p) => (p.width > p.height === wide ? {} : { width: p.height, height: p.width }))
  return (
      <div className="grid">
        <Select
          label={page ? 'Page format' : 'Format'}
          value={format}
          options={Object.fromEntries([...FORMATS.map(([n]) => [n, n]), ['Custom', 'Custom']])}
          disabled={['Custom']}
          onChange={(n) => {
            const [, a, b] = FORMATS.find(([f]) => f === n)!
            const [width, height] = landscape ? [b, a] : [a, b]
            each(() => ({ width: width * MM, height: height * MM, scale: true }))
          }}
        />
        <Segmented
          label={page ? 'Page orientation' : 'Orientation'}
          value={landscape === null ? null : landscape ? 'landscape' : 'portrait'}
          options={ORIENTATIONS}
          onChange={(o) => orient(o === 'landscape')}
        />
      </div>
  )
}
