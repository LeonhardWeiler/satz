# UI G inventory

Every function of main and of design G, with its spec and its place in the G UI.
Places: **Top** top bar, **Left** layers and swatches, **Right** properties,
**Preflight** preflight mode, **Overview** the "." view, **Start** start screen,
**Switcher** "‹ [2] [3] ›" over the canvas, **Palette** Ctrl K, **Help** "?",
**Toast** feedback, **Quick** quick edit above the selection.

## (a) main

### Shell (App.tsx, main.tsx, file.ts)

- [ ] Loading name and bar, WebGL 2 notice, engine failure notice, wasm preload — webgl — unchanged
- [ ] Autosave to IndexedDB, reload keeps document and unsaved mark, nothing to undo — file — unchanged; decides Start vs last document
- [ ] Title `* name — Satz` — file — unchanged
- [ ] Status line `role=status` — masters, file, fonts, images — Toast (keeps `role=status`)
- [ ] Ctrl S save, Ctrl Shift S save as — file — keys unchanged; Palette
- [ ] Ctrl O open (FS Access, input fallback, confirm discard, rejects non-Satz) — file — keys unchanged; Start "Open file…"; Palette
- [ ] Ctrl Alt N new document, asks before discarding — file — opens Start like Ctrl N
- [ ] Ctrl Shift K place images, rejects non-images — images — Top tool; Palette
- [ ] Ctrl Shift E / Export PDF button (worker, warns of issues, still downloads) — pdf, preflight — Top "Export" opens Preflight; its button exports
- [ ] Dialogs block global keys while modal — variables — unchanged

### Toolbar.tsx

- [ ] Move V, Frame F/A, Pen P, Text T — canvas, edit, pdf — Top
- [ ] Shape menu: Rectangle R, Line L, Arrow Shift L, Ellipse O, Polygon, Star — color, pdf — Top popover
- [ ] Place image — images — Top

### keys.ts

- [ ] Esc: threading, pen, tool → move, parent, clear — edit, canvas — unchanged, after mode, overview and master in the Esc chain
- [ ] Ctrl Z, Ctrl Shift Z, Ctrl Y — pdf — unchanged; Ctrl Alt Y checked first
- [ ] Ctrl A siblings — pdf — unchanged
- [ ] Ctrl C, Ctrl X, Ctrl V — pdf — unchanged
- [ ] Del, Backspace — autolayout — unchanged; in Overview deletes pages
- [ ] Enter edits text or selects children, Shift Enter parent — edit, pdf — unchanged
- [ ] Ctrl D duplicate — pdf — unchanged
- [ ] Ctrl G group, Ctrl Alt G frame, Ctrl Shift G ungroup — pdf — unchanged
- [ ] Ctrl R rename — — unchanged, plus F2
- [ ] Shift A auto layout, Shift Alt A remove — autolayout — unchanged
- [ ] Ctrl Alt M mask — canvas — unchanged
- [ ] Ctrl [ ] order, Shift to back/front — — unchanged
- [ ] Arrows nudge 1 mm, Shift 10 mm — canvas — unchanged

### Canvas.tsx

- [ ] Draw every tool, click for default size, Shift square/45°, Alt from centre — canvas, text, pdf — unchanged, with snapping
- [ ] Move, Shift axis lock, Alt drag duplicate, drag across the spine — spreads, pdf — unchanged, with snapping
- [ ] Resize handles, Shift aspect, Alt centre, Ctrl ignores constraints — autolayout, text — unchanged; Ctrl also turns snapping off
- [ ] Line ends — canvas — unchanged
- [ ] Marquee, Shift adds — pdf — unchanged
- [ ] Click, double-click, Ctrl deep select and hover — canvas — unchanged
- [ ] Auto layout reorder with insertion line — autolayout — unchanged
- [ ] Pen tool, close on first anchor, one undo step — canvas, pdf — unchanged
- [ ] Ctrl Shift click overrides a master layer — masters — unchanged
- [ ] Thread ports, thread, unthread by double click, overset port — thread — unchanged
- [ ] Text editing, IME, copy/cut/paste, triple click — edit — unchanged
- [ ] Wheel pan, Ctrl wheel zoom, Space pan, middle button pan — — unchanged; any input cancels the zoom animation
- [ ] Shift 1 fit, Shift 0 100 %, Ctrl +/- — — unchanged, animated; plus Ctrl 0
- [ ] View per spread kept — — unchanged
- [ ] Zoom display — — bottom right of the canvas

