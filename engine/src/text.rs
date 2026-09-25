use crate::color::Color;
use crate::display_list::{Op, Paint};
use crate::linebreak::{Item, break_lines};
use crate::style::{Fill, paints};
use crate::variable::{Scope, Value};
use harfrust::{FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
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

/// A font by its full name and a hash of its bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typeface {
    pub name: String,
    pub hash: String,
}

thread_local! {
    /// The fonts text can be set in by id, the bundled one first.
    static FONTS: RefCell<Vec<(Typeface, Rc<[u8]>)>> =
        RefCell::new(vec![(typeface(FONT).unwrap(), FONT.into())]);
}

pub fn typeface(bytes: &[u8]) -> Result<Typeface, String> {
    let bad = || "not a TrueType or OpenType font".to_string();
    FontRef::new(bytes)
        .and_then(|f| f.cmap())
        .map_err(|_| bad())?;
    let name = skrifa::FontRef::new(bytes)
        .map_err(|_| bad())?
        .localized_strings(StringId::FULL_NAME)
        .english_or_first()
        .ok_or_else(bad)?
        .to_string();
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    Ok(Typeface {
        name,
        hash: format!("{hash:016x}"),
    })
}

/// Adds the font `bytes` to those text can be set in, unless it is there already.
pub fn add_font(bytes: &[u8]) -> Result<Typeface, String> {
    let face = typeface(bytes)?;
    FONTS.with_borrow_mut(|f| {
        if !f.iter().any(|(t, _)| t.hash == face.hash) {
            f.push((face.clone(), bytes.into()));
        }
    });
    Ok(face)
}

/// The id of the font `face`, or the bundled one's marked `MISSING` when it is not there.
pub fn font_id(face: &Option<Typeface>) -> u32 {
    let Some(face) = face else { return 0 };
    FONTS
        .with_borrow(|f| f.iter().position(|(t, _)| t.hash == face.hash))
        .map_or(MISSING, |i| i as u32)
}

pub fn font_bytes(id: u32) -> Rc<[u8]> {
    FONTS.with_borrow(|f| f.get((id & !MISSING) as usize).unwrap_or(&f[0]).1.clone())
}

/// The fonts text can be set in.
pub fn fonts() -> Vec<Typeface> {
    FONTS.with_borrow(|f| f.iter().map(|(t, _)| t.clone()).collect())
}

/// The keys of `Attrs` a text style sets.
pub const STYLED: [&str; 4] = ["size", "lineHeight", "letterSpacing", "paragraphSpacing"];
/// The keys of `Attrs` that hold for a whole paragraph, taken from its first character.
pub const PARAGRAPH: [&str; 4] = ["textAlign", "paragraphSpacing", "hyphenate", "lang"];

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

/// A language that hyphenation knows.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Lang {
    #[default]
    En,
    De,
}

/// What a character looks like. Sizes and spacing are in pt, `letter_spacing` in % of
/// the size; `line_height` 0 is auto, the font's ascent plus descent. `fill` replaces
/// the layer's fills; `text_style` "" means none.
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

/// Named values for the `STYLED` attributes; `bindings` holds number variables by key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,
    pub size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub paragraph_spacing: f64,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

impl TextStyle {
    /// The `STYLED` attributes with bound variables resolved in `scope`.
    pub fn values(&self, scope: &Scope) -> Map<String, serde_json::Value> {
        let own = serde_json::to_value(self).unwrap();
        STYLED
            .iter()
            .map(|&k| {
                let bound = self.bindings.get(k).and_then(|v| match scope.value(v) {
                    Some(&Value::Number(n)) if k == "letterSpacing" => Some(n),
                    Some(&Value::Number(n)) => Some(n.max(0.0)),
                    _ => None,
                });
                (k.into(), bound.or(own[k].as_f64()).unwrap_or(0.0).into())
            })
            .collect()
    }
}

/// Byte offsets in `para` where `lang` allows a hyphen, ascending.
fn syllables(para: &str, lang: Lang) -> Vec<usize> {
    let lang = match lang {
        Lang::En => hypher::Lang::English,
        Lang::De => hypher::Lang::German,
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
            number: String::new(),
        }
    }
}

/// Stands for the number of the page a text is on, as InDesign's Current Page Number.
pub const PAGE_NUMBER: char = '\u{18}';

