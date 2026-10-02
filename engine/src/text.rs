use crate::color::Color;
use crate::content_hash;
use crate::display_list::{CLOSE, CUBIC, LINE, MOVE, Op, Paint, rect};
use crate::linebreak::{Item, break_lines};
use crate::style::{Style, paints};
use crate::variable::{Scope, Value};
use harfrust::{Feature, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::TableProvider;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use skrifa::MetadataProvider;
use skrifa::string::StringId;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub const FONT: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");
/// Marks the font of a glyph run whose own font is missing, drawn in the bundled one.
pub const MISSING: u32 = 1 << 31;

/// A font by its full name, family and style, and a hash of its bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typeface {
    pub name: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub style: String,
    pub hash: String,
}

thread_local! {
    /// The fonts text can be set in by id, the bundled one first.
    static FONTS: RefCell<Vec<(Typeface, Rc<[u8]>)>> =
        RefCell::new(vec![(typeface(FONT).unwrap(), FONT.into())]);
}

pub fn typeface(bytes: &[u8]) -> Result<Typeface, String> {
    let bad = || "not a TrueType or OpenType font".to_string();
    let f = FontRef::new(bytes).map_err(|_| bad())?;
    let sets = f.cmap().is_ok()
        && f.hhea().is_ok()
        && f.hmtx().is_ok()
        && f.maxp().is_ok()
        && f.head().is_ok_and(|h| h.units_per_em() > 0);
    if !sets || krilla::text::Font::new(bytes.to_vec().into(), 0).is_none() {
        return Err(bad());
    }
    let f = skrifa::FontRef::new(bytes).map_err(|_| bad())?;
    let get = |ids: &[StringId]| {
        ids.iter()
            .find_map(|&id| f.localized_strings(id).english_or_first())
            .map(|s| s.to_string())
            .ok_or_else(bad)
    };
    Ok(Typeface {
        name: get(&[StringId::FULL_NAME])?,
        family: get(&[StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME])?,
        style: get(&[
            StringId::TYPOGRAPHIC_SUBFAMILY_NAME,
            StringId::SUBFAMILY_NAME,
        ])?,
        hash: content_hash(bytes),
    })
}

/// Adds the font `bytes`, TrueType, OpenType, WOFF or WOFF2, to those text can be set in, unless it is there already.
pub fn add_font(bytes: &[u8]) -> Result<Typeface, String> {
    let sfnt = match bytes.get(..4) {
        Some(b"wOFF") => wuff::decompress_woff1(bytes),
        Some(b"wOF2") => wuff::decompress_woff2(bytes),
        _ => Ok(bytes.into()),
    }
    .map_err(|e| e.to_string())?;
    let face = typeface(&sfnt)?;
    FONTS.with_borrow_mut(|f| {
        if !f.iter().any(|(t, b)| t.hash == face.hash && !b.is_empty()) {
            f.push((face.clone(), sfnt.into()));
        }
    });
    Ok(face)
}

/// The id of the font `face`, or the bundled one's marked `MISSING` when it is not there.
pub fn font_id(face: &Option<Typeface>) -> u32 {
    let Some(face) = face else { return 0 };
    FONTS
        .with_borrow(|f| {
            f.iter()
                .position(|(t, b)| t.hash == face.hash && !b.is_empty())
        })
        .map_or(MISSING, |i| i as u32)
}

pub fn font_bytes(id: u32) -> Rc<[u8]> {
    FONTS.with_borrow(|f| f.get((id & !MISSING) as usize).unwrap_or(&f[0]).1.clone())
}

/// Removes the added font `hash`; its id stays, with no bytes.
pub fn remove_font(hash: &str) {
    FONTS.with_borrow_mut(|f| {
        for (_, b) in f.iter_mut().skip(1).filter(|(t, _)| t.hash == hash) {
            *b = Rc::from([]);
        }
    });
}

/// The number of fonts, which only grows.
pub fn fonts_added() -> usize {
    FONTS.with_borrow(|f| f.len())
}

/// The fonts text can be set in.
pub fn fonts() -> Vec<Typeface> {
    FONTS.with_borrow(|f| {
        f.iter()
            .filter(|(_, b)| !b.is_empty())
            .map(|(t, _)| t.clone())
            .collect()
    })
}

/// The keys of `Attrs` a text style sets.
pub const STYLED: [&str; 18] = [
    "size",
    "lineHeight",
    "letterSpacing",
    "paragraphSpacing",
    "paragraphIndent",
    "font",
    "textCase",
    "textDecoration",
    "features",
    "position",
    "baselineShift",
    "dropLines",
    "dropChars",
    "keepLines",
    "keepTogether",
    "keepNext",
    "tabs",
    "list",
];
/// The keys of `Attrs` that hold for a whole paragraph, taken from its first character.
pub const PARAGRAPH: [&str; 12] = [
    "textAlign",
    "paragraphSpacing",
    "paragraphIndent",
    "hyphenate",
    "lang",
    "dropLines",
    "dropChars",
    "keepLines",
    "keepTogether",
    "keepNext",
    "tabs",
    "list",
];

/// TeX's \hyphenpenalty.
const HYPHEN_COST: f32 = 50.0;
const FILL: Item = Item::Glue {
    width: 0.0,
    stretch: 1e6,
    shrink: 0.0,
};
const FORCE: Item = Item::Penalty {
    width: 0.0,
    cost: f32::NEG_INFINITY,
};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// A tab stop `at` pt from the column's left edge: the text after a tab starts there,
/// centres or ends on it, or puts its last `.` or `,` there; `leader` repeats over the gap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub at: f64,
    pub align: TabAlign,
    pub leader: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TabAlign {
    Left,
    Center,
    Right,
    Decimal,
}

/// Sets the width of each tab in `ws`, the widths of items from `x` in a column, to
/// reach its stop in `stops`, or else the next multiple of 36 pt; `tab` and `point`
/// tell the tabs and the `.` and `,` apart. Returns each tab with its stop.
fn tabs<'a>(
    stops: &'a [Tab],
    ws: &mut [f32],
    tab: impl Fn(usize) -> bool,
    point: impl Fn(usize) -> bool,
    mut x: f32,
) -> Vec<(usize, Option<&'a Tab>)> {
    let mut out = Vec::new();
    for k in 0..ws.len() {
        if tab(k) {
            let stop = stops
                .iter()
                .filter(|s| s.at as f32 > x + 0.01)
                .min_by(|a, b| a.at.total_cmp(&b.at));
            let next = (k + 1..ws.len()).find(|&j| tab(j)).unwrap_or(ws.len());
            let upto = |j: usize| ws[k + 1..j].iter().sum::<f32>();
            let after = match stop.map(|s| s.align) {
                None | Some(TabAlign::Left) => 0.0,
                Some(TabAlign::Center) => upto(next) / 2.0,
                Some(TabAlign::Right) => upto(next),
                Some(TabAlign::Decimal) => upto((k + 1..next).rfind(|&j| point(j)).unwrap_or(next)),
            };
            let at = stop.map_or(((x + 0.01) / 36.0).floor() * 36.0 + 36.0, |s| s.at as f32);
            ws[k] = (at - x - after).max(0.0);
            out.push((k, stop));
        }
        x += ws[k];
    }
    out
}

/// A paragraph in a list: a bullet, or a number that counts on from the paragraph
/// before, hung in front of its lines.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum List {
    #[default]
    None,
    Bullet,
    Number,
}

/// A language that hyphenation knows.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Lang {
    #[default]
    En,
    De,
    Fr,
    It,
    Es,
    Nl,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextCase {
    #[default]
    Original,
    Upper,
    Lower,
    /// Each word's first letter in upper case.
    Title,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    Strikethrough,
}

/// Superscript and subscript set a character smaller and off its baseline.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Position {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

/// What a character looks like. Sizes and spacing are in pt, `letter_spacing` in % of
/// the size; `line_height` 0 is auto, the font's ascent plus descent. `fill` replaces
/// the layer's fills; `text_style` "" means none. `features` are OpenType features as
/// harfrust parses them, e.g. "smcp" or "liga=0". `baseline_shift` raises the
/// characters off the baseline in pt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Attrs {
    pub size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub paragraph_spacing: f64,
    pub fill: Option<Color>,
    pub text_style: String,
    pub text_align: TextAlign,
    pub hyphenate: bool,
    pub lang: Lang,
    /// `None` is the bundled font.
    pub font: Option<Typeface>,
    pub paragraph_indent: f64,
    pub text_case: TextCase,
    pub text_decoration: TextDecoration,
    pub features: Vec<String>,
    pub position: Position,
    pub baseline_shift: f64,
    /// Lines the first `drop_chars` characters of a paragraph drop over; 0 is off.
    pub drop_lines: u32,
    pub drop_chars: u32,
    /// Lines at the start and at the end of a paragraph that a column break keeps together.
    pub keep_lines: u32,
    pub keep_together: bool,
    pub keep_next: bool,
    pub tabs: Vec<Tab>,
    pub list: List,
}

impl Attrs {
    /// The size the glyphs are drawn at: superscript and subscript at 58.3 %, as in InDesign.
    fn drawn(&self) -> f32 {
        match self.position {
            Position::Normal => self.size as f32,
            _ => self.size as f32 * 0.583,
        }
    }

    /// How far the glyphs sit above the baseline.
    fn rise(&self) -> f32 {
        let by = match self.position {
            Position::Normal => 0.0,
            Position::Superscript => 0.333,
            Position::Subscript => -0.333,
        };
        (self.baseline_shift + self.size * by) as f32
    }
}

