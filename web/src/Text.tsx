import { Fragment, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Check, Field, FontSelect, NameInput, nextName, Section, Segmented, Select, Sides } from './controls'
import { ColorPicker } from './ColorPicker'
import { neutral, type Color } from './color'
import { scopeOf, useEditor, type Editor } from './editor'
import { addLocalFont, useLocalFonts } from './file'
import { Icon, type IconName } from './icons'
import { Popover } from './Popover'
import { PAGE_NUMBER, insert, range } from './textEdit'
import { STYLED, type Attrs, type Props, type Styled, type Tab, type TextNode, type TextProps, type TextStyle, type Typeface, textSizing } from './model'
import { Bindable } from './Variables'


export const ALIGNS = [
  ['left', 'Align left', 'alignLeft'],
  ['center', 'Align center', 'alignCenter'],
  ['right', 'Align right', 'alignRight'],
  ['justify', 'Justify', 'alignJustify'],
] as const
const INSETS = ['insetTop', 'insetRight', 'insetBottom', 'insetLeft'] as const
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
const METRICS: [Exclude<Styled, 'font' | 'textCase' | 'textDecoration' | 'features' | 'paragraphIndent' | 'indentLeft' | 'indentRight' | 'spaceBefore' | 'position' | 'baselineShift' | 'dropLines' | 'dropChars' | 'keepLines' | 'keepTogether' | 'keepNext' | 'tabs' | 'list'>, string, IconName, string, number, string?][] = [
  ['size', 'Font size', 'fontSize', 'pt', 0.1],
  ['lineHeight', 'Line height', 'lineHeight', 'pt', 0, 'Auto'],
  ['letterSpacing', 'Letter spacing', 'letterSpacing', '%', -100],
  ['paragraphSpacing', 'Paragraph spacing', 'paragraphSpacing', 'pt', 0],
]

const CASES = { original: 'As typed', upper: 'Upper case', lower: 'Lower case', title: 'Title case' } as const
const DECORATIONS = { none: 'No decoration', underline: 'Underline', strikethrough: 'Strikethrough' } as const
const LISTS = { none: 'No list', bullet: 'Bullets', number: 'Numbers' } as const
const POSITIONS = { normal: 'Normal position', superscript: 'Superscript', subscript: 'Subscript' } as const
/** OpenType features: tag, title and whether fonts apply it unless turned off. */
const FEATURES: [string, string, boolean?][] = [
  ['kern', 'Kerning', true],
  ['liga', 'Ligatures', true],
  ['dlig', 'Discretionary ligatures'],
  ['smcp', 'Small caps'],
  ['c2sc', 'Capitals to small caps'],
  ['case', 'Case-sensitive forms'],
  ['frac', 'Fractions'],
  ['zero', 'Slashed zero'],
  ['sups', 'Superscript glyphs'],
  ['subs', 'Subscript glyphs'],
  ['ordn', 'Ordinals'],
]
/** Features of which one at most applies: title, a sample and the tags with their titles. */
const FIGURES: [string, string, [string, string][]][] = [
  ['Figures', '196', [['lnum', 'Lining'], ['onum', 'Oldstyle']]],
  ['Figure spacing', '11', [['pnum', 'Proportional'], ['tnum', 'Tabular']]],
]

/** The text style's name set at its size, within what a panel row takes. */
const specimen = (s?: TextStyle) => (s ? { fontSize: Math.min(Math.max(s.size * 0.95, 9), 18), fontFamily: 'var(--doc-font)' } : undefined)

/** The value all spans share, or null. */
export function sameOf<A, T>(spans: A[], get: (a: A) => T): T | null {
  const values = spans.map(get)
  return values.every((v) => v === values[0]) ? values[0] : null
}

