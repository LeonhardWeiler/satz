# UI design rules

Every UI change follows these; a rater's taste that contradicts them is ignored.
`web/src/design.test.ts` checks the token rules on `index.css`; a value outside them
fails vitest.

## Principles

- The document is the content; the chrome is quiet. Colour belongs to the page, the UI
  is grey with one accent.
- One way per thing: one control per value, one look per role, one place per action.
- Flat: no gradients, no glows, no textures. Depth only for things that float.
- Show what is used, hide what is rare behind a section, a tab or a menu; never a
  second copy of the same control.
- Calm motion: short fades and a slight press, nothing moves on its own.
- A control appears only where it acts: the align bar only for several layers, the
  wrap offset only with a wrap, the preflight counter only with issues, the progress
  bar only while exporting, a setting where it takes effect (raster resolution in
  Export).
- Rare settings of a section sit behind a disclosure named for what it holds ("Text wrap",
  "Baseline grid", "Section"), never a generic "More options"; it opens by itself when one
  of them is set. Rare actions (arrange, flip, flatten, booleans) live in the panel's
  ⋯ menu and the palette, not in icon rows.
- Empty sections collapse to their title row; mixed values say "Mixed", the
  explanation goes into the tooltip.
- Long option lists are grouped under short group titles, never one flat list.
- One entry point per action per screen: a dialog does not repeat a button of the
  screen behind it.
- The quick edit bar shows only while the right panel is hidden; both never edit the
  same value at once. The mask toggle lives in the ⋯ menu with the other arrange
  actions.
- Segmented controls use icons only where the icon is unambiguous (alignment, joins);
  otherwise they show short text labels.
- One control per value inside a popover too: opacity is the alpha slider, not also a
  field.
- A field prefix names the value ("Pages", "Opacity"), the same word wherever it shows.
- A collapsed empty section before a filled one ends with a separator, so a run of
  empty titles reads as one block.
- Editors of list items (text styles) open one item at a time; closed rows show only
  name, preview and the edit action.
- Popovers close on Escape and a click outside; they carry no close button. Modal
  dialogs keep one.
- Row actions (show, remove) appear on hover or focus of their row; a hidden item keeps
  its crossed eye visible. Icon-only segmented controls carry no prefix.
- An OpenType variant of a value joins the select of that value (small caps in Case,
  superscript glyphs in Position) instead of a checkbox beside it.
- An action that a menu already offers has no button beside that menu (New text style
  lives in the text style menu).
- A setting that cannot take effect is hidden: Gutter with 1 column, Text wrap on a page
  without text, the spread arrows and the page count of a single spread.
- Groups inside a popover part by spacing and their heading, not by rules.
- Fields in the document unit show no unit suffix; only a value in another unit names it.
- One control type per setting everywhere: an on/off setting is a checkbox (Facing
  pages), not a two-item segmented control.
- A setting that only applies in one mode shows only in it (the profile only in CMYK).
- A chevron always expands inline; a trigger that opens a popover uses another icon.
- Panel titles name the panel only; the current page is shown by the spread navigation,
  not repeated in the layers title.
- A section is titled for what it changes: the fill of text layers is "Text color".
- Short controls of one section share a grid row before a new row starts (Columns and
  vertical align).
- The page overview hides the drawing tools and the spread navigation; neither acts there.
- A section title does not repeat the label of the button it holds (Output, not Export
  above an Export button).

## Tokens

- Type: 11 (captions, section labels in lists, badges), 12 (body, fields, menus), 13
  (panel and popover titles, the palette field), 16 (dialog titles), 24 (the projects
  page title). Weights 400 and 600 only. Previews of
  document content (glyphs, font specimens) use their own size; they are content.
- Space: 2, 4, 8, 12, 16, 24, 32, 48 for padding, margin and gap. Panel inset 16,
  field gap 8, dialog inset 24, page inset 48. Negative margins only as the negative
  of a scale value, to line text up with its neighbours. Tree indents are 16 per level.
- Radius: 2 (marks under 8 px), 4 (chips, keycaps, swatches, badges, colour areas), 6 (controls,
  rows, buttons), 8 (popovers, menus, bars, cards), 12 (dialogs), 999 px (counters),
  50% (dots and knobs).
