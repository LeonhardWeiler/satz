use crate::display_list::{Op, Paint};
use crate::linebreak::{Item, break_lines};
use harfrust::{FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::TableProvider;

pub const FONT: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");

const LEADING: f32 = 1.2;
const FILL: Item = Item::Glue {
    width: 0.0,
    stretch: 1e6,
    shrink: 0.0,
};
const FORCE: Item = Item::Penalty {
    width: 0.0,
    cost: f32::NEG_INFINITY,
};

pub fn layout(text: &str, size: f32, [x, y, w, h]: [f32; 4], paint: &Paint) -> Vec<Op> {
    let font = FontRef::new(FONT).unwrap();
    let scale = size / font.head().unwrap().units_per_em() as f32;
    let hhea = font.hhea().unwrap();
    let ascent = hhea.ascender().to_i16() as f32 * scale;
    let descent = -hhea.descender().to_i16() as f32 * scale;
    let data = ShaperData::new(&font);
    let mut buf = UnicodeBuffer::new();
    buf.push_str(text);
    buf.guess_segment_properties();
    let shaped = data.shaper(&font).build().shape(buf, ShapeOptions::new());

    let mut items = Vec::new();
    let mut glyphs = Vec::new();
    for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        let adv = pos.x_advance as f32 * scale;
        match text[info.cluster as usize..].chars().next() {
            Some('\n') => {
                items.extend([FILL, FORCE]);
                glyphs.extend([None, None]);
            }
            Some(' ') => {
                items.push(Item::Glue {
                    width: adv,
                    stretch: adv / 2.0,
                    shrink: adv / 3.0,
                });
                glyphs.push(None);
            }
            _ => {
                items.push(Item::Box(adv));
                glyphs.push(Some((
                    info.glyph_id as u16,
                    pos.x_offset as f32 * scale,
                    pos.y_offset as f32 * scale,
                )));
            }
        }
    }
    items.extend([FILL, FORCE]);
    glyphs.extend([None, None]);

    let mut ops = Vec::new();
    let mut start = 0;
    let mut baseline = y + ascent;
    for (end, r) in break_lines(&items, w) {
        if baseline + descent > y + h {
            break;
        }
        let r = if r.is_finite() { r.max(-1.0) } else { 0.0 };
        while start < end && !matches!(items[start], Item::Box(_)) {
            start += 1;
        }
        let (mut ids, mut positions, mut cx) = (Vec::new(), Vec::new(), x);
        for k in start..end {
            match items[k] {
                Item::Box(adv) => {
                    let (g, dx, dy) = glyphs[k].unwrap();
                    ids.push(g);
                    positions.extend([cx + dx, baseline - dy]);
                    cx += adv;
                }
                Item::Glue {
                    width,
                    stretch,
                    shrink,
                } => cx += width + r * if r < 0.0 { shrink } else { stretch },
                Item::Penalty { .. } => {}
            }
        }
        if !ids.is_empty() {
            ops.push(Op::GlyphRun {
                font: 0,
                size,
                paint: paint.clone(),
                glyphs: ids,
                positions,
            });
        }
        start = end + 1;
        baseline += LEADING * size;
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Paint = Paint::Solid {
        color: [0.0, 0.0, 0.0, 1.0],
        ink: crate::color::Ink::Rgb,
    };

    const H: u16 = 9;
    const I: u16 = 36;
    const ASCENT: f32 = 10.36;

    fn runs(ops: &[Op]) -> Vec<(Vec<u16>, Vec<f32>)> {
        ops.iter()
            .map(|op| match op {
                Op::GlyphRun {
                    glyphs, positions, ..
                } => (glyphs.clone(), positions.clone()),
                _ => panic!("unexpected {op:?}"),
            })
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
        let r = runs(&layout("Hi", 10.0, [5.0, 7.0, 100.0, 50.0], &BLACK));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, vec![H, I]);
        assert_close(&r[0].1, &[5.0, 7.0 + ASCENT, 12.88, 7.0 + ASCENT]);
    }

    #[test]
    fn shaping_forms_the_fi_ligature() {
        assert_eq!(
            runs(&layout("fi", 10.0, [0.0, 0.0, 100.0, 50.0], &BLACK))[0].0,
            vec![417]
        );
    }

    #[test]
    fn wrapped_lines_are_justified_to_the_frame_width() {
        let r = runs(&layout("Hi Hi Hi", 10.0, [5.0, 7.0, 25.0, 50.0], &BLACK));
        let b1 = 7.0 + ASCENT;
        let b2 = b1 + 12.0;
        assert_eq!(r.len(), 2);
        assert_close(&r[0].1, &[5.0, b1, 12.88, b1, 19.14, b1, 27.02, b1]);
        assert_close(&r[1].1, &[5.0, b2, 12.88, b2]);
    }

    #[test]
    fn lines_below_the_frame_are_not_drawn() {
        assert_eq!(
            runs(&layout("Hi Hi Hi", 10.0, [5.0, 7.0, 25.0, 15.0], &BLACK)).len(),
            1
        );
    }

    #[test]
    fn newline_starts_a_new_line() {
        let r = runs(&layout("Hi\nHi", 10.0, [0.0, 0.0, 100.0, 50.0], &BLACK));
        assert_close(&r[1].1, &[0.0, ASCENT + 12.0, 7.88, ASCENT + 12.0]);
    }
}
