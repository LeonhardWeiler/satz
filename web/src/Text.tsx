import { Fragment } from 'react'
import { Field, NameInput, nextName, Section, Select } from './controls'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Attrs, Node, Styled, TextProps, TextStyle } from './model'
import { Bindable } from './Variables'

type TextNode = Extract<Node, { kind: 'text' }>

const ALIGNS = [
  ['left', 'Align left', 'alignLeft'],
  ['center', 'Align center', 'alignCenter'],
  ['right', 'Align right', 'alignRight'],
  ['justify', 'Justify', 'alignJustify'],
] as const
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
  const format = (props: TextProps) => editor.apply({ type: 'format', id: node.id, range: null, ...props })
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
  const align = same((a) => a.textAlign)

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
      <textarea
        className="content"
        aria-label="Text content"
        rows={6}
        defaultValue={node.text}
        key={`${node.id} ${node.text}`}
        onBlur={(e) => {
          if (e.currentTarget.value !== node.text) editor.apply({ type: 'setText', id: node.id, text: e.currentTarget.value })
        }}
      />
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
