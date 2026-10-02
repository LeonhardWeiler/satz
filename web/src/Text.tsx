import { Fragment, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Field, FontSelect, NameInput, nextName, Section, Segmented, Select } from './controls'
import { useEditor, type Editor } from './editor'
import { addLocalFont, useLocalFonts } from './file'
import { Icon, type IconName } from './icons'
import { Popover } from './Popover'
import { PAGE_NUMBER, insert, range } from './textEdit'
import { STYLED, type Attrs, type Props, type Styled, type TextNode, type TextProps, type TextStyle, type Typeface, textSizing } from './model'
import { Bindable } from './Variables'


export const ALIGNS = [
  ['left', 'Align left', 'alignLeft'],
  ['center', 'Align center', 'alignCenter'],
  ['right', 'Align right', 'alignRight'],
  ['justify', 'Justify', 'alignJustify'],
] as const
const INSETS = [
  ['insetTop', 'Top inset', 'insetTop'],
  ['insetRight', 'Right inset', 'insetRight'],
  ['insetBottom', 'Bottom inset', 'insetBottom'],
  ['insetLeft', 'Left inset', 'insetLeft'],
] as const
const VERTICAL = [
  ['top', 'Align top', 'alignTop'],
  ['center', 'Align middle', 'alignMiddle'],
  ['bottom', 'Align bottom', 'alignBottom'],
] as const
const RESIZING = [
  ['autoWidth', 'Auto width', 'autoWidth'],
  ['autoHeight', 'Auto height', 'autoHeight'],
  ['fixedSize', 'Fixed size', 'fixedSize'],
  ['autoFit', 'Auto fit', 'autoFit'],
] as const
const LANGS = { en: 'English', de: 'German', fr: 'French', it: 'Italian', es: 'Spanish', nl: 'Dutch' } as const
/** Styled numbers: title, label, unit, least value and the text shown for 0. */
const METRICS: [Exclude<Styled, 'font' | 'textCase' | 'textDecoration' | 'features' | 'paragraphIndent' | 'position' | 'baselineShift' | 'dropLines' | 'dropChars' | 'keepLines' | 'keepTogether' | 'keepNext'>, string, IconName, string, number, string?][] = [
  ['size', 'Font size', 'fontSize', 'pt', 0.1],
  ['lineHeight', 'Line height', 'lineHeight', 'pt', 0, 'Auto'],
  ['letterSpacing', 'Letter spacing', 'letterSpacing', '%', -100],
  ['paragraphSpacing', 'Paragraph spacing', 'paragraphSpacing', 'pt', 0],
]

const CASES = { original: 'As typed', upper: 'Upper case', lower: 'Lower case', title: 'Title case' } as const
const DECORATIONS = { none: 'No decoration', underline: 'Underline', strikethrough: 'Strikethrough' } as const
const POSITIONS = { normal: 'Normal position', superscript: 'Superscript', subscript: 'Subscript' } as const
/** OpenType features: tag, title, a sample it changes and whether fonts apply it unless turned off. */
const FEATURES: [string, string, string, boolean?][] = [
  ['kern', 'Kerning', 'AV', true],
  ['liga', 'Ligatures', 'fi', true],
  ['dlig', 'Discretionary ligatures', 'ct'],
  ['smcp', 'Small caps', 'Ab'],
  ['c2sc', 'Capitals to small caps', 'AB'],
  ['case', 'Case-sensitive forms', '(H)'],
  ['lnum', 'Lining figures', '196'],
  ['onum', 'Oldstyle figures', '196'],
  ['pnum', 'Proportional figures', '11'],
  ['tnum', 'Tabular figures', '11'],
  ['frac', 'Fractions', '1/2'],
  ['zero', 'Slashed zero', '0'],
  ['sups', 'Superscript glyphs', 'x2'],
  ['subs', 'Subscript glyphs', 'x2'],
  ['ordn', 'Ordinals', '1st'],
]

/** The text style's name set at its size, within what a panel row takes. */
const specimen = (s?: TextStyle) => (s ? { fontSize: Math.min(Math.max(s.size * 0.95, 9), 18), fontFamily: 'var(--doc-font)' } : undefined)

