use crate::color::{ColorMode, Ink};
use crate::display_list::{CLOSE, CUBIC, LINE, MOVE, Op, Paint, Shadow, Stop as ListStop, close};
use crate::raster::{blur, extent, rasterize, tint};
use crate::text::FONT;
use krilla::Document;
use krilla::blend::BlendMode;
use krilla::color::separation::{SeparationColorant, SeparationSpace};
use krilla::color::{cmyk, rgb, separation};
use krilla::geom::{Path, PathBuilder, Point, Rect, Size, Transform};
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::mask::{Mask, MaskType};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, RadialGradient, SpreadMethod, Stop, Stroke,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};

const MM: f32 = 72.0 / 25.4;
const MARK_SPACE: f32 = 10.0 * MM;
const MARK_OFFSET: f32 = 3.0 * MM;
const MARK_LENGTH: f32 = 5.0 * MM;
const MARK_WIDTH: f32 = 0.25;

/// Shadows and blurs are rasterized at `ppi` in the colour mode `mode`; everything else stays vector.
pub fn pdf(pages: &[Vec<Op>], ppi: f32, mode: ColorMode) -> Vec<u8> {
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
        let env = Env {
            font: &font,
            ppi,
            cmyk: mode == ColorMode::Cmyk,
            page: [-bleed, -bleed, width + 2.0 * bleed, height + 2.0 * bleed],
        };
        draw(&mut s, &env, &ops[1..]);
        s.pop();
        crop_marks(&mut s, width, height);
        s.pop();
        s.finish();
        page.finish();
    }
    doc.finish().unwrap()
}

const BLENDS: [BlendMode; 16] = [
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];

struct Env<'a> {
    font: &'a Font,
    ppi: f32,
    cmyk: bool,
    page: [f32; 4],
}

