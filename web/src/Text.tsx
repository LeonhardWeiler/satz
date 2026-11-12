import { Fragment, useEffect, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Check, Field, FontSelect, Group, NameInput, nextName, Section, Segmented, Select, Sides, useMore, useSides } from './controls'
import { ColorPicker } from './ColorPicker'
import { neutral, type Color } from './color'
import { scopeOf, useEditor, type Editor } from './editor'
import { addFonts, addLocalFont, useLocalFonts } from './file'
import { Icon, type IconName } from './icons'
import { Dialog } from './Popover'
import { MAX_LENGTH, length } from './settings'
import { DATE, PAGE_NUMBER, insert, range } from './textEdit'
import { STYLED, type Attrs, type Props, type Span, type Styled, type Tab, type TextNode, type TextProps, type TextStyle, type Typeface } from './model'
import { Bindable } from './Variables'
import namesUrl from './unicode-names.txt?url'

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
] as const
const LANGS = { en: 'English', de: 'German', fr: 'French', it: 'Italian', es: 'Spanish', nl: 'Dutch' } as const
/** Styled numbers: title, label, unit, least value and the text shown for 0. */
const METRICS: ['size' | 'lineHeight', string, IconName, string, number, number, string?][] = [
  ['size', 'Font size', 'fontSize', 'pt', 0.1, 5000],
  ['lineHeight', 'Line height', 'lineHeight', 'pt', 0, MAX_LENGTH, 'Auto'],
]

const CASES = { original: 'As typed', upper: 'Upper case', lower: 'Lower case', title: 'Title case' } as const
const DECORATIONS = { none: 'None', underline: 'Underline', strikethrough: 'Strikethrough' } as const
const LISTS = { none: 'None', bullet: 'Bullets', number: 'Numbers' } as const
const POSITIONS = { normal: 'Normal', superscript: 'Superscript', subscript: 'Subscript' } as const
const CASE_TAGS = { smcp: 'Small caps', 'smcp c2sc': 'All small caps', c2sc: 'Capitals to small caps' }
const POSITION_TAGS = { sups: 'Superscript glyphs', subs: 'Subscript glyphs' }
/** OpenType features by group: tag, title and whether fonts apply it unless turned off. */
const FEATURES: [string, [string, string, boolean?][]][] = [
  ['Ligatures & kerning', [['kern', 'Kerning', true], ['liga', 'Ligatures', true], ['dlig', 'Discretionary ligatures']]],
  ['Numbers & forms', [['frac', 'Fractions'], ['zero', 'Slashed zero'], ['ordn', 'Ordinals'], ['case', 'Case-sensitive forms']]],
]
/** Features of which one at most applies: title, a sample and the tags with their titles. */
const FIGURES: [string, string, [string, string][]][] = [
  ['Figures', 'Figures', [['lnum', 'Lining'], ['onum', 'Oldstyle']]],
  ['Figure spacing', 'Width', [['pnum', 'Proportional'], ['tnum', 'Tabular']]],
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
              ...overrides,
              null,
              [
                <span className="menu-action">
                  <Icon name="plus" />
                  New text style
                </span>,
                create,
                spans.length > 0,
              ],
            ]}
          />,
          document.body,
        )}
    </>
  )
}

/** Changes the OpenType features of each run of `spans` in `r`, or in all of them, by `f`. */
export const featuresOf = (editor: Editor, id: string, spans: Span[], r: [number, number] | null) => (f: (old: string[]) => string[]) => {
  const [a, b] = r ?? [0, Infinity]
  let at = 0
  const runs = spans.flatMap((s) => {
    const from = Math.max(at, a)
    at += s.len
    return from < Math.min(at, b) ? [{ range: [from, Math.min(at, b)] as [number, number], features: f(s.features) }] : []
  })
  const all = runs[0]?.features ?? f(spans[0]?.features ?? [])
  editor.batch(() => {
    editor.apply({ type: 'format', id, range: r, features: all })
    runs.filter((x) => x.features.join() !== all.join()).forEach((x) => editor.apply({ type: 'format', id, ...x }))
  })
}