/** A button with the text style's name set at its size that opens a menu of the styles, and creates one from the text. */
export function Specimen({ editor, spans, compact, format }: { editor: Editor; spans: Attrs[]; compact?: boolean; format: (p: TextProps) => void }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  const style = sameOf(spans, (a) => a.textStyle)
  const current = styles.find((s) => s.id === style)
  const [menu, setMenu] = useState<DOMRect | null>(null)
  const own = (a: Attrs, bound = {}) => Object.fromEntries(STYLED.filter((k) => !(k in bound)).map((k) => [k, a[k]]))
  const changed = current && spans.some((a) => STYLED.some((k) => !(k in current.bindings) && current[k] !== null && JSON.stringify(a[k]) !== JSON.stringify(current[k])))
  const overrides: Parameters<typeof ContextMenu>[0]['items'] = changed
    ? [
        null,
        ['Reset to style', () => format({ textStyle: current.id }), true],
        ['Update style', () => void editor.apply({ type: 'setTextStyle', id: current.id, ...own(spans[0], current.bindings) }), true],
      ]
    : []
  const create = () => {
    const a = spans[0]
    const [id] = editor.apply({
      type: 'addTextStyle',
      name: nextName('Text style', styles.map((s) => s.name)),
      ...own(a),
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
        <span style={specimen(current)}>{style === null ? 'Mixed' : current ? `${current.name}${changed ? '*' : ''}` : 'No style'}</span>
        {current && !compact && <span className="specimen-size">{`${current.size}/${current.lineHeight || 'Auto'} pt`}</span>}
        <Icon name="chevron" />
      </button>
      {!compact && (
        <button type="button" className="icon-button" aria-label="Create text style" title="Create text style" disabled={!spans.length} onClick={create}>
          <Icon name="plus" />
        </button>
      )}
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
              ...overrides,
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
  const on = (tag: string, dflt?: boolean) => (a: Pick<Attrs, Styled>) => (dflt ? !a.features.includes(`${tag}=0`) : a.features.includes(tag))
  const put = (drop: string[], add: string[]) =>
    set({ features: [...(spans[0]?.features ?? []).filter((f) => !drop.some((t) => f === t || f === `${t}=0`)), ...add].sort() })
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
              <Group title="Character">
                <Select label="Case" value={same((a) => a.textCase)} options={CASES} onChange={(textCase) => set({ textCase })} />
                <Select label="Decoration" value={same((a) => a.textDecoration)} options={DECORATIONS} onChange={(textDecoration) => set({ textDecoration })} />
                <Select label="Position" value={same((a) => a.position)} options={POSITIONS} onChange={(position) => set({ position })} />
                <Field label="Shift" title="Baseline shift" unit="pt" min={-Infinity} reset={0} value={same((a) => a.baselineShift)} onCommit={(baselineShift) => set({ baselineShift })} />
              </Group>
              <Group title="Indents & spacing">
                <Field label="First line" title="First line indent" unit="pt" reset={0} value={same((a) => a.paragraphIndent)} onCommit={(paragraphIndent) => set({ paragraphIndent })} />
                <Field label="Left" title="Left indent" unit="pt" reset={0} value={same((a) => a.indentLeft)} onCommit={(indentLeft) => set({ indentLeft })} />
                <Field label="Right" title="Right indent" unit="pt" reset={0} value={same((a) => a.indentRight)} onCommit={(indentRight) => set({ indentRight })} />
                <Field label="Before" title="Space before" unit="pt" reset={0} value={same((a) => a.spaceBefore)} onCommit={(spaceBefore) => set({ spaceBefore })} />
                <Field label="After" title="Space after" unit="pt" reset={0} value={same((a) => a.paragraphSpacing)} onCommit={(paragraphSpacing) => set({ paragraphSpacing })} />
              </Group>
              <Group title="Paragraph">
                <Select label="List" value={same((a) => a.list)} options={LISTS} onChange={(list) => set({ list })} />
                <Field label="Keep" title="Lines kept together at start and end" unit="" int min={1} max={20} reset={1} value={same((a) => a.keepLines)} onCommit={(keepLines) => set({ keepLines })} />
                <Check label="Keep lines together" value={same((a) => a.keepTogether)} set={(keepTogether) => set({ keepTogether })} />
                <Check label="Keep with next" value={same((a) => a.keepNext)} set={(keepNext) => set({ keepNext })} />
              </Group>
              <Group title="Drop cap">
                <Field label="Lines" title="Drop cap lines" unit="" int min={0} max={20} reset={0} value={same((a) => a.dropLines)} onCommit={(dropLines) => set({ dropLines })} />
                <Field label="Characters" title="Drop cap characters" unit="" int min={1} max={20} reset={1} value={same((a) => a.dropChars)} onCommit={(dropChars) => set({ dropChars })} />
              </Group>
              <TabStops tabs={same((a) => JSON.stringify(a.tabs))} set={(tabs) => set({ tabs })} />
              <Group title="OpenType">
                {FIGURES.map(([title, sample, tags]) => {
                  const value = same((a) => tags.find(([t]) => a.features.includes(t))?.[0] ?? '')
                  return (
                    <div key={title} role="radiogroup" aria-label={title} className="segmented figures">
                      {[['', 'Default'], ...tags].map(([tag, name]) => (
                        <button key={tag} type="button" role="radio" aria-checked={value === tag} aria-label={`${name} ${title.toLowerCase()}`} title={`${name} ${title.toLowerCase()}`} onClick={() => put(tags.map(([t]) => t), tag ? [tag] : [])}>
                          {tag ? <span className="feature" style={{ fontFeatureSettings: `"${tag}"` }}>{sample}</span> : 'Auto'}
                        </button>
                      ))}
                    </div>
                  )
                })}
                {FEATURES.map(([tag, title, dflt]) => (
                  <Check key={tag} label={title} value={same(on(tag, dflt))} set={(c) => put([tag], dflt ? (c ? [] : [`${tag}=0`]) : c ? [tag] : [])} />
                ))}
              </Group>
            </Popover>
          </div>,
          document.body,
        )}
    </>
  )
}