impl Default for Attrs {
    fn default() -> Self {
        Attrs {
            size: 12.0,
            line_height: 0.0,
            letter_spacing: 0.0,
            paragraph_spacing: 0.0,
            fill: None,
            text_style: String::new(),
            text_align: TextAlign::Left,
            hyphenate: false,
            lang: Lang::En,
            font: None,
            paragraph_indent: 0.0,
            text_case: TextCase::Original,
            text_decoration: TextDecoration::None,
            features: Vec::new(),
            position: Position::Normal,
            baseline_shift: 0.0,
            drop_lines: 0,
            drop_chars: 1,
            keep_lines: 1,
            keep_together: false,
            keep_next: false,
            tabs: Vec::new(),
            list: List::None,
        }
    }
}

/// `len` characters in UTF-16 code units that share `attrs`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Span {
    pub len: usize,
    #[serde(flatten)]
    pub attrs: Attrs,
}

/// Named values for the `STYLED` attributes of `attrs`; `bindings` holds variables by key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub attrs: Attrs,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

impl TextStyle {
    /// The `STYLED` attributes with bound variables resolved in `scope`.
    pub fn values(&self, scope: &Scope) -> Map<String, serde_json::Value> {
        let own = serde_json::to_value(&self.attrs).unwrap();
        STYLED
            .iter()
            .map(|&k| {
                let bound = self.bindings.get(k).and_then(|v| match scope.value(v) {
                    Some(&Value::Number(n)) if k == "letterSpacing" => Some(n.into()),
                    Some(&Value::Number(n)) => Some(n.max(0.0).into()),
                    Some(Value::Font(f)) => serde_json::to_value(f).ok(),
                    _ => None,
                });
                (k.into(), bound.unwrap_or_else(|| own[k].clone()))
            })
            .collect()
    }
}

/// Byte offsets in `para` where `lang` allows a hyphen, ascending.
fn syllables(para: &str, lang: Lang) -> Vec<usize> {
    let lang = match lang {
        Lang::En => hypher::Lang::English,
        Lang::De => hypher::Lang::German,
        Lang::Fr => hypher::Lang::French,
        Lang::It => hypher::Lang::Italian,
        Lang::Es => hypher::Lang::Spanish,
        Lang::Nl => hypher::Lang::Dutch,
    };
    let mut out = Vec::new();
    let mut rest = para;
    while let Some(i) = rest.find(char::is_alphabetic) {
        let word_start = para.len() - rest.len() + i;
        let word = &rest[i..];
        let len = word
            .find(|c: char| !c.is_alphabetic())
            .unwrap_or(word.len());
        let mut at = word_start;
        let mut parts = hypher::hyphenate(&word[..len], lang).peekable();
        while let Some(s) = parts.next() {
            at += s.len();
            if parts.peek().is_some() {
                out.push(at);
            }
        }
        rest = &word[len..];
    }
    out
}

fn hyphen_width(it: &Item) -> f32 {
    match *it {
        Item::Penalty { width, .. } => width,
        _ => 0.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

/// How a text layer sets its text: insets and gutter in pt, columns of equal width,
/// and baselines on a grid of `baseline_grid` pt from `baseline_start` below the top
/// inset; a grid of 0 is off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TextFrame {
    pub inset_top: f64,
    pub inset_right: f64,
    pub inset_bottom: f64,
    pub inset_left: f64,
    pub columns: u32,
    pub gutter: f64,
    pub vertical_align: VerticalAlign,
    pub baseline_grid: f64,
    pub baseline_start: f64,
    /// Lines the text is cut after, the last ending in an ellipsis; 0 is off.
    pub max_lines: u32,
    /// What a page number marker stands for: the number of the page the text is on.
    #[serde(skip)]
    pub number: String,
}

impl Default for TextFrame {
    fn default() -> Self {
        TextFrame {
            inset_top: 0.0,
            inset_right: 0.0,
            inset_bottom: 0.0,
            inset_left: 0.0,
            columns: 1,
            gutter: 12.0,
            vertical_align: VerticalAlign::Top,
            baseline_grid: 0.0,
            baseline_start: 0.0,
            max_lines: 0,
            number: String::new(),
        }
    }
}

/// Stands for the number of the page a text is on, as InDesign's Current Page Number.
pub const PAGE_NUMBER: char = '\u{18}';

/// A glyph at (x, y) on its baseline, `size` pt, drawn with the attributes of `span`,
/// for the text from byte `cluster`, or a hyphen inserted there.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub span: usize,
    pub cluster: usize,
    pub hyphen: bool,
}

/// Glyph runs of `text` from the byte `from` set in `frame`; characters without a
/// fill take the fills of `style`, its strokes go around the glyphs.
pub fn draw(
    text: &str,
    spans: &[Span],
    style: &Style,
    frame: [f32; 4],
    tf: &TextFrame,
    s: &Scope,
    from: usize,
) -> Vec<Op> {
    let fonts: Vec<u32> = spans.iter().map(|s| font_id(&s.attrs.font)).collect();
    let span_paints: Vec<Vec<Paint>> = spans
        .iter()
        .map(|sp| match &sp.attrs.fill {
            Some(c) => vec![Paint::Solid {
                color: c.rgba(s),
                ink: c.ink(s),
            }],
            None => paints(&style.fills, frame, s).collect(),
        })
        .collect();
    let mut ops = Vec::new();
    let stroked = style.strokes.iter().any(|f| f.visible);
    let mut outlines = Vec::new();
    let placed = set(text, spans, frame, tf, from).0;
    for (line, geometry) in placed.lines.into_iter().zip(&placed.geometry) {
        let mut at = 0;
        for run in line.chunk_by(|a, b| {
            a.size == b.size
                && span_paints[a.span] == span_paints[b.span]
                && fonts[a.span] == fonts[b.span]
        }) {
            let (run_text, ranges) = source(text, &line, at..at + run.len(), &tf.number);
            at += run.len();
            let glyphs: Vec<u16> = run.iter().map(|g| g.id).collect();
            let positions: Vec<f32> = run.iter().flat_map(|g| [g.x, g.y]).collect();
            if stroked {
                outlines.extend(outline(
                    fonts[run[0].span],
                    run[0].size,
                    &glyphs,
                    &positions,
                ));
            }
            for paint in &span_paints[run[0].span] {
                ops.push(Op::GlyphRun {
                    font: fonts[run[0].span],
                    size: run[0].size,
                    paint: paint.clone(),
                    glyphs: glyphs.clone(),
                    positions: positions.clone(),
                    text: run_text.clone(),
                    ranges: ranges.clone(),
                });
            }
        }
        for run in line.chunk_by(|a, b| {
            let (a, b) = (&spans[a.span].attrs, &spans[b.span].attrs);
            a.text_decoration == b.text_decoration && a.drawn() == b.drawn() && a.fill == b.fill
        }) {
            let run: Vec<_> = run
                .iter()
                .filter(|g| g.size == spans[g.span].attrs.drawn())
                .collect();
            if run.is_empty() {
                continue;
            }
            let a = &spans[run[0].span].attrs;
            let Some([at, thick]) = decoration(font_id(&a.font), a.text_decoration) else {
                continue;
            };
            let last = run[run.len() - 1].cluster;
            let end = geometry.stops.iter().find(|s| s.0 > last);
            let x1 = end.or(geometry.stops.last()).map_or(0.0, |s| s.1);
            let (x0, size) = (run[0].x, a.drawn());
            let line = rect(x0, run[0].y - at * size, x1 - x0, thick * size);
            for paint in &span_paints[run[0].span] {
                ops.push(Op::FillPath {
                    paint: paint.clone(),
                    path: line.clone(),
                });
            }
            if stroked {
                outlines.extend(line);
            }
        }
    }
    if !outlines.is_empty() {
        ops.extend(style.stroke(&outlines, frame, s));
    }
    ops
}

/// The outlines of `glyphs` at `positions` in the font `id` as a display list path.
pub fn outline(id: u32, size: f32, glyphs: &[u16], positions: &[f32]) -> Vec<f32> {
    let bytes = font_bytes(id);
    let Ok(font) = skrifa::FontRef::new(&bytes) else {
        return Vec::new();
    };
    let outlines = font.outline_glyphs();
    let mut pen = Pen {
        path: Vec::new(),
        origin: [0.0; 2],
        at: [0.0; 2],
    };
    for (g, p) in glyphs.iter().zip(positions.chunks(2)) {
        if let Some(o) = outlines.get(skrifa::GlyphId::new(*g as u32)) {
            pen.origin = [p[0], p[1]];
            let size = skrifa::instance::Size::new(size);
            let settings = skrifa::outline::DrawSettings::unhinted(
                size,
                skrifa::instance::LocationRef::default(),
            );
            let _ = o.draw(settings, &mut pen);
        }
    }
    pen.path
}

struct Pen {
    path: Vec<f32>,
    origin: [f32; 2],
    at: [f32; 2],
}

impl Pen {
    fn point(&mut self, x: f32, y: f32) -> [f32; 2] {
        [self.origin[0] + x, self.origin[1] - y]
    }
}

impl skrifa::outline::OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.path.extend([MOVE, self.at[0], self.at[1]]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.path.extend([LINE, self.at[0], self.at[1]]);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let ([ax, ay], [cx, cy], [x, y]) = (self.at, self.point(cx, cy), self.point(x, y));
        let k = 2.0 / 3.0;
        self.path.extend([
            CUBIC,
            ax + k * (cx - ax),
            ay + k * (cy - ay),
            x + k * (cx - x),
            y + k * (cy - y),
            x,
            y,
        ]);
        self.at = [x, y];
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let ([ax, ay], [bx, by]) = (self.point(cx0, cy0), self.point(cx1, cy1));
        self.at = self.point(x, y);
        self.path
            .extend([CUBIC, ax, ay, bx, by, self.at[0], self.at[1]]);
    }
    fn close(&mut self) {
        self.path.push(CLOSE);
    }
}

