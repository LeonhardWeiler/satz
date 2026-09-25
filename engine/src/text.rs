use crate::color::Color;
use crate::display_list::{Op, Paint};
use crate::linebreak::{Item, break_lines};
use crate::style::{Fill, paints};
use crate::variable::{Scope, Value};
use harfrust::{FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::TableProvider;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::collections::BTreeMap;

pub const FONT: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");

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
        }
    }
}

/// A glyph at (x, y) on its baseline, drawn with the attributes of `span`.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub span: usize,
}

/// Glyph runs of `text` set in `frame`; characters without a fill take `fills`.
pub fn draw(
    text: &str,
    spans: &[Span],
    fills: &[Fill],
    frame: [f32; 4],
    tf: &TextFrame,
    s: &Scope,
) -> Vec<Op> {
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
    for line in lay_out(text, spans, frame, tf) {
        for run in line.chunk_by(|a, b| {
            spans[a.span].attrs.size == spans[b.span].attrs.size
                && span_paints[a.span] == span_paints[b.span]
        }) {
            for paint in &span_paints[run[0].span] {
                ops.push(Op::GlyphRun {
                    font: 0,
                    size: spans[run[0].span].attrs.size as f32,
                    paint: paint.clone(),
                    glyphs: run.iter().map(|g| g.id).collect(),
                    positions: run.iter().flat_map(|g| [g.x, g.y]).collect(),
                });
            }
        }
    }
    ops
}

/// The lines of `text` that fit in `frame`, each a list of glyphs.
pub fn lay_out(
    text: &str,
    spans: &[Span],
    [x, y, w, h]: [f32; 4],
    tf: &TextFrame,
) -> Vec<Vec<Glyph>> {
    let [top_in, right_in, bottom_in, left_in] =
        [tf.inset_top, tf.inset_right, tf.inset_bottom, tf.inset_left].map(|v| v as f32);
    let (ix, iy) = (x + left_in, y + top_in);
    let (iw, ih) = (
        (w - left_in - right_in).max(0.0),
        (h - top_in - bottom_in).max(0.0),
    );
    let n = tf.columns.max(1);
    let gutter = tf.gutter as f32;
    let cw = ((iw - (n - 1) as f32 * gutter) / n as f32).max(0.0);
    let font = FontRef::new(FONT).unwrap();
    let upem = font.head().unwrap().units_per_em() as f32;
    let hhea = font.hhea().unwrap();
    let ascent = hhea.ascender().to_i16() as f32 / upem;
    let descent = -hhea.descender().to_i16() as f32 / upem;
    let gap = hhea.line_gap().to_i16() as f32 / upem;
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let mut buf = UnicodeBuffer::new();
    buf.push_str("-");
    buf.guess_segment_properties();
    let dash = shaper.shape(buf, ShapeOptions::new());
    let (hyphen, hyphen_adv) = (
        dash.glyph_infos()[0].glyph_id as u16,
        dash.glyph_positions()[0].x_advance as f32 / upem,
    );

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
    let vertical = |a: &Attrs| {
        let s = a.size as f32;
        let l = match a.line_height as f32 {
            0.0 => (ascent + descent + gap) * s,
            l => l,
        };
        let above = ascent * s + (l - (ascent + descent) * s) / 2.0;
        (above, l - above)
    };

    let mut rows: Vec<Row> = Vec::new();
    let mut start = 0;
    for para in text.split('\n') {
        let first = &spans[span_at(start)].attrs;
        let mut buf = UnicodeBuffer::new();
        buf.push_str(para);
        buf.guess_segment_properties();
        let shaped = shaper.shape(buf, ShapeOptions::new());
        let breaks = if first.hyphenate {
            syllables(para, first.lang)
        } else {
            Vec::new()
        };
        let mut items = Vec::new();
        let mut glyphs = Vec::new();
        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            let span = span_at(start + info.cluster as usize);
            let a = &spans[span].attrs;
            let scale = a.size as f32 / upem;
            let adv = pos.x_advance as f32 * scale + (a.size * a.letter_spacing / 100.0) as f32;
            if breaks.binary_search(&(info.cluster as usize)).is_ok() {
                items.push(Item::Penalty {
                    width: hyphen_adv * a.size as f32,
                    cost: HYPHEN_COST,
                });
                glyphs.push(Some((hyphen, 0.0, 0.0, span)));
            }
            if para[info.cluster as usize..].starts_with(' ') {
                items.push(Item::Glue {
                    width: adv,
                    stretch: adv / 2.0,
                    shrink: adv / 3.0,
                });
                glyphs.push(None);
            } else {
                items.push(Item::Box(adv));
                glyphs.push(Some((
                    info.glyph_id as u16,
                    pos.x_offset as f32 * scale,
                    pos.y_offset as f32 * scale,
                    span,
                )));
            }
        }
        items.extend([FILL, FORCE]);
        glyphs.extend([None, None]);

        let mut from = 0;
        for (end, r) in break_lines(&items, cw) {
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
                vertical(&spans[span_at(start)].attrs)
            } else {
                spans_here
                    .iter()
                    .map(|&s| vertical(&spans[s].attrs))
                    .fold((0.0f32, 0.0f32), |(a, b), (c, d)| (a.max(c), b.max(d)))
            };
            let mut cx = match first.text_align {
                TextAlign::Center => (cw - used) / 2.0,
                TextAlign::Right => cw - used,
                _ => 0.0,
            };
            let mut line = Vec::new();
            for k in from..last {
                if matches!(items[k], Item::Penalty { .. }) && k != end {
                    continue;
                }
                if let Some((id, dx, dy, span)) = glyphs[k] {
                    line.push(Glyph {
                        id,
                        x: cx + dx,
                        y: -dy,
                        span,
                    });
                }
                cx += spread(&items[k]);
            }
            rows.push(Row {
                glyphs: line,
                above,
                below,
                after: 0.0,
            });
            from = end + 1;
        }
        if let Some(r) = rows.last_mut() {
            r.after = first.paragraph_spacing as f32;
        }
        start += para.len() + 1;
    }

    let (grid, grid_top) = (tf.baseline_grid as f32, iy + tf.baseline_start as f32);
    let snap = |b: f32| match grid {
        0.0 => b,
        g => grid_top + ((b - grid_top - 0.001) / g).ceil().max(0.0) * g,
    };
    let bottom = iy + ih + 0.01;
    let mut lines: Vec<Vec<Glyph>> = Vec::new();
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
    }
    lines
}

/// A line with glyphs relative to its column's left edge and baseline, the space
/// it needs above and below the baseline, and the paragraph spacing after it.
struct Row {
    glyphs: Vec<Glyph>,
    above: f32,
    below: f32,
    after: f32,
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
        draw(text, spans, &[Fill::solid(0x000000ffu32)], frame, tf, &s)
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
        );
        let counts: Vec<usize> = lines.iter().map(Vec::len).collect();
        let shortest = counts[..counts.len() - 1].iter().min().unwrap();
        assert!(*shortest >= 14, "{counts:?}");
    }
}
