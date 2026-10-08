# UI/UX overhaul

From a review of all 237 screenshots of `web/shots` (`/tmp/satz-shots/<folder>/<nr>`, the same as the Figma file).
Each item: problem → proposal. Numbers like `05/21` name folder and shot.
Not checked: live interaction, hover states and tooltips beyond the shots, keyboard navigation; alignment was judged by eye on the 2× shots, not measured.

## Priorities

- [ ] **Unlabeled fields** (Hug/Hug, Fixed/Fixed, Left/Top, shadow X/Y, bare `%` for fill and layer opacity, auto size icons, gradient stops `0 % | 100 %`) → inline prefix like `W`/`H`/`Limit`: `W Hug`, `H Hug`, `Opacity 100 %`, `Pos 0 % · Alpha 100 %`.
- [ ] **`−` means 4 things** in the quick bar (no stroke, add, remove, unclear) → empty stroke slot shows a struck-through stroke swatch, `+` adds, removing only in the panel.
- [ ] **Dropdowns are always ~425 px wide**; in the right column, and in the quick fill and fit menus, they open shifted → min width = trigger width, aligned to the trigger edge.
- [ ] **Stroke block layout differs per type** (rectangle, polygon/star, 2 shapes, 5 layers) → 1 fixed 2-column grid: `Weight | Style`, `Align | Joins`, `Caps | Start/End`, `Overprint`.
- [ ] **Text sections come after Fill, Stroke, Effects** → for text frames put Text and Text frame right after Layout.
- [ ] **Layers: selection and hover share one grey** → selection in the accent colour.
- [ ] **No feedback for overset, threads, overrides** → overset badge `+42 words`, connection line between threaded frames, loaded cursor while threading, override marker plus `Reset to master`.
- [ ] **New document is silently CMYK** → fields `Colour RGB/CMYK` and `Bleed 3 mm` under the formats.
- [ ] **Colour pickers without swatches** (shadow, gradient stop, quick fill with only None/Black/White) → one picker everywhere with Swatches | Custom | Variables.
- [ ] **Magenta is overloaded** (snapping, distances, master banner, master letters) → magenta only for measuring/snapping; masters get their own colour and `B · Body`.
- [ ] **Rectangle icon is an empty square** and looks like an unchecked checkbox (layers, properties header) → filled square or square with corner handles.
- [ ] **Shortcut notation varies** (`Ctrl+Shift+E` vs `Ctrl Shift E`, `⌘` before every palette command) → 1 keycap component everywhere, per platform.

## App shell

- [ ] Panel header says `Page` but shows document and page settings, partly twice → without selection 2 sections `Document` and `Page`, each setting once.
- [ ] RGB profile is static text, CMYK profile a dropdown → both `Profile ▾`, disabled when there is no choice.
- [ ] `N` is page count and polygon points → `Pages` and `Points`.
- [ ] Page navigation has no total → `3 / 12`.
- [ ] `>` turns into `+` on the last page → disable `>`, `+` separate and always visible.
- [ ] Page navigation width depends on the document → fixed width.
- [ ] Spread pill does not mark the current page → highlight the active half.
- [ ] Page does not re-centre when panels are hidden → re-centre if it was centred.
- [ ] Shift W gives no hint how to get the UI back → toast `Shift W shows the interface again`.
- [ ] Export tooltip says `Export PDF` though PNG/JPEG exist → `Export`.
- [ ] `List the fonts of this computer` too long → `Use local fonts`.
- [ ] Layout grids header has an extra icon → only `+` like the other sections.
- [ ] Dark/Light dropdown has no label → `Theme ▾`.
- [ ] Text style rows indented ~10 px → flush with swatch rows.
- [ ] Swatch values without C/M/Y/K labels → `C0 M55 Y100 K0` or tooltip.
- [ ] Swatches section has no chevron → collapsible like Layers.
- [ ] Auto layout children listed in reverse → list in layout direction.