/** The value all spans share, or null. */
export function sameOf<A, T>(spans: A[], get: (a: A) => T): T | null {
  const values = spans.map(get)
  return values.every((v) => v === values[0]) ? values[0] : null
}

/** A button with the text style's name set at its size that opens a menu of the styles, and creates one from the text. */
export function Specimen({ editor, spans, format }: { editor: Editor; spans: Attrs[]; format: (p: TextProps) => void }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  const style = sameOf(spans, (a) => a.textStyle)
  const current = styles.find((s) => s.id === style)
  const [menu, setMenu] = useState<DOMRect | null>(null)
  const create = () => {
    const a = spans[0]
    const [id] = editor.apply({
      type: 'addTextStyle',
      name: nextName('Text style', styles.map((s) => s.name)),
      ...Object.fromEntries(STYLED.map((k) => [k, a[k]])),
    })
    format({ textStyle: id })
  }
  return (
    <>
      <button
        type="button"
        className={`specimen${style === '' ? ' none' : ''}`}
        title="Text style"
        aria-haspopup="menu"
        onClick={(e) => setMenu(e.currentTarget.getBoundingClientRect())}
      >
        <span style={specimen(current)}>{style === null ? 'Mixed' : (current?.name ?? 'No style')}</span>
        {current && <span className="specimen-size">{`${current.size}/${current.lineHeight || 'Auto'} pt`}</span>}
        <Icon name="chevron" />
      </button>
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => menu}
            side="bottom"
            label="Text styles"
            onClose={() => setMenu(null)}
            items={[
              ['No style', () => format({ textStyle: '' }), true, style === ''],
              ...styles.map((s): [ReactNode, () => void, boolean, boolean] => [
                <span style={specimen(s)}>{s.name}</span>,
                () => format({ textStyle: s.id }),
                true,
                style === s.id,
              ]),
              null,
              ['Create text style', create, spans.length > 0],
            ]}
          />,
          document.body,
        )}
    </>
  )
}