### textEdit.ts

- [ ] Arrows, Home/End, word jumps, Backspace/Delete, Enter, Ctrl A, undo inside text — edit — unchanged
- [ ] Ctrl Alt Shift N page number — masters — unchanged; Text section button stays

### Layers.tsx

- [ ] Tree, one tab stop, ↑↓ →← — keyboard — Left, plus Enter/Shift Enter/Tab/F2
- [ ] Drag & drop above/below/into, not onto own child — canvas — Left
- [ ] Rename by double click — — Left
- [ ] Mask indicator, kind icons — canvas — Left with G icons
- [ ] Space on a focused row button — canvas — Left

### Pages.tsx

- [ ] Pages list, facing spreads from a first right page — pages — Overview; Switcher
- [ ] Show page by click — pages, masters — Overview double-click/Enter; Switcher boxes; PgUp/PgDn
- [ ] Add page — pages — Overview "Add page"; Palette
- [ ] Drag reorder, Alt ←/→ move — pages, keyboard — Overview
- [ ] Delete key, context menu Duplicate/Delete page — pages, keyboard — Overview, toast "Ctrl Z restores"
- [ ] Masters list, Add master (like the current page) — masters — Overview "New master"; context menu "Master like page"
- [ ] Rename master (double click on name, context menu), duplicate names rejected — masters — Overview: double click on the name, F2, context menu
- [ ] Delete master — — Overview context menu
- [ ] Master prefix badge — masters — Overview thumbnails
- [ ] Context menu arrow keys — keyboard — Overview

### Properties.tsx

- [ ] Header Export PDF — preflight — Top "Export"
- [ ] Page W/H/bleed per page — pages — Right "Document" and "Page"
- [ ] Raster ppi, colour mode, facing pages — color, masters — Right "Document"
- [ ] Master select, variable modes of the page, Reset overrides — masters, variables — Right "Page"
- [ ] Fonts: add, list, missing, find missing — fonts — Right, no selection
- [ ] Variables: Local variables dialog — variables — Right, no selection
- [ ] Layout: X Y W H, L ∠, R, N, Ratio, sizing, absolute, constraints, clip — canvas, constraints, text — Right
- [ ] Auto layout section — autolayout — Right
- [ ] Layer: opacity, blend, modes, use as mask — canvas, color — Right
- [ ] Fill/Stroke paint lists, stroke weight/align/join/cap/arrow ends — color, pdf — Right
- [ ] Effects — pdf — Right
- [ ] Text and text frame sections, hyphenation, baseline grid — text — Right
- [ ] Text styles: create, edit, delete — text — Right, specimen menu
- [ ] Reset to master — masters — Right
- [ ] Mixed values, rounding, rejecting out-of-range values — canvas — every Field

### Paints.tsx, ColorPicker.tsx, Swatches.tsx, Variables.tsx

- [ ] Picker: hex, saturation, hue, alpha, CMYK, swatches, variables, stays in the window — color — unchanged in G look
- [ ] Gradients with stops — pdf — unchanged
- [ ] Swatches: add, click fills or edits, Enter edits, spot, delete, unique names — color — Left, G rows
- [ ] Variables dialog: collections, modes, variables, bind and detach — variables — unchanged in G look

## (b) G and core.js

### Look

- [ ] `:root` tokens, cyan selection, magenta guides, canvas #37393c, bleed and margin lines — Canvas, all panels
- [ ] Hanken Grotesk self-hosted with OFL — web/src/fonts
- [ ] Icon set 16 px, 1.25 stroke, own icons for Pen, Image, Auto Layout … — Icon component
- [ ] Transitions 120–260 ms, reduced motion, hover not animated — index.css
- [ ] WCAG AA text — index.css

### Layout

- [ ] Fixed grid: top bar, left, right, rulers in mm — App
- [ ] Alt 1, Alt 2, Ctrl \ panel toggles and their top-bar buttons — Top
- [ ] Alt 3 bottom strip — not built: no bottom bar
- [ ] Rulers with selection extent (cyan) and snapped guides (magenta) — Canvas
- [ ] Switcher ‹ [2] [3] ›, L/R for masters — Switcher
- [ ] Master banner "Editing master X, used by N pages", Done Esc — Canvas
- [ ] Layout at 880 px — App

