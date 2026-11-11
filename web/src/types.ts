// SATZ_WRITE_FIXTURES=1 cargo test fixture writes this file.
export type Align = "inside" | "center" | "outside";
export type Align3 = "start" | "center" | "end";
export type Blend = "normal" | "multiply" | "screen" | "overlay" | "darken" | "lighten" | "colorDodge" | "colorBurn" | "hardLight" | "softLight" | "difference" | "exclusion" | "hue" | "saturation" | "color" | "luminosity";
export type BooleanOp = "union" | "subtract" | "intersect" | "exclude";
export type Cap = "none" | "round" | "square";
export type Collection = { id: string, name: string, modes: Array<Mode>, };
export type Color = number | { cmyk: Array<number>, alpha: number, } | { swatch: string, tint: number, alpha: number, } | { variable: string, alpha: number, };
export type ColorMode = "rgb" | "cmyk";
export type Command = { "type": "create", parent: string, kind: NewKind, x: number, y: number, w: number, h: number, } | { "type": "placeImage", parent: string, image: string, name: string, x: number, y: number, w: number, h: number, } | { "type": "setFrame", id: string, x: number, y: number, w: number, h: number, crop?: boolean, } | { "type": "setText", id: string, text: string, } | { "type": "editText", id: string, range: [number, number], text: string, } | { "type": "format", id: string, range?: [number, number] | null, size?: number | null, lineHeight?: number | null, letterSpacing?: number | null, paragraphSpacing?: number | null, fill?: Color | null, textStyle?: string | null, textAlign?: TextAlign | null, hyphenate?: boolean | null, lang?: Lang | null, font?: Typeface | null, paragraphIndent?: number | null, indentLeft?: number | null, indentRight?: number | null, spaceBefore?: number | null, textCase?: TextCase | null, textDecoration?: TextDecoration | null, features?: Array<string> | null, position?: Position | null, baselineShift?: number | null, dropLines?: number | null, dropChars?: number | null, keepLines?: number | null, keepTogether?: boolean | null, keepNext?: boolean | null, tabs?: Array<Tab> | null, list?: List | null, 
/**
 * `Some(None)` takes the shading off.
 */
shading?: Color | null | null, } | { "type": "scaleText", id: string, by: number, } | { "type": "addTextStyle", name: string, size?: number | null, lineHeight?: number | null, letterSpacing?: number | null, paragraphSpacing?: number | null, fill?: Color | null, textStyle?: string | null, textAlign?: TextAlign | null, hyphenate?: boolean | null, lang?: Lang | null, font?: Typeface | null, paragraphIndent?: number | null, indentLeft?: number | null, indentRight?: number | null, spaceBefore?: number | null, textCase?: TextCase | null, textDecoration?: TextDecoration | null, features?: Array<string> | null, position?: Position | null, baselineShift?: number | null, dropLines?: number | null, dropChars?: number | null, keepLines?: number | null, keepTogether?: boolean | null, keepNext?: boolean | null, tabs?: Array<Tab> | null, list?: List | null, 
/**
 * `Some(None)` takes the shading off.
 */
shading?: Color | null | null, } | { "type": "setTextStyle", id: string, name?: string | null, size?: number | null, lineHeight?: number | null, letterSpacing?: number | null, paragraphSpacing?: number | null, fill?: Color | null, textStyle?: string | null, textAlign?: TextAlign | null, hyphenate?: boolean | null, lang?: Lang | null, font?: Typeface | null, paragraphIndent?: number | null, indentLeft?: number | null, indentRight?: number | null, spaceBefore?: number | null, textCase?: TextCase | null, textDecoration?: TextDecoration | null, features?: Array<string> | null, position?: Position | null, baselineShift?: number | null, dropLines?: number | null, dropChars?: number | null, keepLines?: number | null, keepTogether?: boolean | null, keepNext?: boolean | null, tabs?: Array<Tab> | null, list?: List | null, 
/**
 * `Some(None)` takes the shading off.
 */
shading?: Color | null | null, } | { "type": "deleteTextStyle", id: string, } | { "type": "set", id: string, name?: string, hidden?: boolean, locked?: boolean, keepRatio?: boolean, rotation?: number, clip?: boolean, radius?: number, corners?: Array<number>, start?: number, sweep?: number, inner?: number, count?: number, ratio?: number, path?: Array<number>, fills?: Array<Fill>, constraints?: Constraints, strokes?: Array<Fill>, strokeWeight?: number, strokeAlign?: Align, strokeSides?: Array<number>, join?: Join, cap?: Cap, lineStyle?: LineStyle, arrowStart?: boolean, arrowEnd?: boolean, opacity?: number, blend?: Blend, effects?: Array<Effect>, mask?: boolean, overprintFill?: boolean, overprintStroke?: boolean, direction?: Direction, gap?: number, paddingTop?: number, paddingRight?: number, paddingBottom?: number, paddingLeft?: number, alignMain?: MainAlign, alignCross?: Align3, sizing?: Sizing, absolute?: boolean, insetTop?: number, insetRight?: number, insetBottom?: number, insetLeft?: number, columns?: number, gutter?: number, verticalAlign?: VerticalAlign, baselineGrid?: number, baselineStart?: number, maxLines?: number, trim?: boolean, roundBaselines?: boolean, columnRule?: Rule, minHeight?: number, wrap?: Wrap, wrapOffsetX?: number, wrapOffsetY?: number, } | { "type": "setPath", id: string, path: Array<number>, } | { "type": "fitImage", id: string, fit: Fit, } | { "type": "flip", id: string, vertical: boolean, } | { "type": "flatten", id: string, } | { "type": "boolean", ids: Array<string>, op: BooleanOp, } | { "type": "fillArea", id: string, x: number, y: number, } | { "type": "delete", ids: Array<string>, } | { "type": "group", ids: Array<string>, frame: boolean, } | { "type": "ungroup", ids: Array<string>, } | { "type": "mask", ids: Array<string>, } | { "type": "autoLayout", ids: Array<string>, } | { "type": "move", ids: Array<string>, parent: string, index: number, } | { "type": "order", ids: Array<string>, to: Order, } | { "type": "duplicate", ids: Array<string>, } | { "type": "copy", ids: Array<string>, } | { "type": "setDocument" } & Settings | { "type": "paste", above: Array<string>, page?: string | null, } | { "type": "addPage", after?: string | null, } | { "type": "setSection", id: string, section?: Section | null, } | { "type": "duplicatePage", id: string, } | { "type": "setPage", id: string, width?: number | null, height?: number | null, bleed?: number | null, scale?: boolean, } | { "type": "setGrids", id: string, grids: Array<Grid>, } | { "type": "setGuides", id: string, guides: Guides, } | { "type": "deletePage", id: string, } | { "type": "movePages", ids: Array<string>, index: number, } | { "type": "addMaster", like?: string | null, layers?: boolean, } | { "type": "setMaster", id: string, name: string, } | { "type": "deleteMaster", id: string, } | { "type": "useMaster", page: string, master?: string | null, } | { "type": "override", page: string, id: string, } | { "type": "resetToMaster", ids: Array<string>, } | { "type": "thread", from: string, to: string, } | { "type": "threadBefore", id: string, to: string, } | { "type": "unthread", id: string, } | { "type": "addSwatch", name: string, color: Color, spot: boolean, } | { "type": "setSwatch", id: string, name?: string | null, color?: Color | null, spot?: boolean | null, } | { "type": "deleteSwatch", id: string, } | { "type": "addCollection", name: string, } | { "type": "setCollection", id: string, name: string, } | { "type": "deleteCollection", id: string, } | { "type": "setMode", collection: string, id: string, name: string, } | { "type": "deleteMode", collection: string, id: string, } | { "type": "deleteVariable", id: string, } | { "type": "addMode", collection: string, name: string, } | { "type": "addVariable", collection: string, name: string, value: Value, } | { "type": "setVariable", id: string, name?: string | null, mode?: string | null, value?: Value | null, } | { "type": "bind", id: string, prop: string, variable?: string | null, } | { "type": "useMode", id: string, collection: string, mode?: string | null, } | { "type": "undo" } | { "type": "redo" } | { "type": "beginUndoGroup" } | { "type": "endUndoGroup" };
export type Constraint = "min" | "max" | "stretch" | "center" | "scale";
export type Constraints = { horizontal: Constraint, vertical: Constraint, };
export type DateFormat = "english" | "german" | "us" | "dotted" | "iso";
export type Direction = "none" | "horizontal" | "vertical";
export type Effect = { type: EffectKind, x: number, y: number, radius: number, color: Color, visible: boolean, };
export type EffectKind = "dropShadow" | "blur";
export type Fill = { type: FillKind, color: Color, stops: Array<FillStop>, transform: Array<number>, visible: boolean, image?: string, adjust?: [number, number, number], };
export type FillKind = "solid" | "linear" | "radial" | "image";
export type FillStop = { at: number, color: Color, };
export type Fit = "cover" | "contain" | "frame";
export type Grid = { kind: GridKind, count: number, gutter: number, margin: number, size: number, };
export type GridKind = "columns" | "rows" | "grid";
export type Guides = { x: Array<number>, y: Array<number>, };
export type Issue = { page: string, layer: string, name: string, } & ({ "problem": "overset" } | { "problem": "missingFont", font: string, } | { "problem": "shortOfBleed" } | { "problem": "rgb" } | { "problem": "lowPpi", ppi: number, } | { "problem": "gamut" } | { "problem": "ink", ink: number, } | { "problem": "missingGlyph", character: string, } | { "problem": "nearTrim", distance: number, });
export type Join = "miter" | "round" | "bevel";
export type Lang = "en" | "de" | "fr" | "it" | "es" | "nl";
export type Layout = { direction: Direction, gap: number, paddingTop: number, paddingRight: number, paddingBottom: number, paddingLeft: number, alignMain: MainAlign, alignCross: Align3, sizing: Sizing, 
/**
 * Left out of its parent's auto layout.
 */
absolute: boolean, };
export type LineStyle = "solid" | "dashed" | "dotted" | "wavy" | "zigzag";
export type List = "none" | "bullet" | "number";
export type MainAlign = "start" | "center" | "end" | "spaceBetween";
export type MissingFont = { font: Typeface, stories: Array<string>, };
export type Mode = { id: string, name: string, };
export type NewKind = "rect" | "ellipse" | "polygon" | "star" | "line" | "path" | "text" | "frame";
export type Node = { id: string, name: string, x: number, y: number, w: number, h: number, 
/**
 * The upright box on the page that the layer and what it does not clip cover,
 * turned by its rotation and its ancestors'.
 */
bounds: [number, number, number, number], 
/**
 * Modes chosen on this layer.
 */
modes: { [key in string]: string }, 
/**
 * Modes this layer's variables resolve in, its own and inherited.
 */
activeModes: { [key in string]: string }, 
/**
 * Number variables by property.
 */
bindings: { [key in string]: string }, 
/**
 * The master layer this page layer overrides.
 */
overrideOf?: string, 
/**
 * Effective pixels per inch of the coarsest visible image fill.
 */
ppi?: number, 
/**
 * Not drawn, exported, hit or preflighted, with its children.
 */
hidden: boolean, 
/**
 * Not hit on the canvas, with its children.
 */
locked: boolean, 
/**
 * Its width and height change together.
 */
keepRatio: boolean, 
/**
 * Degrees counterclockwise around the centre, with its children.
 */
rotation: number, 
/**
 * How the text of the text layers under it flows around it, `wrap_offset_x` pt away
 * beside it and `wrap_offset_y` pt above and below.
 */
wrap: Wrap, wrapOffsetX: number, wrapOffsetY: number, fills: Array<Fill>, strokes: Array<Fill>, strokeWeight: number, strokeAlign: Align, 
/**
 * The stroke weights of the top, right, bottom and left side, or none for `stroke_weight` all round.
 */
strokeSides: Array<number>, join: Join, cap: Cap, lineStyle: LineStyle, arrowStart: boolean, arrowEnd: boolean, opacity: number, blend: Blend, effects: Array<Effect>, mask: boolean, overprintFill: boolean, overprintStroke: boolean, constraints: Constraints, direction: Direction, gap: number, paddingTop: number, paddingRight: number, paddingBottom: number, paddingLeft: number, alignMain: MainAlign, alignCross: Align3, sizing: Sizing, 
/**
 * Left out of its parent's auto layout.
 */
absolute: boolean, } & ({ "kind": "shape" } & Shape | { "kind": "text", story: string, prev: string | null, next: string | null, overset: boolean, insetTop: number, insetRight: number, insetBottom: number, insetLeft: number, columns: number, gutter: number, verticalAlign: VerticalAlign, baselineGrid: number, baselineStart: number, 
/**
 * Lines the text is cut after, the last ending in an ellipsis; 0 is off.
 */
maxLines: number, 
/**
 * Trims the frame to the cap height of the first line and the last baseline.
 */
trim: boolean, 
/**
 * Rounds each baseline to whole points below the top inset.
 */
roundBaselines: boolean, 
/**
 * The line down the middle of each gutter.
 */
columnRule: Rule | null, 
/**
 * The least height of an auto height frame in pt.
 */
minHeight: number, } | { "kind": "group", children: Array<Node>, } | { "kind": "frame", clip: boolean, radius: number, corners: Array<number>, children: Array<Node>, });
export type Numbering = "arabic" | "upperRoman" | "lowerRoman";
export type Order = "forward" | "backward" | "front" | "back";
export type Page = { id: string, name: string, width: number, height: number, bleed: number, 
/**
 * The side of its spread a page is on with facing pages.
 */
side: Side | null, 
/**
 * Where the page's left edge sits on its spread, whose spine is at 0.
 */
x: number, master: string | null, detached: Array<string>, grids: Array<Grid>, guides: Guides, section: Section | null, modes: { [key in string]: string }, children: Array<Node>, };
export type Palette = { swatches: Array<Swatch>, collections: Array<Collection>, variables: Array<Variable>, textStyles: Array<TextStyle>, };
export type Position = "normal" | "superscript" | "subscript";
export type Preset = "screen" | "x4" | "x1a";
export type Props = { name?: string, hidden?: boolean, locked?: boolean, keepRatio?: boolean, rotation?: number, clip?: boolean, radius?: number, corners?: Array<number>, start?: number, sweep?: number, inner?: number, count?: number, ratio?: number, path?: Array<number>, fills?: Array<Fill>, constraints?: Constraints, strokes?: Array<Fill>, strokeWeight?: number, strokeAlign?: Align, strokeSides?: Array<number>, join?: Join, cap?: Cap, lineStyle?: LineStyle, arrowStart?: boolean, arrowEnd?: boolean, opacity?: number, blend?: Blend, effects?: Array<Effect>, mask?: boolean, overprintFill?: boolean, overprintStroke?: boolean, direction?: Direction, gap?: number, paddingTop?: number, paddingRight?: number, paddingBottom?: number, paddingLeft?: number, alignMain?: MainAlign, alignCross?: Align3, sizing?: Sizing, absolute?: boolean, insetTop?: number, insetRight?: number, insetBottom?: number, insetLeft?: number, columns?: number, gutter?: number, verticalAlign?: VerticalAlign, baselineGrid?: number, baselineStart?: number, maxLines?: number, trim?: boolean, roundBaselines?: boolean, columnRule?: Rule, minHeight?: number, wrap?: Wrap, wrapOffsetX?: number, wrapOffsetY?: number, };
export type Rule = { weight: number, color: Color, };
export type Section = { start: number, style: Numbering, prefix: string, };
export type Settings = { rasterPpi?: number, colorMode?: ColorMode, 
/**
 * Pairs the pages into spreads, as in InDesign.
 */
facingPages?: boolean, inkLimit?: number, preset?: Preset, cropMarks?: boolean, includeBleed?: boolean, 
/**
 * 0 keeps the images as they are.
 */
imagePpi?: number, 
/**
 * 0 keeps the images lossless.
 */
jpegQuality?: number, dateFormat?: DateFormat, };
export type Shape = { "shape": "rect", radius: number, 
/**
 * Top left, top right, bottom right and bottom left; empty for `radius` at each.
 */
corners?: Array<number>, } | { "shape": "ellipse", start: number, sweep: number, inner: number, } | { "shape": "polygon", count: number, } | { "shape": "star", count: number, ratio: number, } | { "shape": "path", path: Array<number>, 
/**
 * A move into each area that the fills paint; empty for the whole path.
 */
areas?: Array<number>, };
export type Side = "left" | "right";
export type Size = "fixed" | "hug" | "fill";
export type Sizing = { horizontal: Size, vertical: Size, };
export type Snapshot = { pages: Array<Page>, masters: Array<Page>, facingPages: boolean, 
/**
 * The ids of the pages of each spread from left to right: with facing pages the
 * first page alone on the right, then pairs; else each page alone.
 */
spreads: Array<Array<string>>, 
/**
 * The number each page shows, as its section counts it.
 */
labels: Array<string>, 
/**
 * The story of each thread by its first frame.
 */
stories: Record<string, Story>, 
/**
 * Resolution at which the PDF rasterizes shadows and blurs.
 */
rasterPpi: number, colorMode: ColorMode, 
/**
 * The name of the uploaded CMYK profile, `None` for FOGRA51.
 */
profile: string | null, 
/**
 * The highest total of inks in % that preflight allows.
 */
inkLimit: number, 
/**
 * How the PDF exports; an RGB document exports PDF/X-4 in place of PDF/X-1a.
 */
preset: Preset, cropMarks: boolean, includeBleed: boolean, 
/**
 * Images in the PDF shrink to no less than this unless it is 0.
 */
imagePpi: number, 
/**
 * Opaque images in the PDF are JPEGs of this quality unless it is 0.
 */
jpegQuality: number, dateFormat: DateFormat, 
/**
 * The fonts text can be set in, the bundled one first.
 */
fonts: Array<Typeface>, missingFonts: Array<MissingFont>, 
/**
 * The colors of each image by its hash.
 */
images: { [key in string]: Space }, preflight: Array<Issue>, canUndo: boolean, canRedo: boolean, swatches: Array<Swatch>, collections: Array<Collection>, variables: Array<Variable>, textStyles: Array<TextStyle>, };
export type Space = "Gray" | "RGB" | "CMYK";
export type Span = { len: number, size: number, lineHeight: number, letterSpacing: number, paragraphSpacing: number, fill: Color | null, textStyle: string, textAlign: TextAlign, hyphenate: boolean, lang: Lang, 
/**
 * `None` is the bundled font.
 */
font: Typeface | null, paragraphIndent: number, 
/**
 * The paragraph's lines move in by `indent_left` and `indent_right`, and it starts
 * `space_before` below the paragraph before it in its column.
 */
indentLeft: number, indentRight: number, spaceBefore: number, textCase: TextCase, textDecoration: TextDecoration, features: Array<string>, position: Position, baselineShift: number, 
/**
 * Lines the first `drop_chars` characters of a paragraph drop over; 0 is off.
 */
dropLines: number, dropChars: number, 
/**
 * Lines at the start and at the end of a paragraph that a column break keeps together.
 */
keepLines: number, keepTogether: boolean, keepNext: boolean, tabs: Array<Tab>, list: List, 
/**
 * The color behind the paragraph across its column.
 */
shading: Color | null, };
export type Story = { text: string, spans: Array<Span>, };
export type Style = { fills: Array<Fill>, strokes: Array<Fill>, strokeWeight: number, strokeAlign: Align, 
/**
 * The stroke weights of the top, right, bottom and left side, or none for `stroke_weight` all round.
 */
strokeSides: Array<number>, join: Join, cap: Cap, lineStyle: LineStyle, arrowStart: boolean, arrowEnd: boolean, opacity: number, blend: Blend, effects: Array<Effect>, mask: boolean, overprintFill: boolean, overprintStroke: boolean, constraints: Constraints, };
export type Swatch = { id: string, name: string, color: Color, spot: boolean, };
export type Tab = { at: number, align: TabAlign, leader: string, };
export type TabAlign = "left" | "center" | "right" | "decimal";
export type TextAlign = "left" | "center" | "right" | "justify";
export type TextCase = "original" | "upper" | "lower" | "title";
export type TextDecoration = "none" | "underline" | "strikethrough";
export type TextFrame = { insetTop: number, insetRight: number, insetBottom: number, insetLeft: number, columns: number, gutter: number, verticalAlign: VerticalAlign, baselineGrid: number, baselineStart: number, 
/**
 * Lines the text is cut after, the last ending in an ellipsis; 0 is off.
 */
maxLines: number, 
/**
 * Trims the frame to the cap height of the first line and the last baseline.
 */
trim: boolean, 
/**
 * Rounds each baseline to whole points below the top inset.
 */
roundBaselines: boolean, 
/**
 * The line down the middle of each gutter.
 */
columnRule: Rule | null, 
/**
 * The least height of an auto height frame in pt.
 */
minHeight: number, };
export type TextStyle = { id: string, name: string, bindings: { [key in string]: string }, size: number, lineHeight: number, letterSpacing: number, paragraphSpacing: number, fill: Color | null, textStyle: string, textAlign: TextAlign, hyphenate: boolean, lang: Lang, 
/**
 * `None` is the bundled font.
 */
font: Typeface | null, paragraphIndent: number, 
/**
 * The paragraph's lines move in by `indent_left` and `indent_right`, and it starts
 * `space_before` below the paragraph before it in its column.
 */
indentLeft: number, indentRight: number, spaceBefore: number, textCase: TextCase, textDecoration: TextDecoration, features: Array<string>, position: Position, baselineShift: number, 
/**
 * Lines the first `drop_chars` characters of a paragraph drop over; 0 is off.
 */
dropLines: number, dropChars: number, 
/**
 * Lines at the start and at the end of a paragraph that a column break keeps together.
 */
keepLines: number, keepTogether: boolean, keepNext: boolean, tabs: Array<Tab>, list: List, 
/**
 * The color behind the paragraph across its column.
 */
shading: Color | null, };
export type Typeface = { name: string, family: string, style: string, hash: string, };
export type Value = { "color": Color } | { "number": number } | { "font": Typeface | null };
export type Variable = { id: string, collection: string, name: string, values: { [key in string]: Value }, };
export type VerticalAlign = "top" | "center" | "bottom";
export type Wrap = "none" | "around" | "jump";