- Control height 28; icon buttons 28 in panels, 24 inside fields and rows.
- Colour roles, defined once in `:root` per theme: `--bg` panels, `--canvas` behind
  the pages, `--raised` popovers and cards, `--input` fields, `--hover`, `--selected`,
  `--border` separators and resting outlines, `--edge` hover outlines and floating surfaces,
  `--accent`, `--bad`, `--warn`. Text on the accent is `--bg`. No raw colours in rules
  except paper white, masks, the checker, the colour picker's scales and the process
  inks of the plates.
- Greys: `--text` values, `--text-2` labels, `--text-3` placeholders, units and disabled values. Never two of them for one role.
- One accent for selection, focus, primary action and links. Masters, spots and snaps are not accents: masters use text colours, snap guides magenta only on the canvas.
- Elevation: flat surfaces have no shadow. Floating surfaces have a 1 px `--edge`
  border and one of 2 shadows: `--shadow-pop` (menus, popovers, tooltips, bars over the
  canvas) and `--shadow-dialog` (dialogs, palette).
- Lines: 1 px. Separators and control outlines at rest `--border`; control outlines on
  hover, floating surfaces and separators on them `--edge`; focus ring 2 px `--accent`.
- Motion: `--fast` (120 ms) for hover, press and colour changes, `--open` (160 ms) for
  popovers, dialogs and toasts, always with `--ease`; nothing longer except progress
  bars and the error shake. Pressed controls scale to 0.93. `prefers-reduced-motion`
  turns all of it off.

## Components

- Selected: `--selected` background, text in `--text`; accent only for the focus ring, the current page and checked controls.
- Buttons: radius 6, 12 type. Primary: 28 high, accent fill, `--bg` text in weight 600,
  at most one per dialog or bar, last in its row. Secondary (`.button`): 24 high in
  panels and rows, 28 in bars and dialogs, `--input` fill, outlined as a control. Icon
  buttons have no fill until hover. Destructive actions use `--bad` text, never a red
  fill.
- Shortcuts: filled keycap chips in dialogs, the palette and tooltips, mouse actions too ("Ctrl" + "click"); plain `--text-3` text in menus. Buttons carry no keycap; Enter on a primary button is implied.
- Default values (Auto, None, All, No style) in `--text`; only Mixed and search hints in `--text-3`.
- Swatches: 16 px squares, radius 4, 1 px inset ring; stroke swatches are the same square as a ring.
- Icons: 16 px outline, 1.5 stroke, no filled glyphs.
- Fields: label inside on the left in `--text-2`, value in `--text`, a unit other than the document unit in `--text-3` on the right; steppers show the unit after the value, before the plus.
- Titles: panel and popover titles 13 in weight 600, `--text`; section titles 12 in weight
  600, `--text`; group labels inside lists and the overview 11 in weight 600, `--text-2`;
  sentence case, no capitals.
- Menus: 28 high items, 8 inset, radius 6 inside a radius 8 menu with 4 padding;
  separators are 1 px `--edge` with 4 space around.
- Popovers open beside their trigger and fit 640 px of height; longer content is split into sections or tabs.
- Dialogs: 24 inset, title 16 in weight 600, actions at the bottom with the primary last
  on the right.
- Empty states: a whole empty view shows an icon, one sentence and the action that fills it; an empty panel section shows nothing below its title and its +.
- One entry per action: adding a page lives in the pages panel header, not again as a slot; a button label does not repeat the value chosen beside it (Export, not Export PNG); a rare arrangement such as Tidy up sits in the ⋯ menu.
- Copy: sentence case everywhere, no trailing period on labels, US spelling, numbers
  as digits.

## Recurring rater findings (r2–r7), by cause

1. Inconsistent system (keycaps, selected states, title sizes, swatches, greys, icons): every round.
2. Properties panel: unlabeled icon rows, long panel, text styles fully open: every round.
3. Type options popover ~900 px tall, mixed control types (drop cap): r4–r7.
4. Projects: examples as a list next to thumbnail cards, weak empty state: r4–r7.
5. Left panel: dead space under layers, swatches squeezed at the bottom: r5–r7.
6. Canvas: text edit mode not distinct, no caret, labels over handles: every round.
7. Colour picker: opacity twice, hex without #: r5–r7.
8. Quick bar: borders, unclear stroke swatch, mixed labels: every round.
9. Example content: text frame without inset, empty masters, missing shadow in the shot: r5–r7.
