use crate::color::Color;
use crate::content_hash;
use crate::display_list::{CLOSE, CUBIC, LINE, MOVE, Op, Paint, rect};
use crate::linebreak::{Item, break_lines};
use crate::style::{Cap, Join, Style, paints};
use crate::variable::{Scope, Value};
use harfrust::{Feature, FontRef, ShapeOptions, ShaperData, ShaperInstance, UnicodeBuffer};
use read_fonts::TableProvider;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use skrifa::MetadataProvider;
use skrifa::string::StringId;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::rc::Rc;
use std::thread::LocalKey;

pub const FONT: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");
const FAMILY: [&[u8]; 3] = [
    include_bytes!("../fonts/SourceSerif4-Bold.ttf"),
    include_bytes!("../fonts/SourceSerif4-It.ttf"),
    include_bytes!("../fonts/SourceSerif4-BoldIt.ttf"),
];
pub const BUNDLED: usize = 1 + FAMILY.len();
pub const MISSING: u32 = 1 << 31;
pub const VARIED: u32 = 1 << 30;

#[cfg_attr(test, derive(ts_rs::TS))]
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
    static FONTS: RefCell<Vec<(Typeface, Rc<[u8]>)>> =
        RefCell::new([FONT].iter().chain(&FAMILY).map(|b| (typeface(b).unwrap(), Rc::from(*b))).collect());
    static INSTANCES: RefCell<Vec<(u32, Location)>> = RefCell::default();
}

type Location = Vec<(String, f32)>;

type Shaping = (String, String, Vec<(usize, u32, Vec<String>, TextCase)>);

type Shaped = Rc<[(usize, u16, f32, f32, f32)]>;

type Breaks = Rc<[(usize, f32)]>;

type Cache<K, V> = RefCell<HashMap<K, V>>;

thread_local! {
    static SHAPED: Cache<Shaping, Shaped> = RefCell::default();
    static SYLLABLES: Cache<(String, Lang), Rc<[usize]>> = RefCell::default();
    static BREAKS: Cache<Vec<u32>, Breaks> = RefCell::default();
}

