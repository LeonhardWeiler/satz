import { neutral, solid, type ColorMode } from './color'
import { Field, Section, Segmented, Select } from './controls'
import { type ReactNode } from 'react'
import { bounds, ends, MM, scopeOf, useEditor, type Editor } from './editor'
import { Icon, KindIcon } from './icons'
import { FORMATS, ORIENTATIONS } from './Start'
import { addFonts, canFindFonts, findFonts, removeFont } from './file'
import type { Bindable as Prop, Blend, Constraint, Command, Grid, Node, Page, Props, Size } from './model'
import { isOpen, radiusOf } from './model'
import { align, ALIGNS } from './align'
import { AutoLayout } from './AutoLayout'
import { EffectList, PaintList } from './Paints'
import { TextFrameSection, TextSection, TextStyles } from './Text'
import { Bindable, ModeSelects, Variables } from './Variables'

const BLENDS: Record<Blend, string> = {
  normal: 'Normal', multiply: 'Multiply', screen: 'Screen', overlay: 'Overlay', darken: 'Darken',
  lighten: 'Lighten', colorDodge: 'Color dodge', colorBurn: 'Color burn', hardLight: 'Hard light',
  softLight: 'Soft light', difference: 'Difference', exclusion: 'Exclusion', hue: 'Hue',
  saturation: 'Saturation', color: 'Color', luminosity: 'Luminosity',
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
const STARTS = [
  ['none', 'No start arrow', 'plainLine'],
  ['arrow', 'Start arrow', 'arrowLeft'],
] as const
const ENDS = [
  ['none', 'No end arrow', 'plainLine'],
  ['arrow', 'End arrow', 'arrowRight'],
] as const
const MODES: Record<ColorMode, string> = { rgb: 'RGB', cmyk: 'CMYK' }
const HORIZONTAL: Record<Constraint, string> = { min: 'Left', max: 'Right', stretch: 'Left & right', center: 'Center', scale: 'Scale' }
const SIZES: Record<Size, string> = { fixed: 'Fixed', hug: 'Hug', fill: 'Fill' }
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
    each((n) => ({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [key]: v }))

  const one = nodes.length === 1 ? nodes[0] : undefined
  const ids = snapshot.pages.map((p) => p.id)
  const picked = overview?.filter((id) => ids.includes(id)) ?? []
  const targets = isPage && picked.length ? picked : [page.id]
  const numbers = targets.map((id) => ids.indexOf(id) + 1)
  const sheets = targets.flatMap((id) => (id === page.id ? page : (snapshot.pages.find((p) => p.id === id) ?? [])))
  const master = sameOf(sheets, (p) => p.master ?? '')
  const setSheets = (props: PageProps) => editor.batch(() => sheets.forEach((p) => editor.apply({ type: 'setPage', id: p.id, ...props(p) })))
  const scope = scopeOf(snapshot, one?.activeModes)
  const parent = one && editor.nodes.get(one.id)?.parent
  const flows = !!one && parent?.kind === 'frame' && parent.direction !== 'none' && !one.absolute
  const hugs = (one?.kind === 'frame' && one.direction !== 'none') || (one?.kind === 'text' && flows)
  const sizes = Object.fromEntries(Object.entries(SIZES).filter(([s]) => s === 'fixed' || (s === 'hug' ? hugs : flows))) as Record<Size, string>
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
  const stroked = nodes.length > 0 && nodes.every((n) => n.kind === 'shape' || n.kind === 'frame')
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
      {box && (
        <div role="toolbar" aria-label="Align" className="align">
          {ALIGNS.map(([how, title, icon, key]) => (
            <button
              key={how}
              type="button"
              className="icon-button"
              aria-label={title}
              title={`${title} (${key})`}
              disabled={nodes.length < (how === 'tidy' ? 2 : how.startsWith('distribute') ? 3 : 1)}
              onClick={() => align(editor, how)}
            >
              <Icon name={icon} />
            </button>
          ))}
        </div>
      )}
      {!box && <DocumentSection editor={editor} />}
      {!box && (
        <Section
          title={isPage ? (targets.length > 1 ? `Pages ${numbers.join(', ')}` : `Page ${numbers[0]}`) : 'Master'}
          actions={
            isPage &&
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
            )
          }
        >
          <FormatRow page sheets={sheets} each={setSheets} />
          <div className="grid">
            {(['width', 'height', 'bleed'] as const).map((k) => (
              <Field
                key={k}
                label={k === 'bleed' ? 'Bleed' : k === 'width' ? 'W' : 'H'}
                value={sameOf(sheets, (p) => p[k])}
                unit="mm"
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
          {isPage && page.detached.length > 0 && (
            <button type="button" className="button" onClick={() => editor.apply({ type: 'resetToMaster', ids: [page.id] })}>
              Reset overrides
            </button>
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
          {canFindFonts && missingFonts.length > 0 && (
            <button type="button" className="button" onClick={() => findFonts(editor, say)}>
              Find missing fonts on this computer
            </button>
          )}
        </Section>
      )}
      {one?.overrideOf && (
        <Section title="Master">
          <button type="button" className="button" onClick={() => editor.apply({ type: 'resetToMaster', ids: [one.id] })}>
            Reset to master
          </button>
        </Section>
      )}
      {!box && (
        <Variables editor={editor} />
      )}
      {!box && <TextStyles editor={editor} />}
      {box && (
        <Section title="Layout">
          <div className="grid">
            <Field label="X" min={-Infinity} value={nodes.length > 1 ? same((n) => n.x) : box.x} unit="mm" onCommit={frame('x')} />
            <Field label="Y" min={-Infinity} value={nodes.length > 1 ? same((n) => n.y) : box.y} unit="mm" onCommit={frame('y')} />
            {line ? (
              <>
                <Field label="L" title="Length in mm" value={length!} unit="mm" onCommit={(v) => setLine(v, angle!)} />
                <Field label="∠" title="Angle in °" min={-Infinity} value={angle!} unit="°" onCommit={(v) => setLine(length!, v)} />
              </>
            ) : (
              <>
                {bindable('w', 'W in mm', 'W', <Field label="W" value={same((n) => n.w)} unit="mm" onCommit={frame('w')} />)}
                {bindable('h', 'H in mm', 'H', <Field label="H" value={same((n) => n.h)} unit="mm" onCommit={frame('h')} />)}
              </>
            )}
            {!line && nodes.every((n) => n.kind !== 'group') && (
              <Field label="∠" title="Rotation in °" unit="°" min={-Infinity} value={same((n) => n.rotation)} onCommit={(v) => set({ rotation: ((v % 360) + 360) % 360 })} />
            )}
            {nodes.length > 0 && nodes.every((n) => n.kind === 'shape' && n.shape === 'rect') && (
              bindable(
                'radius',
                'Corner radius in mm',
                'R',
                <Field
                  label="R"
                  title="Corner radius in mm"
                  unit="mm"
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
                <Field label="Start" title="Arc start in °" unit="°" min={-Infinity} value={one.start} onCommit={(v) => set({ start: ((v % 360) + 360) % 360 })} />
                <Field label="Sweep" title="Arc sweep in %" unit="%" max={100} value={one.sweep * 100} onCommit={(v) => set({ sweep: v / 100 })} />
                <Field label="Ratio" title="Inner radius in %" unit="%" max={100} value={one.inner * 100} onCommit={(v) => set({ inner: v / 100 })} />
              </>
            )}
            {one?.kind === 'shape' && one.shape === 'star' && (
              <Field label="Ratio" title="Star ratio in %" unit="%" min={1} max={100} value={one.ratio * 100} onCommit={(v) => set({ ratio: v / 100 })} />
            )}
          </div>
          {one && (hugs || flows) && (
            <div className="grid">
              {(['horizontal', 'vertical'] as const).map((axis) => (
                <Select
                  key={axis}
                  label={`${axis === 'horizontal' ? 'Width' : 'Height'} sizing`}
                  value={one.sizing[axis]}
                  options={sizes}
                  onChange={(s) => set({ sizing: { ...one.sizing, [axis]: s } })}
                />
              ))}
            </div>
          )}
          {one && parent?.kind === 'frame' && parent.direction !== 'none' && (
            <label className="check">
              <input type="checkbox" checked={one.absolute} onChange={(e) => set({ absolute: e.currentTarget.checked })} />
              Absolute position
            </label>
          )}
          {one && parent?.kind === 'frame' && !flows && (
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
          {one?.kind === 'frame' && (
            <label className="check">
              <input
                type="checkbox"
                checked={one.clip}
                onChange={(e) => editor.apply({ type: 'set', id: one.id, clip: e.currentTarget.checked })}
              />
              Clip content
            </label>
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
          onChange={(fills) => set({ fills })} />
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
              {!nodes.some(isOpen) && <Segmented label="Stroke position" value={same((n) => n.strokeAlign)} options={STROKE_ALIGNS} onChange={(strokeAlign) => set({ strokeAlign })} />}
              {open && (
                <>
                  <Segmented label="Stroke cap" value={same((n) => n.cap)} options={CAPS} onChange={(cap) => set({ cap })} />
                  <Segmented label="Start point" value={same((n) => (n.arrowStart ? 'arrow' : 'none'))} options={STARTS} onChange={(v) => set({ arrowStart: v === 'arrow' })} />
                  <Segmented label="End point" value={same((n) => (n.arrowEnd ? 'arrow' : 'none'))} options={ENDS} onChange={(v) => set({ arrowEnd: v === 'arrow' })} />
                </>
              )}
              <Segmented label="Stroke join" value={same((n) => n.join)} options={JOINS} onChange={(join) => set({ join })} />
            </div>
          )}
        </PaintList>
      )}
      {box && <EffectList effects={sameList((n) => n.effects)} mode={mode} scope={scope} onChange={(effects) => set({ effects })} />}
      {one?.kind === 'text' && <TextSection editor={editor} node={one} />}
      {one?.kind === 'text' && <TextFrameSection node={one} set={set} />}
    </aside>
  )
}

const sameOf = <T, U>(items: T[], get: (t: T) => U): U | null => {
  const values = items.map(get)
  return values.every((v) => v === values[0]) ? values[0] : null
}
const near = (a: number, b: number) => Math.abs(a - b) < 0.5

/** Format, orientation, size, bleed and number of all pages, facing pages, colour mode and raster resolution. */
function DocumentSection({ editor }: { editor: Editor }) {
  const { pages, masters, facingPages, colorMode, rasterPpi } = useEditor(editor, (e) => e.snapshot)
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
        <Field label="W" title="Width of all pages in mm" unit="mm" value={w} onCommit={(width) => each(() => ({ width }))} />
        <Field label="H" title="Height of all pages in mm" unit="mm" value={h} onCommit={(height) => each(() => ({ height }))} />
        <Field label="Bleed" title="Bleed of all pages in mm" unit="mm" value={sameOf(pages, (p) => p.bleed)} onCommit={(bleed) => each(() => ({ bleed }))} />
        <Field label="Pages" title="Pages" unit="" int min={1} value={pages.length} onCommit={count} />
      </div>
      <label className="check">
        <input type="checkbox" checked={facingPages} onChange={(e) => editor.apply({ type: 'setDocument', facingPages: e.currentTarget.checked })} />
        Facing pages
      </label>
      <div className="grid">
        <Select label="Color mode" value={colorMode} options={MODES} onChange={(colorMode) => editor.apply({ type: 'setDocument', colorMode })} />
        <div className="kv" title="Output profile">
          <span>Profile</span>
          <strong>{colorMode === 'cmyk' ? 'FOGRA51' : 'sRGB'}</strong>
        </div>
        <Field label="Raster" value={rasterPpi} unit="ppi" min={72} max={1200} onCommit={(v) => editor.apply({ type: 'setDocument', rasterPpi: v })} />
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
                    <Field label="Size" title="Cell size in mm" unit="mm" value={g.size} onCommit={(size) => size > 0 && set({ size })} />
                  ) : (
                    <>
                      <Field label="N" title={GRIDS[g.kind]} unit="" int min={1} value={g.count} onCommit={(count) => set({ count })} />
                      <Field label="Gutter" title="Gutter in mm" unit="mm" value={g.gutter} onCommit={(gutter) => set({ gutter })} />
                      <Field label="Margin" title="Margin in mm" unit="mm" value={g.margin} onCommit={(margin) => set({ margin: Math.max(0, margin) })} />
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
      <div className="row">
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
