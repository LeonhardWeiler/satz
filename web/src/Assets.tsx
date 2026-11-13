import { useState } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Section } from './controls'
import { useEditor, type Editor } from './editor'
import { addFonts, canFindFonts, findFonts, readLocalFonts, relink, removeFont, useLocalFonts } from './file'
import { Icon } from './icons'
import { TextStyles } from './Text'
import { Variables } from './Variables'

export function Assets({ editor }: { editor: Editor }) {
  const fonts = useEditor(editor, (e) => e.snapshot.fonts)
  const missingFonts = useEditor(editor, (e) => e.snapshot.missingFonts)
  const locals = useLocalFonts()
  const [fontMenu, setFontMenu] = useState<DOMRect | null>(null)
  return (
    <div className="panel assets" role="tabpanel" aria-label="Assets" onPointerDown={editor.gesture}>
      <Section title="Fonts" onAdd={(e) => (canFindFonts && !locals.length ? setFontMenu(e.currentTarget.getBoundingClientRect()) : addFonts(editor))}>
        {fontMenu &&
          createPortal(
            <ContextMenu
              anchor={() => fontMenu}
              side="bottom"
              label="Add font"
              onClose={() => setFontMenu(null)}
              items={[
                ['Font files…', () => addFonts(editor), true],
                ['Use local fonts', () => readLocalFonts(editor), true],
              ]}
            />,
            document.body,
          )}
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
          <button type="button" className="button" onClick={() => findFonts(editor)}>
            Find missing fonts on this computer
          </button>
        )}
      </Section>
      <LinkedImages editor={editor} />
      <Variables editor={editor} />
      <TextStyles editor={editor} />
    </div>
  )
}

/** The images placed as links to their files, those not read with a button to read them again, and one that embeds the read ones. */
function LinkedImages({ editor }: { editor: Editor }) {
  const { links, images } = useEditor(editor, (e) => e.snapshot)
  const missing = Object.keys(links).filter((h) => !(h in images))
  const read = Object.keys(links).filter((h) => !missing.includes(h))
  if (!Object.keys(links).length) return null
  return (
    <Section title="Linked images">
      <ul className="fonts">
        {Object.entries(links).map(([hash, name]) => (
          <li key={hash} className={missing.includes(hash) ? 'missing' : undefined} title={missing.includes(hash) ? 'Missing: the layers it fills show empty' : undefined}>
            <span>{name}</span>
            {missing.includes(hash) && (
              <button type="button" className="icon-button" aria-label={`Relink ${name}`} title="Relink" onClick={() => relink(editor, hash)}>
                <Icon name="folder" />
              </button>
            )}
          </li>
        ))}
      </ul>
      {read.length > 0 && (
        <button type="button" className="button" onClick={() => editor.apply({ type: 'embed', images: read })}>
          Embed linked images
        </button>
      )}
    </Section>
  )
}
