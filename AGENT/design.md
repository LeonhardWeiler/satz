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
- Rare settings of a section sit behind the ⋯ button in its header, whose tooltip names
  what it holds ("Text wrap", "Section numbering"); it stays open while one of them is set. Rare actions (arrange, flip, flatten, booleans) live in the panel's
  ⋯ menu and the palette, not in icon rows.
- Empty sections collapse to their title row; mixed values say "Mixed", the
  explanation goes into the tooltip.
- Long option lists are grouped under short group titles, never one flat list.
- One entry point per action per screen: a dialog does not repeat a button of the
  screen behind it.
- Properties live only in the right panel. The mask toggle, flatten, tidy up and booleans live in
  the context menu's Transform submenu and the command palette. The left panel has 2 tabs: Layers and
  Assets (swatches, fonts, variables, text styles); the right panel shows only the selection and page.
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
- Row actions (show, remove) are always shown in paint rows; layer rows show lock and eye
  on hover, a hidden or locked layer keeps its icon. Icon-only segmented controls carry no prefix.
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
- A section is titled for what it changes and sits beside its kin: the fill of text layers
  is "Text color", directly under Text.
- Short controls of one section share a grid row before a new row starts (Columns and
  vertical align).
- The page overview hides the drawing tools and the spread navigation; neither acts there.
- A section title does not repeat the label of the button it holds (Output, not Export
  above an Export button).
- Every panel section uses one 2-column grid (X Y, W H, rotation radius); keep aspect
  ratio is a toggle in the Layout header.
- Group headings inside a section or popover are labels (11, `--muted`), never as loud
  as a section title.

## Tokens

- Type: 11 (captions, section labels in lists, badges), 12 (body, fields, menus), 13
  (panel and popover titles, the palette field), 16 (dialog titles), 24 (the projects
  page title). Weights 400 and 600 only. Previews of
  document content (glyphs, font specimens) use their own size; they are content.
- Space: 2, 4, 8, 12, 16, 24, 32 for padding, margin and gap. Panel content edge 12,
  field gap 8, icon button gap 4, popover gap 8, dialog inset 24, page inset 32. Negative margins only as the negative
  of a scale value, to line text up with its neighbours. Tree indents are 16 per level.
- Radius: 4 (chips, keycaps, swatches, badges, colour areas), 6 (controls, rows, buttons, tooltips),
  12 (popovers, menus, bars, cards, dialogs), 999 px (counters), 50% (dots and knobs).
- Control height 28 (fields, selects, segmented, buttons, colour chips); icon buttons 24,
  centred in 28 rows.
- Edges: every box in a panel (field, select, tabs, rows, icon buttons) sits on the 12 px
  content edge, left and right; no optical overhang. Text inside a box starts 8 in.
- `web/shots/audit.spec.ts` measures edges, heights, text insets, fonts and gaps of every
  panel, popover and dialog into `audit.txt` beside the shots
  (`FULL=1 pnpm exec playwright test -c shots/playwright.config.ts --project=light shots/audit.spec.ts`).
- Colour roles, defined once in `:root` per theme: `--bg` panels, `--canvas` behind
  the pages, `--raised` popovers and cards, `--input` fields, `--hover`, `--selected`,
  `--border` separators and resting outlines, `--edge` hover outlines and floating surfaces,
  `--accent`, `--select`, `--good`, `--bad`, `--warn`. Text on the accent is `--bg`. No raw colours in rules
  except paper white, masks, the checker, the colour picker's scales and the process
  inks of the plates.
- Greys: `--text` values, `--muted` labels, placeholders and units. Disabled controls
  keep their colours at `--dim` opacity, set by one rule.
- Greys are neutral, without a tint; dark panels #141414 with #f5f5f5 text, the canvas near black.
- The UI accent is the text colour (white in dark, black in light): focus, checked controls, primary action, the current page. Colour only carries meaning: `--good` green, `--warn` yellow, `--bad` red.
- The canvas has its own colour for selection chrome, `--select` blue (#4d7cff), apart from the cyan guides and bleed, the magenta snaps and the red grid; white would vanish on paper.
- Elevation: flat surfaces have no shadow. Floating surfaces have a 1 px `--edge`
  border and one of 2 shadows: `--shadow-pop` (menus, popovers, tooltips, bars over the
  canvas) and `--shadow-dialog` (dialogs, palette).
- Lines: 1 px. Separators and control outlines at rest `--border`; control outlines on
  hover, floating surfaces and separators on them `--edge`; focus ring 1.5 px `--accent`.
- Motion: `--fast` (120 ms) for hover, press and colour changes, `--open` (160 ms) for
  popovers, dialogs and toasts, always with `--ease`; nothing longer except progress
  bars and the error shake. Pressed controls scale to 0.93. `prefers-reduced-motion`
  turns all of it off.

## Components

- Selected: `--selected` background, text in `--text`, the same for rows, tabs and
  segmented controls; accent only for the focus ring, the current page and checked controls.
- Buttons: radius 6, 12 type. Primary: 28 high, accent fill, `--bg` text in weight 600,
  at most one per dialog or bar, last in its row. Secondary (`.button`): 28 high, `--input` fill, outlined as a control. Icon
  buttons have no fill until hover. Destructive actions use `--bad` text, never a red
  fill.
- Shortcuts: filled keycap chips in dialogs, the palette and tooltips, mouse actions too ("Ctrl" + "click"); plain `--muted` text in menus. Buttons carry no keycap; Enter on a primary button is implied.
- Default values (Auto, None, All, No style) in `--text`; only Mixed and search hints in `--muted`.
- Swatches: 16 px squares in lists, 28 px chips in paint rows, radius 4, 1 px inset ring; stroke swatches are the same square as a ring.
- Paint rows: the swatch and one value field (Opacity for solid, Angle for linear, Tint for a swatch). The paint type and gradient stops live in the picker; stops are markers on a gradient bar, a click on the bar adds one.
- Icons: 16 px outline, 1.5 stroke, no filled glyphs.
- Fields: label inside on the left in `--muted`, value in `--text`, a unit other than the document unit in `--muted` on the right; steppers show the unit after the value, before the plus.
- Labels: geometry fields take a letter (X, Y, W, H, R), type metrics an icon, every other field a word; one quantity has one label everywhere (Opacity). A select shows a prefix only when its value alone does not name the property (None, Keep, 300 ppi); values that name themselves (RGB, PDF/X-4, FOGRA51, Normal, a dash) carry none.
- Titles: panel and popover titles 13 in weight 600, `--text`; section titles 12 in weight
  600, `--text`; group labels inside lists and the overview 11 in weight 600, `--muted`;
  sentence case, no capitals.
- Menus: 28 high items, 8 inset, radius 6 inside a radius 12 menu with 4 padding; the
  chosen item carries a check, not a heavier weight;
  separators are 1 px `--edge` with 4 space around.
- Popovers open beside their trigger and fit 640 px of height; longer content is split into sections or tabs.
- Dialogs: 24 inset, title 16 in weight 600, actions at the bottom with the primary last
  on the right.
- Empty states: a whole empty view shows an icon, one sentence and the action that fills it; an empty panel section is a row like the others, its title on the left and its + on the right.
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
8. Example content: text frame without inset, empty masters, missing shadow in the shot: r5–r7.