fn draw(s: &mut Surface, env: &Env, ops: &[Op]) {
    let mut pops = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        match &ops[i] {
            Op::FillPath { paint, path } => {
                if let Some(p) = build(path) {
                    s.set_fill(Some(fill(paint)));
                    s.draw_path(&p);
                }
            }
            Op::StrokePath {
                paint,
                width,
                cap,
                join,
                path,
            } => {
                if let Some(p) = build(path) {
                    let (paint, opacity) = convert(paint);
                    s.set_fill(None);
                    s.set_stroke(Some(Stroke {
                        paint,
                        opacity,
                        width: *width,
                        miter_limit: 4.0,
                        line_cap: [LineCap::Butt, LineCap::Round, LineCap::Square][*cap as usize],
                        line_join: [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel]
                            [*join as usize],
                        dash: None,
                    }));
                    s.draw_path(&p);
                    s.set_stroke(None);
                }
            }
            Op::GlyphRun {
                size,
                paint,
                glyphs,
                positions,
                text,
                ranges,
                ..
            } => {
                let Some(&[x0, y0]) = positions.first_chunk() else {
                    i += 1;
                    continue;
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
                            ranges.get(i).cloned().unwrap_or(0..0),
                            None,
                        )
                    })
                    .collect();
                s.set_fill(Some(fill(paint)));
                s.draw_glyphs(
                    Point::from_xy(x0, y0),
                    &run,
                    env.font.clone(),
                    text,
                    *size,
                    false,
                );
            }
            Op::PushClip { path, invert } => {
                let mut pb = PathBuilder::new();
                if *invert {
                    pb.push_rect(Rect::from_xywh(-1e5, -1e5, 2e5, 2e5).unwrap());
                }
                append(&mut pb, path);
                let rule = if *invert {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                };
                s.push_clip_path(&pb.finish().unwrap(), &rule);
                pops.push(1);
            }
            Op::PushLayer {
                opacity,
                blend,
                blur: sigma,
                shadows,
            } => {
                let mut n = 1;
                if *blend != 0 {
                    s.push_blend_mode(BLENDS[*blend as usize]);
                    s.push_isolated();
                    n += 2;
                }
                s.push_opacity(NormalizedF32::new(*opacity).unwrap_or(NormalizedF32::ONE));
                pops.push(n);
                let end = close(ops, i);
                let inner = &ops[i + 1..end];
                for sh in shadows {
                    raster(s, env, inner, sh.offset, sh.blur, Some(sh));
                }
                if *sigma > 0.0 {
                    raster(s, env, inner, [0.0; 2], *sigma, None);
                    i = end - 1;
                }
            }
            Op::BeginMask => {
                let end = close(ops, i);
                let mut sb = s.stream_builder();
                let mut ms = sb.surface();
                draw(&mut ms, env, &ops[i + 1..end]);
                ms.finish();
                let stream = sb.finish();
                s.push_mask(Mask::new(stream, MaskType::Alpha));
                pops.push(1);
                i = end;
            }
            Op::PopClip | Op::PopLayer | Op::PopMask => {
                for _ in 0..pops.pop().unwrap_or(0) {
                    s.pop();
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Draws `ops` as an image, blurred by `sigma` pt, moved by `offset` and,
/// for shadows, tinted with the shadow's colour. CMYK documents get CMYK images.
fn raster(
    s: &mut Surface,
    env: &Env,
    ops: &[Op],
    offset: [f32; 2],
    sigma: f32,
    shadow: Option<&Shadow>,
) {
    let Some([x, y, w, h]) = extent(ops) else {
        return;
    };
    let m = 3.0 * sigma;
    let [px, py, pw, ph] = env.page;
    let l = (x + offset[0] - m).max(px);
    let t = (y + offset[1] - m).max(py);
    let r = (x + w + offset[0] + m).min(px + pw);
    let b = (y + h + offset[1] + m).min(py + ph);
    if l >= r || t >= b {
        return;
    }
    let plane = |ops: &[Op], tint_with: Option<[f32; 4]>| {
        let mut px = rasterize(ops, [l - offset[0], t - offset[1], r - l, b - t], env.ppi)?;
        if let Some(c) = tint_with {
            tint(&mut px, c);
        }
        blur(&mut px, sigma * env.ppi / 72.0);
        Some(px)
    };
    let image = if env.cmyk {
        let plate = |pick: fn([f32; 4]) -> [f32; 3]| {
            let to = |rgba: &[f32; 4], ink: &Ink| {
                let [a, b, c] = pick(ink.cmyk(rgba));
                [a, b, c, rgba[3]]
            };
            plane(&recolor(ops, &to), shadow.map(|s| to(&s.color, &s.ink)))
        };
        let (Some(cmy), Some(k)) = (plate(|c| [c[0], c[1], c[2]]), plate(|c| [c[3], 0.0, 0.0]))
        else {
            return;
        };
        let size = (cmy.width(), cmy.height());
        let (cmy, k) = (cmy.take_demultiplied(), k.take_demultiplied());
        let image = CmykImage {
            color: cmy
                .chunks(4)
                .zip(k.chunks(4))
                .flat_map(|(a, b)| [a[0], a[1], a[2], b[0]])
                .collect(),
            alpha: cmy.chunks(4).map(|a| a[3]).collect(),
            size,
        };
        Image::from_custom(image, true).unwrap()
    } else {
        let Some(px) = plane(ops, shadow.map(|s| s.color)) else {
            return;
        };
        let (w, h) = (px.width(), px.height());
        Image::from_rgba8(px.take_demultiplied(), w, h)
    };
    let scale = 72.0 / env.ppi;
    let (pw, ph) = image.size();
    s.push_transform(&Transform::from_translate(l, t));
    s.draw_image(
        image,
        Size::from_wh(pw as f32 * scale, ph as f32 * scale).unwrap(),
    );
    s.pop();
}

/// `ops` with every colour replaced by `f(colour, ink)`.
fn recolor(ops: &[Op], f: &impl Fn(&[f32; 4], &Ink) -> [f32; 4]) -> Vec<Op> {
    let stops = |stops: &mut Vec<ListStop>| {
        for s in stops {
            s.color = f(&s.color, &s.ink);
        }
    };
    ops.iter()
        .map(|op| {
            let mut op = op.clone();
            match &mut op {
                Op::FillPath { paint, .. }
                | Op::StrokePath { paint, .. }
                | Op::GlyphRun { paint, .. } => match paint {
                    Paint::Solid { color, ink } => *color = f(color, ink),
                    Paint::Linear { stops: s, .. } | Paint::Radial { stops: s, .. } => stops(s),
                },
                Op::PushLayer { shadows, .. } => {
                    for s in shadows {
                        s.color = f(&s.color, &s.ink);
                    }
                }
                _ => {}
            }
            op
        })
        .collect()
}

#[derive(Hash, Clone)]
struct CmykImage {
    color: Vec<u8>,
    alpha: Vec<u8>,
    size: (u32, u32),
}

impl CustomImage for CmykImage {
    fn color_channel(&self) -> &[u8] {
        &self.color
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        Some(&self.alpha)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        None
    }

    fn color_space(&self) -> ImageColorspace {
        ImageColorspace::Cmyk
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
        paint: krilla::color::Color::from(separation::Color::new(
            255,
            SeparationSpace::new(
                SeparationColorant::AllColorants,
                cmyk::Color::new(255, 255, 255, 255).into(),
            ),
        ))
        .into(),
        width: MARK_WIDTH,
        ..Default::default()
    }));
    s.draw_path(&pb.finish().unwrap());
    s.set_stroke(None);
}