/// A glyph at (x, y) on its baseline, drawn with the attributes of `span`, for the
/// text from byte `cluster`, or a hyphen inserted there.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub span: usize,
    pub cluster: usize,
    pub hyphen: bool,
}

/// Glyph runs of `text` from the byte `from` set in `frame`; characters without a
/// fill take `fills`.
pub fn draw(
    text: &str,
    spans: &[Span],
    fills: &[Fill],
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
            None => paints(fills, frame, s).collect(),
        })
        .collect();
    let mut ops = Vec::new();
    for line in lay_out(text, spans, frame, tf, from) {
        let mut at = 0;
        for run in line.chunk_by(|a, b| {
            spans[a.span].attrs.size == spans[b.span].attrs.size
                && span_paints[a.span] == span_paints[b.span]
                && fonts[a.span] == fonts[b.span]
        }) {
            let (run_text, ranges) = source(text, &line, at..at + run.len(), &tf.number);
            at += run.len();
            for paint in &span_paints[run[0].span] {
                ops.push(Op::GlyphRun {
                    font: fonts[run[0].span],
                    size: spans[run[0].span].attrs.size as f32,
                    paint: paint.clone(),
                    glyphs: run.iter().map(|g| g.id).collect(),
                    positions: run.iter().flat_map(|g| [g.x, g.y]).collect(),
                    text: run_text.clone(),
                    ranges: ranges.clone(),
                });
            }
        }
    }
    ops
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
    let rows = rows(text, spans, Some(cw), from, &tf.number, room).0;
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
                rows(text, spans, Some(cw), from, &tf.number, f32::INFINITY).0,
                cw,
                w,
            )
        }
        None => {
            let (rows, natural) = rows(text, spans, None, from, &tf.number, f32::INFINITY);
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
    ascent: f32,
    descent: f32,
    gap: f32,
    hyphen: u16,
    hyphen_adv: f32,
}

/// A shaped paragraph: its items, glyphs and clusters, attributes, first byte and
/// natural width.
type Para = (
    Vec<Item>,
    Vec<Option<(u16, f32, f32, usize, usize, bool)>>,
    Vec<usize>,
    Attrs,
    usize,
    f32,
);