/// Top and thickness in em of the line that `decoration` draws in the font `id`, with
/// the top above the baseline.
fn decoration(id: u32, decoration: TextDecoration) -> Option<[f32; 2]> {
    let bytes = font_bytes(id);
    let f = FontRef::new(&bytes).ok()?;
    let upem = f.head().ok()?.units_per_em() as f32;
    let [at, thick] = match decoration {
        TextDecoration::None => return None,
        TextDecoration::Underline => f.post().ok().map_or([-100, 50], |p| {
            [
                p.underline_position().to_i16(),
                p.underline_thickness().to_i16(),
            ]
        }),
        TextDecoration::Strikethrough => f.os2().ok().map_or([300, 50], |o| {
            [o.y_strikeout_position(), o.y_strikeout_size()]
        }),
    }
    .map(|v| v as f32 / upem);
    Some([at, thick.max(0.02)])
}

/// The text that the glyphs `run` of `line` stand for and each glyph's range in it:
/// up to the next glyph's text or space, so a ligature stands for all its letters.
/// Spaces have no glyph; PDF readers find them in the gaps. A page number marker
/// stands for `number`.
fn source(
    text: &str,
    line: &[Glyph],
    run: std::ops::Range<usize>,
    number: &str,
) -> (String, Vec<std::ops::Range<usize>>) {
    let mut out = String::new();
    let mut ranges = Vec::new();
    for k in run {
        let g = &line[k];
        let start = out.len();
        if g.hyphen {
            out.push('-');
        } else if k == 0 || line[k - 1].hyphen || line[k - 1].cluster < g.cluster {
            let next = line[k + 1..]
                .iter()
                .map(|n| n.cluster)
                .find(|&n| n > g.cluster)
                .unwrap_or(text.len());
            let word = text[g.cluster..next]
                .find(char::is_whitespace)
                .map_or(next, |i| g.cluster + i);
            out.push_str(&text[g.cluster..word].replace(PAGE_NUMBER, number));
        }
        ranges.push(start..out.len());
    }
    (out, ranges)
}

/// The lines of `text` from the byte `from` that fit in `frame`, each a list of glyphs.
#[cfg(test)]
pub fn lay_out(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    from: usize,
) -> Vec<Vec<Glyph>> {
    set(text, spans, frame, tf, from).0.lines
}

/// The byte where the text from `from` that does not fit in `frame` starts, for the
/// next frame of a thread; `None` when it all fits.
pub fn overflow(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    from: usize,
) -> Option<usize> {
    set(text, spans, frame, tf, from).1
}

/// The text from the byte `from` set in `frame`, and the byte where the text that
/// does not fit starts. Only about as many lines as the columns can hold are broken.
fn set(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    from: usize,
) -> (Placed, Option<usize>) {
    let (inner, cw) = columns(frame, tf);
    let room = (inner[3] + 0.01) * tf.columns.max(1) as f32;
    let rows = rows(text, spans, Some(cw), from, tf, room).0;
    let starts: Vec<usize> = rows.iter().map(|r| r.stops[0].0).collect();
    let placed = place(rows, inner, cw, tf);
    let rest = starts.get(placed.placed).copied();
    (placed, rest)
}