## Dialogs

### New document
- [ ] Format names wrap onto 2 lines → shorter labels or wider tiles.
- [ ] Selected state depends on focus → own selected state with accent border.
- [ ] Custom thumbnail looks like A5 → dashed sheet with `+`.
- [ ] Button reads `Create Custom` → always `Create`.
- [ ] `Open file…` has a magnifier and sits under Examples → folder icon, secondary button bottom left.
- [ ] Esc opens the booklet → Esc only closes.
- [ ] `Examples` heading baseline is off.
- [ ] Poster description ends in an orphan word.

### Shortcuts
- [ ] Gestures (drag, scroll) shown as keycaps → italic text.
- [ ] Cryptic labels → full sentences.
- [ ] 2-line row has a tall keycap → keycap top-aligned, fixed height.
- [ ] Reset button shifts the header → reserve its space.
- [ ] Remapped shortcuts have no marker → dot plus per-row reset.
- [ ] Recording gives no Esc hint → `Press a key · Esc cancels`.
- [ ] Search matches not highlighted; dialog height jumps → highlight, fixed height.
- [ ] `100 %` has a stray space.

### Settings
- [ ] Focus lands on X → first field.
- [ ] `Layers: Of the page` unclear → `Layers panel shows: Current page / Whole document`.
- [ ] Checkbox rows built differently → one row layout.
- [ ] Modal too large for its content.

### Command palette
- [ ] `⌘` before every command → no icon or one per command.
- [ ] Group `Commands` appears twice.
- [ ] Ranking off, matches not highlighted → prefix matches first, matches bold.
- [ ] Shortcut `.` invisible; shortcuts are text → keycaps.

### Find & replace
- [ ] Matches not highlighted on the canvas.
- [ ] Fields have different widths.
- [ ] `Case` sticks to the next checkbox.
- [ ] `0 found` has no error state → red field border.
- [ ] `All` ambiguous → `Replace all`.
- [ ] `Any style` unclear → `Style: any`.

## Toolbar

- [ ] Eyedropper uses a pencil icon → eyedropper icon.
- [ ] Shape menu: no mark for the current shape, no shortcuts for Polygon/Star, opens offset.
- [ ] Path bar icons unclear, `Done` is text → tooltips, `Done` as primary button with `Enter` hint.
- [ ] Panel toggles sit among the tools → separator, move right to the help group.

## Canvas

- [ ] No W×H label while drawing shapes and text frames → live label under the cursor.
- [ ] 3 distance labels overlap; a label covers its line → labels on the line in a pill, offset on collision.
- [ ] Stray handles during marquee.
- [ ] Pen: no hint how to finish/close → `Enter finishes · click the start point closes`.
- [ ] Path edit: hovering a point gives no feedback (both shots identical); quick bar stays during path edit.
- [ ] New text frame has no inset; text touches the frame edge.
- [ ] Text frame has only corner handles → side handles for width alone.
- [ ] Text quick bar wider than the page column (`No style` select ~150 px) → style as icon button, font and weight in one select.
- [ ] Quick bar covers content above the frame; in 43 it sits on the guide → place below when there is no room above.
- [ ] Overset `+` covers the last glyph, no count.
- [ ] Threading: no loaded cursor, no connection line; in-port barely visible.
- [ ] Crop mode hard to tell apart; panel identical to normal mode (05/21 = 05/20) → dim outside, show image bounds, panel `Crop · Done`.
- [ ] Image: 2 kinds of handles (corner squares, white inner dots) unexplained.
- [ ] Placing an image shows an empty blue rectangle → image preview.
- [ ] Layout grid column stripes cross the header and bleed → clip to the margins.
- [ ] Square grid too faint.
- [ ] Dragging a guide: 2 unlabeled magenta labels → `↑ 6.7 mm · ↓ 11.3 mm`.
- [ ] Guide hover shows no change.
- [ ] Text wrap offset not shown → dashed outline.
- [ ] Overridden master layers have no marker on canvas or in layers.
- [ ] `Use as mask` is a checkbox for 1 object, a button for several → button everywhere.
- [ ] Multi selection fill shows one colour instead of `mixed`.
- [ ] Distribute icons look disabled for 2 objects (15) → hide instead of grey.