/** A button that opens the case, decoration, indent and OpenType features of `spans`. */
export function TypeOptions({ editor, spans, set, features, paragraph }: { editor: Editor; spans: Pick<Attrs, Styled>[]; set: (p: TextProps) => void; features: (f: (old: string[]) => string[]) => void; paragraph?: ReactNode }) {
  const [at, setAt] = useState<Element | null>(null)
  const [tab, setTab] = useState<'Character' | 'Paragraph' | 'Tabs'>('Character')
  const same = <T,>(get: (a: Pick<Attrs, Styled>) => T) => sameOf(spans, get)
  const on = (tag: string, dflt?: boolean) => (a: Pick<Attrs, Styled>) => (dflt ? !a.features.includes(`${tag}=0`) : a.features.includes(tag))
  const put = (drop: string[], add: string[]) => features((old) => [...old.filter((f) => !drop.some((t) => f === t || f === `${t}=0`)), ...add].sort())
  const fold = <K extends 'textCase' | 'position'>(key: K, options: Record<string, string>, tags: Record<string, string>, plain: string) => {
    const known = [...new Set(Object.keys(tags).flatMap((t) => t.split(' ')))]
    const tagged = (a: Pick<Attrs, Styled>) => known.filter((t) => a.features.includes(t)).join(' ')
    const label = key === 'textCase' ? 'Case' : 'Position'
    return (
      <Select
        label={label}
        prefix={label}
        value={same((a) => tagged(a) || a[key])}
        options={{ ...options, ...tags }}
        onChange={(v) => editor.batch(() => {
          const tag = v in tags
          if (same((a) => a[key]) !== (tag ? plain : v)) set({ [key]: tag ? plain : v } as TextProps)
          if (same(tagged) !== (tag ? v : '')) put(known, tag ? v.split(' ') : [])
        })}
      />
    )
  }
  return (
    <>
      <button type="button" className="icon-button" aria-label="Type options" title="Type options" aria-haspopup="dialog" aria-expanded={!!at} onClick={(e) => setAt(e.currentTarget)}>
        <Icon name="more" />
      </button>
      {at && (
            <Dialog label="Type options" at={at} onClose={() => setAt(null)}>
              <header className="picker-header">
                <div role="tablist" className="tabs">
                  {(['Character', 'Paragraph', 'Tabs'] as const).map((t) => (
                    <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
                      {t}
                    </button>
                  ))}
                </div>
              </header>
              {tab === 'Character' && (
                <>
                  <Group title="Spacing">
                    <Field label={<Icon name="letterSpacing" />} title="Letter spacing" unit="%" min={-100} max={10000} reset={0} value={same((a) => a.letterSpacing)} onCommit={(letterSpacing) => set({ letterSpacing })} />
                  </Group>
                  <Group title="Case & position">
                    {fold('textCase', CASES, CASE_TAGS, 'original')}
                    <Select label="Decoration" prefix="Decoration" value={same((a) => a.textDecoration)} options={DECORATIONS} onChange={(textDecoration) => set({ textDecoration })} />
                    {fold('position', POSITIONS, POSITION_TAGS, 'normal')}
                    <Field label="Shift" title="Baseline shift" unit="pt" min={-Infinity} reset={0} value={same((a) => a.baselineShift)} onCommit={(baselineShift) => set({ baselineShift })} />
                  </Group>
                  {FEATURES.map(([group, list]) => (
                    <Group key={group} title={group}>
                      {group === 'Numbers & forms' &&
                        FIGURES.map(([title, prefix, tags]) => (
                          <Select
                            key={title}
                            label={title}
                            prefix={prefix}
                            value={same((a) => tags.find(([t]) => a.features.includes(t))?.[0] ?? '')}
                            options={Object.fromEntries([['', 'Default'], ...tags])}
                            onChange={(tag) => put(tags.map(([t]) => t), tag ? [tag] : [])}
                          />
                        ))}
                      {list.map(([tag, title, dflt]) => (
                        <Check key={tag} label={title} value={same(on(tag, dflt))} set={(c) => put([tag], dflt ? (c ? [] : [`${tag}=0`]) : c ? [tag] : [])} />
                      ))}
                    </Group>
                  ))}
                </>
              )}
              {tab === 'Paragraph' && (
                <>
                  {paragraph && <Group title="Hyphenation & shading">{paragraph}</Group>}
                  <Group title="Indents & spacing">
                    <Field label="First line" title="First line indent" unit="pt" reset={0} value={same((a) => a.paragraphIndent)} onCommit={(paragraphIndent) => set({ paragraphIndent })} />
                    <Field label="Left" title="Left indent" unit="pt" reset={0} value={same((a) => a.indentLeft)} onCommit={(indentLeft) => set({ indentLeft })} />
                    <Field label="Right" title="Right indent" unit="pt" reset={0} value={same((a) => a.indentRight)} onCommit={(indentRight) => set({ indentRight })} />
                    <Field label="Before" title="Space before" unit="pt" reset={0} value={same((a) => a.spaceBefore)} onCommit={(spaceBefore) => set({ spaceBefore })} />
                    <Field label="After" title="Space after" unit="pt" reset={0} value={same((a) => a.paragraphSpacing)} onCommit={(paragraphSpacing) => set({ paragraphSpacing })} />
                  </Group>
                  <Group title="Lists & breaks">
                    <Select label="List" prefix="List" value={same((a) => a.list)} options={LISTS} onChange={(list) => set({ list })} />
                    <Field label="Min lines" title="Lines kept together at start and end" unit="" int min={1} max={20} reset={1} value={same((a) => a.keepLines)} onCommit={(keepLines) => set({ keepLines })} />
                    <Check label="Keep together" value={same((a) => a.keepTogether)} set={(keepTogether) => set({ keepTogether })} />
                    <Check label="Keep with next" value={same((a) => a.keepNext)} set={(keepNext) => set({ keepNext })} />
                  </Group>
                  <Group title="Drop cap">
                    <Field label="Lines" title="Drop cap lines" unit="" int min={0} max={20} reset={0} value={same((a) => a.dropLines)} onCommit={(dropLines) => set({ dropLines })} />
                    <Field label="Characters" title="Drop cap characters" disabled={same((a) => a.dropLines) === 0} unit="" int min={1} max={20} reset={1} value={same((a) => a.dropChars)} onCommit={(dropChars) => set({ dropChars })} />
                  </Group>
                </>
              )}
              {tab === 'Tabs' && <TabStops tabs={same((a) => JSON.stringify(a.tabs))} set={(tabs) => set({ tabs })} />}
            </Dialog>
      )}
    </>
  )
}

