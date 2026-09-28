# UI G inventory

Every function of main and of design G, with its spec and its place in the G UI.
Places: **Top** top bar, **Left** layers and swatches, **Right** properties,
**Preflight** preflight mode, **Overview** the "." view, **Start** start screen,
**Switcher** "‹ [2] [3] ›" over the canvas, **Palette** Ctrl K, **Help** "?",
**Toast** feedback, **Quick** quick edit above the selection.

## (a) main

### Shell (App.tsx, main.tsx, file.ts)

- [x] Loading name and bar, WebGL 2 notice, engine failure notice, wasm preload — webgl — unchanged
- [x] Autosave to IndexedDB, reload keeps document and unsaved mark, nothing to undo — file — unchanged; decides Start vs last document
- [x] Title `* name — Satz` — file — unchanged
- [x] Status line `role=status` — masters, file, fonts, images — Toast (keeps `role=status`)
- [x] Ctrl S save, Ctrl Shift S save as — file — keys unchanged; Palette
- [x] Ctrl O open (FS Access, input fallback, confirm discard, rejects non-Satz) — file — keys unchanged; Start "Open file…"; Palette
- [x] Ctrl Alt N new document, asks before discarding — file — opens Start like Ctrl N
- [x] Ctrl Shift K place images, rejects non-images — images — Top tool; Palette
- [x] Ctrl Shift E / Export PDF button (worker, warns of issues, still downloads) — pdf, preflight — Top "Export" opens Preflight; its button exports
- [x] Dialogs block global keys while modal — variables — unchanged

### Toolbar.tsx

- [x] Move V, Frame F/A, Pen P, Text T — canvas, edit, pdf — Top
- [x] Shape menu: Rectangle R, Line L, Arrow Shift L, Ellipse O, Polygon, Star — color, pdf — Top popover
- [x] Place image — images — Top

### keys.ts

- [x] Esc: threading, pen, tool → move, parent, clear — edit, canvas — unchanged, after mode, overview and master in the Esc chain
- [x] Ctrl Z, Ctrl Shift Z, Ctrl Y — pdf — unchanged; Ctrl Alt Y checked first
- [x] Ctrl A siblings — pdf — unchanged
- [x] Ctrl C, Ctrl X, Ctrl V — pdf — unchanged
- [x] Del, Backspace — autolayout — unchanged; in Overview deletes pages
- [x] Enter edits text or selects children, Shift Enter parent — edit, pdf — unchanged
- [x] Ctrl D duplicate — pdf — unchanged
- [x] Ctrl G group, Ctrl Alt G frame, Ctrl Shift G ungroup — pdf — unchanged
- [x] Ctrl R rename — — unchanged, plus F2
- [x] Shift A auto layout, Shift Alt A remove — autolayout — unchanged
- [x] Ctrl Alt M mask — canvas — unchanged
- [x] Ctrl [ ] order, Shift to back/front — — unchanged
- [x] Arrows nudge 1 mm, Shift 10 mm — canvas — unchanged

### Canvas.tsx

- [x] Draw every tool, click for default size, Shift square/45°, Alt from centre — canvas, text, pdf — unchanged, with snapping
- [x] Move, Shift axis lock, Alt drag duplicate, drag across the spine — spreads, pdf — unchanged, with snapping
- [x] Resize handles, Shift aspect, Alt centre, Ctrl ignores constraints — autolayout, text — unchanged; Ctrl also turns snapping off
- [x] Line ends — canvas — unchanged
- [x] Marquee, Shift adds — pdf — unchanged
- [x] Click, double-click, Ctrl deep select and hover — canvas — unchanged
- [x] Auto layout reorder with insertion line — autolayout — unchanged
- [x] Pen tool, close on first anchor, one undo step — canvas, pdf — unchanged
- [x] Ctrl Shift click overrides a master layer — masters — unchanged
- [x] Thread ports, thread, unthread by double click, overset port — thread — unchanged
- [x] Text editing, IME, copy/cut/paste, triple click — edit — unchanged
- [x] Wheel pan, Ctrl wheel zoom, Space pan, middle button pan — — unchanged; any input cancels the zoom animation
- [x] Shift 1 fit, Shift 0 100 %, Ctrl +/- — — unchanged, animated; plus Ctrl 0
- [x] View per spread kept — — unchanged
- [x] Zoom display — — bottom right of the canvas

