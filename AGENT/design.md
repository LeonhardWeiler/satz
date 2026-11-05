# UI design rules

Every UI change follows these; a rater's taste that contradicts them is ignored.

## Tokens

- Type: 11 (captions, section labels in lists), 12 (body, fields, menus), 13 (panel and popover titles), 16 (dialog titles). Weights 400 and 600 only.
- Space: 2, 4, 8, 12, 16, 24. Panel inset 16, field gap 8.
- Radius: 4 (chips, keycaps, swatches), 6 (controls, rows), 8 (popovers, cards), 12 (dialogs).
- Control height 28; icon buttons 28 in panels, 24 inside fields and rows.
- Greys: `--text` values, `--text-2` labels, `--text-3` placeholders, units and disabled values. Never two of them for one role.
- One accent for selection, focus, primary action and links. Masters, spots and snaps are not accents: masters use text colours, snap guides magenta only on the canvas.

## Components

- Selected: `--selected` background, text in `--text`; accent only for the focus ring, the current page and checked controls.
- Shortcuts: keycap chips in dialogs, the palette, tooltips and buttons; plain `--text-3` text in menus. Mouse actions in the help read as words after the chips ("Ctrl" chip + "click").
- Swatches: 16 px squares, radius 4, 1 px inset ring; stroke swatches are the same square as a ring.
- Icons: 16 px outline, 1.5 stroke, no filled glyphs.
- Fields: label inside on the left in `--text-2`, value in `--text`, unit in `--text-3` on the right; steppers keep the unit before the minus.
- Popovers open beside their trigger and fit 640 px of height; longer content is split into sections or tabs.
- Empty states: an icon, one sentence and the action that fills them.

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