const TAB_ALIGNS: Record<Tab['align'], string> = { left: 'Left', center: 'Center', right: 'Right', decimal: 'Decimal' }

/** The tab stops of `tabs`, as JSON or null when mixed, each with its alignment and leader. */
function TabStops({ tabs, set }: { tabs: string | null; set: (tabs: Tab[]) => void }) {
  const list: Tab[] = tabs === null ? [] : JSON.parse(tabs)
  const put = (i: number, t: Partial<Tab>) => set(list.with(i, { ...list[i], ...t }).sort((a, b) => a.at - b.at))
  return (
    <div className="tab-stops">
      <Group
        title="Tab stops"
        action={
          <button type="button" className="icon-button" aria-label="Add tab stop" title="Add tab stop" onClick={() => set([...list, { at: (list.at(-1)?.at ?? 0) + 36, align: 'left', leader: '' }])}>
            <Icon name="plus" />
          </button>
        }
      />
      {!list.length && <p className="pf-note">No tab stops. Tabs go to every {length(36)}.</p>}
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

/** Inserts the page number, the date or a character that keyboards lack into the edited text, or at the end of `node`. */
function Characters({ editor, node, font }: { editor: Editor; node: TextNode; font?: string }) {
  const [at, setAt] = useState<Element | null>(null)
  const [all, setAll] = useState<Element | null>(null)
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
        onClick={(e) => setAt(e.currentTarget)}
      >
        <Icon name="omega" />
      </button>
      {at &&
        createPortal(
          <ContextMenu
            anchor={() => at.getBoundingClientRect()}
            side="bottom"
            label="Insert character"
            onClose={() => setAt(null)}
            items={[
              [<span data-glyph="#">Page number</span>, () => put(PAGE_NUMBER), true, undefined, 'Ctrl Alt Shift N'],
              [<span data-glyph="#">Date</span>, () => put(DATE), true],
              null,
              ...CHARACTERS.map(([name, c]): [ReactNode, () => void, boolean] => [<span data-glyph={c.trim() || '␣'}>{name}</span>, () => put(c), true]),
              null,
              ['All characters…', () => setAll(at), true],
            ]}
          />,
          document.body,
        )}
      {all && <Glyphs editor={editor} font={font} at={all} put={put} onClose={() => setAll(null)} />}
    </>
  )
}

const faces = new Set<number>()

let names: Promise<Map<number, string>> | undefined
/** The Unicode names by code point, fetched once. */
const loadNames = () =>
  (names ??= fetch(namesUrl)
    .then((r) => r.text())
    .then((t) => new Map(t.split('\n').filter(Boolean).map((l) => [parseInt(l, 16), l.slice(l.indexOf(' ') + 1)])))
    .catch(() => new Map()))

/** All characters of the font `font`, found by name, code point or themselves, and inserted on click. */
function Glyphs({ editor, font, at, put, onClose }: { editor: Editor; font?: string; at: Element; put: (c: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState('')
  const [hover, setHover] = useState('')
  const [[id, codes]] = useState(() => editor.engine.characters(font) as [number, number[]])
  const [named, setNamed] = useState(new Map<number, string>())
  useEffect(() => void loadNames().then(setNamed), [])
  const chars = codes.map((c) => [c, named.get(c) ?? ''] as const)
  const family = `satz-font-${id}`
  if (!faces.has(id)) {
    faces.add(id)
    document.fonts.add(new FontFace(family, editor.engine.font(id).slice().buffer))
  }
  const q = query.trim().toLowerCase()
  const shown = chars.filter(([c, name]) => !q || name.toLowerCase().includes(q) || String.fromCodePoint(c) === query.trim() || c.toString(16) === q.replace(/^u\+/, ''))
  return (
    <Dialog label="All characters" at={at} className="glyphs" onClose={onClose}>
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
      </Dialog>
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
      onAdd={() => addFonts(editor)}
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

/** Text style, type and alignment of text layers. */
export function TextSection({ editor, nodes }: { editor: Editor; nodes: TextNode[] }) {
  const stories = useEditor(editor, (e) => e.snapshot.stories)
  const one = nodes.length === 1 ? nodes[0] : undefined
  const edited = useEditor(editor, (e) => (one && e.editing?.id === one.id && e.editing.anchor !== e.editing.focus ? e.editing : null))
  const r = edited && range(edited)
  const all = nodes.map((n) => stories[n.story]?.spans ?? [])
  let at = 0
  const spans = r ? all[0].filter((s) => (at += s.len) > r[0] && at - s.len < r[1]) : all.flat()
  const same = <T,>(get: (a: Attrs) => T) => sameOf(spans, get)
  /** Formats the selection of the text being edited, or else all of it. */
  const format = (props: TextProps) => editor.batch(() => nodes.forEach((n) => editor.apply({ type: 'format', id: n.id, range: r, ...props })))
  const features = (f: (old: string[]) => string[]) => editor.batch(() => nodes.forEach((n, i) => featuresOf(editor, n.id, all[i], r)(f)))
  const align = same((a) => a.textAlign)
  const hyphenate = same((a) => a.hyphenate)
  const lang = same((a) => a.lang)
  const shading = same((a) => JSON.stringify(a.shading))
  const langs: Record<string, string> = { ...LANGS }
  if (lang === null) langs.mixed = 'Mixed'

  return (
    <Section
      title="Text"
      actions={
        <>
          {one && <Characters editor={editor} node={one} font={same((a) => a.font?.hash ?? null) ?? undefined} />}
          <TypeOptions
            editor={editor}
            spans={spans}
            set={format}
            features={features}
            paragraph={
              <>
                <Check label="Hyphenate" value={hyphenate} set={(hyphenate) => format({ hyphenate })} />
                <Select
                  label="Hyphenation language"
                  prefix="Language"
                  value={lang ?? 'mixed'}
                  options={langs}
                  onChange={(l) => l !== 'mixed' && format({ lang: l as Attrs['lang'] })}
                />
                <ColorCheck
                  editor={editor}
                  node={nodes[0]}
                  label="Shading"
                  color={shading === null ? undefined : JSON.parse(shading)}
                  set={(c) => format({ shading: c })}
                />
              </>
            }
          />
        </>
      }
    >
      <Specimen editor={editor} spans={spans} format={format} />
      <Font editor={editor} id={one?.id} fonts={spans.map((a) => a.font)} set={(font) => format({ font })} />
      <div className="grid">
        {METRICS.map(([prop, title, label, unit, min, max, zero]) => {
          const field = (
            <Field
              label={<Icon name={label} />}
              title={title}
              unit={unit}
              min={min}
              max={max}
              zero={zero}
              reset={prop === 'size' ? undefined : 0}
              value={same((a) => a[prop])}
              onCommit={(v) => format({ [prop]: v })}
            />
          )
          return prop === 'size' && one ? (
            <Bindable key={prop} editor={editor} id={one.id} prop={prop} title={title} label={<Icon name={label} />}>
              {field}
            </Bindable>
          ) : (
            <Fragment key={prop}>{field}</Fragment>
          )
        })}
      </div>
      <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => format({ textAlign })} />
    </Section>
  )
}

/** The document's text styles with their values, each bindable to a variable. */
export function TextStyles({ editor }: { editor: Editor }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  return (
    <Section title="Text styles" onAdd={() => editor.apply({ type: 'addTextStyle', name: nextName('Text style', styles.map((s) => s.name)) })}>
      {styles.map((s) => (
        <StyleRow key={s.id} editor={editor} style={s} />
      ))}
    </Section>
  )
}

function StyleRow({ editor, style }: { editor: Editor; style: TextStyle }) {
  const [open, setOpen] = useState(false)
  return (
    <div role="group" aria-label={style.name} className="text-style">
      <div className="row">
        <button type="button" className="icon-button chevron" data-open={open || undefined} aria-expanded={open} aria-label={`Edit ${style.name}`} title={open ? 'Close' : 'Edit'} onClick={() => setOpen(!open)}>
          <Icon name="chevron" />
        </button>
        <NameInput label="Text style name" value={style.name} onCommit={(name) => editor.apply({ type: 'setTextStyle', id: style.id, name })} />
        {open && <TypeOptions editor={editor} spans={[style]} set={(p) => editor.apply({ type: 'setTextStyle', id: style.id, ...p })} features={(f) => editor.apply({ type: 'setTextStyle', id: style.id, features: f(style.features) })} />}
        <button
          type="button"
          className="icon-button row-action"
          aria-label={`Delete text style ${style.name}`}
          title="Delete text style"
          onClick={() => editor.apply({ type: 'deleteTextStyle', id: style.id })}
        >
          <Icon name="minus" />
        </button>
      </div>
      {!open && <small className="text-style-sum">{`${style.font?.family ?? 'Source Serif 4'} ${style.font?.style ?? 'Regular'} · ${style.size}/${style.lineHeight || 'auto'} pt`}</small>}
      {open && (
        <>
          <Font editor={editor} id={style.id} fonts={[style.font]} set={(font) => editor.apply({ type: 'setTextStyle', id: style.id, font })} />
          <div className="grid">
            {METRICS.map(([prop, title, label, unit, min, max, zero]) => (
              <Bindable key={prop} editor={editor} id={style.id} prop={prop} title={title} label={<Icon name={label} />}>
                <Field
                  label={<Icon name={label} />}
                  title={title}
                  unit={unit}
                  min={min}
                  max={max}
                  zero={zero}
                  reset={prop === 'size' ? undefined : 0}
                  value={style[prop]}
                  onCommit={(v) => editor.apply({ type: 'setTextStyle', id: style.id, [prop]: v })}
                />
              </Bindable>
            ))}
          </div>
        </>
      )}
    </div>
  )
}

/** Resizing, insets, columns, vertical alignment and baseline grid of text layers. */
export function TextFrameSection({ editor, nodes, set }: { editor: Editor; nodes: TextNode[]; set: (p: Props) => void }) {
  const same = <T,>(get: (n: TextNode) => T) => sameOf(nodes, get)
  const resizing = same((n) => (n.sizing.horizontal === 'hug' ? 'autoWidth' : n.sizing.vertical === 'hug' ? 'autoHeight' : 'fixedSize'))
  const ruled = same((n) => JSON.stringify(n.columnRule?.weight ? n.columnRule : null))
  const rule: TextNode['columnRule'] | null | undefined = ruled === null ? undefined : JSON.parse(ruled)
  const insets = INSETS.map((p) => same((n) => n[p]))
  const ids = nodes.map((n) => n.id).join()
  const [each, toggle] = useSides('inset', ids, insets)
  const columns = same((n) => n.columns)
  const columned = nodes.every((n) => n.columns > 1)
  const used = nodes.some((n) => n.columns > 1 || INSETS.some((p) => n[p]) || !!(n.baselineGrid || n.baselineStart || n.minHeight || n.maxLines || n.trim || n.roundBaselines))
  const [shown, moreButton] = useMore('Frame options', 'Inset, columns and baseline grid', used)
  return (
    <Section
      title="Text frame"
      actions={
        <>
          {shown && toggle}
          {moreButton}
        </>
      }
    >
      <Segmented
        label="Resizing"
        text
        value={resizing}
        options={RESIZING}
        disabled={(v) => nodes.some((n) => (v === 'autoWidth' && !!(n.prev || n.next)) || (v === 'autoHeight' && !!n.next))}
        onChange={(mode) => editor.resize(nodes, mode)}
      />
      <Segmented label="Vertical align" value={same((n) => n.verticalAlign)} options={VERTICAL} onChange={(verticalAlign) => set({ verticalAlign })} />
      {shown && (
        <>
          <Sides what="inset" values={insets} each={each} set={(v) => set(Object.fromEntries(INSETS.flatMap((p, i) => (v[i] === null ? [] : [[p, v[i]]]))))} />
          <div className="grid">
            <Field label="Columns" title="Columns" unit="" int min={1} max={20} reset={1} value={columns} onCommit={(columns) => set({ columns })} />
            {columned && <Field label="Gutter" title="Gutter" unit="length" value={same((n) => n.gutter)} onCommit={(gutter) => set({ gutter })} />}
          </div>
          {columned && (
            <ColorCheck
              editor={editor}
              node={nodes[0]}
              label="Column rule"
              color={rule && rule.color}
              set={(color) => set({ columnRule: color ? { weight: 1, color } : { ...rule!, weight: 0 } })}
            >
              {rule && <Field label="Weight" title="Column rule weight" unit="pt" value={rule.weight} onCommit={(weight) => set({ columnRule: { ...rule, weight } })} />}
            </ColorCheck>
          )}
          <div className="grid">
            <Field label="Baseline" title="Baseline grid" unit="pt" min={1} zero="Off" reset={0} value={same((n) => n.baselineGrid)} onCommit={(baselineGrid) => set({ baselineGrid })} />
            <Field label="Start" title="Baseline grid start" unit="pt" reset={0} value={same((n) => n.baselineStart)} onCommit={(baselineStart) => set({ baselineStart })} />
            {resizing === 'autoHeight' && <Field label="Min" title="Min height" unit="length" zero="None" reset={0} value={same((n) => n.minHeight)} onCommit={(minHeight) => set({ minHeight })} />}
            <Field label="Lines" title="Max lines" unit="" int max={1000} zero="All" reset={0} value={same((n) => n.maxLines)} onCommit={(maxLines) => set({ maxLines })} />
          </div>
          <Check label="Trim to cap height" value={same((n) => n.trim)} set={(trim) => set({ trim })} />
          <Check label="Whole-point baselines" value={same((n) => n.roundBaselines)} set={(roundBaselines) => set({ roundBaselines })} />
        </>
      )}
    </Section>
  )
}

/** A check that turns a color on or off, and the color's picker while it is on; `undefined` is mixed. */
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