fn fill(p: &Paint) -> Fill {
    let (paint, opacity) = convert(p);
    Fill {
        paint,
        opacity,
        rule: FillRule::NonZero,
    }
}

fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn process(c: [f32; 4]) -> cmyk::Color {
    cmyk::Color::new(byte(c[0]), byte(c[1]), byte(c[2]), byte(c[3]))
}

fn color(rgba: &[f32; 4], ink: &Ink) -> krilla::color::Color {
    match ink {
        Ink::Rgb => rgb::Color::new(byte(rgba[0]), byte(rgba[1]), byte(rgba[2])).into(),
        Ink::Cmyk(c) => process(*c).into(),
        Ink::Spot { name, cmyk, tint } => separation::Color::new(
            byte(*tint),
            SeparationSpace::new(
                SeparationColorant::Custom(name.clone()),
                process(*cmyk).into(),
            ),
        )
        .into(),
    }
}

fn opacity(rgba: &[f32; 4]) -> NormalizedF32 {
    NormalizedF32::new(rgba[3].clamp(0.0, 1.0)).unwrap()
}

/// The stops' inks, or all of them as CMYK when they do not share one colour space.
fn inks(stops: &[ListStop]) -> Vec<Ink> {
    let shared = stops.windows(2).all(|w| match (&w[0].ink, &w[1].ink) {
        (Ink::Rgb, Ink::Rgb) | (Ink::Cmyk(_), Ink::Cmyk(_)) => true,
        (
            Ink::Spot {
                name: a, cmyk: x, ..
            },
            Ink::Spot {
                name: b, cmyk: y, ..
            },
        ) => a == b && x == y,
        _ => false,
    });
    stops
        .iter()
        .map(|s| match &s.ink {
            ink if shared => ink.clone(),
            ink => Ink::Cmyk(ink.cmyk(&s.color)),
        })
        .collect()
}

fn convert(p: &Paint) -> (krilla::paint::Paint, NormalizedF32) {
    let (transform, stops) = match p {
        Paint::Solid { color: c, ink } => return (color(c, ink).into(), opacity(c)),
        Paint::Linear { transform, stops } | Paint::Radial { transform, stops } => (
            Transform::from_row(
                transform[0],
                transform[1],
                transform[2],
                transform[3],
                transform[4],
                transform[5],
            ),
            stops
                .iter()
                .zip(inks(stops))
                .map(|(s, ink)| Stop {
                    offset: NormalizedF32::new(s.at.clamp(0.0, 1.0)).unwrap(),
                    color: color(&s.color, &ink),
                    opacity: opacity(&s.color),
                })
                .collect(),
        ),
    };
    let paint = match p {
        Paint::Linear { .. } => LinearGradient {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 0.0,
            transform,
            spread_method: SpreadMethod::Pad,
            stops,
            anti_alias: false,
        }
        .into(),
        _ => RadialGradient {
            fx: 0.0,
            fy: 0.0,
            fr: 0.0,
            cx: 0.0,
            cy: 0.0,
            cr: 1.0,
            transform,
            spread_method: SpreadMethod::Pad,
            stops,
            anti_alias: false,
        }
        .into(),
    };
    (paint, NormalizedF32::ONE)
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_xywh(x, y, w, h).unwrap());
    pb.finish().unwrap()
}

fn build(cmds: &[f32]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    append(&mut pb, cmds);
    pb.finish()
}