fn kept<K: Eq + Hash, V: Clone>(
    cache: &'static LocalKey<Cache<K, V>>,
    key: K,
    make: impl FnOnce() -> V,
) -> V {
    if let Some(v) = cache.with_borrow(|c| c.get(&key).cloned()) {
        return v;
    }
    let v = make();
    cache.with_borrow_mut(|c| {
        if c.len() >= 4096 {
            c.clear();
        }
        c.insert(key, v.clone());
    });
    v
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

pub fn add_font(bytes: &[u8]) -> Result<Typeface, String> {
    let sfnt = match bytes.get(..4) {
        Some(b"wOFF") => wuff::decompress_woff1(bytes),
        Some(b"wOF2") => wuff::decompress_woff2(bytes),
        _ => Ok(bytes.into()),
    }
    .map_err(|e| e.to_string())?;
    let face = typeface(&sfnt)?;
    let glyphs: Vec<u16> =
        (0..FontRef::new(&sfnt).map_or(0, |f| f.maxp().map_or(0, |m| m.num_glyphs()))).collect();
    let all = subsetter::GlyphRemapper::new_from_glyphs_sorted(&glyphs);
    subsetter::subset(&sfnt, 0, &all).map_err(|_| "the font is damaged")?;
    FONTS.with_borrow_mut(|f| {
        if !f.iter().any(|(t, b)| t.hash == face.hash && !b.is_empty()) {
            f.push((face.clone(), sfnt.into()));
        }
    });
    Ok(face)
}

pub fn font_id(face: &Option<Typeface>) -> u32 {
    let Some(face) = face else { return 0 };
    FONTS
        .with_borrow(|f| {
            f.iter()
                .position(|(t, b)| t.hash == face.hash && !b.is_empty())
        })
        .map_or(MISSING, |i| i as u32)
}

pub fn font_of(a: &Attrs) -> u32 {
    let id = font_id(&a.font);
    if id & MISSING != 0 || a.axes.is_empty() {
        return id;
    }
    let bytes = font_bytes(id);
    let Ok(f) = skrifa::FontRef::new(&bytes) else {
        return id;
    };
    let at: Location = (f.axes().iter())
        .filter_map(|x| {
            let tag = x.tag().to_string();
            let v = (*a.axes.get(&tag)? as f32).clamp(x.min_value(), x.max_value());
            (v != x.default_value()).then_some((tag, v))
        })
        .collect();
    if at.is_empty() {
        return id;
    }
    INSTANCES.with_borrow_mut(|i| {
        let n = i.iter().position(|e| e.0 == id && e.1 == at);
        VARIED
            | n.unwrap_or_else(|| {
                i.push((id, at));
                i.len() - 1
            }) as u32
    })
}

pub fn located(id: u32) -> (u32, Location) {
    if id & (VARIED | MISSING) != VARIED {
        return (id & !MISSING, Vec::new());
    }
    INSTANCES.with_borrow(|i| i[(id & !VARIED) as usize].clone())
}

pub fn font_bytes(id: u32) -> Rc<[u8]> {
    let id = located(id).0;
    FONTS.with_borrow(|f| f.get(id as usize).unwrap_or(&f[0]).1.clone())
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Axis {
    pub tag: String,
    pub name: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

pub fn axes() -> BTreeMap<String, Vec<Axis>> {
    FONTS.with_borrow(|fonts| {
        (fonts.iter())
            .filter_map(|(t, b)| {
                let f = skrifa::FontRef::new(b).ok()?;
                let axes: Vec<Axis> = (f.axes().iter())
                    .filter(|x| !x.is_hidden())
                    .map(|x| Axis {
                        tag: x.tag().to_string(),
                        name: (f.localized_strings(x.name_id()).english_or_first())
                            .map_or_else(|| x.tag().to_string(), |s| s.to_string()),
                        min: x.min_value(),
                        default: x.default_value(),
                        max: x.max_value(),
                    })
                    .collect();
                (!axes.is_empty()).then(|| (t.hash.clone(), axes))
            })
            .collect()
    })
}

pub fn characters(hash: Option<&str>) -> (u32, Vec<u32>) {
    let id = hash
        .and_then(|h| {
            FONTS.with_borrow(|f| f.iter().position(|(t, b)| t.hash == h && !b.is_empty()))
        })
        .unwrap_or(0) as u32;
    let bytes = font_bytes(id);
    let Ok(f) = skrifa::FontRef::new(&bytes) else {
        return (id, Vec::new());
    };
    let chars = f
        .charmap()
        .mappings()
        .filter_map(|(c, _)| char::from_u32(c))
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .map(|c| c as u32)
        .collect();
    (id, chars)
}

pub fn missing_glyph(text: &str, spans: &[Span]) -> Option<char> {
    let mut chars = text.chars();
    spans.iter().find_map(|s| {
        let id = font_of(&s.attrs);
        let bytes = font_bytes(id);
        let font = skrifa::FontRef::new(&bytes).ok();
        let map = font.as_ref().map(|f| f.charmap());
        let mut found = None;
        let mut n = 0;
        while n < s.len {
            let Some(c) = chars.next() else { break };
            n += c.len_utf16();
            if found.is_none()
                && id & MISSING == 0
                && !c.is_whitespace()
                && c != PAGE_NUMBER
                && c != DATE
                && c != ANCHOR
                && map.as_ref().is_some_and(|m| m.map(c).is_none())
            {
                found = Some(c);
            }
        }
        found
    })
}

pub fn remove_font(hash: &str) {
    FONTS.with_borrow_mut(|f| {
        for (_, b) in f.iter_mut().skip(BUNDLED).filter(|(t, _)| t.hash == hash) {
            *b = Rc::from([]);
        }
    });
}

pub fn fonts_added() -> usize {
    FONTS.with_borrow(|f| f.len())
}

pub fn fonts() -> Vec<Typeface> {
    FONTS.with_borrow(|f| {
        f.iter()
            .filter(|(_, b)| !b.is_empty())
            .map(|(t, _)| t.clone())
            .collect()
    })
}

pub const STYLED: [&str; 22] = [
    "size",
    "lineHeight",
    "letterSpacing",
    "paragraphSpacing",
    "paragraphIndent",
    "indentLeft",
    "indentRight",
    "spaceBefore",
    "font",
    "textCase",
    "textDecoration",
    "features",
    "axes",
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
pub const PARAGRAPH: [&str; 16] = [
    "textAlign",
    "shading",
    "paragraphSpacing",
    "paragraphIndent",
    "indentLeft",
    "indentRight",
    "spaceBefore",
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

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub at: f64,
    pub align: TabAlign,
    pub leader: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TabAlign {
    Left,
    Center,
    Right,
    Decimal,
}

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

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum List {
    #[default]
    None,
    Bullet,
    Number,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextCase {
    #[default]
    Original,
    Upper,
    Lower,
    Title,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    Strikethrough,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Position {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

#[cfg_attr(test, derive(ts_rs::TS))]
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
    pub font: Option<Typeface>,
    pub paragraph_indent: f64,
    pub indent_left: f64,
    pub indent_right: f64,
    pub space_before: f64,
    pub text_case: TextCase,
    pub text_decoration: TextDecoration,
    pub features: Vec<String>,
    pub axes: BTreeMap<String, f64>,
    pub position: Position,
    pub baseline_shift: f64,
    pub drop_lines: u32,
    pub drop_chars: u32,
    pub keep_lines: u32,
    pub keep_together: bool,
    pub keep_next: bool,
    pub tabs: Vec<Tab>,
    pub list: List,
    pub shading: Option<Color>,
    pub object: String,
    #[serde(skip)]
    pub object_size: [f64; 2],
    pub condition: String,
    #[serde(skip)]
    pub hidden: bool,
}

impl Attrs {
    fn drawn(&self) -> f32 {
        match self.position {
            Position::Normal => self.size as f32,
            _ => self.size as f32 * 0.583,
        }
    }

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
            indent_left: 0.0,
            indent_right: 0.0,
            space_before: 0.0,
            text_case: TextCase::Original,
            text_decoration: TextDecoration::None,
            features: Vec::new(),
            axes: BTreeMap::new(),
            position: Position::Normal,
            baseline_shift: 0.0,
            drop_lines: 0,
            drop_chars: 1,
            keep_lines: 1,
            keep_together: false,
            keep_next: false,
            tabs: Vec::new(),
            list: List::None,
            shading: None,
            object: String::new(),
            object_size: [0.0; 2],
            condition: String::new(),
            hidden: false,
        }
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Span {
    pub len: usize,
    #[serde(flatten)]
    pub attrs: Attrs,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub attrs: Attrs,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub grep: Vec<Grep>,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grep {
    pub pattern: String,
    pub style: String,
}

impl TextStyle {
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

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Wrap {
    #[default]
    None,
    Around,
    Jump,
}

#[cfg_attr(test, derive(ts_rs::TS))]
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
    pub max_lines: u32,
    pub trim: bool,
    pub round_baselines: bool,
    pub column_rule: Option<Rule>,
    pub min_height: f64,
    pub toc: Vec<String>,
    #[serde(skip)]
    pub number: String,
    #[serde(skip)]
    pub date: String,
    #[serde(skip)]
    pub wrap: Vec<([f32; 4], bool)>,
}

impl Default for TextFrame {
    fn default() -> Self {
        TextFrame {
            inset_top: 0.0,
            inset_right: 0.0,
            inset_bottom: 0.0,
            inset_left: 0.0,
            columns: 1,
            gutter: 4.0 * crate::doc::MM,
            vertical_align: VerticalAlign::Top,
            baseline_grid: 0.0,
            baseline_start: 0.0,
            max_lines: 0,
            trim: false,
            round_baselines: false,
            column_rule: None,
            min_height: 0.0,
            toc: Vec::new(),
            number: String::new(),
            date: String::new(),
            wrap: Vec::new(),
        }
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub weight: f32,
    pub color: Color,
}

pub const PAGE_NUMBER: char = '\u{18}';

pub const DATE: char = '\u{19}';

pub const ANCHOR: char = '\u{FFFC}';

impl TextFrame {
    pub fn marker(&self, c: char) -> &str {
        if c == DATE { &self.date } else { &self.number }
    }
}

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

pub fn draw(
    text: &str,
    spans: &[Span],
    style: &Style,
    frame: [f32; 4],
    tf: &TextFrame,
    s: &Scope,
    placed: &Placed,
) -> Vec<Op> {
    let fonts: Vec<u32> = spans.iter().map(|s| font_of(&s.attrs)).collect();
    let span_paints: Vec<Vec<Paint>> = spans
        .iter()
        .map(|sp| match &sp.attrs.fill {
            Some(c) => vec![Paint::Solid {
                color: c.rgba(s),
                ink: c.ink(s),
                overprint: style.overprint_fill,
            }],
            None => paints(&style.fills, frame, s, style.overprint_fill).collect(),
        })
        .collect();
    let mut ops = shading(text, spans, placed, s, style.overprint_fill);
    let stroked = style.strokes.iter().any(|f| f.visible);
    let mut outlines = Vec::new();
    for (line, geometry) in placed.lines.iter().zip(&placed.geometry) {
        let line: Vec<_> = (line.iter())
            .filter(|g| spans[g.span].attrs.object.is_empty())
            .cloned()
            .collect();
        let mut at = 0;
        for run in line.chunk_by(|a, b| {
            a.size == b.size
                && (a.span == b.span
                    || fonts[a.span] == fonts[b.span] && span_paints[a.span] == span_paints[b.span])
        }) {
            let (text, ranges) = source(text, &line, at..at + run.len(), tf);
            at += run.len();
            let glyphs: Vec<u16> = run.iter().map(|g| g.id).collect();
            let positions: Vec<f32> = run.iter().flat_map(|g| [g.x, g.y]).collect();
            let (font, size) = (fonts[run[0].span], run[0].size);
            if stroked {
                outlines.extend(outline(font, size, &glyphs, &positions));
            }
            let Some((last, rest)) = span_paints[run[0].span].split_last() else {
                continue;
            };
            for paint in rest {
                ops.push(Op::GlyphRun {
                    font,
                    size,
                    paint: paint.clone(),
                    glyphs: glyphs.clone(),
                    positions: positions.clone(),
                    text: text.clone(),
                    ranges: ranges.clone(),
                });
            }
            ops.push(Op::GlyphRun {
                font,
                size,
                paint: last.clone(),
                glyphs,
                positions,
                text,
                ranges,
            });
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
            let Some([at, thick]) = decoration(font_of(a), a.text_decoration) else {
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
    if let Some(r) = tf.column_rule.as_ref().filter(|r| r.weight > 0.0) {
        let ([x, y, _, h], cw) = columns(frame, tf);
        let gutter = tf.gutter as f32;
        for k in 1..tf.columns.max(1) {
            let at = x + k as f32 * (cw + gutter) - gutter / 2.0;
            ops.push(Op::StrokePath {
                paint: Paint::Solid {
                    color: r.color.rgba(s),
                    ink: r.color.ink(s),
                    overprint: style.overprint_stroke,
                },
                width: r.weight,
                cap: Cap::None as u32,
                join: Join::Miter as u32,
                path: vec![MOVE, at, y, LINE, at, y + h],
            });
        }
    }
    ops
}

fn shading(text: &str, spans: &[Span], placed: &Placed, s: &Scope, overprint: bool) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut i = 0;
    while i < placed.lines.len() {
        let first = &placed.geometry[i];
        let color = placed.lines[i]
            .first()
            .and_then(|g| spans[g.span].attrs.shading.as_ref());
        let mut bottom = first.bottom;
        i += 1;
        while let Some(l) = placed.geometry.get(i).filter(|l| {
            l.left == first.left
                && l.top >= bottom - 0.01
                && !text[first.start..l.start].contains('\n')
        }) {
            bottom = l.bottom;
            i += 1;
        }
        if let Some(c) = color {
            ops.push(Op::FillPath {
                paint: Paint::Solid {
                    color: c.rgba(s),
                    ink: c.ink(s),
                    overprint,
                },
                path: rect(
                    first.left,
                    first.top,
                    first.right - first.left,
                    bottom - first.top,
                ),
            });
        }
    }
    ops
}

pub fn outline(id: u32, size: f32, glyphs: &[u16], positions: &[f32]) -> Vec<f32> {
    let bytes = font_bytes(id);
    let Ok(font) = skrifa::FontRef::new(&bytes) else {
        return Vec::new();
    };
    let at = located(id).1;
    let location = font
        .axes()
        .location(at.iter().map(|(t, v)| (t.as_str(), *v)));
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
            let settings = skrifa::outline::DrawSettings::unhinted(size, &location);
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

fn source(
    text: &str,
    line: &[Glyph],
    run: std::ops::Range<usize>,
    tf: &TextFrame,
) -> (String, Vec<std::ops::Range<usize>>) {
    let mut out = String::new();
    let mut ranges = Vec::with_capacity(run.len());
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
            for c in text[g.cluster..word].chars() {
                match c {
                    PAGE_NUMBER | DATE => out.push_str(tf.marker(c)),
                    c => out.push(c),
                }
            }
        }
        ranges.push(start..out.len());
    }
    (out, ranges)
}

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

pub fn set(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    from: usize,
) -> (Placed, Option<usize>) {
    let (inner, cw) = columns(frame, tf);
    let room = (inner[3] + 0.01) * tf.columns.max(1) as f32;
    let Some(s) = Shown::of(text, spans) else {
        let (placed, starts) = wrapped(text, spans, from, tf, room, inner, cw);
        let rest = starts.get(placed.placed).copied();
        return (placed, rest);
    };
    let (placed, starts) = wrapped(&s.text, &s.spans, s.shown(from), tf, room, inner, cw);
    let rest = starts.get(placed.placed).map(|&b| s.original(b));
    (s.placed(placed), rest)
}

struct Shown {
    text: String,
    spans: Vec<Span>,
    at: Vec<(usize, usize, usize)>,
}

impl Shown {
    fn of(text: &str, spans: &[Span]) -> Option<Shown> {
        if !spans.iter().any(|s| s.attrs.hidden) {
            return None;
        }
        let mut s = Shown {
            text: String::new(),
            spans: Vec::new(),
            at: Vec::new(),
        };
        let (mut byte, mut chars) = (0, text.chars());
        for (i, span) in spans.iter().enumerate() {
            let (start, mut n) = (byte, 0);
            while n < span.len {
                let Some(c) = chars.next() else { break };
                n += c.len_utf16();
                byte += c.len_utf8();
            }
            if !span.attrs.hidden {
                s.at.push((i, start, s.text.len()));
                s.text.push_str(&text[start..byte]);
                s.spans.push(span.clone());
            }
        }
        if s.spans.is_empty() {
            s.at.push((0, 0, 0));
            s.spans.push(Span {
                len: 0,
                ..spans[0].clone()
            });
        }
        Some(s)
    }

    fn shown(&self, b: usize) -> usize {
        let i = self.at.partition_point(|a| a.1 <= b).max(1) - 1;
        let end = self.at.get(i + 1).map_or(self.text.len(), |a| a.2);
        (self.at[i].2 + b.saturating_sub(self.at[i].1)).min(end)
    }

    fn original(&self, b: usize) -> usize {
        let (_, o, s) = self.at[self.at.partition_point(|a| a.2 <= b).max(1) - 1];
        o + b - s
    }

    fn placed(&self, p: Placed) -> Placed {
        let lines = p.lines.into_iter().map(|l| {
            l.into_iter()
                .map(|g| Glyph {
                    cluster: self.original(g.cluster),
                    span: self.at[g.span].0,
                    ..g
                })
                .collect()
        });
        let geometry = p.geometry.into_iter().map(|l| Line {
            start: self.original(l.start),
            end: self.original(l.end),
            stops: (l.stops.iter())
                .flat_map(|&(b, x)| {
                    let i = self.at.partition_point(|a| a.2 <= b).max(1) - 1;
                    let before = (i > 0 && self.at[i].2 == b)
                        .then(|| self.at[i - 1].1 + b - self.at[i - 1].2)
                        .filter(|&e| e != self.at[i].1);
                    before
                        .into_iter()
                        .chain([self.original(b)])
                        .map(move |b| (b, x))
                })
                .collect(),
            ..l
        });
        Placed {
            lines: lines.collect(),
            geometry: geometry.collect(),
            ..p
        }
    }
}

fn wrapped(
    text: &str,
    spans: &[Span],
    from: usize,
    tf: &TextFrame,
    room: f32,
    inner: [f32; 4],
    cw: f32,
) -> (Placed, Vec<usize>) {
    let (mut insets, mut tries) = (Vec::new(), 0);
    loop {
        let rows = rows(text, spans, Some(cw), from, tf, room, &insets).0;
        let starts: Vec<usize> = rows.iter().map(|r| r.stops[0].0).collect();
        let placed = place(rows, inner, cw, tf);
        tries += 1;
        if tf.wrap.is_empty() || placed.insets == insets || tries == 8 {
            return (placed, starts);
        }
        insets = placed.insets;
    }
}

pub fn measure(
    text: &str,
    spans: &[Span],
    tf: &TextFrame,
    w: Option<f32>,
    from: usize,
) -> [f32; 2] {
    if let Some(s) = Shown::of(text, spans) {
        return measure(&s.text, &s.spans, tf, w, s.shown(from));
    }
    let (h_in, v_in) = (
        (tf.inset_left + tf.inset_right) as f32,
        (tf.inset_top + tf.inset_bottom) as f32,
    );
    let n = tf.columns.max(1) as f32;
    let wrap = if w.is_some() {
        tf.wrap.clone()
    } else {
        Vec::new()
    };
    let (rows, cw, w) = match w {
        Some(w) => {
            let (_, cw) = columns([0.0, 0.0, w, 0.0], tf);
            (
                rows(text, spans, Some(cw), from, tf, f32::INFINITY, &[]).0,
                cw,
                w,
            )
        }
        None => {
            let (rows, natural) = rows(text, spans, None, from, tf, f32::INFINITY, &[]);
            let cw = ceil(natural);
            (rows, cw, n * cw + (n - 1.0) * tf.gutter as f32 + h_in)
        }
    };
    let top = tf.inset_top as f32;
    let fit = |ih: f32, columns: u32| {
        let tf = TextFrame {
            columns,
            vertical_align: VerticalAlign::Top,
            wrap: wrap.clone(),
            ..tf.clone()
        };
        let inner = [tf.inset_left as f32, top, cw, ih];
        if wrap.is_empty() {
            let placed = place(rows.clone(), inner, cw, &tf);
            let all = placed.placed == rows.len();
            (placed, all)
        } else {
            let (placed, starts) = wrapped(text, spans, from, &tf, f32::INFINITY, inner, cw);
            let all = placed.placed == starts.len();
            (placed, all)
        }
    };
    let one = fit(f32::MAX, 1).0.bottom - top;
    let ih = if tf.columns <= 1 {
        one
    } else {
        let (mut lo, mut hi) = (0.0, one);
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if fit(mid, tf.columns).1 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    };
    [w, ih.max(0.0) + v_in]
}

fn ceil(v: f32) -> f32 {
    (v * 100.0).ceil() / 100.0
}

fn columns([x, y, w, h]: [f32; 4], tf: &TextFrame) -> ([f32; 4], f32) {
    let [top, right, bottom, left] =
        [tf.inset_top, tf.inset_right, tf.inset_bottom, tf.inset_left].map(|v| v as f32);
    let (iw, ih) = ((w - left - right).max(0.0), (h - top - bottom).max(0.0));
    let n = tf.columns.max(1);
    let cw = ((iw - (n - 1) as f32 * tf.gutter as f32) / n as f32).max(0.0);
    ([x + left, y + top, iw, ih], cw)
}

struct Face {
    bytes: Rc<[u8]>,
    data: ShaperData,
    instance: ShaperInstance,
    metrics: Metrics,
}

thread_local! {
    static FACES: RefCell<HashMap<(*const u8, u32), Rc<Face>>> = RefCell::default();
}

fn face(id: u32) -> Rc<Face> {
    let bytes = font_bytes(id);
    FACES.with_borrow_mut(|faces| {
        let face = faces.entry((bytes.as_ptr(), id)).or_insert_with(|| {
            let font = FontRef::new(&bytes).unwrap();
            let data = ShaperData::new(&font);
            let at = located(id).1;
            let instance =
                ShaperInstance::from_variations(&font, at.iter().map(|(t, v)| (t.as_str(), *v)));
            let shaper = data.shaper(&font).instance(Some(&instance)).build();
            let upem = font.head().unwrap().units_per_em() as f32;
            let hhea = font.hhea().unwrap();
            let mut buf = UnicodeBuffer::new();
            buf.push_str("-");
            buf.guess_segment_properties();
            let dash = shaper.shape(buf, ShapeOptions::new());
            let metrics = Metrics {
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
            };
            Rc::new(Face {
                bytes: bytes.clone(),
                data,
                instance,
                metrics,
            })
        });
        face.clone()
    })
}

struct Metrics {
    upem: f32,
    cap: f32,
    ascent: f32,
    descent: f32,
    gap: f32,
    hyphen: u16,
    hyphen_adv: f32,
}

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

fn rows(
    text: &str,
    spans: &[Span],
    cw: Option<f32>,
    from: usize,
    tf: &TextFrame,
    room: f32,
    insets: &[[f32; 2]],
) -> (Vec<Row>, f32) {
    let ids: Vec<u32> = spans.iter().map(|s| font_of(&s.attrs)).collect();
    let mut used = ids.clone();
    used.sort_unstable();
    used.dedup();
    let slot: Vec<usize> = ids.iter().map(|i| used.binary_search(i).unwrap()).collect();
    let faces: Vec<Rc<Face>> = used.iter().map(|&i| face(i)).collect();
    let fonts: Vec<_> = faces
        .iter()
        .map(|f| FontRef::new(&f.bytes).unwrap())
        .collect();
    let shapers: Vec<_> = fonts
        .iter()
        .zip(&faces)
        .map(|(f, c)| c.data.shaper(f).instance(Some(&c.instance)).build())
        .collect();
    let metrics: Vec<&Metrics> = faces.iter().map(|f| &f.metrics).collect();

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
        let (below, above) = (l - above, above.max(a.object_size[1] as f32));
        (above + a.rise(), below - a.rise())
    };

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
        let set = rows.len();
        if let Some(r) = rows.last_mut() {
            r.after += first.space_before as f32;
        }
        let inset = |i: usize| insets.get(set + i).copied().unwrap_or_default();
        let n = left.len().max(insets.len().saturating_sub(set));
        let widths: Vec<f32> = (0..=n)
            .map(|i| {
                let [l, r] = if i < n { inset(i) } else { [0.0; 2] };
                (cw - left[i.min(left.len() - 1)] - l - r - first.indent_right as f32).max(0.0)
            })
            .collect();
        let mut from = 0;
        let key = widths
            .iter()
            .chain(&[f32::NAN])
            .copied()
            .chain(items.iter().flat_map(|it| match *it {
                Item::Box(w) => [0.0, w, 0.0, 0.0],
                Item::Glue {
                    width,
                    stretch,
                    shrink,
                } => [1.0, width, stretch, shrink],
                Item::Penalty { width, cost } => [2.0, width, cost, 0.0],
            }))
            .map(f32::to_bits)
            .collect();
        let breaks = kept(&BREAKS, key, || break_lines(&items, &widths).into());
        for (i, &(end, r)) in breaks.iter().enumerate() {
            let (left, cw) = (left[i.min(left.len() - 1)] + inset(i)[0], widths[i.min(n)]);
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
            let spans_here = if spans_here.is_empty() {
                vec![span_at(start)]
            } else {
                spans_here
            };
            let (above, below) = spans_here
                .iter()
                .map(|&s| vertical(s))
                .fold((0.0f32, 0.0f32), |(a, b), (c, d)| (a.max(c), b.max(d)));
            let cap = spans_here
                .iter()
                .map(|&s| metrics[slot[s]].cap * spans[s].attrs.drawn() + spans[s].attrs.rise())
                .fold(0.0, f32::max);
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
                cap,
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

    let shape_para = |para: &str, start: usize| -> Shaped {
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
                    (PAGE_NUMBER | DATE, _) => tf.marker(c).chars().for_each(add),
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
        shaped.into()
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
        let mut runs = Vec::new();
        let mut s = span_at(start);
        loop {
            let a = &spans[s].attrs;
            let at = if s == 0 { 0 } else { ends[s - 1] }.max(start) - start;
            runs.push((at, ids[s], a.features.clone(), a.text_case));
            if s + 1 == spans.len() || ends[s] >= start + para.len() {
                break;
            }
            s += 1;
        }
        let shown: String = [PAGE_NUMBER, DATE]
            .into_iter()
            .filter(|&m| para.contains(m))
            .map(|m| format!("{m}{}", tf.marker(m)))
            .collect();
        let shaped = kept(&SHAPED, (para.to_string(), shown, runs), || {
            shape_para(para, start)
        });
        let breaks = if first.hyphenate {
            kept(&SYLLABLES, (para.to_string(), first.lang), || {
                syllables(para, first.lang).into()
            })
        } else {
            Rc::default()
        };
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
        for &(at, id, adv, dx, dy) in shaped.iter() {
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
            let adv = match a.object.is_empty() {
                true => adv * size + size * (a.letter_spacing / 100.0) as f32,
                false => a.object_size[0] as f32,
            };
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
        let indent = first.indent_left as f32;
        let set = |j: usize| tab_at(&items[j], clusters[j]);
        let point = |j: usize| point_at(&items[j], clusters[j]);
        for (k, _) in tabs(&first.tabs, &mut ws, set, point, indent + dw + hang) {
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
        let natural = natural + dw + hang + indent + first.indent_right as f32;
        widest = widest.max(natural);
        let mut left = vec![indent + dw + hang; if dw > 0.0 { lines } else { 1 }];
        left.push(indent + hang);
        marks.iter_mut().for_each(|g| g.x += indent);
        lead.iter_mut().for_each(|s| s.1 += indent);
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

fn place(rows: Vec<Row>, [ix, iy, _, ih]: [f32; 4], cw: f32, tf: &TextFrame) -> Placed {
    let (ox, oy) = (ix - tf.inset_left as f32, iy - tf.inset_top as f32);
    let areas: Vec<([f32; 4], bool)> = tf
        .wrap
        .iter()
        .map(|&([x0, y0, x1, y1], jump)| ([ox + x0, oy + y0, ox + x1, oy + y1], jump))
        .collect();
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
    let mut insets: Vec<[f32; 2]> = Vec::new();
    let mut columns: Vec<(usize, f32)> = Vec::new();
    let (mut col, mut top, mut i, mut first) = (0, iy, 0, 0);
    while let Some(row) = rows.get(i) {
        let above = if tf.trim && i == first {
            row.cap
        } else {
            row.above
        };
        let below = if tf.trim { 0.0 } else { row.below };
        let baseline = snap(top + above);
        let cx = ix + col as f32 * (cw + gutter);
        let (band, end) = ([baseline - row.above, baseline + row.below], cx + cw);
        let hit: Vec<_> = areas
            .iter()
            .filter(|(a, _)| a[1] < band[1] && a[3] > band[0] && a[0] < end && a[2] > cx)
            .collect();
        let mut room = vec![[cx, end]];
        for (a, _) in &hit {
            room = room
                .into_iter()
                .flat_map(|[l, r]| [[l, r.min(a[0])], [l.max(a[2]), r]])
                .filter(|[l, r]| r > l)
                .collect();
        }
        let [l, r] = room
            .into_iter()
            .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
            .unwrap_or([cx, cx]);
        let ends = |jump: bool| hit.iter().filter(move |h| h.1 || !jump).map(|h| h.0[3]);
        if hit.iter().any(|h| h.1) {
            top += ends(true).fold(band[0], f32::max) - band[0];
            continue;
        }
        if !hit.is_empty() && r - l < 4.0 * (row.above + row.below) {
            top += ends(false).fold(f32::INFINITY, f32::min) - band[0];
            continue;
        }
        if baseline + below > bottom {
            let mut j = i;
            while j > first && rows[j - 1].keep {
                j -= 1;
            }
            if j > first {
                lines.truncate(j);
                geometry.truncate(j);
                insets.truncate(j);
                columns[col as usize].1 =
                    geometry[j - 1].bottom - if tf.trim { rows[j - 1].below } else { 0.0 };
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
        insets.push([l - cx, end - r]);
        geometry.push(Line {
            start: row.stops[0].0,
            end: row.stops.last().unwrap().0,
            top: baseline - row.above,
            bottom: baseline + row.below,
            left: l,
            right: r,
            stops: row.stops.iter().map(|&(b, x)| (b, cx + x)).collect(),
        });
        lines.push(
            row.glyphs
                .iter()
                .map(|g| Glyph {
                    x: cx + g.x,
                    y: if tf.round_baselines {
                        iy + (baseline - iy).round()
                    } else {
                        baseline
                    } + g.y,
                    ..g.clone()
                })
                .collect(),
        );
        columns[col as usize].1 = baseline + below;
        top = baseline + rows.get(i + 1).map_or(row.below, |r| r.below) + row.after;
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
        insets,
        bottom: columns.iter().map(|c| c.1).fold(iy, f32::max),
        lines,
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Placed {
    pub lines: Vec<Vec<Glyph>>,
    pub geometry: Vec<Line>,
    insets: Vec<[f32; 2]>,
    placed: usize,
    bottom: f32,
}

impl Placed {
    pub fn shifted(&self, bytes: isize, spans: isize) -> Placed {
        let at = |v: usize| v.wrapping_add_signed(bytes);
        let lines = self.lines.iter().map(|l| {
            l.iter()
                .map(|g| Glyph {
                    cluster: at(g.cluster),
                    span: g.span.wrapping_add_signed(spans),
                    ..g.clone()
                })
                .collect()
        });
        let geometry = self.geometry.iter().map(|l| Line {
            start: at(l.start),
            end: at(l.end),
            stops: l.stops.iter().map(|&(s, x)| (at(s), x)).collect(),
            ..l.clone()
        });
        Placed {
            lines: lines.collect(),
            geometry: geometry.collect(),
            insets: self.insets.clone(),
            placed: self.placed,
            bottom: self.bottom,
        }
    }
}

#[derive(Clone)]
struct Row {
    glyphs: Vec<Glyph>,
    above: f32,
    below: f32,
    after: f32,
    stops: Vec<(usize, f32)>,
    keep: bool,
    cap: f32,
}

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

pub fn line_of(lines: &[Line], at: usize) -> usize {
    (0..lines.len())
        .find(|&i| {
            at < lines[i].end
                || at == lines[i].end && lines.get(i + 1).is_none_or(|n| n.start != at)
        })
        .unwrap_or(lines.len().saturating_sub(1))
}

pub fn caret(text: &str, lines: &[Line], at: usize) -> [f32; 3] {
    match lines.get(line_of(lines, at)) {
        Some(l) => [x_at(text, l, at), l.top, l.bottom],
        None => [0.0; 3],
    }
}

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
mod tests;
