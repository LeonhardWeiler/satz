use crate::display_list::{CLOSE, LINE, MOVE, Op};
use crate::text::FONT;
use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point, Rect, Transform};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};

const MM: f32 = 72.0 / 25.4;
const MARK_SPACE: f32 = 10.0 * MM;
const MARK_OFFSET: f32 = 3.0 * MM;
const MARK_LENGTH: f32 = 5.0 * MM;
const MARK_WIDTH: f32 = 0.25;

pub fn pdf(pages: &[Vec<Op>]) -> Vec<u8> {
    let font = Font::new(FONT.into(), 0).unwrap();
    let mut doc = Document::new();
    for ops in pages {
        let Some(&Op::Page {
            width,
            height,
            bleed,
        }) = ops.first()
        else {
            continue;
        };
        let o = bleed + MARK_SPACE;
        let settings = PageSettings::from_wh(width + 2.0 * o, height + 2.0 * o)
            .unwrap()
            .with_trim_box(Rect::from_xywh(o, o, width, height))
            .with_bleed_box(Rect::from_xywh(
                MARK_SPACE,
                MARK_SPACE,
                width + 2.0 * bleed,
                height + 2.0 * bleed,
            ));
        let mut page = doc.start_page_with(settings);
        let mut s = page.surface();
        s.push_transform(&Transform::from_translate(o, o));
        s.push_clip_path(
            &rect(-bleed, -bleed, width + 2.0 * bleed, height + 2.0 * bleed),
            &FillRule::NonZero,
        );
        for op in &ops[1..] {
            draw(&mut s, &font, op);
        }
        s.pop();
        crop_marks(&mut s, width, height);
        s.pop();
        s.finish();
        page.finish();
    }
    doc.finish().unwrap()
}

fn draw(s: &mut Surface, font: &Font, op: &Op) {
    match op {
        Op::FillPath { color, path } => {
            s.set_fill(Some(fill(color)));
            if let Some(p) = build(path) {
                s.draw_path(&p);
            }
        }
        Op::GlyphRun {
            size,
            color,
            glyphs,
            positions,
            ..
        } => {
            let Some(&[x0, y0]) = positions.first_chunk() else {
                return;
            };
            let run: Vec<KrillaGlyph> = glyphs
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let (x, y) = (positions[2 * i], positions[2 * i + 1]);
                    let next = positions.get(2 * i + 2).map_or(x, |nx| *nx);
                    KrillaGlyph::new(
                        GlyphId::new(*g as u32),
                        (next - x) / size,
                        0.0,
                        (y0 - y) / size,
                        0.0,
                        0..0,
                        None,
                    )
                })
                .collect();
            s.set_fill(Some(fill(color)));
            s.draw_glyphs(Point::from_xy(x0, y0), &run, font.clone(), "", *size, false);
        }
        _ => {}
    }
}

fn crop_marks(s: &mut Surface, w: f32, h: f32) {
    let mut pb = PathBuilder::new();
    for (x, dx) in [(0.0, -1.0), (w, 1.0)] {
        for (y, dy) in [(0.0, -1.0), (h, 1.0)] {
            pb.move_to(x + dx * MARK_OFFSET, y);
            pb.line_to(x + dx * (MARK_OFFSET + MARK_LENGTH), y);
            pb.move_to(x, y + dy * MARK_OFFSET);
            pb.line_to(x, y + dy * (MARK_OFFSET + MARK_LENGTH));
        }
    }
    s.set_fill(None);
    s.set_stroke(Some(Stroke {
        paint: rgb::Color::black().into(),
        width: MARK_WIDTH,
        ..Default::default()
    }));
    s.draw_path(&pb.finish().unwrap());
    s.set_stroke(None);
}

fn fill(c: &[f32; 4]) -> Fill {
    let [r, g, b] = [c[0], c[1], c[2]].map(|v| (v * 255.0).round() as u8);
    Fill {
        paint: rgb::Color::new(r, g, b).into(),
        opacity: NormalizedF32::new(c[3]).unwrap_or(NormalizedF32::ONE),
        rule: FillRule::NonZero,
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_xywh(x, y, w, h).unwrap());
    pb.finish().unwrap()
}

fn build(cmds: &[f32]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let mut i = 0;
    while i < cmds.len() {
        let v = cmds[i];
        if v == MOVE {
            pb.move_to(cmds[i + 1], cmds[i + 2]);
            i += 3;
        } else if v == LINE {
            pb.line_to(cmds[i + 1], cmds[i + 2]);
            i += 3;
        } else if v == CLOSE {
            pb.close();
            i += 1;
        } else {
            return None;
        }
    }
    pb.finish()
}

#[cfg(test)]
mod tests {
    use crate::Doc;

    fn default_pdf() -> String {
        let d = Doc::new();
        String::from_utf8_lossy(&super::pdf(&[d.render(0)])).into_owned()
    }

    fn page_box(pdf: &str, name: &str) -> Vec<f32> {
        let at = pdf.find(&format!("/{name}[")).unwrap() + name.len() + 2;
        let end = at + pdf[at..].find(']').unwrap();
        pdf[at..end]
            .split(' ')
            .map(|v| v.parse().unwrap())
            .collect()
    }

    fn assert_close(a: &[f32], b: &[f32]) {
        assert!(
            a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01),
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn a5_page_with_3mm_bleed_has_trim_bleed_and_media_boxes() {
        let pdf = default_pdf();
        assert_close(&page_box(&pdf, "MediaBox"), &[0.0, 0.0, 493.23, 668.98]);
        assert_close(&page_box(&pdf, "BleedBox"), &[28.35, 28.35, 464.88, 640.63]);
        assert_close(&page_box(&pdf, "TrimBox"), &[36.85, 36.85, 456.38, 632.13]);
    }

    #[test]
    fn the_font_is_embedded() {
        assert!(default_pdf().contains("/FontFile"));
    }
}