fn append(pb: &mut PathBuilder, cmds: &[f32]) {
    let mut i = 0;
    while i < cmds.len() {
        let v = cmds[i];
        if v == MOVE {
            pb.move_to(cmds[i + 1], cmds[i + 2]);
            i += 3;
        } else if v == LINE {
            pb.line_to(cmds[i + 1], cmds[i + 2]);
            i += 3;
        } else if v == CUBIC {
            let c = &cmds[i + 1..i + 7];
            pb.cubic_to(c[0], c[1], c[2], c[3], c[4], c[5]);
            i += 7;
        } else if v == CLOSE {
            pb.close();
            i += 1;
        } else {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Doc;
    use crate::color::{ColorMode, Ink};
    use crate::display_list::{Op, Paint, Shadow, Stop, rect};

    fn default_pdf() -> String {
        let d = Doc::new();
        String::from_utf8_lossy(&super::pdf(&[d.render(0)], 300.0, ColorMode::Rgb)).into_owned()
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

    fn image_width(ops: &[Op], ppi: f32) -> Option<u32> {
        let pdf =
            String::from_utf8_lossy(&super::pdf(&[ops.to_vec()], ppi, ColorMode::Rgb)).into_owned();
        let at = pdf
            .find("/Subtype /Image")
            .or_else(|| pdf.find("/Subtype/Image"))?;
        let w = at + pdf[at..].find("/Width")? + 6;
        pdf[w..]
            .trim_start()
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    }

    #[test]
    fn shadows_are_rasterized_at_the_document_resolution() {
        let ops = vec![
            Op::Page {
                width: 100.0,
                height: 100.0,
                bleed: 0.0,
            },
            Op::PushLayer {
                opacity: 1.0,
                blend: 0,
                blur: 0.0,
                shadows: vec![Shadow {
                    offset: [0.0, 0.0],
                    blur: 1.0,
                    color: [0.0, 0.0, 0.0, 0.5],
                    ink: Ink::Rgb,
                }],
            },
            Op::FillPath {
                paint: Paint::Solid {
                    color: [1.0, 0.0, 0.0, 1.0],
                    ink: Ink::Rgb,
                },
                path: rect(10.0, 10.0, 12.0, 12.0),
            },
            Op::PopLayer,
        ];
        assert_eq!(image_width(&ops, 72.0), Some(18));
        assert_eq!(image_width(&ops, 144.0), Some(36));
        assert_eq!(image_width(&ops[2..3], 72.0), None);
    }

    fn page(paints: Vec<Paint>) -> String {
        let mut ops = vec![Op::Page {
            width: 100.0,
            height: 100.0,
            bleed: 0.0,
        }];
        ops.extend(paints.into_iter().map(|paint| Op::FillPath {
            paint,
            path: rect(10.0, 10.0, 10.0, 10.0),
        }));
        String::from_utf8_lossy(&super::pdf(&[ops], 72.0, ColorMode::Rgb)).into_owned()
    }

    #[test]
    fn spot_colours_are_separations_with_their_cmyk_alternate() {
        let pdf = page(vec![Paint::Solid {
            color: [0.0, 0.5, 0.8, 1.0],
            ink: Ink::Spot {
                name: "HKS 43".into(),
                cmyk: [1.0, 0.6, 0.0, 0.0],
                tint: 0.5,
            },
        }]);
        assert!(pdf.contains("/Separation/HKS#2043/DeviceCMYK"), "{pdf}");
    }

    #[test]
    fn a_gradient_mixing_rgb_and_cmyk_stops_is_written_in_cmyk() {
        let stop = |at, ink| Stop {
            at,
            color: [1.0, 0.0, 0.0, 1.0],
            ink,
        };
        let pdf = page(vec![Paint::Linear {
            transform: [10.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            stops: vec![
                stop(0.0, Ink::Rgb),
                stop(1.0, Ink::Cmyk([0.0, 1.0, 1.0, 0.0])),
            ],
        }]);
        assert!(pdf.contains("/ColorSpace/DeviceCMYK"), "{pdf}");
    }

    fn shadowed() -> Vec<Op> {
        let black = Ink::Cmyk([0.0, 0.0, 0.0, 1.0]);
        vec![
            Op::Page {
                width: 100.0,
                height: 100.0,
                bleed: 0.0,
            },
            Op::PushLayer {
                opacity: 1.0,
                blend: 0,
                blur: 0.0,
                shadows: vec![Shadow {
                    offset: [2.0, 2.0],
                    blur: 1.0,
                    color: [0.0, 0.0, 0.0, 0.5],
                    ink: black.clone(),
                }],
            },
            Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                    ink: black,
                },
                path: rect(10.0, 10.0, 12.0, 12.0),
            },
            Op::PopLayer,
        ]
    }

    fn image_dict(mode: ColorMode) -> String {
        let pdf = String::from_utf8_lossy(&super::pdf(&[shadowed()], 72.0, mode)).into_owned();
        pdf.match_indices("/Subtype/Image")
            .map(|(at, _)| &pdf[pdf[..at].rfind("<<").unwrap()..at + pdf[at..].find(">>").unwrap()])
            .find(|d| d.contains("/SMask"))
            .unwrap()
            .to_string()
    }

    #[test]
    fn shadows_are_rasterized_in_the_documents_colour_mode() {
        let cmyk = image_dict(ColorMode::Cmyk);
        assert!(cmyk.contains("/ColorSpace/DeviceCMYK"), "{cmyk}");
        let rgb = image_dict(ColorMode::Rgb);
        assert!(rgb.contains("/ColorSpace/DeviceRGB"), "{rgb}");
    }
}