### textEdit.ts

- [x] Arrows, Home/End, word jumps, Backspace/Delete, Enter, Ctrl A, undo inside text — edit — unchanged
- [x] Ctrl Alt Shift N page number — masters — unchanged; Text section button stays

### Layers.tsx

- [x] Tree, one tab stop, ↑↓ →← — keyboard — Left, plus Enter/Shift Enter/Tab/F2
- [x] Drag & drop above/below/into, not onto own child — canvas — Left
- [x] Rename by double click — — Left
- [x] Mask indicator, kind icons — canvas — Left with G icons
- [x] Space on a focused row button — canvas — Left

### Pages.tsx

- [x] Pages list, facing spreads from a first right page — pages — Overview; Switcher
- [x] Show page by click — pages, masters — Overview double-click/Enter; Switcher boxes; PgUp/PgDn
- [x] Add page — pages — Overview "Add page"; Palette
- [x] Drag reorder, Alt ←/→ move — pages, keyboard — Overview
- [x] Delete key, context menu Duplicate/Delete page — pages, keyboard — Overview, toast "Ctrl Z restores"
- [x] Masters list, Add master (like the current page) — masters — Overview "New master"; context menu "Master like page"
- [x] Rename master (double click on name, context menu), duplicate names rejected — masters — Overview: double click on the name, F2, context menu
- [x] Delete master — — Overview context menu
- [x] Master prefix badge — masters — Overview thumbnails
- [x] Context menu arrow keys — keyboard — Overview

### Properties.tsx

- [x] Header Export PDF — preflight — Top "Export"
- [x] Page W/H/bleed per page — pages — Right "Document" and "Page"
- [x] Raster ppi, colour mode, facing pages — color, masters — Right "Document"
- [x] Master select, variable modes of the page, Reset overrides — masters, variables — Right "Page"
- [x] Fonts: add, list, missing, find missing — fonts — Right, no selection
- [x] Variables: Local variables dialog — variables — Right, no selection
- [x] Layout: X Y W H, L ∠, R, N, Ratio, sizing, absolute, constraints, clip — canvas, constraints, text — Right
- [x] Auto layout section — autolayout — Right
- [x] Layer: opacity, blend, modes, use as mask — canvas, color — Right
- [x] Fill/Stroke paint lists, stroke weight/align/join/cap/arrow ends — color, pdf — Right
- [x] Effects — pdf — Right
- [x] Text and text frame sections, hyphenation, baseline grid — text — Right
- [x] Text styles: create, edit, delete — text — Right, specimen menu
- [x] Reset to master — masters — Right
- [x] Mixed values, rounding, rejecting out-of-range values — canvas — every Field

### Paints.tsx, ColorPicker.tsx, Swatches.tsx, Variables.tsx

- [x] Picker: hex, saturation, hue, alpha, CMYK, swatches, variables, stays in the window — color — unchanged in G look
- [x] Gradients with stops — pdf — unchanged
- [x] Swatches: add, click fills or edits, Enter edits, spot, delete, unique names — color — Left, G rows
- [x] Variables dialog: collections, modes, variables, bind and detach — variables — unchanged in G look

## (b) G and core.js

### Look

- [x] `:root` tokens, cyan selection, magenta guides, canvas #37393c, bleed and margin lines — Canvas, all panels
- [x] Hanken Grotesk self-hosted with OFL — web/src/fonts
- [x] Icon set 16 px, 1.25 stroke, own icons for Pen, Image, Auto Layout … — Icon component
- [x] Transitions 120–260 ms, reduced motion, hover not animated — index.css
- [x] WCAG AA text — index.css

### Layout

- [x] Fixed grid: top bar, left, right, rulers in mm — App
- [x] Alt 1, Alt 2, Ctrl \ panel toggles and their top-bar buttons — Top
- [x] Alt 3 bottom strip — dropped by the spec: no bottom bar
- [x] Rulers with selection extent (cyan) and snapped guides (magenta) — Canvas
- [x] Switcher ‹ [2] [3] ›, L/R for masters — Switcher
- [x] Master banner "Editing master X, used by N pages", Done Esc — Canvas
- [x] Layout at 880 px — App