/// The lines of `text` from the byte `from`, a line start, broken to the column width
/// `cw`, or each paragraph on one line when `None`; and the width of the widest
/// paragraph set on one line. A page number marker is set as `number`. Paragraphs
/// stop once their lines need more than `room` in height.
fn rows(
    text: &str,
    spans: &[Span],
    cw: Option<f32>,
    from: usize,
    number: &str,
    room: f32,
) -> (Vec<Row>, f32) {
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
        let s = a.size as f32;
        let l = match a.line_height as f32 {
            0.0 => (ascent + descent + gap) * s,
            l => l,
        };
        let above = ascent * s + (l - (ascent + descent) * s) / 2.0;
        (above, l - above)
    };

    let break_para =
        |(items, glyphs, clusters, first, start, _): Para, cw: f32, rows: &mut Vec<Row>| {
            let mut from = 0;
            for (end, r) in break_lines(&items, cw) {
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
                let used: f32 = items[from..end].iter().map(spread).sum::<f32>()
                    + if hyphenated {
                        hyphen_width(&items[end])
                    } else {
                        0.0
                    };
                let spans_here: Vec<usize> = (from..last)
                    .filter(|&k| matches!(items[k], Item::Box(_)) || k == end)
                    .filter_map(|k| glyphs[k].map(|g| g.3))
                    .collect();
                let (above, below) = if spans_here.is_empty() {
                    vertical(span_at(start))
                } else {
                    spans_here
                        .iter()
                        .map(|&s| vertical(s))
                        .fold((0.0f32, 0.0f32), |(a, b), (c, d)| (a.max(c), b.max(d)))
                };
                let mut cx = match first.text_align {
                    TextAlign::Center => (cw - used) / 2.0,
                    TextAlign::Right => cw - used,
                    _ => 0.0,
                };
                let mut line = Vec::new();
                let mut stops: Vec<(usize, f32)> = Vec::new();
                let mut end_x = None;
                for k in from..last {
                    if items[k] == FILL {
                        end_x.get_or_insert(cx);
                    }
                    if matches!(items[k], Item::Penalty { .. }) && k != end {
                        continue;
                    }
                    if !matches!(items[k], Item::Penalty { .. })
                        && stops.last().is_none_or(|s| s.0 < clusters[k])
                    {
                        stops.push((clusters[k], cx));
                    }
                    if let Some((id, dx, dy, span, cluster, hyphen)) = glyphs[k] {
                        line.push(Glyph {
                            id,
                            x: cx + dx,
                            y: -dy,
                            span,
                            cluster,
                            hyphen,
                        });
                    }
                    cx += spread(&items[k]);
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
                });
                from = end + 1;
            }
            if let Some(r) = rows.last_mut() {
                r.after = first.paragraph_spacing as f32;
            }
        };

    let mut rows: Vec<Row> = Vec::new();
    let (mut widest, mut used) = (0.0f32, 0.0f32);
    let mut paras = Vec::new();
    let mut start = 0;
    for para in text.split('\n') {
        if start + para.len() < from {
            start += para.len() + 1;
            continue;
        }
        let first = &spans[span_at(start)].attrs;
        let mut shaped = Vec::new();
        let mut chars = para.char_indices().peekable();
        while let Some(&(i, _)) = chars.peek() {
            let k = slot[span_at(start + i)];
            let mut buf = UnicodeBuffer::new();
            while let Some((i, c)) = chars.next_if(|&(i, _)| slot[span_at(start + i)] == k) {
                match c {
                    PAGE_NUMBER => number.chars().for_each(|d| buf.add(d, i as u32)),
                    c => buf.add(c, i as u32),
                }
            }
            buf.guess_segment_properties();
            let out = shapers[k].shape(buf, ShapeOptions::new());
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
        let mut items = Vec::new();
        let mut glyphs = Vec::new();
        let mut clusters = Vec::new();
        for &(at, id, adv, dx, dy) in &shaped {
            let cluster = start + at;
            let span = span_at(cluster);
            let (a, m) = (&spans[span].attrs, &metrics[slot[span]]);
            let size = a.size as f32;
            let adv = adv * size + (a.size * a.letter_spacing / 100.0) as f32;
            if breaks.binary_search(&at).is_ok() {
                items.push(Item::Penalty {
                    width: m.hyphen_adv * size,
                    cost: HYPHEN_COST,
                });
                glyphs.push(Some((m.hyphen, 0.0, 0.0, span, cluster, true)));
                clusters.push(cluster);
            }
            clusters.push(cluster);
            if para[at..].starts_with(' ') {
                items.push(Item::Glue {
                    width: adv,
                    stretch: adv / 2.0,
                    shrink: adv / 3.0,
                });
                glyphs.push(None);
            } else {
                items.push(Item::Box(adv));
                glyphs.push(Some((id, dx * size, dy * size, span, cluster, false)));
            }
        }
        let natural: f32 = items
            .iter()
            .map(|it| match *it {
                Item::Box(w) | Item::Glue { width: w, .. } => w,
                Item::Penalty { .. } => 0.0,
            })
            .sum();
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
        widest = widest.max(natural);
        let para = (items, glyphs, clusters, first.clone(), line0, natural);
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
    let (mut col, mut top) = (0, iy);
    for row in rows {
        let mut baseline = snap(top + row.above);
        if baseline + row.below > bottom {
            col += 1;
            if col >= n {
                break;
            }
            baseline = snap(iy + row.above);
            if baseline + row.below > bottom {
                break;
            }
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
                .into_iter()
                .map(|g| Glyph {
                    x: cx + g.x,
                    y: baseline + g.y,
                    ..g
                })
                .collect(),
        );
        columns[col as usize].1 = baseline + row.below;
        top = baseline + row.below + row.after;
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
    use crate::variable::{Modes, Palette};

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
        draw(text, spans, &[Fill::solid(0x000000ffu32)], frame, tf, &s, 0)
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
        let black = [Fill::solid(0x000000ffu32)];
        let ops = draw(&marked, &one(&marked, a.clone()), &black, frame, &tf, &s, 0);
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
        draw(
            text,
            &spans,
            &[Fill::solid(0x000000ffu32)],
            frame,
            &TextFrame::default(),
            &s,
            0,
        )
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