/** A button that opens the case, decoration, indent and OpenType features of `spans`. */
export function TypeOptions({ spans, set }: { spans: Pick<Attrs, Styled>[]; set: (p: TextProps) => void }) {
  const [at, setAt] = useState<DOMRect | null>(null)
  const same = <T,>(get: (a: Pick<Attrs, Styled>) => T) => sameOf(spans, get)
  const features = same((a) => a.features.join(' '))
  const on = (tag: string, dflt?: boolean) => (a: Pick<Attrs, Styled>) => (dflt ? !a.features.includes(`${tag}=0`) : a.features.includes(tag))
  const toggle = (tag: string, dflt: boolean | undefined, checked: boolean) => {
    const rest = (spans[0]?.features ?? []).filter((f) => f !== tag && f !== `${tag}=0`)
    const own = dflt ? (checked ? [] : [`${tag}=0`]) : checked ? [tag] : []
    set({ features: [...rest, ...own].sort() })
  }
  return (
    <>
      <button
        type="button"
        className="icon-button"
        aria-label="Type options"
        title="Type options"
        aria-haspopup="dialog"
        aria-expanded={!!at}
        onClick={(e) => setAt(e.currentTarget.getBoundingClientRect())}
      >
        <Icon name="more" />
      </button>
      {at &&
        createPortal(
          <div className="menu-backdrop" onPointerDown={() => setAt(null)}>
            <Popover
              anchor={() => at}
              side="bottom"
              className="picker options"
              role="dialog"
              aria-label="Type options"
              tabIndex={-1}
              onPointerDown={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key !== 'Escape') return
                e.stopPropagation()
                setAt(null)
              }}
            >
              <div className="grid">
                <Select label="Case" value={same((a) => a.textCase)} options={CASES} onChange={(textCase) => set({ textCase })} />
                <Select label="Decoration" value={same((a) => a.textDecoration)} options={DECORATIONS} onChange={(textDecoration) => set({ textDecoration })} />
                <Field
                  label={<Icon name="paragraphIndent" />}
                  title="Paragraph indent"
                  unit="pt"
                  reset={0}
                  value={same((a) => a.paragraphIndent)}
                  onCommit={(paragraphIndent) => set({ paragraphIndent })}
                />
                <Select label="Position" value={same((a) => a.position)} options={POSITIONS} onChange={(position) => set({ position })} />
                <Field
                  label={<Icon name="baselineShift" />}
                  title="Baseline shift in pt"
                  unit="pt"
                  min={-Infinity}
                  reset={0}
                  value={same((a) => a.baselineShift)}
                  onCommit={(baselineShift) => set({ baselineShift })}
                />
                <Field label="Drop" title="Drop cap lines" unit="" int min={0} max={20} reset={0} value={same((a) => a.dropLines)} onCommit={(dropLines) => set({ dropLines })} />
                <Field label="Chars" title="Drop cap characters" unit="" int min={1} max={20} reset={1} value={same((a) => a.dropChars)} onCommit={(dropChars) => set({ dropChars })} />
                <Field label="Keep" title="Lines kept together at start and end" unit="" int min={1} max={20} reset={1} value={same((a) => a.keepLines)} onCommit={(keepLines) => set({ keepLines })} />
                <Check label="Keep lines together" value={same((a) => a.keepTogether)} set={(keepTogether) => set({ keepTogether })} />
                <Check label="Keep with next" value={same((a) => a.keepNext)} set={(keepNext) => set({ keepNext })} />
              </div>
              <div className="grid" role="group" aria-label="OpenType features">
                {FEATURES.map(([tag, title, sample, dflt]) => {
                  const checked = same(on(tag, dflt))
                  return (
                    <label key={tag} className="check">
                      <input
                        type="checkbox"
                        checked={checked === true}
                        ref={(el) => {
                          if (el) el.indeterminate = features === null && checked === null
                        }}
                        onChange={(e) => toggle(tag, dflt, e.currentTarget.checked)}
                      />
                      <span className="feature" style={{ fontFeatureSettings: `"${tag}"` }} aria-hidden="true">{sample}</span>
                      {title}
                    </label>
                  )
                })}
              </div>
            </Popover>
          </div>,
          document.body,
        )}
    </>
  )
}

/** A checkbox that shows `null`, a mixed value, as indeterminate. */
function Check({ label, value, set }: { label: string; value: boolean | null; set: (v: boolean) => void }) {
  return (
    <label className="check">
      <input
        type="checkbox"
        checked={value === true}
        ref={(el) => {
          if (el) el.indeterminate = value === null
        }}
        onChange={(e) => set(e.currentTarget.checked)}
      />
      {label}
    </label>
  )
}

const CHARACTERS = [
  ['Em dash', '\u2014'],
  ['En dash', '\u2013'],
  ['Ellipsis', '\u2026'],
  ['Bullet', '\u2022'],
  ['Non-breaking space', '\u00a0'],
  ['Thin space', '\u2009'],
  ['Low quote', '\u201e'],
  ['Left quote', '\u201c'],
  ['Right quote', '\u201d'],
  ['Apostrophe', '\u2019'],
  ['Right-pointing guillemet', '\u00bb'],
  ['Left-pointing guillemet', '\u00ab'],
  ['Copyright', '\u00a9'],
  ['Registered', '\u00ae'],
  ['Trademark', '\u2122'],
  ['Section', '\u00a7'],
  ['Degree', '\u00b0'],
  ['Multiplication', '\u00d7'],
] as const

/** Inserts the page number or a character that keyboards lack into the edited text. */
function Characters({ editor }: { editor: Editor }) {
  const [at, setAt] = useState<DOMRect | null>(null)
  return (
    <>
      <button
        type="button"
        className="icon-button page-number"
        aria-label="Insert character"
        title="Insert character"
        aria-haspopup="menu"
        // Keeps the focus in the edited text.
        onMouseDown={(e) => e.preventDefault()}
        onClick={(e) => setAt(e.currentTarget.getBoundingClientRect())}
      >
        <Icon name="pageNumber" />
      </button>
      {at &&
        createPortal(
          <ContextMenu
            anchor={() => at}
            side="bottom"
            label="Insert character"
            onClose={() => setAt(null)}
            items={[
              ['Page number', () => insert(editor, PAGE_NUMBER), true, undefined, 'Ctrl Alt Shift N'],
              null,
              ...CHARACTERS.map(([name, c]): [string, () => void, boolean, undefined, string] => [name, () => insert(editor, c), true, undefined, c.trim() || '␣']),
            ]}
          />,
          document.body,
        )}
    </>
  )
}