### Start

- [ ] Formats A2–A6, DL, US Letter, Custom; orientation; page count; facing; arrow keys; Enter — Start
- [ ] Open file…, examples poster and booklet — Start
- [ ] Shown without autosave, Ctrl N, asks before discarding, Esc back or booklet — Start

### Overview

- [ ] Masters row with New master, spreads vertical, Add page — Overview
- [ ] Real thumbnails through the renderer, redrawn on change — Overview
- [ ] Click, Shift/Ctrl select, double-click/Enter open, Alt ←/→, drag, Del with toast, double-click master edits — Overview
- [ ] "." and Esc close — Overview

### Navigation

- [ ] PgUp/PgDn, Home/End — Canvas
- [ ] Shift 2 zoom to selection, Ctrl 0 100 % — Canvas
- [ ] Zoom animation 180 ms, cancelled by input — Canvas

### Fields

- [ ] Label drag, 1 undo step — Field
- [ ] ↑/↓ ±1, Shift ±10, Alt ±0.1 — Field
- [ ] Expressions, units — Field (pure function `evalExpr`)
- [ ] +/− on integer fields, shake on invalid — Field

### Properties

- [ ] Header with icon and name — Right
- [ ] Collapsible sections with height animation, state kept — Right
- [ ] Document section: preset, orientation, W, H, bleed, pages, facing, colour mode, profile, raster — Right
- [ ] Page section with Master select, Mixed — Right
- [ ] Text style specimen menu — Right

### Quick edit

- [ ] Per kind: text style and align, fill, radius, opacity, image ppi, star/polygon points, More — Quick
- [ ] Hidden while dragging, editing text and in modes; no size label — Quick

### Layers

- [ ] Hover highlights on the canvas — Left
- [ ] Visible/lock buttons on hover, set states stay, hidden dimmed — Left
- [ ] Enter children, Shift Enter parent, Tab/Shift Tab siblings, F2 — Left and Canvas
- [ ] Master layers row, empty state — Left

### Hidden and locked

- [ ] `hidden`, `locked` node props, undo, old files false — engine
- [ ] Hidden: not drawn, exported, hit or preflighted; locked: not hit; parents apply — engine
- [ ] Ctrl Shift H, Ctrl Shift L — keys

### Palette, help, toasts

- [ ] Ctrl K: commands, layers, pages, swatches, text styles, fuzzy, shortcuts — Palette
- [ ] "?" shortcut overview grouped as core.js plus main — Help
- [ ] Toasts, apply errors — Toast

### Swatches

- [ ] Chip, name, CMYK values, RGB-only warning, click fills — Left

### Snapping

- [ ] Targets: objects, page, centre, margins, columns, baseline grid; 5 px — snap.ts
- [ ] Move, resize, draw; Ctrl off; Shift axis — Canvas
- [ ] Distances to 2 nearest while moving; Alt distance to hovered or page; mm — Canvas

### Preflight

- [ ] Ctrl Alt Y, count button, Ctrl Shift E and Export open it, Esc ends — Preflight
- [ ] Issues by severity, click zooms; Gamut, ink limit, 3 mm from trim — Preflight, engine
- [ ] Proof always on, gamut marking — Preflight, engine
- [ ] Separations C M Y K and spots with max — Preflight, engine
- [ ] Ink coverage limit, mark, max with object and page, value under pointer — Preflight, engine
- [ ] Export presets X-4, X-1a, screen; crop marks, bleed; in the document; button with error count, progress, toast — Preflight, engine

### Keys from core.js

- [ ] Ctrl K, Ctrl Alt Y, Ctrl Shift E, Ctrl N, Ctrl \, Alt 1/2, Shift 1/2, Ctrl 0, Ctrl +/-, Ctrl Z/Y, Ctrl D, Ctrl G, Ctrl A, Ctrl Shift H/L, F2, Ctrl R, PgUp/PgDn, Home/End, ?, ., Tab, Enter, Esc, Del, arrows, V F R O T, Alt, Space — keys.ts, Canvas