### Start

- [x] Formats A2–A6, DL, US Letter, Custom; orientation; page count; facing; arrow keys; Enter — Start
- [x] Open file…, examples poster and booklet — Start
- [x] Shown without autosave, Ctrl N, asks before discarding, Esc back or booklet — Start

### Overview

- [x] Masters row with New master, spreads vertical, Add page — Overview
- [x] Real thumbnails through the renderer, redrawn on change — Overview
- [x] Click, Shift/Ctrl select, double-click/Enter open, Alt ←/→, drag, Del with toast, double-click master edits — Overview
- [x] "." and Esc close — Overview

### Navigation

- [x] PgUp/PgDn, Home/End — Canvas
- [x] Shift 2 zoom to selection, Ctrl 0 100 % — Canvas
- [x] Zoom animation 180 ms, cancelled by input — Canvas

### Fields

- [x] Label drag, 1 undo step — Field
- [x] ↑/↓ ±1, Shift ±10, Alt ±0.1 — Field
- [x] Expressions, units — Field (pure function `evalExpr`)
- [x] +/− on integer fields, shake on invalid — Field

### Properties

- [x] Header with icon and name — Right
- [x] Collapsible sections with height animation, state kept — Right
- [x] Document section: preset, orientation, W, H, bleed, pages, facing, colour mode, profile, raster — Right
- [x] Page section with Master select, Mixed — Right
- [x] Text style specimen menu — Right

### Quick edit

- [x] Per kind: text style and align, fill, radius, opacity, image ppi, star/polygon points, More — Quick
- [x] Hidden while dragging, editing text and in modes; no size label — Quick

### Layers

- [x] Hover highlights on the canvas — Left
- [x] Visible/lock buttons on hover, set states stay, hidden dimmed — Left
- [x] Enter children, Shift Enter parent, Tab/Shift Tab siblings, F2 — Left and Canvas
- [x] Master layers row, empty state — Left

### Hidden and locked

- [x] `hidden`, `locked` node props, undo, old files false — engine
- [x] Hidden: not drawn, exported, hit or preflighted; locked: not hit; parents apply — engine
- [x] Ctrl Shift H, Ctrl Shift L — keys

### Palette, help, toasts

- [x] Ctrl K: commands, layers, pages, swatches, text styles, fuzzy, shortcuts — Palette
- [x] "?" shortcut overview grouped as core.js plus main — Help
- [x] Toasts, apply errors — Toast

### Swatches

- [x] Chip, name, CMYK values, RGB-only warning, click fills — Left

### Snapping

- [x] Targets: objects, page, centre, margins, columns, baseline grid; 5 px — snap.ts
- [x] Move, resize, draw; Ctrl off; Shift axis — Canvas
- [x] Distances to 2 nearest while moving; Alt distance to hovered or page; mm — Canvas

### Preflight

- [x] Ctrl Alt Y, count button, Ctrl Shift E and Export open it, Esc ends — Preflight
- [x] Issues by severity, click zooms; Gamut, ink limit, 3 mm from trim — Preflight, engine
- [x] Proof always on, gamut marking — Preflight, engine
- [x] Separations C M Y K and spots with max — Preflight, engine
- [x] Ink coverage limit, mark, max with object and page, value under pointer — Preflight, engine
- [x] Export button with error count, progress, toast — Preflight
- [ ] Export presets X-4, X-1a, screen; crop marks and bleed switches in the document — not built: requirement 15 (PDF presets in the engine) was dropped, see AGENT/TODO.md

### Keys from core.js

- [x] Ctrl K, Ctrl Alt Y, Ctrl Shift E, Ctrl N, Ctrl \, Alt 1/2, Shift 1/2, Ctrl 0, Ctrl +/-, Ctrl Z/Y, Ctrl D, Ctrl G, Ctrl A, Ctrl Shift H/L, F2, Ctrl R, PgUp/PgDn, Home/End, ?, ., Tab, Enter, Esc, Del, arrows, V F R O T, Alt, Space — keys.ts, Canvas
