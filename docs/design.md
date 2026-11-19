# UI design rules

Every UI change follows these; a rater's taste that contradicts them is ignored.
Scales, weights, line widths, shadows, durations, raw colours, grey contrast and action
titles are enforced by `web/src/design.test.ts` and `web/src/commands.test.ts`; behaviour by
`web/e2e`. This file holds only what no test checks.

## Principles

- The document is the content; the chrome is quiet. Colour belongs to the page, the UI
  is grey with one accent.
- One way per thing: one control per value, one look per role, one place per action.
- Flat: no gradients, no glows, no textures. Depth only for things that float.
- Show what is used, hide what is rare behind a section, a tab or a menu; never a
  second copy of the same control.
- A control appears only where it acts: the wrap offset only with a wrap, the preflight
  counter only with issues, the progress bar only while exporting, a setting where it
  takes effect (raster resolution in Export). A setting that cannot take effect is hidden
  (Gutter with 1 column); one that applies in one mode shows only in it (the profile only in CMYK).
- The align bar shows for any selection outside an auto layout; several layers align to
  their bounds, a single layer to the frame or page it sits in.
- Placeholders for an unknown value are an en dash (–). A title that does not fit ends in
  an ellipsis and carries the full name as its tooltip.
- Rare settings of a section sit behind the ⋯ button in its header, whose tooltip names
  what it holds ("Text wrap"); it stays open while one of them is set. Rare actions
  (arrange, flip, flatten, booleans, mask, tidy up) live in the context menu's Transform
  submenu, the panel's ⋯ menu and the palette, not in icon rows.
- Empty sections collapse to their title row; a collapsed empty section before a filled one
  ends with a separator. Mixed values say "Mixed" (a group mixes its Fill and Stroke), the
  explanation goes into the tooltip.
- Long option lists are grouped under short group titles, never one flat list.
- One entry point per action per screen: a dialog does not repeat a button of the screen
  behind it; an action a menu offers has no button beside it; a setting in a dialog and a
  panel is one shared control (colour mode, grid fields).
- Properties live only in the right panel, which shows only the selection and page. The
  left panel has 2 tabs: Layers and Assets (swatches, fonts, variables, text styles).
  Swatches show only in Assets, at a fixed height.
- Segmented controls use icons only where the icon is unambiguous (alignment, joins),
  otherwise short text; icon-only ones carry no prefix. An on/off setting is a checkbox.
- One control per value inside a popover too: opacity is the alpha slider, not also a field.
- A field prefix names the value ("Pages", "Opacity"), the same word wherever it shows.
- List item editors (text styles) open one item at a time; closed rows show name,
  preview and the edit action.
- Popovers close on Escape and a click outside and carry no close button; modal dialogs keep one.
- Paint rows always show their actions; layer rows show lock and eye on hover, a hidden or
  locked layer keeps its icon.
- An OpenType variant joins the select of its value (small caps in Case), not a checkbox.
- Groups inside a popover part by spacing and their heading, not by rules.
- A section's actions on its subject sit in header menus (`MenuButton`: + adds, ⋯ the
  rest), the same actions in the context menu and the palette; no rows of text buttons.
- A layer that only exists inside another (a table cell) shows only what applies to it;
  empty cells are named by address (A1).
- Text format keys work on the edited range and on selected text layers.
- A chevron always expands inline; a trigger that opens a popover uses another icon.
- Panel titles name the panel only. A section title names what it changes and sits beside
  its kin (the fill of text is "Text color", under Text), and does not repeat the label of
  the button it holds.
- Every panel section uses one 2-column grid; short controls share a row.
- The page overview hides the drawing tools and the spread navigation. Its left panel shows
  the layers of the one selected page or master, read only, or nothing.

## Roles

- Type: 11 captions and group labels (600, `--muted`), 12 body, fields, menus and section
  titles (600), 13 panel and popover titles (600), 16 dialog titles, 24 the projects title.
  Previews of document content use their own size.
- Space: panel content edge 12, field gap 8, icon button gap 4, popover gap 8, dialog
  inset 24, page inset 32, tree indent 16 per level, text inside a box 8 in. Every box in a
  panel sits on the 12 px edge. Negative margins only as the negative of a scale value.
  `web/shots/audit.spec.ts` measures this into `audit.txt`.
- Radius: 4 chips, keycaps, swatches, badges; 6 controls, rows, buttons, tooltips; 12
  popovers, menus, bars, cards, dialogs; 999 px counters; 50% dots and knobs.
- Height 28 for fields, selects, segmented controls, buttons, chips and menu items; icon
  buttons 24, centred in 28 rows.
- Colour: `--bg` panels, `--canvas` behind the pages, `--raised` popovers and cards,
  `--input` fields, `--border` separators and resting outlines, `--edge` hover outlines and
  floating surfaces. The accent is the text colour: focus, checked controls, primary action,
  the current page; text on it is `--bg`. `--good`, `--warn`, `--bad` carry meaning only.
  `--select` blue is the canvas selection, apart from the cyan guides, magenta snaps and red grid.
- Floating surfaces have a 1 px `--edge` border and `--shadow-pop` (menus, popovers,
  tooltips, bars) or `--shadow-dialog` (dialogs, palette). Focus rings are inset, outside
  only on accent-filled and small controls.
- Disabled controls keep their colours at `--dim` opacity.

## Components

- Selected: `--selected` background and `--text`, the same for rows, tabs and segments.
- Buttons: primary has the accent fill and 600 text, at most one per dialog or bar, last in
  its row. Secondary (`.button`) has the `--input` fill. Icon buttons have no fill until
  hover. Destructive actions use `--bad` text, never a red fill.
- Shortcuts: keycap chips in dialogs, the palette and tooltips, mouse actions too ("Ctrl"
  + "click"); plain `--muted` text in menus; buttons carry no keycap.
- Default values (Auto, None, All) in `--text`; only Mixed and search hints in `--muted`.
- Swatches: 16 px squares in lists, 28 px chips in paint rows, 1 px inset ring; stroke
  swatches are the same square as a ring.
- Paint rows: chip, main field, value field (fits "Tint 100 %"), eye, minus. Solid shows its
  hex, in CMYK its 4 percentages ("100 73 2 0"); swatch its name and Tint or opacity;
  gradient its type and angle; image a fit select and ppi. Gradient stops are markers on a
  bar in the picker, a click adds one; crop and adjust live in the image chip's popover.
- Icons: 16 px outline, 1.5 stroke, no filled glyphs.
- Fields: label inside on the left in `--muted`, value in `--text`, a unit other than the
  document unit in `--muted` on the right; steppers show the unit after the value.
- Labels: geometry fields a letter (X, Y, W, H, R), type metrics, opacity and angle an
  icon, every other field a word; one quantity has one label everywhere (◐ opacity). A
  select shows a prefix only when its value does not name the property; its whole value
  shows.
- Menus: 8 inset, radius 6 items in a radius 12 menu with 4 padding; the chosen item
  carries a check; separators 1 px `--edge` with 4 around. Rare commands (booleans, Reset
  to master) are left out when they cannot act.
- Popovers open beside their trigger and fit 640 px of height; longer content splits into
  sections or tabs. A dialog docked over the canvas moves the selection out from under it.
- Dialogs: actions at the bottom, the primary last on the right.
- Empty states: an empty view shows an icon, one sentence and the action that fills it;
  an empty panel section is a row with its title and its +.
- A button label does not repeat the value chosen beside it (Export, not Export PNG).
- Copy: sentence case, no trailing period on labels, US spelling, numbers as digits.