## Quick edit bar (12)

- [ ] `Inside` (group, mask) cryptic → `Align to: Selection/Parent` icon toggle with tooltip.
- [ ] `15 two shapes (boolean bar)` has no boolean ops → 4 boolean icons for 2+ shapes.
- [ ] Line with arrow (06) shows no arrow heads → `Start ▾ End ▾`.
- [ ] Stroke swatch (white square, black core) looks like a stop icon → ring swatch in the stroke colour.
- [ ] `≡ 1 pt`: icon reads as paragraph align; big gap between number and unit → stroke weight icon, unit right after the number.
- [ ] `N − 3 +`: number clipped and faint → `Points`, min width.
- [ ] Image (20–22): first item `−`, icons unlabeled; no `RGB → CMYK` hint in a CMYK document.
- [ ] Fit menu (30): no mark for the current option, empty indent → use the check column.
- [ ] Adjust (31): 3 steppers in 2 columns, one cell empty, no reset → 3 rows plus `Reset`.
- [ ] Quick fill (32): only None/Black/White → document swatches plus `Custom…`.
- [ ] Auto layout (18, 19): `Hug | Hug`, `Fixed | Fixed` unlabeled → `W Hug ▾ H Hug ▾`.
- [ ] Bar covers text (wrap object, image) → avoid content, allow placement below.

## Properties

- [ ] Arrow: caps/start/end/joins crossed → `Start ▾ End ▾` in 1 row.
- [ ] Ellipse: `Start °` next to `Sweep %` → both °; `Ratio` → `Inner radius`.
- [ ] Star: Ratio and N under Layout → own section `Shape`; link button far right.
- [ ] Shadow: X/Y look like position → `Offset X/Y`; colour and 40 % unlabeled.
- [ ] Stroke: 2 dropdowns `Solid` (paint, dash) → dash style with pattern preview; stroke icons unlabeled.
- [ ] Overprint always visible → only in CMYK documents.
- [ ] Constraints and sizing dropdowns unlabeled.
- [ ] Frame: `Clip content` next to the angle field → own row.
- [ ] Auto layout padding as T R / B L → horizontal/vertical, expandable to 4.
- [ ] Auto layout: `−` next to the direction toggle; `Packed` → `Spacing: Fixed / Space between`.
- [ ] Text frame header shows the first words → `Text frame`.
- [ ] Auto size icons unlabeled; inset icons tiny.
- [ ] Gutter `4.23 mm` → round.
- [ ] Baseline field shows `Off`.
- [ ] `Lines` and `Min` rows appear only sometimes, layout jumps → always show, disable when not applicable.
- [ ] Vertical align buttons full width.
- [ ] Letter spacing in %, spacing in pt.
- [ ] `#` button opens special characters → `Ω`.
- [ ] Image section has no `Replace` and `Fit`.
- [ ] Text wrap offsets labelled `X/Y`, row half empty → `Offset`.
- [ ] Field bound to a variable (51): chip shows `¿?` and drops the unit.
- [ ] Hexagon replaces the chevron only on hover; bound fill has no marker → one binding marker, always visible.
- [ ] `Click + to replace mixed fills` wordy → `Mixed — + replaces`.
- [ ] Mixed: Stroke still shows controls while Fill does not.
- [ ] Icon rows (align, flip, boolean) without group labels → tooltips and separators.
- [ ] Mask shape shows the group's props (03 = 04), not selectable or distinguishable.

## Popovers