function Group({ title, action, children }: { title: string; action?: ReactNode; children?: ReactNode }) {
  return (
    <div role="group" aria-label={title} className="options-group">
      <header>
        <h4>{title}</h4>
        {action}
      </header>
      {children && <div className="grid">{children}</div>}
    </div>
  )
}

const TAB_ALIGNS: Record<Tab['align'], string> = { left: 'Left', center: 'Centre', right: 'Right', decimal: 'Decimal' }

/** The tab stops of `tabs`, as JSON or null when mixed, each with its alignment and leader. */
function TabStops({ tabs, set }: { tabs: string | null; set: (tabs: Tab[]) => void }) {
  const list: Tab[] = tabs === null ? [] : JSON.parse(tabs)
  const put = (i: number, t: Partial<Tab>) => set(list.with(i, { ...list[i], ...t }).sort((a, b) => a.at - b.at))
  return (
    <div className="tab-stops">
      <Group
        title="Tabs"
        action={
          <button type="button" className="icon-button" aria-label="Add tab stop" title="Add tab stop" onClick={() => set([...list, { at: (list.at(-1)?.at ?? 0) + 36, align: 'left', leader: '' }])}>
            <Icon name="plus" />
          </button>
        }
      />
      {list.map((t, i) => (
        <div key={i} className="row">
          <Field label="Tab" title={`Tab stop ${i + 1}`} unit="length" value={t.at} onCommit={(at) => put(i, { at })} />
          <Select label={`Tab alignment ${i + 1}`} value={t.align} options={TAB_ALIGNS} onChange={(align) => put(i, { align })} />
          <label className="field">
            <span className="field-label">Leader</span>
            <NameInput label={`Tab leader ${i + 1}`} value={t.leader} blank onCommit={(leader) => put(i, { leader })} />
          </label>
          <button type="button" className="icon-button" aria-label={`Remove tab stop ${i + 1}`} title="Remove tab stop" onClick={() => set(list.toSpliced(i, 1))}>
            <Icon name="minus" />
          </button>
        </div>
      ))}
    </div>
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

/** Inserts the page number or a character that keyboards lack into the edited text, or at the end of `node`. */
function Characters({ editor, node, font }: { editor: Editor; node: TextNode; font?: string }) {
  const [at, setAt] = useState<DOMRect | null>(null)
  const [all, setAll] = useState<DOMRect | null>(null)
  const put = (c: string) => {
    const end = editor.storyOf(node).text.length
    if (editor.editing?.id !== node.id) editor.set({ editing: { id: node.id, anchor: end, focus: end } })
    insert(editor, c)
  }
  return (
    <>
      <button
        type="button"
        className="icon-button"
        aria-label="Insert character"
        title="Insert character"
        aria-haspopup="menu"
        // Keeps the focus in the edited text.
        onMouseDown={(e) => e.preventDefault()}
        onClick={(e) => setAt(e.currentTarget.getBoundingClientRect())}
      >
        <Icon name="omega" />
      </button>
      {at &&
        createPortal(
          <ContextMenu
            anchor={() => at}
            side="bottom"
            label="Insert character"
            onClose={() => setAt(null)}
            items={[
              ['Page number', () => put(PAGE_NUMBER), true, undefined, 'Ctrl Alt Shift N'],
              null,
              ...CHARACTERS.map(([name, c]): [string, () => void, boolean, undefined, string] => [name, () => put(c), true, undefined, c.trim() || '␣']),
              null,
              ['All characters…', () => setAll(at), true],
            ]}
          />,
          document.body,
        )}
      {all && createPortal(<Glyphs editor={editor} font={font} at={all} put={put} onClose={() => setAll(null)} />, document.body)}
    </>
  )
}

const faces = new Set<number>()

/** All characters of the font `font`, found by name, code point or themselves, and inserted on click. */
function Glyphs({ editor, font, at, put, onClose }: { editor: Editor; font?: string; at: DOMRect; put: (c: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState('')
  const [hover, setHover] = useState('')
  const [[id, chars]] = useState(() => editor.engine.characters(font) as [number, [number, string][]])
  const family = `satz-font-${id}`
  if (!faces.has(id)) {
    faces.add(id)
    document.fonts.add(new FontFace(family, editor.engine.font(id).slice().buffer))
  }
  const q = query.trim().toLowerCase()
  const shown = chars.filter(([c, name]) => !q || name.toLowerCase().includes(q) || String.fromCodePoint(c) === query.trim() || c.toString(16) === q.replace(/^u\+/, ''))
  return (
    <div className="menu-backdrop" onPointerDown={onClose}>
      <Popover
        anchor={() => at}
        side="bottom"
        className="picker glyphs"
        role="dialog"
        aria-label="All characters"
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key !== 'Escape') return
          e.stopPropagation()
          onClose()
        }}
      >
        <input className="help-search" aria-label="Search characters" placeholder="Search" autoFocus value={query} onChange={(e) => setQuery(e.target.value)} />
        <div className="glyph-grid" style={{ fontFamily: family }}>
          {shown.map(([c, name]) => {
            const code = `U+${c.toString(16).toUpperCase().padStart(4, '0')}`
            return (
              <button key={c} type="button" title={`${name} ${code}`.trim()} aria-label={name || code} onMouseEnter={() => setHover(`${name} ${code}`.trim())} onMouseDown={(e) => e.preventDefault()} onClick={() => put(String.fromCodePoint(c))}>
                {String.fromCodePoint(c)}
              </button>
            )
          })}
        </div>
        <div className="glyph-name">{hover || `${shown.length} characters`}</div>
      </Popover>
    </div>
  )
}

/** Family and style of `fonts`, null for the bundled one, bindable to a font variable on `id` if given. */
export function Font({ editor, id, fonts, compact, set }: { editor: Editor; id?: string; fonts: (Typeface | null)[]; compact?: boolean; set: (f: Typeface) => void }) {
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
      compact={compact}
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
  /** Formats the selection of the text being edited, or else all of it. */
  const format = (props: TextProps) => editor.apply({ type: 'format', id: node.id, range: edited && range(edited), ...props })
  const align = same((a) => a.textAlign)
  const hyphenate = same((a) => a.hyphenate)
  const lang = same((a) => a.lang)
  const shading = same((a) => JSON.stringify(a.shading))
  const langs: Record<string, string> = { ...LANGS }
  if (lang === null) langs.mixed = 'Mixed'

  return (
    <Section title="Text">
      <div className="row">
        <Specimen editor={editor} spans={spans} format={format} />
        <Characters editor={editor} node={node} font={same((a) => a.font?.hash ?? null) ?? undefined} />
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
      <ColorCheck
        editor={editor}
        node={node}
        label="Shading"
        color={shading === null ? undefined : JSON.parse(shading)}
        set={(c) => format({ shading: c })}
      />
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
  const rule = node.columnRule?.weight ? node.columnRule : null
  return (
    <Section title="Text frame">
      <Segmented
        label="Resizing"
        prefix="Size"
        value={resizing}
        options={RESIZING}
        disabled={(v) => (v === 'autoWidth' && !!(node.prev || node.next)) || (v !== 'autoWidth' && v !== 'fixedSize' && !!node.next)}
        onChange={(mode) => (mode === 'autoFit' ? editor.resize([node], mode) : set({ sizing: textSizing(node.sizing, mode) }))}
      />
      <Sides key={node.id} what="inset" values={INSETS.map((p) => node[p])} set={(v) => set(Object.fromEntries(INSETS.map((p, i) => [p, v[i]])))} />
      <div className="grid">
        <Field label="Columns" title="Columns" unit="" int min={1} max={20} reset={1} value={node.columns} onCommit={(columns) => set({ columns })} />
        <Field label="Gutter" title="Gutter" unit="length" value={node.gutter} onCommit={(gutter) => set({ gutter })} />
        <Field label="Baseline" title="Baseline grid" unit="pt" zero="Off" reset={0} value={node.baselineGrid} onCommit={(baselineGrid) => set({ baselineGrid })} />
        <Field label="Start" title="Baseline grid start" unit="pt" reset={0} value={node.baselineStart} onCommit={(baselineStart) => set({ baselineStart })} />
        <Field label="Min" title="Min height" unit="length" zero="Off" reset={0} disabled={resizing !== 'autoHeight'} value={node.minHeight} onCommit={(minHeight) => set({ minHeight })} />
        <Field label="Lines" title="Max lines" unit="" int max={1000} zero="Off" reset={0} value={node.maxLines} onCommit={(maxLines) => set({ maxLines })} />
        <Segmented label="Vertical align" prefix="Align" value={node.verticalAlign} options={VERTICAL} onChange={(verticalAlign) => set({ verticalAlign })} />
        <Check label="Trim to cap height" value={node.trim} set={(trim) => set({ trim })} />
      </div>
      {node.columns > 1 && (
        <ColorCheck
          editor={editor}
          node={node}
          label="Column rule"
          color={rule && rule.color}
          set={(color) => set({ columnRule: color ? { weight: 1, color } : { ...rule!, weight: 0 } })}
        >
          {rule && <Field label="Weight" title="Column rule weight" unit="pt" value={rule.weight} onCommit={(weight) => set({ columnRule: { ...rule, weight } })} />}
        </ColorCheck>
      )}
    </Section>
  )
}

/** A check that turns a colour on or off, and the colour's picker while it is on; `undefined` is mixed. */
function ColorCheck({ editor, node, label, color, set, children }: { editor: Editor; node: TextNode; label: string; color: Color | null | undefined; set: (c: Color | null) => void; children?: ReactNode }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  return (
    <div className="row">
      <Check label={label} value={color === undefined ? null : !!color} set={(on) => set(on ? neutral('gray', snapshot.colorMode) : null)} />
      {color && <ColorPicker label={`${label} color`} color={color} mode={snapshot.colorMode} scope={scopeOf(snapshot, node.activeModes)} onChange={set} />}
      {children}
    </div>
  )
}
