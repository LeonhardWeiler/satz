import { Fragment } from 'react'
import { Field, NameInput, nextName, Section, Select } from './controls'
import { MM, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { range } from './textEdit'
import type { Attrs, Node, Props, Sizing, Styled, TextProps, TextStyle } from './model'
import { Bindable } from './Variables'

type TextNode = Extract<Node, { kind: 'text' }>

const ALIGNS = [
  ['left', 'Align left', 'alignLeft'],
  ['center', 'Align center', 'alignCenter'],
  ['right', 'Align right', 'alignRight'],
  ['justify', 'Justify', 'alignJustify'],
] as const
const INSETS = [
  ['insetTop', 'Top inset', 'T'],
  ['insetRight', 'Right inset', 'R'],
  ['insetBottom', 'Bottom inset', 'B'],
  ['insetLeft', 'Left inset', 'L'],
] as const
const VERTICAL = [
  ['top', 'Align top', 'alignTop'],
  ['center', 'Align middle', 'alignMiddle'],
  ['bottom', 'Align bottom', 'alignBottom'],
] as const
/** Figma's text resize modes as sizing: auto width hugs both sides, auto height the height. */
const RESIZING = [
  ['autoWidth', 'Auto width'],
  ['autoHeight', 'Auto height'],
  ['fixedSize', 'Fixed size'],
] as const
const LANGS = { en: 'English', de: 'German' } as const
/** Styled attributes: title, label, unit and the text shown for 0. */
const STYLED: [Styled, string, string, string, string?][] = [
  ['size', 'Font size', 'Size', 'pt'],
  ['lineHeight', 'Line height', 'Line', 'pt', 'Auto'],
  ['letterSpacing', 'Letter spacing', 'Track', '%'],
  ['paragraphSpacing', 'Paragraph spacing', 'Para', 'pt'],
]

/** Text style, type and alignment of a text layer. */
export function TextSection({ editor, node }: { editor: Editor; node: TextNode }) {
  const styles = useEditor(editor, (e) => e.snapshot.textStyles)
  const same = <T,>(get: (a: Attrs) => T): T | null => {
    const values = node.spans.map(get)
    return values.every((v) => v === values[0]) ? values[0] : null
  }
  const edited = useEditor(editor, (e) => (e.editing?.id === node.id && e.editing.anchor !== e.editing.focus ? e.editing : null))
  /** Formats the selection of the text being edited, or else all of it. */
  const format = (props: TextProps) => editor.apply({ type: 'format', id: node.id, range: edited && range(edited), ...props })
  const style = same((a) => a.textStyle)
  const options: Record<string, string> = { '': 'No style', ...Object.fromEntries(styles.map((s) => [s.id, s.name])) }
  if (style === null) options.mixed = 'Mixed'
  const create = () => {
    const a = node.spans[0]
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
  const { horizontal, vertical } = node.sizing
  const resizing = horizontal === 'hug' ? 'autoWidth' : vertical === 'hug' ? 'autoHeight' : 'fixedSize'
  const resize = (mode: (typeof RESIZING)[number][0]) => {
    const width = horizontal === 'hug' ? 'fixed' : horizontal
    const sizing: Sizing =
      mode === 'autoWidth' ? { horizontal: 'hug', vertical: 'hug' }
      : mode === 'autoHeight' ? { horizontal: width, vertical: 'hug' }
      : { horizontal: width, vertical: vertical === 'fill' ? 'fill' : 'fixed' }
    editor.apply({ type: 'set', id: node.id, sizing })
  }
  const align = same((a) => a.textAlign)
  const hyphenate = same((a) => a.hyphenate)
  const lang = same((a) => a.lang)
  const langs: Record<string, string> = { ...LANGS }
  if (lang === null) langs.mixed = 'Mixed'

  return (
    <Section title="Text">
      <div className="row">
        <Select label="Text style" value={style ?? 'mixed'} options={options} onChange={(textStyle) => format({ textStyle })} />
        <button type="button" className="icon-button" aria-label="Create text style" title="Create text style" onClick={create}>
          <Icon name="plus" size={16} />
        </button>
      </div>
      <div className="grid">
        {STYLED.map(([prop, title, label, unit, zero]) => {
          const field = (
            <Field
              label={label}
              title={`${title} in ${unit}`}
              unit={unit}
              zero={zero}
              value={same((a) => a[prop])}
              onCommit={(v) => format({ [prop]: v })}
            />
          )
          return prop === 'size' ? (
            <Bindable key={prop} editor={editor} id={node.id} prop={prop} title={`${title} in ${unit}`} label={label}>
              {field}
            </Bindable>
          ) : (
            <Fragment key={prop}>{field}</Fragment>
          )
        })}
      </div>
      <div role="radiogroup" aria-label="Text align" className="segmented">
        {ALIGNS.map(([value, title, icon]) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={align === value}
            aria-label={title}
            title={title}
            onClick={() => format({ textAlign: value })}
          >
            <Icon name={icon} size={16} />
          </button>
        ))}
      </div>
      <div role="radiogroup" aria-label="Resizing" className="segmented">
        {RESIZING.map(([value, title]) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={resizing === value}
            aria-label={title}
            title={title}
            onClick={() => resize(value)}
          >
            <Icon name={value} size={16} />
          </button>
        ))}
      </div>
      <div className="row">
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
          <Icon name="minus" size={16} />
        </button>
      </div>
      <div className="grid">
        {STYLED.map(([prop, title, label, unit, zero]) => (
          <Bindable key={prop} editor={editor} id={style.id} prop={prop} title={`${title} in ${unit}`} label={label}>
            <Field
              label={label}
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

/** Insets, columns, vertical alignment and baseline grid of a text layer. */
export function TextFrameSection({ node, set }: { node: TextNode; set: (p: Props) => void }) {
  return (
    <Section title="Text frame">
      <div className="grid">
        {INSETS.map(([prop, title, label]) => (
          <Field key={prop} label={label} title={`${title} in mm`} unit="mm" value={node[prop] / MM} onCommit={(v) => set({ [prop]: v * MM })} />
        ))}
        <Field label="Cols" title="Columns" unit="" value={node.columns} onCommit={(v) => set({ columns: Math.round(v) })} />
        <Field label="Gutter" title="Gutter in mm" unit="mm" value={node.gutter / MM} onCommit={(v) => set({ gutter: v * MM })} />
        <Field label="Grid" title="Baseline grid in pt" unit="pt" zero="Off" value={node.baselineGrid} onCommit={(baselineGrid) => set({ baselineGrid })} />
        <Field label="Start" title="Baseline grid start in pt" unit="pt" value={node.baselineStart} onCommit={(baselineStart) => set({ baselineStart })} />
      </div>
      <div role="radiogroup" aria-label="Vertical align" className="segmented">
        {VERTICAL.map(([value, title, icon]) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={node.verticalAlign === value}
            aria-label={title}
            title={title}
            onClick={() => set({ verticalAlign: value })}
          >
            <Icon name={icon} size={16} />
          </button>
        ))}
      </div>
    </Section>
  )
}