/** Family and style of `fonts`, null for the bundled one, bindable to a font variable on `id` if given. */
export function Font({ editor, id, fonts, set }: { editor: Editor; id?: string; fonts: (Typeface | null)[]; set: (f: Typeface) => void }) {
  const added = useEditor(editor, (e) => e.snapshot.fonts)
  const missing = useEditor(editor, (e) => e.snapshot.missingFonts).map((m) => m.font)
  const locals = useLocalFonts().filter((l) => !added.some((f) => f.name === l.fullName))
  const hashes = fonts.map((f) => (f ?? added[0]).hash)
  const select = (
    <FontSelect
      label="Font"
      faces={[...added, ...missing, ...locals.map((l) => ({ name: l.fullName, family: l.family, style: l.style, hash: `local:${l.postscriptName}` }))]}
      missing={missing.map((f) => f.hash)}
      value={hashes.every((h) => h === hashes[0]) ? hashes[0] : null}
      onChange={(f) => {
        const l = locals.find((l) => `local:${l.postscriptName}` === f.hash)
        if (!l) set(f)
        else void addLocalFont(editor, l).then((f) => f && set(f))
      }}
    />
  )
  return id ? (
    <Bindable editor={editor} id={id} prop="font" title="Font" label={<Icon name="text" />}>
      {select}
    </Bindable>
  ) : (
    select
  )
}

/** Text style, type and alignment of a text layer. */
export function TextSection({ editor, node }: { editor: Editor; node: TextNode }) {
  const spans = useEditor(editor, (e) => e.snapshot.stories[node.story])?.spans ?? []
  const same = <T,>(get: (a: Attrs) => T) => sameOf(spans, get)
  const edited = useEditor(editor, (e) => (e.editing?.id === node.id && e.editing.anchor !== e.editing.focus ? e.editing : null))
  const editing = useEditor(editor, (e) => e.editing?.id === node.id)
  /** Formats the selection of the text being edited, or else all of it. */
  const format = (props: TextProps) => editor.apply({ type: 'format', id: node.id, range: edited && range(edited), ...props })
  const align = same((a) => a.textAlign)
  const hyphenate = same((a) => a.hyphenate)
  const lang = same((a) => a.lang)
  const langs: Record<string, string> = { ...LANGS }
  if (lang === null) langs.mixed = 'Mixed'

  return (
    <Section title="Text">
      <div className="row">
        <Specimen editor={editor} spans={spans} format={format} />
        {editing && <Characters editor={editor} />}
      </div>
      <Font editor={editor} id={node.id} fonts={spans.map((a) => a.font)} set={(font) => format({ font })} />
      <div className="grid">
        {METRICS.map(([prop, title, label, unit, min, zero]) => {
          const field = (
            <Field
              label={<Icon name={label} />}
              title={title}
              unit={unit}
              min={min}
              zero={zero}
              reset={prop === 'size' ? undefined : 0}
              value={same((a) => a[prop])}
              onCommit={(v) => format({ [prop]: v })}
            />
          )
          return prop === 'size' ? (
            <Bindable key={prop} editor={editor} id={node.id} prop={prop} title={title} label={<Icon name={label} />}>
              {field}
            </Bindable>
          ) : (
            <Fragment key={prop}>{field}</Fragment>
          )
        })}
      </div>
      <div className="row">
        <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => format({ textAlign })} />
        <TypeOptions spans={spans} set={format} />
      </div>
      <div className="grid">
        <Check label="Hyphenate" value={hyphenate} set={(hyphenate) => format({ hyphenate })} />
        <Select
          label="Hyphenation language"
          value={lang ?? 'mixed'}
          options={langs}
          onChange={(l) => l !== 'mixed' && format({ lang: l as Attrs['lang'] })}
        />
      </div>
    </Section>
  )
}

