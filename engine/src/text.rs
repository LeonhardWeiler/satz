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
pub const PARAGRAPH: [&str; 2] = ["textAlign", "paragraphSpacing"];

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

/// A glyph at (x, y) on its baseline, drawn with the attributes of `span`.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub span: usize,
}

/// Glyph runs of `text` set in `frame`; characters without a fill take `fills`.
pub fn draw(text: &str, spans: &[Span], fills: &[Fill], frame: [f32; 4], s: &Scope) -> Vec<Op> {
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
    for line in lay_out(text, spans, frame) {
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
pub fn lay_out(text: &str, spans: &[Span], [x, y, w, h]: [f32; 4]) -> Vec<Vec<Glyph>> {
    let font = FontRef::new(FONT).unwrap();
    let upem = font.head().unwrap().units_per_em() as f32;
    let hhea = font.hhea().unwrap();
    let ascent = hhea.ascender().to_i16() as f32 / upem;
    let descent = -hhea.descender().to_i16() as f32 / upem;
    let gap = hhea.line_gap().to_i16() as f32 / upem;
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();

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

    let mut lines = Vec::new();
    let mut top = y;
    let mut start = 0;
    for para in text.split('\n') {
        let first = &spans[span_at(start)].attrs;
        let mut buf = UnicodeBuffer::new();
        buf.push_str(para);
        buf.guess_segment_properties();
        let shaped = shaper.shape(buf, ShapeOptions::new());
        let mut items = Vec::new();
        let mut glyphs = Vec::new();
        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            let span = span_at(start + info.cluster as usize);
            let a = &spans[span].attrs;
            let scale = a.size as f32 / upem;
            let adv = pos.x_advance as f32 * scale + (a.size * a.letter_spacing / 100.0) as f32;
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
        for (end, r) in break_lines(&items, w) {
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
            let used: f32 = items[from..end].iter().map(spread).sum();
            let spans_here: Vec<usize> = glyphs[from..end].iter().flatten().map(|g| g.3).collect();
            let (above, below) = if spans_here.is_empty() {
                vertical(&spans[span_at(start)].attrs)
            } else {
                spans_here
                    .iter()
                    .map(|&s| vertical(&spans[s].attrs))
                    .fold((0.0f32, 0.0f32), |(a, b), (c, d)| (a.max(c), b.max(d)))
            };
            if top + above + below > y + h + 0.01 {
                return lines;
            }
            let baseline = top + above;
            let mut cx = x + match first.text_align {
                TextAlign::Center => (w - used) / 2.0,
                TextAlign::Right => w - used,
                _ => 0.0,
            };
            let mut line = Vec::new();
            for k in from..end {
                if let Some((id, dx, dy, span)) = glyphs[k] {
                    line.push(Glyph {
                        id,
                        x: cx + dx,
                        y: baseline - dy,
                        span,
                    });
                }
                cx += spread(&items[k]);
            }
            lines.push(line);
            top = baseline + below;
            from = end + 1;
        }
        top += first.paragraph_spacing as f32;
        start += para.len() + 1;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::{Modes, Palette};

    const H: u16 = 9;
    const I: u16 = 36;
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
        let palette = Palette::default();
        let modes = Modes::new();
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        draw(text, spans, &[Fill::solid(0x000000ffu32)], frame, &s)
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
}