/// The frame size [w, h] that `text` from the byte `from` needs at the width `w`, or
/// on unbroken lines: the least height at which every line fits, over all columns.
pub fn measure(
    text: &str,
    spans: &[Span],
    tf: &TextFrame,
    w: Option<f32>,
    from: usize,
) -> [f32; 2] {
    let (h_in, v_in) = (
        (tf.inset_left + tf.inset_right) as f32,
        (tf.inset_top + tf.inset_bottom) as f32,
    );
    let n = tf.columns.max(1) as f32;
    let (rows, cw, w) = match w {
        Some(w) => {
            let (_, cw) = columns([0.0, 0.0, w, 0.0], tf);
            (
                rows(text, spans, Some(cw), from, tf, f32::INFINITY).0,
                cw,
                w,
            )
        }
        None => {
            let (rows, natural) = rows(text, spans, None, from, tf, f32::INFINITY);
            let cw = ceil(natural);
            (rows, cw, n * cw + (n - 1.0) * tf.gutter as f32 + h_in)
        }
    };
    let top = tf.inset_top as f32;
    let fit = |ih: f32, columns: u32| {
        let tf = TextFrame {
            columns,
            vertical_align: VerticalAlign::Top,
            ..tf.clone()
        };
        place(rows.clone(), [0.0, top, cw, ih], cw, &tf)
    };
    let one = fit(f32::MAX, 1).bottom - top;
    let ih = if tf.columns <= 1 {
        one
    } else {
        let (mut lo, mut hi) = (0.0, one);
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if fit(mid, tf.columns).placed == rows.len() {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    };
    [w, ih.max(0.0) + v_in]
}

/// `v` rounded up to 0.01, so that a line measured at its natural width fits again.
fn ceil(v: f32) -> f32 {
    (v * 100.0).ceil() / 100.0
}

/// The box inside the insets of `frame` and the width of each column in it.
fn columns([x, y, w, h]: [f32; 4], tf: &TextFrame) -> ([f32; 4], f32) {
    let [top, right, bottom, left] =
        [tf.inset_top, tf.inset_right, tf.inset_bottom, tf.inset_left].map(|v| v as f32);
    let (iw, ih) = ((w - left - right).max(0.0), (h - top - bottom).max(0.0));
    let n = tf.columns.max(1);
    let cw = ((iw - (n - 1) as f32 * tf.gutter as f32) / n as f32).max(0.0);
    ([x + left, y + top, iw, ih], cw)
}

/// Metrics of a font in em, and its hyphen.
struct Metrics {
    upem: f32,
    cap: f32,
    ascent: f32,
    descent: f32,
    gap: f32,
    hyphen: u16,
    hyphen_adv: f32,
}

/// A shaped paragraph: its items, their glyphs and clusters, the attributes of its
/// first character and the byte its first line starts at. Line `i`
/// starts `left[i]` in, the last for the lines after; `marks` are glyphs beside the
/// lines, placed from the first baseline, with `stops` for the text they stand for,
/// and the paragraph takes at least `least` lines, which stay together, as do its
/// first `head`.
struct Para {
    items: Vec<Item>,
    glyphs: Vec<Option<Glyph>>,
    clusters: Vec<usize>,
    first: Attrs,
    start: usize,
    left: Vec<f32>,
    marks: Vec<Glyph>,
    stops: Vec<(usize, f32)>,
    least: usize,
    head: usize,
}

/// The lines of `text` from the byte `from`, a line start, broken to the column width
/// `cw`, or each paragraph on one line when `None`; and the width of the widest
/// paragraph set on one line. A page number marker is set as `number`. Paragraphs
/// stop once their lines need more than `room` in height.
fn rows(
    text: &str,
    spans: &[Span],
    cw: Option<f32>,
    from: usize,
    tf: &TextFrame,
    room: f32,
) -> (Vec<Row>, f32) {
    let number = &tf.number;
    let ids: Vec<u32> = spans.iter().map(|s| font_id(&s.attrs.font)).collect();
    let mut used = ids.clone();
    used.sort_unstable();
    used.dedup();
    let slot: Vec<usize> = ids.iter().map(|i| used.binary_search(i).unwrap()).collect();
    let bytes: Vec<_> = used.iter().map(|&i| font_bytes(i)).collect();
    let fonts: Vec<_> = bytes.iter().map(|b| FontRef::new(b).unwrap()).collect();
    let data: Vec<_> = fonts.iter().map(ShaperData::new).collect();
    let shapers: Vec<_> = fonts
        .iter()
        .zip(&data)
        .map(|(f, d)| d.shaper(f).build())
        .collect();
    let metrics: Vec<Metrics> = fonts
        .iter()
        .zip(&shapers)
        .map(|(font, shaper)| {
            let upem = font.head().unwrap().units_per_em() as f32;
            let hhea = font.hhea().unwrap();
            let mut buf = UnicodeBuffer::new();
            buf.push_str("-");
            buf.guess_segment_properties();
            let dash = shaper.shape(buf, ShapeOptions::new());
            Metrics {
                upem,
                cap: font
                    .os2()
                    .ok()
                    .and_then(|o| o.s_cap_height())
                    .map_or(0.7, |c| c as f32 / upem),
                ascent: hhea.ascender().to_i16() as f32 / upem,
                descent: -hhea.descender().to_i16() as f32 / upem,
                gap: hhea.line_gap().to_i16() as f32 / upem,
                hyphen: dash.glyph_infos()[0].glyph_id as u16,
                hyphen_adv: dash.glyph_positions()[0].x_advance as f32 / upem,
            }
        })
        .collect();

    let mut ends = Vec::with_capacity(spans.len());
    let mut chars = text.chars();
    let mut at = 0;
    for s in spans {
        let mut n = 0;
        while n < s.len {
            let Some(c) = chars.next() else { break };
            n += c.len_utf16();
            at += c.len_utf8();
        }
        ends.push(at);
    }
    let span_at = |byte: usize| ends.partition_point(|&e| e <= byte).min(spans.len() - 1);
    let vertical = |span: usize| {
        let (a, m) = (&spans[span].attrs, &metrics[slot[span]]);
        let (ascent, descent, gap) = (m.ascent, m.descent, m.gap);
        let s = a.drawn();
        let l = match a.line_height as f32 {
            0.0 => (ascent + descent + gap) * s,
            l => l,
        };
        let above = ascent * s + (l - (ascent + descent) * s) / 2.0;
        (above + a.rise(), l - above - a.rise())
    };

    // `text` repeated in whole copies over `x..x + w`, on a grid of its width.
    // `text` shaped in the font of `span`, from x 0 on the baseline, and its width.
    let shape = |text: &str, span: usize, cluster: usize| {
        let (a, k) = (&spans[span].attrs, slot[span]);
        let mut buf = UnicodeBuffer::new();
        buf.push_str(text);
        buf.guess_segment_properties();
        let out = shapers[k].shape(buf, ShapeOptions::new());
        let scale = a.drawn() / metrics[k].upem;
        let mut x = 0.0;
        let mut glyphs = Vec::new();
        for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions()) {
            glyphs.push(Glyph {
                id: info.glyph_id as u16,
                x: x + pos.x_offset as f32 * scale,
                y: -a.rise(),
                size: a.drawn(),
                span,
                cluster,
                hyphen: false,
            });
            x += pos.x_advance as f32 * scale;
        }
        (glyphs, x)
    };
    // `text` repeated in whole copies over `x..x + w`, on a grid of its width.
    let leader = |text: &str, span: usize, cluster: usize, x: f32, w: f32| {
        let (copy, lw) = shape(text, span, cluster);
        let mut glyphs = Vec::new();
        let mut at = if lw > 0.0 {
            (x / lw).ceil() * lw
        } else {
            f32::INFINITY
        };
        while at + lw <= x + w + 0.01 {
            glyphs.extend(copy.iter().map(|g| Glyph {
                x: at + g.x,
                ..g.clone()
            }));
            at += lw;
        }
        glyphs
    };
    let tab_at =
        |it: &Item, c: usize| matches!(it, Item::Glue { .. }) && text[c..].starts_with('\t');
    let point_at =
        |it: &Item, c: usize| matches!(it, Item::Box(_)) && text[c..].starts_with(['.', ',']);
    let break_para = |Para {
                          items,
                          glyphs,
                          clusters,
                          first,
                          start,
                          left,
                          marks,
                          stops: lead,
                          least,
                          head,
                      }: Para,
                      cw: f32,
                      rows: &mut Vec<Row>| {
        let widths: Vec<f32> = left.iter().map(|l| (cw - l).max(0.0)).collect();
        let set = rows.len();
        let mut from = 0;
        for (i, (end, r)) in break_lines(&items, &widths).into_iter().enumerate() {
            let (left, cw) = (left[i.min(left.len() - 1)], widths[i.min(left.len() - 1)]);
            let line_start = if from == 0 { start } else { clusters[from] };
            while from < end && !matches!(items[from], Item::Box(_)) {
                from += 1;
            }
            let r = match first.text_align {
                TextAlign::Justify if r.is_finite() => r.max(-1.0),
                _ if r.is_finite() => r.clamp(-1.0, 0.0),
                _ => 0.0,
            };
            let spread = |it: &Item| match *it {
                Item::Box(adv) => adv,
                Item::Glue {
                    width,
                    stretch,
                    shrink,
                } => width + r * if r < 0.0 { shrink } else { stretch },
                Item::Penalty { .. } => 0.0,
            };
            let hyphenated = matches!(items[end], Item::Penalty { width, .. } if width > 0.0);
            let last = if hyphenated { end + 1 } else { end };
            let mut ws: Vec<f32> = items[from..last].iter().map(spread).collect();
            let leaders = tabs(
                &first.tabs,
                &mut ws,
                |j| tab_at(&items[from + j], clusters[from + j]),
                |j| point_at(&items[from + j], clusters[from + j]),
                left,
            );
            let used: f32 = ws.iter().sum::<f32>()
                + if hyphenated {
                    hyphen_width(&items[end])
                } else {
                    0.0
                };
            let spans_here: Vec<usize> = (from..last)
                .filter(|&k| matches!(items[k], Item::Box(_)) || k == end)
                .filter_map(|k| glyphs[k].as_ref().map(|g| g.span))
                .collect();
            let (above, below) = if spans_here.is_empty() {
                vertical(span_at(start))
            } else {
                spans_here
                    .iter()
                    .map(|&s| vertical(s))
                    .fold((0.0f32, 0.0f32), |(a, b), (c, d)| (a.max(c), b.max(d)))
            };
            let mut cx = left
                + match first.text_align {
                    TextAlign::Center => (cw - used) / 2.0,
                    TextAlign::Right => cw - used,
                    _ => 0.0,
                };
            let mut line = if i == 0 { marks.clone() } else { Vec::new() };
            let mut stops: Vec<(usize, f32)> = if i == 0 { lead.clone() } else { Vec::new() };
            let mut end_x = None;
            for k in from..last {
                if items[k] == FILL {
                    end_x.get_or_insert(cx);
                }
                if matches!(items[k], Item::Penalty { .. }) && k != end {
                    continue;
                }
                let indent = matches!(items[k], Item::Box(_)) && glyphs[k].is_none();
                if !matches!(items[k], Item::Penalty { .. })
                    && !indent
                    && stops.last().is_none_or(|s| s.0 < clusters[k])
                {
                    stops.push((clusters[k], cx));
                }
                if let Some(g) = &glyphs[k] {
                    line.push(Glyph {
                        x: cx + g.x,
                        ..g.clone()
                    });
                }
                if let Some(&(_, Some(stop))) = leaders.iter().find(|l| from + l.0 == k) {
                    line.extend(leader(
                        &stop.leader,
                        span_at(clusters[k]),
                        clusters[k],
                        cx,
                        ws[k - from],
                    ));
                }
                cx += ws[k - from];
            }
            if stops.first().is_none_or(|s| s.0 > line_start) {
                stops.insert(0, (line_start, stops.first().map_or(cx, |s| s.1)));
            }
            stops.push((clusters[end].max(line_start), end_x.unwrap_or(cx)));
            rows.push(Row {
                glyphs: line,
                above,
                below,
                after: 0.0,
                stops,
                keep: false,
            });
            from = end + 1;
        }
        let short: f32 = rows[set.min(rows.len())..]
            .iter()
            .map(|r| r.above + r.below)
            .sum::<f32>()
            / (rows.len() - set).max(1) as f32
            * least.saturating_sub(rows.len() - set) as f32;
        let (n, tail) = (rows.len() - set, first.keep_lines as usize);
        for (k, r) in rows[set..].iter_mut().enumerate() {
            r.keep = if k + 1 == n {
                first.keep_next
            } else {
                first.keep_together || k + 1 < head.max(least) || k + tail >= n
            };
        }
        if let Some(r) = rows.last_mut() {
            r.after = first.paragraph_spacing as f32 + short;
        }
    };

    let mut rows: Vec<Row> = Vec::new();
    let (mut widest, mut used) = (0.0f32, 0.0f32);
    let mut paras = Vec::new();
    let mut start = 0;
    let mut count = 0;
    for para in text.split('\n') {
        let list = spans[span_at(start)].attrs.list;
        count = if list == List::Number { count + 1 } else { 0 };
        if start + para.len() < from {
            start += para.len() + 1;
            continue;
        }
        let first = &spans[span_at(start)].attrs;
        let mut shaped = Vec::new();
        let mut chars = para.char_indices().peekable();
        let mut prev = None;
        while let Some(&(i, _)) = chars.peek() {
            let first = span_at(start + i);
            let (k, a) = (slot[first], &spans[first].attrs);
            let same = |i: usize| {
                let s = span_at(start + i);
                slot[s] == k && spans[s].attrs.features == a.features
            };
            let mut buf = UnicodeBuffer::new();
            while let Some((i, c)) = chars.next_if(|&(i, _)| same(i)) {
                let mut add = |c| buf.add(c, i as u32);
                let word = prev.is_some_and(char::is_alphanumeric);
                match (c, spans[span_at(start + i)].attrs.text_case) {
                    (PAGE_NUMBER, _) => number.chars().for_each(add),
                    (c, TextCase::Upper) => c.to_uppercase().for_each(add),
                    (c, TextCase::Title) if !word => c.to_uppercase().for_each(add),
                    (c, TextCase::Lower) => c.to_lowercase().for_each(add),
                    (c, _) => add(c),
                }
                prev = Some(c);
            }
            buf.guess_segment_properties();
            let features: Vec<Feature> = a.features.iter().filter_map(|f| f.parse().ok()).collect();
            let out = shapers[k].shape(buf, ShapeOptions::new().features(&features));
            shaped.extend(out.glyph_infos().iter().zip(out.glyph_positions()).map(
                |(info, pos)| {
                    let scale = 1.0 / metrics[k].upem;
                    let cluster = info.cluster as usize;
                    let [adv, dx, dy] =
                        [pos.x_advance, pos.x_offset, pos.y_offset].map(|v| v as f32 * scale);
                    (cluster, info.glyph_id as u16, adv, dx, dy)
                },
            ));
        }
        let breaks = if first.hyphenate {
            syllables(para, first.lang)
        } else {
            Vec::new()
        };
        // The first characters drop over `drop_lines` lines, their cap height from the
        // first line's to the last line's baseline.
        let lines = first.drop_lines as usize;
        let dropped = match para.char_indices().nth(first.drop_chars as usize) {
            _ if lines < 2 || from > start => 0,
            Some((i, _)) => i,
            None => para.len(),
        };
        let first_span = span_at(start);
        let (above, below) = vertical(first_span);
        let cap = metrics[slot[first_span]].cap;
        let drop = first.drawn() + (lines.max(1) - 1) as f32 * (above + below) / cap;
        let (mut marks, mut lead, mut dw) = (Vec::new(), Vec::new(), 0.0);
        let mut items = Vec::new();
        let mut glyphs = Vec::new();
        let mut clusters = Vec::new();
        for &(at, id, adv, dx, dy) in &shaped {
            let cluster = start + at;
            let span = span_at(cluster);
            let (a, m) = (&spans[span].attrs, &metrics[slot[span]]);
            if at < dropped {
                if lead.last().is_none_or(|s: &(usize, f32)| s.0 < cluster) {
                    lead.push((cluster, dw));
                }
                marks.push(Glyph {
                    id,
                    x: dw + dx * drop,
                    y: (above + below) * (lines - 1) as f32 - dy * drop,
                    size: drop,
                    span,
                    cluster,
                    hyphen: false,
                });
                dw += adv * drop + drop * (a.letter_spacing / 100.0) as f32;
                continue;
            }
            let (size, rise) = (a.drawn(), a.rise());
            let adv = adv * size + size * (a.letter_spacing / 100.0) as f32;
            if breaks.binary_search(&at).is_ok() {
                items.push(Item::Penalty {
                    width: m.hyphen_adv * size,
                    cost: HYPHEN_COST,
                });
                glyphs.push(Some(Glyph {
                    id: m.hyphen,
                    x: 0.0,
                    y: -rise,
                    size,
                    span,
                    cluster,
                    hyphen: true,
                }));
                clusters.push(cluster);
            }
            clusters.push(cluster);
            if para[at..].starts_with('\t') {
                items.push(Item::Glue {
                    width: 0.0,
                    stretch: 0.0,
                    shrink: 0.0,
                });
                glyphs.push(None);
            } else if para[at..].starts_with(' ') {
                items.push(Item::Glue {
                    width: adv,
                    stretch: adv / 2.0,
                    shrink: adv / 3.0,
                });
                glyphs.push(None);
            } else {
                items.push(Item::Box(adv));
                glyphs.push(Some(Glyph {
                    id,
                    x: dx * size,
                    y: -(dy * size + rise),
                    size,
                    span,
                    cluster,
                    hyphen: false,
                }));
            }
        }
        if dw > 0.0 {
            dw += first.drawn() / 4.0;
        } else if first.paragraph_indent > 0.0 {
            items.insert(0, Item::Box(first.paragraph_indent as f32));
            glyphs.insert(0, None);
            clusters.insert(0, start);
        }
        let hang = match list {
            List::None => 0.0,
            List::Bullet | List::Number => {
                let marker = match list {
                    List::Number => format!("{count}."),
                    _ => "\u{2022}".into(),
                };
                let (mut mark, w) = shape(&marker, first_span, start);
                let hang = (first.drawn() * 1.5).max(w + first.drawn() / 4.0);
                marks.iter_mut().for_each(|g| g.x += hang);
                lead.iter_mut().for_each(|s| s.1 += hang);
                if from <= start {
                    mark.append(&mut marks);
                    marks = mark;
                }
                hang
            }
        };
        let mut ws: Vec<f32> = items
            .iter()
            .map(|it| match *it {
                Item::Box(w) | Item::Glue { width: w, .. } => w,
                Item::Penalty { .. } => 0.0,
            })
            .collect();
        let set = |j: usize| tab_at(&items[j], clusters[j]);
        let point = |j: usize| point_at(&items[j], clusters[j]);
        for (k, _) in tabs(&first.tabs, &mut ws, set, point, dw + hang) {
            items[k] = Item::Glue {
                width: ws[k],
                stretch: 0.0,
                shrink: 0.0,
            };
        }
        let natural: f32 = ws.iter().sum();
        items.extend([FILL, FORCE]);
        glyphs.extend([None, None]);
        clusters.extend([start + para.len(); 2]);
        // A paragraph that an earlier frame began goes on at its first glyph from
        // `from`, without the space or hyphen it broke at.
        let mut line0 = start;
        if from > start {
            let mut k = clusters.partition_point(|&c| c < from);
            while k < items.len() - 2 && !matches!(items[k], Item::Box(_)) {
                k += 1;
            }
            items.drain(..k);
            glyphs.drain(..k);
            clusters.drain(..k);
            line0 = from;
        }
        start += para.len() + 1;
        let natural = natural + dw + hang;
        widest = widest.max(natural);
        let mut left = vec![dw + hang; if dw > 0.0 { lines } else { 1 }];
        left.push(hang);
        let para = Para {
            items,
            glyphs,
            clusters,
            first: first.clone(),
            start: line0,
            left,
            marks,
            stops: lead,
            least: if dw > 0.0 { lines } else { 0 },
            head: if from > start {
                0
            } else {
                first.keep_lines as usize
            },
        };
        let Some(cw) = cw else {
            paras.push(para);
            continue;
        };
        let set = rows.len();
        break_para(para, cw, &mut rows);
        used += rows[set..]
            .iter()
            .map(|r| r.above + r.below + r.after)
            .sum::<f32>();
        if used > room {
            break;
        }
    }
    if cw.is_none() {
        for para in paras {
            break_para(para, ceil(widest), &mut rows);
        }
    }
    let max = tf.max_lines as usize;
    if max > 0 && rows.len() > max {
        rows.truncate(max);
        let row = &mut rows[max - 1];
        let span = row
            .glyphs
            .last()
            .map_or(span_at(row.stops[0].0), |g| g.span);
        let (dots, w) = shape("\u{2026}", span, text.len());
        let right = cw.unwrap_or(widest);
        let mut x = row.stops.last().unwrap().1;
        while x + w > right + 0.01
            && let Some(g) = row.glyphs.pop()
        {
            x = g.x;
        }
        row.glyphs
            .extend(dots.into_iter().map(|g| Glyph { x: x + g.x, ..g }));
        row.after = 0.0;
    }
    (rows, widest)
}