- [ ] CMYK picker fields clipped (`M 73.`) → integers or wider fields.
- [ ] Blend modes: no group separators → separators after Darken, Lighten, Contrast.
- [ ] Format menu: `Custom` greyed out.
- [ ] Variable editor opens far from its row.
- [ ] Variable name and mode are text in one state, inputs in another.
- [ ] `Delete variable` weighs like `Add mode` → red, separated.
- [ ] Variable rows show a swatch or `−`.
- [ ] `auto (Mode 1)` lower case.
- [ ] `No number variables yet` → `Create from value`.
- [ ] Empty swatches tab only explains → button `Save as swatch`.
- [ ] Swatch icons ⊙/⊕ (spot/process) unclear → badge `Spot`.
- [ ] Context menu `Delete` not red.
- [ ] Tooltip covers the neighbouring field → show above.

## Text

- [ ] Type options mixes too much; first-line indent is an icon, the rest text; `After` lives elsewhere; `Drop`, `Chars`, `Keep` cryptic; full-width `Add tab stop` in the middle → 3 groups (Indents & spacing, Drop cap, Tabs), text labels.
- [ ] OpenType 2-part toggle unclear; Lining/Oldstyle and Proportional/Tabular exclusive → segmented controls.
- [ ] Insert character: tiny grey glyphs; All characters without names → name as tooltip and footer.
- [ ] Font select has no search.
- [ ] `Create text style` inside the select → `+` next to it.
- [ ] Style applied with overrides shows no marker → `Heading*` plus reset/update.
- [ ] Shot 31 `word selected` shows only a caret.

## Layers, swatches, pages, masters

- [ ] Drop above shows no insertion line (07).
- [ ] Page with a master shows no master entries → collapsed group `Master B`.
- [ ] Spread has page headers, a single page none.
- [ ] Master layer names duplicated (#, Rule, EARTHRISE) without left/right grouping.
- [ ] Edit swatch: no hex field; RGB picker for CMYK → 4 sliders for CMYK.
- [ ] Page labels `1 C` use letters the masters column does not show → `B · Body`.
- [ ] Empty slot left of page 1 solid, after the last page dashed with `+` → both dashed or drop the left one.
- [ ] Dragging a page: no drop indicator or ghost.
- [ ] Page menu: `Master like page` → `Create master from page`; add `Apply master`, `Insert page`.
- [ ] Master menu lacks `Duplicate master`.
- [ ] Master rename field overflows, name cut (`ody`).
- [ ] While editing a master, the master slot shows `auto (Light)`, a different meaning in the same place.
- [ ] Page switcher shows no total.

## Preflight & export

- [ ] `Images Keep` looks like an input.
- [ ] `JPEG − Lossless +` unclear → `JPEG quality ▾` (Lossless, 90, 75).
- [ ] Page range `7-x` only greys out the button → error text under the field.
- [ ] PNG/JPEG export has no resolution field.
- [ ] Export toast bottom left, far from the button.
- [ ] Badge `0` always visible → green check or nothing.
- [ ] Separations at first open (20): spot `Satz Orange` missing, `Highest 0 %` → loading state `…` instead of a false 0.
- [ ] Separation rows are boxed cards, other checkboxes are not.
- [ ] Black dot nearly invisible on dark ground.
- [ ] Gamut (22): checkbox off, no marks visible → fix the shot or the feedback.
- [ ] Issues (23): `Background ink at 297 %, above the limit` wraps → `Background · 297 % ink` in 1 line, page as link.
- [ ] `Under pointer: Point at a colour` is an instruction in a value slot → `—` plus grey hint.
- [ ] Overset issue text truncated.

## Context menus (11)

- [ ] 19 items → submenu `Transform` for flip, mask, flatten.
- [ ] `Delete` not red, between Duplicate and Group → last, red.
- [ ] Missing: `Bring forward` / `Send backward`, `Copy/Paste style`, `Apply master`.
- [ ] `Hide selection` / `Lock selection` → `Hide` / `Lock`.
- [ ] `Page overview` has a barely visible marker; `Add page` has no shortcut.