/** The document's text styles with their values, each bindable to a variable. */
export function TextStyles({ editor }: { editor: Editor }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  if (styles.length === 0) return null
  return (
    <Section title="Text styles">
      {styles.map((s) => (
        <StyleRow key={s.id} editor={editor} style={s} />
      ))}
    </Section>
  )
}

function StyleRow({ editor, style }: { editor: Editor; style: TextStyle }) {
  return (
    <div role="group" aria-label={style.name} className="text-style">
      <div className="row">
        <NameInput label="Text style name" value={style.name} onCommit={(name) => editor.apply({ type: 'setTextStyle', id: style.id, name })} />
        <TypeOptions spans={[style]} set={(p) => editor.apply({ type: 'setTextStyle', id: style.id, ...p })} />
        <button
          type="button"
          className="icon-button"
          aria-label={`Delete text style ${style.name}`}
          title="Delete text style"
          onClick={() => editor.apply({ type: 'deleteTextStyle', id: style.id })}
        >
          <Icon name="minus" />
        </button>
      </div>
      <Font editor={editor} id={style.id} fonts={[style.font]} set={(font) => editor.apply({ type: 'setTextStyle', id: style.id, font })} />
      <div className="grid">
        {METRICS.map(([prop, title, label, unit, min, zero]) => (
          <Bindable key={prop} editor={editor} id={style.id} prop={prop} title={title} label={<Icon name={label} />}>
            <Field
              label={<Icon name={label} />}
              title={title}
              unit={unit}
              min={min}
              zero={zero}
              reset={prop === 'size' ? undefined : 0}
              value={style[prop]}
              onCommit={(v) => editor.apply({ type: 'setTextStyle', id: style.id, [prop]: v })}
            />
          </Bindable>
        ))}
      </div>
    </div>
  )
}

/** Resizing, insets, columns, vertical alignment and baseline grid of a text layer. */
export function TextFrameSection({ editor, node, set }: { editor: Editor; node: TextNode; set: (p: Props) => void }) {
  const { horizontal, vertical } = node.sizing
  const resizing = horizontal === 'hug' ? 'autoWidth' : vertical === 'hug' ? 'autoHeight' : 'fixedSize'
  return (
    <Section title="Text frame">
      <Segmented
        label="Resizing"
        value={resizing}
        options={RESIZING}
        disabled={(v) => (v === 'autoWidth' && !!(node.prev || node.next)) || (v !== 'autoWidth' && v !== 'fixedSize' && !!node.next)}
        onChange={(mode) => (mode === 'autoFit' ? editor.resize([node], mode) : set({ sizing: textSizing(node.sizing, mode) }))}
      />
      <div className="grid">
        {INSETS.map(([prop, title, label]) => (
          <Field key={prop} label={<Icon name={label} />} title={title} unit="length" reset={0} value={node[prop]} onCommit={(v) => set({ [prop]: v })} />
        ))}
        <Field label={<Icon name="columns" />} title="Columns" unit="" int min={1} max={20} reset={1} value={node.columns} onCommit={(columns) => set({ columns })} />
        <Field label={<Icon name="gutter" />} title="Gutter" unit="length" value={node.gutter} onCommit={(gutter) => set({ gutter })} />
        <Field label={<Icon name="baselineGrid" />} title="Baseline grid" unit="pt" zero="Off" reset={0} value={node.baselineGrid} onCommit={(baselineGrid) => set({ baselineGrid })} />
        <Field label={<Icon name="baselineStart" />} title="Baseline grid start" unit="pt" reset={0} value={node.baselineStart} onCommit={(baselineStart) => set({ baselineStart })} />
      </div>
      <Segmented label="Vertical align" value={node.verticalAlign} options={VERTICAL} onChange={(verticalAlign) => set({ verticalAlign })} />
    </Section>
  )
}