/// `rows` placed in columns `cw` wide in the box `[ix, iy, _, ih]` inside the
/// insets, as many as fit.
fn place(rows: Vec<Row>, [ix, iy, _, ih]: [f32; 4], cw: f32, tf: &TextFrame) -> Placed {
    let n = tf.columns.max(1);
    let gutter = tf.gutter as f32;
    let (grid, grid_top) = (tf.baseline_grid as f32, iy + tf.baseline_start as f32);
    let snap = |b: f32| match grid {
        0.0 => b,
        g => grid_top + ((b - grid_top - 0.001) / g).ceil().max(0.0) * g,
    };
    let bottom = iy + ih + 0.01;
    let mut lines: Vec<Vec<Glyph>> = Vec::new();
    let mut geometry: Vec<Line> = Vec::new();
    let mut columns: Vec<(usize, f32)> = Vec::new();
    let (mut col, mut top, mut i, mut first) = (0, iy, 0, 0);
    while let Some(row) = rows.get(i) {
        let baseline = snap(top + row.above);
        if baseline + row.below > bottom {
            let mut j = i;
            while j > first && rows[j - 1].keep {
                j -= 1;
            }
            if j > first {
                lines.truncate(j);
                geometry.truncate(j);
                columns[col as usize].1 = geometry[j - 1].bottom;
                i = j;
            }
            col += 1;
            if col >= n {
                break;
            }
            (top, first) = (iy, i);
            continue;
        }
        if columns.len() <= col as usize {
            columns.push((lines.len(), 0.0));
        }
        let cx = ix + col as f32 * (cw + gutter);
        geometry.push(Line {
            start: row.stops[0].0,
            end: row.stops.last().unwrap().0,
            top: baseline - row.above,
            bottom: baseline + row.below,
            left: cx,
            right: cx + cw,
            stops: row.stops.iter().map(|&(b, x)| (b, cx + x)).collect(),
        });
        lines.push(
            row.glyphs
                .iter()
                .map(|g| Glyph {
                    x: cx + g.x,
                    y: baseline + g.y,
                    ..g.clone()
                })
                .collect(),
        );
        columns[col as usize].1 = baseline + row.below;
        top = baseline + row.below + row.after;
        i += 1;
    }
    let k = match tf.vertical_align {
        VerticalAlign::Top => 0.0,
        VerticalAlign::Center => 0.5,
        VerticalAlign::Bottom => 1.0,
    };
    for (c, &(first, end)) in columns.iter().enumerate() {
        let last = columns.get(c + 1).map_or(lines.len(), |n| n.0);
        let mut shift = (iy + ih - end) * k;
        if grid > 0.0 {
            shift = (shift / grid).floor() * grid;
        }
        for g in lines[first..last].iter_mut().flatten() {
            g.y += shift;
        }
        for l in &mut geometry[first..last] {
            l.top += shift;
            l.bottom += shift;
        }
    }
    Placed {
        placed: lines.len(),
        geometry,
        bottom: columns.iter().map(|c| c.1).fold(iy, f32::max),
        lines,
    }
}

/// The lines that fit, how many of the rows that is, and the lowest line bottom.
struct Placed {
    lines: Vec<Vec<Glyph>>,
    geometry: Vec<Line>,
    placed: usize,
    bottom: f32,
}

/// A line with glyphs relative to its column's left edge and baseline, the space
/// it needs above and below the baseline, and the paragraph spacing after it.
#[derive(Clone)]
struct Row {
    glyphs: Vec<Glyph>,
    above: f32,
    below: f32,
    after: f32,
    /// See `Line::stops`, relative to the column.
    stops: Vec<(usize, f32)>,
    /// A column break after this row moves it on with the next.
    keep: bool,
}

/// A line set in a frame: the bytes `start..end` of the text it holds, the space it
/// takes from `top` to `bottom`, its column from `left` to `right`, and `stops`, the
/// byte offsets where its characters start with their x, ascending from `start` and
/// ending at `end`.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
    pub stops: Vec<(usize, f32)>,
}

/// The lines of `text` from the byte `from` that fit in `frame`, with where their
/// characters sit.
pub fn lines(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    from: usize,
) -> Vec<Line> {
    set(text, spans, frame, tf, from).0.geometry
}

/// The line holding the byte offset `at`: the first that reaches it, unless the next
/// line starts right there, as after a hyphen; text past the last line is on it.
pub fn line_of(lines: &[Line], at: usize) -> usize {
    (0..lines.len())
        .find(|&i| {
            at < lines[i].end
                || at == lines[i].end && lines.get(i + 1).is_none_or(|n| n.start != at)
        })
        .unwrap_or(lines.len().saturating_sub(1))
}

/// [x, top, bottom] of a caret before the byte offset `at` of `text`.
pub fn caret(text: &str, lines: &[Line], at: usize) -> [f32; 3] {
    match lines.get(line_of(lines, at)) {
        Some(l) => [x_at(text, l, at), l.top, l.bottom],
        None => [0.0; 3],
    }
}

