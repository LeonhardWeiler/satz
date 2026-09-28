import { Fragment, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Field, NameInput, nextName, Section, Segmented, Select } from './controls'
import { useEditor, type Editor } from './editor'
import { Icon, type IconName } from './icons'
import { PAGE_NUMBER, insert, range } from './textEdit'
import type { Attrs, Props, Sizing, Styled, TextNode, TextProps, TextStyle } from './model'
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
/** Figma's text resize modes as sizing: auto width hugs both sides, auto height the height. */
const RESIZING = [
  ['autoWidth', 'Auto width', 'autoWidth'],
  ['autoHeight', 'Auto height', 'autoHeight'],
  ['fixedSize', 'Fixed size', 'fixedSize'],
] as const
const LANGS = { en: 'English', de: 'German' } as const
/** Styled attributes: title, label, unit and the text shown for 0. */
const STYLED: [Styled, string, IconName, string, string?][] = [
  ['size', 'Font size', 'fontSize', 'pt'],
  ['lineHeight', 'Line height', 'lineHeight', 'pt', 'Auto'],
  ['letterSpacing', 'Letter spacing', 'letterSpacing', '%'],
  ['paragraphSpacing', 'Paragraph spacing', 'paragraphSpacing', 'pt'],
]

/** The text style's name set at its size, within what a panel row takes. */
const specimen = (s?: TextStyle) => (s ? { fontSize: Math.min(Math.max(s.size * 0.95, 9), 22), fontFamily: 'var(--doc-font)' } : undefined)

/** The value all spans share, or null. */
export function sameOf<T>(spans: Attrs[], get: (a: Attrs) => T): T | null {
  const values = spans.map(get)
  return values.every((v) => v === values[0]) ? values[0] : null
}

/** A button with the text style's name set at its size that opens a menu of the styles. */
export function Specimen({ editor, style, onPick }: { editor: Editor; style: string | null; onPick: (id: string) => void }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  const current = styles.find((s) => s.id === style)
  const [menu, setMenu] = useState<DOMRect | null>(null)
  if (!styles.length) return null
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
              ['No style', () => onPick(''), true, style === ''],
              ...styles.map((s): [ReactNode, () => void, boolean, boolean] => [
                <span style={specimen(s)}>{s.name}</span>,
                () => onPick(s.id),
                true,
                style === s.id,
              ]),
            ]}
          />,
          document.body,
        )}
    </>
  )
}

/** Text style, type and alignment of a text layer. */
export function TextSection({ editor, node }: { editor: Editor; node: TextNode }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  const fonts = useEditor(editor, (e) => e.snapshot.fonts)
  const missing = useEditor(editor, (e) => e.snapshot.missingFonts)
  const spans = useEditor(editor, (e) => e.snapshot.stories[node.story])?.spans ?? []
  const same = <T,>(get: (a: Attrs) => T) => sameOf(spans, get)
  const edited = useEditor(editor, (e) => (e.editing?.id === node.id && e.editing.anchor !== e.editing.focus ? e.editing : null))
  const editing = useEditor(editor, (e) => e.editing?.id === node.id)
  /** Formats the selection of the text being edited, or else all of it. */
  const format = (props: TextProps) => editor.apply({ type: 'format', id: node.id, range: edited && range(edited), ...props })
  const style = same((a) => a.textStyle)
  const create = () => {
    const a = spans[0]
    const [id] = editor.apply({
      type: 'addTextStyle',
      name: nextName('Text style', styles.map((s) => s.name)),
      size: a.size,
      lineHeight: a.lineHeight,
      letterSpacing: a.letterSpacing,
      paragraphSpacing: a.paragraphSpacing,
    })
    format({ textStyle: id })
  }
  const align = same((a) => a.textAlign)
  const font = same((a) => (a.font ?? fonts[0]).hash)
  const faces = [...fonts, ...missing.map((m) => m.font)]
  const fontNames: Record<string, string> = Object.fromEntries(faces.map((f) => [f.hash, f.name]))
  const hyphenate = same((a) => a.hyphenate)
  const lang = same((a) => a.lang)
  const langs: Record<string, string> = { ...LANGS }
  if (lang === null) langs.mixed = 'Mixed'

  return (
    <Section title="Text">
      <div className="row">
        <Specimen editor={editor} style={style} onPick={(textStyle) => format({ textStyle })} />
        <button
          type="button"
          className={styles.length ? 'icon-button' : 'button add'}
          aria-label="Create text style"
          title="Create text style"
          onClick={create}
        >
          <Icon name="plus" />
          {!styles.length && 'Text style'}
        </button>
        {editing && (
          <button
            type="button"
            className="icon-button page-number"
            aria-label="Insert page number"
            title="Insert page number (Ctrl+Alt+Shift+N)"
            // Keeps the focus in the edited text.
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => insert(editor, PAGE_NUMBER)}
          >
            <Icon name="pageNumber" />
          </button>
        )}
      </div>
      <Bindable editor={editor} id={node.id} prop="font" title="Font" label={<Icon name="text" />}>
        <Select
          label="Font"
          value={font}
          options={fontNames}
          disabled={missing.map((m) => m.font.hash)}
          onChange={(hash) => format({ font: faces.find((f) => f.hash === hash) })}
        />
      </Bindable>
      <div className="grid">
        {STYLED.map(([prop, title, label, unit, zero]) => {
          const field = (
            <Field
              label={<Icon name={label} />}
              title={`${title} in ${unit}`}
              unit={unit}
              zero={zero}
              value={same((a) => a[prop])}
              onCommit={(v) => format({ [prop]: v })}
            />
          )
          return prop === 'size' ? (
            <Bindable key={prop} editor={editor} id={node.id} prop={prop} title={`${title} in ${unit}`} label={<Icon name={label} />}>
              {field}
            </Bindable>
          ) : (
            <Fragment key={prop}>{field}</Fragment>
          )
        })}
      </div>
      <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => format({ textAlign })} />
      <div className="grid">
        <label className="check">
          <input
            type="checkbox"
            checked={hyphenate === true}
            ref={(el) => {
              if (el) el.indeterminate = hyphenate === null
            }}
            onChange={(e) => format({ hyphenate: e.currentTarget.checked })}
          />
          Hyphenate
        </label>
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