/// The x of a caret before the byte offset `at` of `text` on the line `l`; inside
/// a ligature the characters share its width.
pub fn x_at(text: &str, l: &Line, at: usize) -> f32 {
    let at = at.clamp(l.start, l.end);
    let i = l.stops.partition_point(|s| s.0 <= at).max(1) - 1;
    let (b0, x0) = l.stops[i];
    match l.stops.get(i + 1) {
        Some(&(b1, x1)) if at > b0 => {
            let chars = |r: std::ops::Range<usize>| text[r].chars().count().max(1) as f32;
            x0 + (x1 - x0) * chars(b0..at) / chars(b0..b1)
        }
        _ => x0,
    }
}

/// The byte offset of the character boundary nearest to (x, y): on the line under
/// y, or the nearest line, in the column nearest to x.
pub fn index_at(lines: &[Line], x: f32, y: f32) -> usize {
    let away = |lo: f32, hi: f32, v: f32| (lo - v).max(v - hi).max(0.0);
    let Some(l) = lines.iter().min_by(|a, b| {
        let d = |l: &Line| (away(l.left, l.right, x), away(l.top, l.bottom, y));
        d(a).partial_cmp(&d(b)).unwrap()
    }) else {
        return 0;
    };
    l.stops
        .iter()
        .min_by(|a, b| (a.1 - x).abs().partial_cmp(&(b.1 - x).abs()).unwrap())
        .map_or(l.start, |s| s.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Fill;
    use crate::variable::{Modes, Palette};

    fn black() -> Style {
        Style {
            fills: vec![Fill::solid(0x000000ffu32)],
            ..Style::default()
        }
    }

    #[test]
    fn a_font_without_the_tables_text_is_set_with_is_rejected() {
        for table in [b"hhea", b"head", b"cmap", b"hmtx"] {
            let mut bytes = FONT.to_vec();
            let at = bytes.windows(4).position(|w| w == table).unwrap();
            bytes[at] = b'x';
            assert!(
                add_font(&bytes).is_err(),
                "{}",
                str::from_utf8(table).unwrap()
            );
        }
    }

    #[test]
    fn a_font_is_named_by_its_family_and_style() {
        let f = typeface(FONT).unwrap();
        assert_eq!(
            (f.family.as_str(), f.style.as_str()),
            ("Source Serif 4", "Regular")
        );
    }

    const H: u16 = 9;
    const I: u16 = 36;
    const HYPHEN: u16 = 502;
    const ASCENT: f32 = 10.36;
    const AUTO: f32 = 13.71;

    fn attrs(size: f64) -> Attrs {
        Attrs {
            size,
            ..Attrs::default()
        }
    }

    fn one(text: &str, a: Attrs) -> Vec<Span> {
        vec![Span {
            len: text.encode_utf16().count(),
            attrs: a,
        }]
    }

    fn runs(text: &str, spans: &[Span], frame: [f32; 4]) -> Vec<(f32, Vec<u16>, Vec<f32>)> {
        framed(text, spans, frame, &TextFrame::default())
    }

    fn framed(
        text: &str,
        spans: &[Span],
        frame: [f32; 4],
        tf: &TextFrame,
    ) -> Vec<(f32, Vec<u16>, Vec<f32>)> {
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        draw(text, spans, &black(), frame, tf, &s, 0)
            .into_iter()
            .map(|op| match op {
                Op::GlyphRun {
                    size,
                    glyphs,
                    positions,
                    ..
                } => (size, glyphs, positions),
                _ => panic!("unexpected {op:?}"),
            })
            .collect()
    }

    fn plain(text: &str, a: Attrs, frame: [f32; 4]) -> Vec<(Vec<u16>, Vec<f32>)> {
        runs(text, &one(text, a), frame)
            .into_iter()
            .map(|(_, g, p)| (g, p))
            .collect()
    }

    fn assert_close(a: &[f32], b: &[f32]) {
        assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 0.01, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn a_stroke_goes_around_the_glyphs_outside_of_them() {
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let style = Style {
            strokes: vec![Fill::solid(0xff0000ffu32)],
            stroke_align: crate::style::Align::Outside,
            ..Style::default()
        };
        let frame = [0.0, 0.0, 100.0, 50.0];
        let ops = draw(
            "Hi",
            &one("Hi", attrs(10.0)),
            &style,
            frame,
            &TextFrame::default(),
            &s,
            0,
        );
        let [
            Op::PushClip { path, invert: true },
            Op::StrokePath { width, .. },
            Op::PopClip,
        ] = &ops[..]
        else {
            panic!("{ops:?}");
        };
        assert_eq!(*width, 2.0);
        let [x, y, w, h] = crate::geom::bounds(path);
        assert!(
            x > 0.0 && x < 2.0 && w > 10.0 && y > 0.0 && y + h < ASCENT + 0.5,
            "{x} {y} {w} {h}"
        );
        assert!(
            draw(
                "Hi",
                &one("Hi", attrs(10.0)),
                &black(),
                frame,
                &TextFrame::default(),
                &s,
                0
            )
            .iter()
            .all(|op| matches!(op, Op::GlyphRun { .. }))
        );
    }

    #[test]
    fn short_text_sits_on_the_first_baseline() {
        let r = plain("Hi", attrs(10.0), [5.0, 7.0, 100.0, 50.0]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, vec![H, I]);
        assert_close(&r[0].1, &[5.0, 7.0 + ASCENT, 12.88, 7.0 + ASCENT]);
    }

    #[test]
    fn a_page_number_marker_sets_the_number_of_the_page_in_its_place() {
        let frame = [0.0, 0.0, 100.0, 50.0];
        let marked = format!("p{PAGE_NUMBER}.");
        let tf = TextFrame {
            number: "12".into(),
            ..TextFrame::default()
        };
        let a = attrs(10.0);
        let runs = framed(&marked, &one(&marked, a.clone()), frame, &tf);
        assert_eq!(runs, framed("p12.", &one("p12.", a.clone()), frame, &tf));
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let ops = draw(
            &marked,
            &one(&marked, a.clone()),
            &black(),
            frame,
            &tf,
            &s,
            0,
        );
        assert!(matches!(&ops[0], Op::GlyphRun { text, .. } if text == "p12."));
        let lines = lines(&marked, &one(&marked, a), frame, &tf, 0);
        let [before, after] =
            [1, 1 + PAGE_NUMBER.len_utf8()].map(|at| caret(&marked, &lines, at)[0]);
        assert!(after - before > 9.9, "{before} {after}");
    }

    #[test]
    fn shaping_forms_the_fi_ligature() {
        assert_eq!(
            plain("fi", attrs(10.0), [0.0, 0.0, 100.0, 50.0])[0].0,
            vec![417]
        );
    }

    #[test]
    fn justified_lines_but_the_last_fill_the_frame_width() {
        let a = Attrs {
            text_align: TextAlign::Justify,
            ..attrs(10.0)
        };
        let r = plain("Hi Hi Hi", a, [5.0, 7.0, 25.0, 50.0]);
        let b1 = 7.0 + ASCENT;
        let b2 = b1 + AUTO;
        assert_eq!(r.len(), 2);
        assert_close(&r[0].1, &[5.0, b1, 12.88, b1, 19.14, b1, 27.02, b1]);
        assert_close(&r[1].1, &[5.0, b2, 12.88, b2]);
    }

    #[test]
    fn left_aligned_lines_keep_their_natural_spacing() {
        let r = plain("Hi Hi Hi", attrs(10.0), [5.0, 7.0, 25.0, 50.0]);
        assert_close(
            &r[0].1,
            &[5.0, 17.36, 12.88, 17.36, 18.19, 17.36, 26.07, 17.36],
        );
    }

    #[test]
    fn center_and_right_move_the_line_into_the_free_space() {
        let at = |text_align| {
            plain(
                "Hi",
                Attrs {
                    text_align,
                    ..attrs(10.0)
                },
                [0.0, 0.0, 100.0, 50.0],
            )[0]
            .1[0]
        };
        assert_close(
            &[at(TextAlign::Center), at(TextAlign::Right)],
            &[44.57, 89.14],
        );
    }

    #[test]
    fn lines_below_the_frame_are_not_drawn() {
        assert_eq!(
            plain("Hi Hi Hi", attrs(10.0), [5.0, 7.0, 25.0, 15.0]).len(),
            1
        );
    }

    #[test]
    fn a_drop_cap_spans_its_lines_and_moves_them_in() {
        let a = Attrs {
            drop_lines: 3,
            ..attrs(10.0)
        };
        let text = "Haha hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi";
        let lines = lay_out(
            text,
            &one(text, a),
            [0.0, 0.0, 60.0, 200.0],
            &TextFrame::default(),
            0,
        );
        let cap = &lines[0][0];
        assert!(cap.size > 25.0, "{cap:?}");
        assert!((cap.y - lines[2][0].y).abs() < 0.01);
        let starts: Vec<f32> = lines
            .iter()
            .map(|l| l.iter().find(|g| g.size == 10.0).unwrap().x)
            .collect();
        assert!(
            starts[0] > 15.0 && (starts[0] - starts[2]).abs() < 0.01,
            "{starts:?}"
        );
        assert_eq!(starts[3], 0.0);
    }

    #[test]
    fn kept_lines_move_on_to_the_next_column_together() {
        let tf = TextFrame {
            columns: 2,
            gutter: 10.0,
            ..TextFrame::default()
        };
        let text = "Hi\nhi hi hi";
        let columns = |keep_lines| {
            let a = Attrs {
                keep_lines,
                ..attrs(10.0)
            };
            lay_out(text, &one(text, a), [0.0, 0.0, 40.0, 42.0], &tf, 0)
                .iter()
                .map(|l| l[0].x > 0.0)
                .collect::<Vec<_>>()
        };
        assert_eq!(columns(1), [false, false, false, true]);
        assert_eq!(columns(2), [false, true, true, true]);
    }

    #[test]
    fn a_tab_goes_to_its_stop_and_a_leader_fills_the_gap() {
        let at = |text: &str, tabs: Vec<Tab>, byte: usize| {
            let a = Attrs {
                tabs,
                ..attrs(10.0)
            };
            let lines = lay_out(
                text,
                &one(text, a),
                [0.0, 0.0, 200.0, 50.0],
                &TextFrame::default(),
                0,
            );
            let g = lines[0].iter().find(|g| g.cluster == byte).unwrap();
            (g.x, lines[0].len())
        };
        let stop = |at, align, leader: &str| Tab {
            at,
            align,
            leader: leader.into(),
        };
        assert_eq!(at("a\tb", vec![], 2), (36.0, 2));
        assert_eq!(at("a\tb", vec![stop(50.0, TabAlign::Left, "")], 2).0, 50.0);
        let (end, _) = at("a\tb", vec![stop(100.0, TabAlign::Right, "")], 2);
        let (centre, _) = at("a\tb", vec![stop(100.0, TabAlign::Center, "")], 2);
        assert!(end < centre && centre < 100.0, "{end} {centre}");
        let (point, _) = at("a\t12.5", vec![stop(80.0, TabAlign::Decimal, "")], 4);
        assert_eq!(point, 80.0);
        let (_, glyphs) = at("a\tb", vec![stop(100.0, TabAlign::Left, ".")], 2);
        assert!(glyphs > 20, "{glyphs}");
    }

    #[test]
    fn list_markers_hang_in_front_and_numbers_count_on() {
        let text = "a\nb b b b b b b b b b\nc";
        let set = |list| {
            let a = Attrs {
                list,
                ..attrs(10.0)
            };
            lay_out(
                text,
                &one(text, a),
                [0.0, 0.0, 60.0, 200.0],
                &TextFrame::default(),
                0,
            )
        };
        let lines = set(List::Number);
        let marker = |l: &[Glyph]| l.iter().filter(|g| g.x < 15.0).count();
        assert_eq!(
            lines.iter().map(|l| marker(l)).collect::<Vec<_>>(),
            [2, 2, 0, 2]
        );
        assert_eq!(lines[1].iter().find(|g| g.x >= 15.0).unwrap().x, 15.0);
        assert_eq!(lines[2][0].x, 15.0);
        assert_eq!(set(List::Bullet)[0].len(), 2);
    }

    #[test]
    fn max_lines_cuts_the_text_with_an_ellipsis() {
        let text = "hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi";
        let tf = TextFrame {
            max_lines: 2,
            ..TextFrame::default()
        };
        let lines = lay_out(
            text,
            &one(text, attrs(10.0)),
            [0.0, 0.0, 40.0, 200.0],
            &tf,
            0,
        );
        assert_eq!(lines.len(), 2);
        let dots = lines[1].last().unwrap();
        assert!(dots.cluster == text.len() && dots.x < 40.0, "{dots:?}");
        let h = measure(text, &one(text, attrs(10.0)), &tf, Some(40.0), 0)[1];
        assert_close(&[h], &[27.42]);
    }

    #[test]
    fn auto_line_height_is_the_fonts_ascent_plus_descent() {
        let r = plain("Hi\nHi", attrs(10.0), [0.0, 0.0, 100.0, 50.0]);
        assert_close(&r[1].1, &[0.0, ASCENT + AUTO, 7.88, ASCENT + AUTO]);
    }

    #[test]
    fn a_fixed_line_height_centres_the_glyphs_in_each_line() {
        let a = Attrs {
            line_height: 20.0,
            ..attrs(10.0)
        };
        let r = plain("Hi\nHi", a, [0.0, 0.0, 100.0, 50.0]);
        assert_close(&[r[0].1[1], r[1].1[1]], &[13.5, 33.5]);
    }

    #[test]
    fn paragraph_spacing_follows_each_paragraph() {
        let a = Attrs {
            paragraph_spacing: 5.0,
            ..attrs(10.0)
        };
        let r = plain("Hi\nHi", a, [0.0, 0.0, 100.0, 50.0]);
        assert_close(&[r[1].1[1]], &[ASCENT + AUTO + 5.0]);
    }

    #[test]
    fn letter_spacing_widens_every_glyph() {
        let a = Attrs {
            letter_spacing: 10.0,
            ..attrs(10.0)
        };
        let r = plain("Hi", a, [0.0, 0.0, 100.0, 50.0]);
        assert_close(&r[0].1, &[0.0, ASCENT, 8.88, ASCENT]);
    }

    #[test]
    fn case_features_and_indent_change_the_glyphs_and_where_they_sit() {
        let with = |a: Attrs| plain("Hi fi", a, [0.0, 0.0, 100.0, 50.0])[0].clone();
        let upper = with(Attrs {
            text_case: TextCase::Upper,
            ..attrs(10.0)
        });
        assert_eq!(upper.0[0], H);
        assert_ne!(upper.0[1], I);
        let no_liga = with(Attrs {
            features: vec!["liga=0".into()],
            ..attrs(10.0)
        });
        assert_eq!(no_liga.0.len(), 4);
        assert_eq!(with(attrs(10.0)).0.len(), 3);
        let indented = with(Attrs {
            paragraph_indent: 20.0,
            ..attrs(10.0)
        });
        assert_close(&indented.1[..1], &[20.0]);
    }

    #[test]
    fn underline_and_strikethrough_draw_a_line_under_and_through_the_text() {
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let lines = |text_decoration| {
            let a = Attrs {
                text_decoration,
                ..attrs(10.0)
            };
            draw(
                "Hi Hi",
                &one("Hi Hi", a),
                &black(),
                [0.0, 0.0, 100.0, 50.0],
                &TextFrame::default(),
                &s,
                0,
            )
            .into_iter()
            .filter_map(|op| match op {
                Op::FillPath { path, .. } => Some([path[1], path[2], path[4]]),
                _ => None,
            })
            .collect::<Vec<_>>()
        };
        assert!(lines(TextDecoration::None).is_empty());
        let [under] = lines(TextDecoration::Underline)[..] else {
            panic!()
        };
        let [through] = lines(TextDecoration::Strikethrough)[..] else {
            panic!()
        };
        assert!(
            under[1] > ASCENT && through[1] < ASCENT,
            "{under:?} {through:?}"
        );
        assert!(under[2] > 20.0, "{under:?}");
    }

    #[test]
    fn superscript_and_subscript_set_smaller_above_and_below_and_a_shift_raises() {
        let with = |position, baseline_shift| {
            let spans = [
                Span {
                    len: 1,
                    attrs: attrs(20.0),
                },
                Span {
                    len: 1,
                    attrs: Attrs {
                        position,
                        baseline_shift,
                        ..attrs(20.0)
                    },
                },
            ];
            let glyphs: Vec<(f32, f32)> = runs("H2", &spans, [0.0, 0.0, 100.0, 50.0])
                .iter()
                .flat_map(|(size, _, at)| at.chunks(2).map(|p| (*size, p[1])))
                .collect();
            (glyphs[1].0, glyphs[1].1 - glyphs[0].1)
        };
        assert_eq!(with(Position::Normal, 0.0), (20.0, 0.0));
        let (size, dy) = with(Position::Normal, 3.0);
        assert_eq!(size, 20.0);
        assert_close(&[dy], &[-3.0]);
        let (size, dy) = with(Position::Superscript, 0.0);
        assert_close(&[size, dy], &[11.66, -6.66]);
        let (size, dy) = with(Position::Subscript, 1.0);
        assert_close(&[size, dy], &[11.66, 5.66]);
    }

    #[test]
    fn a_bigger_span_draws_its_own_run_and_lowers_the_baseline() {
        let spans = [
            Span {
                len: 1,
                attrs: attrs(20.0),
            },
            Span {
                len: 1,
                attrs: attrs(10.0),
            },
        ];
        let r = runs("Hi", &spans, [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].0, r[1].0), (20.0, 10.0));
        assert_close(&r[0].2, &[0.0, 20.72]);
        assert_close(&r[1].2, &[15.76, 20.72]);
    }

    #[test]
    fn hyphenation_breaks_a_long_word_at_a_syllable_and_sets_a_hyphen() {
        let de = Attrs {
            hyphenate: true,
            lang: Lang::De,
            ..attrs(10.0)
        };
        let r = plain("Silbentrennung", de, [0.0, 0.0, 45.0, 50.0]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0.last(), Some(&HYPHEN));
        let off = plain("Silbentrennung", attrs(10.0), [0.0, 0.0, 45.0, 50.0]);
        assert_eq!(off.len(), 1);
        assert!(!off[0].0.contains(&HYPHEN));
    }

    #[test]
    fn english_hyphenates_other_syllables_than_german() {
        let lang = |lang| Attrs {
            hyphenate: true,
            lang,
            ..attrs(10.0)
        };
        let frame = [0.0, 0.0, 38.0, 50.0];
        let en = plain("hyphenation", lang(Lang::En), frame);
        let de = plain("hyphenation", lang(Lang::De), frame);
        assert_eq!(en[0].0.last(), Some(&HYPHEN));
        assert_ne!(en[0].0.len(), de[0].0.len());
    }

    #[test]
    fn french_italian_spanish_and_dutch_hyphenate() {
        for (word, lang) in [
            ("constitutionnellement", Lang::Fr),
            ("precipitevolissimevolmente", Lang::It),
            ("electroencefalografista", Lang::Es),
            ("ziekenhuisopname", Lang::Nl),
        ] {
            assert!(syllables(word, lang).len() > 2, "{word}");
        }
    }

    fn baselines(text: &str, frame: [f32; 4], tf: TextFrame) -> Vec<[f32; 2]> {
        framed(text, &one(text, attrs(10.0)), frame, &tf)
            .iter()
            .map(|r| [r.2[0], r.2[1]])
            .collect()
    }

    #[test]
    fn insets_move_the_text_in_from_the_frame_edges() {
        let tf = TextFrame {
            inset_top: 3.0,
            inset_left: 5.0,
            inset_right: 70.0,
            ..TextFrame::default()
        };
        let b = baselines("Hi Hi Hi", [0.0, 0.0, 100.0, 50.0], tf);
        assert_eq!(b.len(), 2);
        assert_close(&b[0], &[5.0, 3.0 + ASCENT]);
    }

    #[test]
    fn text_flows_into_the_next_column_when_one_is_full() {
        let tf = TextFrame {
            columns: 2,
            gutter: 10.0,
            ..TextFrame::default()
        };
        let b = baselines("Hi\nHi\nHi", [0.0, 0.0, 100.0, 30.0], tf);
        assert_eq!(b.len(), 3);
        assert_close(&b[1], &[0.0, ASCENT + AUTO]);
        assert_close(&b[2], &[55.0, ASCENT]);
    }

    #[test]
    fn vertical_alignment_moves_the_lines_down_in_the_frame() {
        let at = |vertical_align| {
            let tf = TextFrame {
                vertical_align,
                ..TextFrame::default()
            };
            baselines("Hi", [0.0, 0.0, 100.0, 50.0], tf)[0][1]
        };
        assert_close(
            &[at(VerticalAlign::Center), at(VerticalAlign::Bottom)],
            &[28.5, 46.65],
        );
    }

    #[test]
    fn a_baseline_grid_moves_each_baseline_down_to_the_next_grid_line() {
        let tf = TextFrame {
            inset_top: 2.0,
            baseline_grid: 15.0,
            baseline_start: 1.0,
            ..TextFrame::default()
        };
        let b = baselines("Hi\nHi", [0.0, 0.0, 100.0, 50.0], tf);
        assert_close(&[b[0][1], b[1][1]], &[18.0, 33.0]);
    }

    #[test]
    fn measure_fits_the_frame_around_the_text_with_its_insets() {
        let tf = TextFrame {
            inset_top: 2.0,
            inset_right: 5.0,
            inset_bottom: 3.0,
            inset_left: 4.0,
            ..TextFrame::default()
        };
        let t = "Hi\nHi Hi";
        let spans = one(t, attrs(10.0));
        let [w, h] = measure(t, &spans, &tf, None, 0);
        assert_close(&[h], &[5.0 + 2.0 * AUTO]);
        assert_eq!(lay_out(t, &spans, [0.0, 0.0, w, h], &tf, 0).len(), 2);
        assert_eq!(
            lay_out(t, &spans, [0.0, 0.0, w - 1.0, 99.0], &tf, 0).len(),
            3
        );
        let [w, h] = measure(t, &spans, &tf, Some(25.0), 0);
        assert_close(&[w, h], &[25.0, 5.0 + 3.0 * AUTO]);
    }

    #[test]
    fn measure_keeps_a_trailing_space_on_the_line() {
        let tf = TextFrame::default();
        let t = "Hello ";
        let spans = one(t, attrs(10.0));
        let [w, h] = measure(t, &spans, &tf, None, 0);
        let [bare, _] = measure("Hello", &one("Hello", attrs(10.0)), &tf, None, 0);
        assert!(w > bare, "{w} {bare}");
        assert_close(&[h], &[AUTO]);
        assert_eq!(lay_out(t, &spans, [0.0, 0.0, w, h], &tf, 0).len(), 1);
    }

    #[test]
    fn measure_balances_the_height_over_the_columns() {
        let tf = TextFrame {
            columns: 2,
            ..TextFrame::default()
        };
        let t = "Hi\nHi\nHi\nHi";
        let [_, h] = measure(t, &one(t, attrs(10.0)), &tf, Some(100.0), 0);
        assert!((h - 2.0 * AUTO).abs() < 0.05, "{h}");
    }

    #[test]
    fn carets_sit_where_the_characters_start_and_wrap_with_the_lines() {
        let t = "Hi Hi Hi";
        let frame = [5.0, 7.0, 25.0, 50.0];
        let ls = lines(t, &one(t, attrs(10.0)), frame, &TextFrame::default(), 0);
        assert_eq!(ls.len(), 2);
        assert_eq!(
            (ls[0].start, ls[0].end, ls[1].start, ls[1].end),
            (0, 5, 6, 8)
        );
        assert_close(&caret(t, &ls, 0), &[5.0, 7.0, 7.0 + AUTO]);
        assert_close(&caret(t, &ls, 1)[..1], &[12.88]);
        assert_close(&caret(t, &ls, 6), &[5.0, 7.0 + AUTO, 7.0 + 2.0 * AUTO]);
        let [end, ..] = caret(t, &ls, 8);
        assert!(end > 12.88 && end < 20.0, "{end}");
        assert_close(&caret(t, &ls, 5)[1..], &[7.0, 7.0 + AUTO]);
    }

    #[test]
    fn the_caret_at_the_end_of_a_justified_paragraph_follows_its_last_word() {
        let t = "Hi Hi Hi";
        let a = Attrs {
            text_align: TextAlign::Justify,
            ..attrs(10.0)
        };
        let ls = lines(
            t,
            &one(t, a),
            [5.0, 7.0, 25.0, 50.0],
            &TextFrame::default(),
            0,
        );
        let [end, ..] = caret(t, &ls, 8);
        assert!(end > 12.88 && end < 20.0, "{end}");
    }

    #[test]
    fn a_point_finds_the_nearest_character_boundary_on_its_line() {
        let t = "Hi Hi Hi";
        let ls = lines(
            t,
            &one(t, attrs(10.0)),
            [5.0, 7.0, 25.0, 50.0],
            &TextFrame::default(),
            0,
        );
        assert_eq!(index_at(&ls, 12.0, 10.0), 1);
        assert_eq!(index_at(&ls, 0.0, 0.0), 0);
        assert_eq!(index_at(&ls, 100.0, 25.0), 8);
        assert_eq!(index_at(&ls, 6.0, 100.0), 6);
    }

    fn texts(text: &str, a: Attrs, frame: [f32; 4]) -> Vec<String> {
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let spans = one(text, a);
        draw(text, &spans, &black(), frame, &TextFrame::default(), &s, 0)
            .into_iter()
            .map(|op| match op {
                Op::GlyphRun { text, .. } => text,
                _ => panic!("unexpected {op:?}"),
            })
            .collect()
    }

    #[test]
    fn each_run_carries_the_text_of_its_glyphs_with_ligatures_and_hyphens() {
        assert_eq!(
            texts("Hi fine Hi", attrs(10.0), [0.0, 0.0, 30.0, 50.0]),
            ["Hifine", "Hi"]
        );
        let de = Attrs {
            hyphenate: true,
            lang: Lang::De,
            ..attrs(10.0)
        };
        assert_eq!(
            texts("Silbentrennung", de, [0.0, 0.0, 45.0, 50.0]),
            ["Silben-", "trennung"]
        );
    }

    #[test]
    fn a_narrow_column_spreads_its_looseness_instead_of_one_very_loose_line() {
        let t = "Satz sets type in the browser. The engine shapes this paragraph with \
                 harfrust, breaks it into lines with the Knuth-Plass algorithm and \
                 justifies every line but the last to the width of its frame.";
        let a = Attrs {
            letter_spacing: 4.0,
            text_align: TextAlign::Justify,
            hyphenate: true,
            lang: Lang::De,
            ..attrs(13.0)
        };
        let lines = lay_out(
            t,
            &one(t, a),
            [0.0, 0.0, 150.0, 1000.0],
            &TextFrame::default(),
            0,
        );
        let counts: Vec<usize> = lines.iter().map(Vec::len).collect();
        let shortest = counts[..counts.len() - 1].iter().min().unwrap();
        assert!(*shortest >= 14, "{counts:?}");
    }

    #[test]
    fn text_left_over_by_a_frame_is_set_from_where_it_stops_in_the_next() {
        let t = "Hi Hi Hi";
        let spans = one(t, attrs(10.0));
        let tf = TextFrame::default();
        assert_eq!(overflow(t, &spans, [0.0, 0.0, 25.0, 15.0], &tf, 0), Some(6));
        assert_eq!(overflow(t, &spans, [0.0, 0.0, 25.0, 50.0], &tf, 0), None);
        assert_eq!(overflow(t, &spans, [0.0, 0.0, 25.0, 50.0], &tf, 6), None);
        let rest = lay_out(t, &spans, [100.0, 0.0, 100.0, 50.0], &tf, 6);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].iter().map(|g| g.id).collect::<Vec<_>>(), [H, I]);
        assert_close(&[rest[0][0].x, rest[0][0].y], &[100.0, ASCENT]);
        let ls = lines(t, &spans, [100.0, 0.0, 100.0, 50.0], &tf, 6);
        assert_eq!((ls[0].start, ls[0].end), (6, 8));
        assert_close(&measure(t, &spans, &tf, Some(25.0), 6), &[25.0, AUTO]);
        let de = Attrs {
            hyphenate: true,
            lang: Lang::De,
            ..attrs(10.0)
        };
        let w = "Silbentrennung";
        let spans = one(w, de);
        let at = overflow(w, &spans, [0.0, 0.0, 45.0, 15.0], &tf, 0).unwrap();
        assert_eq!(&w[at..], "trennung");
        let rest = lay_out(w, &spans, [0.0, 0.0, 100.0, 50.0], &tf, at);
        assert_eq!(rest.len(), 1);
        assert!(!rest[0].iter().any(|g| g.hyphen));
        assert_eq!(rest[0].len(), "trennung".len());
    }

    #[test]
    fn a_frame_that_starts_at_a_later_paragraph_skips_the_ones_before() {
        let t = "Hi\nHi Hi";
        let spans = one(t, attrs(10.0));
        let tf = TextFrame::default();
        assert_eq!(
            overflow(t, &spans, [0.0, 0.0, 100.0, 15.0], &tf, 0),
            Some(3)
        );
        let rest = lay_out(t, &spans, [0.0, 0.0, 100.0, 50.0], &tf, 3);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].len(), 4);
    }
}