/** The document's text styles with their values, each bindable to a number variable. */
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
      <div className="grid">
        {STYLED.map(([prop, title, label, unit, zero]) => (
          <Bindable key={prop} editor={editor} id={style.id} prop={prop} title={`${title} in ${unit}`} label={<Icon name={label} />}>
            <Field
              label={<Icon name={label} />}
              title={`${title} in ${unit}`}
              unit={unit}
              zero={zero}
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
export function TextFrameSection({ node, set }: { node: TextNode; set: (p: Props) => void }) {
  const { horizontal, vertical } = node.sizing
  const resizing = horizontal === 'hug' ? 'autoWidth' : vertical === 'hug' ? 'autoHeight' : 'fixedSize'
  const resize = (mode: (typeof RESIZING)[number][0]) => {
    const width = horizontal === 'hug' ? 'fixed' : horizontal
    const sizing: Sizing =
      mode === 'autoWidth' ? { horizontal: 'hug', vertical: 'hug' }
      : mode === 'autoHeight' ? { horizontal: width, vertical: 'hug' }
      : { horizontal: width, vertical: vertical === 'fill' ? 'fill' : 'fixed' }
    set({ sizing })
  }
  return (
    <Section title="Text frame">
      <Segmented
        label="Resizing"
        value={resizing}
        options={RESIZING}
        disabled={(v) => (v === 'autoWidth' && !!(node.prev || node.next)) || (v === 'autoHeight' && !!node.next)}
        onChange={resize}
      />
      <div className="grid">
        {INSETS.map(([prop, title, label]) => (
          <Field key={prop} label={<Icon name={label} />} title={`${title} in mm`} unit="mm" value={node[prop]} onCommit={(v) => set({ [prop]: v })} />
        ))}
        <Field label={<Icon name="columns" />} title="Columns" unit="" int value={node.columns} onCommit={(v) => set({ columns: Math.round(v) })} />
        <Field label={<Icon name="gutter" />} title="Gutter in mm" unit="mm" value={node.gutter} onCommit={(gutter) => set({ gutter })} />
        <Field label={<Icon name="baselineGrid" />} title="Baseline grid in pt" unit="pt" zero="Off" value={node.baselineGrid} onCommit={(baselineGrid) => set({ baselineGrid })} />
        <Field label={<Icon name="baselineStart" />} title="Baseline grid start in pt" unit="pt" value={node.baselineStart} onCommit={(baselineStart) => set({ baselineStart })} />
      </div>
      <Segmented label="Vertical align" value={node.verticalAlign} options={VERTICAL} onChange={(verticalAlign) => set({ verticalAlign })} />
    </Section>
  )
}
