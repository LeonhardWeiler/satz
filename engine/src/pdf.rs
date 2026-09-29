use crate::color::{ColorMode, Ink};
use crate::display_list::{
    CLOSE, CUBIC, LINE, MOVE, Op, Paint, Shadow, Stop as ListStop, close, recolor,
};
use crate::image::{self, plate};
use crate::pdfx;
use crate::raster::{blur, extent, rasterize, tint};
use crate::text::{MISSING, font_bytes, fonts};
use krilla::blend::BlendMode;
use krilla::color::separation::{SeparationColorant, SeparationSpace};
use krilla::color::{cmyk, rgb, separation};
use krilla::configure::{ConfigurationBuilder, PdfVersion};
use krilla::geom::{Path, PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::mask::{Mask, MaskType};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, RadialGradient, SpreadMethod, Stop, Stroke,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use krilla::{Document, SerializeSettings};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use tiny_skia::Pixmap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    /// RGB as the canvas shows it, the trim box only.
    Screen,
    X4,
    /// CMYK and spots only, transparency flattened at the raster ppi.
    X1a,
}

const MM: f32 = 72.0 / 25.4;
const MARK_SPACE: f32 = 10.0 * MM;
const MARK_OFFSET: f32 = 3.0 * MM;
const MARK_LENGTH: f32 = 5.0 * MM;
const MARK_WIDTH: f32 = 0.25;

/// How a document exports to PDF.
pub struct Export<'a> {
    pub preset: Preset,
    pub crop_marks: bool,
    pub bleed: bool,
    /// Shadows, blurs and, for PDF/X-1a, transparency are rasterized at `ppi`.
    pub ppi: f32,
    pub mode: ColorMode,
    pub title: &'a str,
    /// ISO 8601 in UTC, as `2026-09-28T12:00:00Z`.
    pub date: &'a str,
}

pub fn pdf(pages: &[Vec<Op>], x: &Export) -> Vec<u8> {
    let fonts: Vec<Font> = (0..fonts().len() as u32)
        .map(|i| Font::new(font_bytes(i).to_vec().into(), 0).unwrap())
        .collect();
    let print = x.preset != Preset::Screen;
    let version = match x.preset {
        Preset::Screen => PdfVersion::Pdf17,
        Preset::X4 => PdfVersion::Pdf16,
        Preset::X1a => PdfVersion::Pdf14,
    };
    let mut doc = Document::new_with(SerializeSettings {
        no_device_cs: x.preset == Preset::X4,
        xmp_metadata: !print,
        configuration: ConfigurationBuilder::new()
            .with_version(version)
            .finish()
            .unwrap(),
        ..SerializeSettings::default()
    });
    for ops in pages {
        let Some(&Op::Page {
            width,
            height,
            bleed,
        }) = ops.first()
        else {
            continue;
        };
        let bleed = if print && x.bleed { bleed } else { 0.0 };
        let slug = if print && x.crop_marks {
            MARK_SPACE
        } else {
            0.0
        };
        let o = bleed + slug;
        let mut settings = PageSettings::from_wh(width + 2.0 * o, height + 2.0 * o)
            .unwrap()
            .with_trim_box(Rect::from_xywh(o, o, width, height));
        if print {
            settings = settings.with_bleed_box(Rect::from_xywh(
                slug,
                slug,
                width + 2.0 * bleed,
                height + 2.0 * bleed,
            ));
        }
        let mut page = doc.start_page_with(settings);
        let mut s = page.surface();
        s.push_transform(&Transform::from_translate(o, o));
        let area = [-bleed, -bleed, width + 2.0 * bleed, height + 2.0 * bleed];
        s.push_clip_path(
            &rect(area[0], area[1], area[2], area[3]),
            &FillRule::NonZero,
        );
        let env = Env {
            fonts: &fonts,
            ppi: x.ppi,
            cmyk: match x.preset {
                Preset::Screen => false,
                Preset::X4 => x.mode == ColorMode::Cmyk,
                Preset::X1a => true,
            },
            preset: x.preset,
            proof: !print && x.mode == ColorMode::Cmyk,
            page: area,
        };
        if x.preset == Preset::X1a {
            let (opaque, areas) = flatten(&ops[1..]);
            draw(&mut s, &env, &opaque);
            for a in areas {
                paper(&mut s, &env, &ops[1..], a);
            }
        } else {
            draw(&mut s, &env, &ops[1..]);
        }
        s.pop();
        if slug > 0.0 {
            crop_marks(&mut s, width, height);
        }
        s.pop();
        s.finish();
        page.finish();
    }
    let pdf = doc.finish().unwrap();
    match print {
        true => pdfx::pdfx(pdf, x.preset, x.mode == ColorMode::Rgb, x.title, x.date),
        false => pdf,
    }
}

/// `ops` without what is transparent, and the areas that it covers.
fn flatten(ops: &[Op]) -> (Vec<Op>, Vec<[f32; 4]>) {
    let translucent = |p: &Paint| match p {
        Paint::Solid { color, .. } => color[3] < 1.0,
        Paint::Linear { stops, .. } | Paint::Radial { stops, .. } => {
            stops.iter().any(|s| s.color[3] < 1.0)
        }
    };
    let (mut opaque, mut areas) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < ops.len() {
        let end = match &ops[i] {
            Op::PushLayer {
                opacity,
                blend,
                blur,
                shadows,
            } if *opacity < 1.0 || *blend != 0 || *blur > 0.0 || !shadows.is_empty() => {
                close(ops, i)
            }
            Op::BeginMask => close(ops, close(ops, i)),
            Op::FillPath { paint, .. }
            | Op::StrokePath { paint, .. }
            | Op::GlyphRun { paint, .. }
                if translucent(paint) =>
            {
                i
            }
            Op::Image { image, .. }
                if image::cmyk(*image).is_some_and(|c| c.alpha.iter().any(|&a| a < 255)) =>
            {
                i
            }
            op => {
                opaque.push(op.clone());
                i += 1;
                continue;
            }
        };
        let group = &ops[i..=end.min(ops.len() - 1)];
        let reach = group
            .iter()
            .map(|op| match op {
                Op::PushLayer { blur, shadows, .. } => shadows
                    .iter()
                    .map(|s| s.offset[0].abs().max(s.offset[1].abs()) + 3.0 * s.blur)
                    .fold(3.0 * blur, f32::max),
                _ => 0.0,
            })
            .fold(0.0, f32::max);
        areas.extend(
            extent(group)
                .map(|[x, y, w, h]| [x - reach, y - reach, w + 2.0 * reach, h + 2.0 * reach]),
        );
        i = end + 1;
    }
    (opaque, areas)
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
    fonts: &'a [Font],
    ppi: f32,
    /// Images and rasterized effects are CMYK.
    cmyk: bool,
    preset: Preset,
    /// Images are drawn as the canvas shows them in a CMYK document.
    proof: bool,
    page: [f32; 4],
}

fn draw(s: &mut Surface, env: &Env, ops: &[Op]) {
    let mut pops = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        match &ops[i] {
            Op::FillPath { paint, path } => {
                if let Some(p) = build(path) {
                    s.set_fill(Some(fill(paint, env.preset)));
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
                    let (paint, opacity) = convert(paint, env.preset);
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
                font,
                size,
                paint,
                glyphs,
                positions,
                text,
                ranges,
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
                s.set_fill(Some(fill(paint, env.preset)));
                s.draw_glyphs(
                    Point::from_xy(x0, y0),
                    &run,
                    env.fonts[(font & !MISSING) as usize].clone(),
                    text,
                    *size,
                    false,
                );
            }
            Op::Image { image, transform } => {
                let id = if env.proof {
                    image | image::PROOF
                } else {
                    *image
                };
                if let Some(img) = image::pdf(id, env.cmyk) {
                    let [a, b, c, d, e, f] = *transform;
                    s.push_transform(&Transform::from_row(a, b, c, d, e, f));
                    s.draw_image(img, Size::from_wh(1.0, 1.0).unwrap());
                    s.pop();
                }
            }
            Op::PushClip { path, invert } => {
                let mut pb = PathBuilder::new();
                if *invert {
                    let [x, y, w, h] = env.page;
                    pb.push_rect(Rect::from_xywh(x, y, w, h).unwrap());
                }
                append(&mut pb, path);
                let rule = if *invert {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                };
                match pb.finish() {
                    Some(p) => {
                        s.push_clip_path(&p, &rule);
                        pops.push(1);
                    }
                    None => i = close(ops, i),
                }
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
            Op::PushTransform {
                transform: [a, b, c, d, e, f],
            } => {
                s.push_transform(&Transform::from_row(*a, *b, *c, *d, *e, *f));
                pops.push(1);
            }
            Op::PopClip | Op::PopLayer | Op::PopMask | Op::PopTransform => {
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
    let area = [
        x + offset[0] - m,
        y + offset[1] - m,
        w + 2.0 * m,
        h + 2.0 * m,
    ];
    let Some([l, t, w, h]) = within(env.page, area) else {
        return;
    };
    let rect = [l - offset[0], t - offset[1], w, h];
    let image = if env.cmyk {
        separate(ops, rect, env.ppi, shadow, sigma, false)
            .map(|c| Image::from_custom(c, true).unwrap())
    } else {
        plane(
            ops,
            rect,
            env.ppi,
            &image::pixmap,
            shadow.map(|s| s.color),
            sigma,
        )
        .map(|px| {
            let (w, h) = (px.width(), px.height());
            Image::from_rgba8(px.take_demultiplied(), w, h)
        })
    };
    if let Some(image) = image {
        place(s, image, l, t, env.ppi);
    }
}

/// Draws `ops` over `area` as one opaque CMYK image, as they print on white paper.
fn paper(s: &mut Surface, env: &Env, ops: &[Op], area: [f32; 4]) {
    let Some([x, y, w, h]) = within(env.page, area) else {
        return;
    };
    let mut all = vec![Op::FillPath {
        paint: Paint::Solid {
            color: [1.0; 4],
            ink: Ink::Cmyk([0.0; 4]),
        },
        path: crate::display_list::rect(x, y, w, h),
    }];
    all.extend_from_slice(ops);
    if let Some(c) = separate(&all, [x, y, w, h], env.ppi, None, 0.0, true) {
        place(s, Image::from_custom(c, true).unwrap(), x, y, env.ppi);
    }
}

/// The part of `area` on `page`, both as [x, y, w, h].
fn within(page: [f32; 4], [x, y, w, h]: [f32; 4]) -> Option<[f32; 4]> {
    let (l, t) = (x.max(page[0]), y.max(page[1]));
    let (r, b) = (
        (x + w).min(page[0] + page[2]),
        (y + h).min(page[1] + page[3]),
    );
    (l < r && t < b).then_some([l, t, r - l, b - t])
}

/// Draws `image`, rasterized at `ppi`, with its top left at (`x`, `y`).
fn place(s: &mut Surface, image: Image, x: f32, y: f32, ppi: f32) {
    let scale = 72.0 / ppi;
    let (pw, ph) = image.size();
    s.push_transform(&Transform::from_translate(x, y));
    s.draw_image(
        image,
        Size::from_wh(pw as f32 * scale, ph as f32 * scale).unwrap(),
    );
    s.pop();
}

/// `ops` over `rect` at `ppi`, tinted with `tint_with` and blurred by `sigma` pt.
fn plane(
    ops: &[Op],
    rect: [f32; 4],
    ppi: f32,
    images: &dyn Fn(u32) -> Option<Rc<Pixmap>>,
    tint_with: Option<[f32; 4]>,
    sigma: f32,
) -> Option<Pixmap> {
    let mut px = rasterize(ops, rect, ppi, images)?;
    if let Some(c) = tint_with {
        tint(&mut px, c);
    }
    blur(&mut px, sigma * ppi / 72.0);
    Some(px)
}

/// `ops` over `rect` at `ppi` in CMYK, drawn plate by plate, tinted with the colour
/// of `shadow` and blurred by `sigma` pt. With `paper` each plate is drawn as white
/// less its ink, so that it blends as ink does on paper.
fn separate(
    ops: &[Op],
    rect: [f32; 4],
    ppi: f32,
    shadow: Option<&Shadow>,
    sigma: f32,
    paper: bool,
) -> Option<image::Cmyk> {
    let layer = |pick: fn([f32; 4]) -> [f32; 3]| {
        let to = |rgba: &[f32; 4], ink: &Ink| {
            let [a, b, c] = pick(ink.cmyk(rgba));
            [a, b, c, rgba[3]]
        };
        let images = |id| image::cmyk(id).and_then(|c| plate(&c, pick)).map(Rc::new);
        let tint_with = shadow.map(|s| to(&s.color, &s.ink));
        plane(&recolor(ops, &to), rect, ppi, &images, tint_with, sigma)
    };
    let (cmy, k) = if paper {
        (
            layer(|c| [1.0 - c[0], 1.0 - c[1], 1.0 - c[2]])?,
            layer(|c| [1.0 - c[3], 1.0, 1.0])?,
        )
    } else {
        (layer(|c| [c[0], c[1], c[2]])?, layer(|c| [c[3], 0.0, 0.0])?)
    };
    let size = (cmy.width(), cmy.height());
    let (cmy, k) = (cmy.take_demultiplied(), k.take_demultiplied());
    let ink = |v: u8| if paper { 255 - v } else { v };
    Some(image::Cmyk {
        color: cmy
            .chunks(4)
            .zip(k.chunks(4))
            .flat_map(|(a, b)| [a[0], a[1], a[2], b[0]].map(ink))
            .collect(),
        alpha: cmy.chunks(4).map(|a| a[3]).collect(),
        size,
    })
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

fn fill(p: &Paint, preset: Preset) -> Fill {
    let (paint, opacity) = convert(p, preset);
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

fn color(rgba: &[f32; 4], ink: &Ink, preset: Preset) -> krilla::color::Color {
    match (preset, ink) {
        (Preset::Screen, _) | (Preset::X4, Ink::Rgb) => {
            rgb::Color::new(byte(rgba[0]), byte(rgba[1]), byte(rgba[2])).into()
        }
        (Preset::X1a, Ink::Rgb) => process(ink.cmyk(rgba)).into(),
        (_, Ink::Cmyk(c)) => process(*c).into(),
        (_, Ink::Spot { name, cmyk, tint }) => separation::Color::new(
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

fn convert(p: &Paint, preset: Preset) -> (krilla::paint::Paint, NormalizedF32) {
    let (transform, stops) = match p {
        Paint::Solid { color: c, ink } => return (color(c, ink, preset).into(), opacity(c)),
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
                    color: color(&s.color, &ink, preset),
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
    use super::{Export, Preset};
    use crate::Doc;
    use crate::color::{ColorMode, Ink};
    use crate::display_list::{Op, Paint, Shadow, Stop, rect};

    fn export(pages: &[Vec<Op>], preset: Preset, marks: bool, mode: ColorMode) -> Vec<u8> {
        let x = Export {
            preset,
            crop_marks: marks,
            bleed: marks,
            ppi: 72.0,
            mode,
            title: "Test <1>",
            date: "2026-09-28T12:00:00Z",
        };
        super::pdf(pages, &x)
    }

    fn write(pages: &[Vec<Op>], preset: Preset, ppi: f32, mode: ColorMode) -> String {
        let x = Export {
            preset,
            crop_marks: true,
            bleed: true,
            ppi,
            mode,
            title: "Test <1>",
            date: "2026-09-28T12:00:00Z",
        };
        String::from_utf8_lossy(&super::pdf(pages, &x)).into_owned()
    }

    fn default_pdf() -> String {
        let d = Doc::new();
        let page = d.render(&d.snapshot().pages[0].id);
        write(&[page], Preset::X4, 300.0, ColorMode::Rgb)
    }

    fn page_box(pdf: &str, name: &str) -> Vec<f32> {
        let at = pdf.find(&format!("/{name}")).unwrap();
        let at = at + pdf[at..].find('[').unwrap() + 1;
        let end = at + pdf[at..].find(']').unwrap();
        pdf[at..end]
            .split_whitespace()
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
        let pdf = write(&[ops.to_vec()], Preset::X4, ppi, ColorMode::Rgb);
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
    fn an_image_is_embedded_at_its_own_resolution() {
        let png = crate::image::tests::png(40, 20);
        let hash = crate::image::register(png.into()).unwrap().hash;
        let ops = vec![
            Op::Page {
                width: 100.0,
                height: 100.0,
                bleed: 0.0,
            },
            Op::Image {
                image: crate::image::id(&hash).unwrap(),
                transform: [50.0, 0.0, 0.0, 25.0, 10.0, 10.0],
            },
        ];
        assert_eq!(image_width(&ops, 72.0), Some(40));
    }

    #[test]
    fn a_clip_to_a_single_point_exports() {
        let ops = vec![
            Op::Page {
                width: 100.0,
                height: 100.0,
                bleed: 0.0,
            },
            Op::PushClip {
                path: vec![crate::display_list::MOVE, 0.5, 0.5],
                invert: false,
            },
            Op::FillPath {
                paint: Paint::Solid {
                    color: [1.0, 0.0, 0.0, 1.0],
                    ink: Ink::Rgb,
                },
                path: rect(0.0, 0.0, 10.0, 10.0),
            },
            Op::PopClip,
        ];
        write(&[ops], Preset::X4, 72.0, ColorMode::Rgb);
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
        write(&[ops], Preset::X4, 72.0, ColorMode::Rgb)
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

    fn image_dict(preset: Preset, mode: ColorMode) -> String {
        let pdf = write(&[shadowed()], preset, 72.0, mode);
        pdf.match_indices("/Subtype/Image")
            .map(|(at, _)| &pdf[pdf[..at].rfind("<<").unwrap()..at + pdf[at..].find(">>").unwrap()])
            .find(|d| d.contains("/SMask"))
            .unwrap()
            .to_string()
    }

    #[test]
    fn shadows_are_rasterized_in_the_documents_colour_mode() {
        let cmyk = image_dict(Preset::X4, ColorMode::Cmyk);
        assert!(cmyk.contains("/ColorSpace/DeviceCMYK"), "{cmyk}");
        let rgb = image_dict(Preset::Screen, ColorMode::Rgb);
        assert!(rgb.contains("/ColorSpace/DeviceRGB"), "{rgb}");
    }

    fn red_image() -> Vec<Op> {
        let hash = crate::image::register(crate::image::tests::png(4, 4).into())
            .unwrap()
            .hash;
        vec![
            Op::Page {
                width: 100.0,
                height: 100.0,
                bleed: 0.0,
            },
            Op::Image {
                image: crate::image::id(&hash).unwrap(),
                transform: [10.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            },
        ]
    }

    #[test]
    fn images_in_a_cmyk_document_are_separated() {
        let pdf = |mode| write(&[red_image()], Preset::X4, 72.0, mode);
        assert!(pdf(ColorMode::Cmyk).contains("/ColorSpace/DeviceCMYK"));
        assert!(!pdf(ColorMode::Cmyk).contains("/DeviceRGB"));
        assert!(pdf(ColorMode::Rgb).contains("/ICCBased"));
    }

    #[test]
    fn a_blurred_image_rasterizes_its_cmyk_plates() {
        let ops = red_image();
        let cmy = |id| {
            crate::image::cmyk(id)
                .and_then(|c| super::plate(&c, |c| [c[0], c[1], c[2]]))
                .map(std::rc::Rc::new)
        };
        let px = crate::raster::rasterize(&ops[1..], [10.0, 10.0, 10.0, 10.0], 72.0, &cmy).unwrap();
        let p = px.pixel(5, 5).unwrap();
        assert!(p.red() < 20 && p.green() > 200 && p.blue() > 200, "{p:?}");
    }

    /// `pdf` with its streams decompressed by MuPDF, which must read it without
    /// repairs, and its whitespace collapsed to single spaces.
    fn clean(pdf: &[u8]) -> String {
        let dir = std::env::temp_dir();
        let name = |s| {
            dir.join(format!(
                "satz-{s}-{:x}.pdf",
                pdf.len() ^ std::process::id() as usize
            ))
        };
        let (from, to) = (name("in"), name("out"));
        std::fs::write(&from, pdf).unwrap();
        let out = std::process::Command::new("mutool")
            .args(["clean", "-d"])
            .arg(&from)
            .arg(&to)
            .output()
            .expect("mutool runs");
        assert!(
            out.status.success() && out.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&std::fs::read(&to).unwrap())
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        std::fs::write("/tmp/satz-clean.pdf", &text).unwrap();
        std::fs::remove_file(from).unwrap();
        std::fs::remove_file(to).unwrap();
        text
    }

    /// A CMYK page with an RGB fill, a translucent fill, a shadow and an image.
    fn busy() -> Vec<Op> {
        let mut ops = shadowed();
        ops.extend(red_image().into_iter().skip(1));
        ops.extend([
            Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0, 0.5, 1.0, 1.0],
                    ink: Ink::Rgb,
                },
                path: rect(40.0, 40.0, 10.0, 10.0),
            },
            Op::FillPath {
                paint: Paint::Solid {
                    color: [1.0, 0.0, 0.0, 0.5],
                    ink: Ink::Cmyk([0.0, 1.0, 1.0, 0.0]),
                },
                path: rect(60.0, 60.0, 10.0, 10.0),
            },
            Op::PushClip {
                path: rect(10.0, 10.0, 5.0, 5.0),
                invert: true,
            },
            Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                    ink: Ink::Cmyk([0.0, 0.0, 0.0, 1.0]),
                },
                path: rect(5.0, 5.0, 15.0, 15.0),
            },
            Op::PopClip,
        ]);
        ops
    }

    fn assert_pdfx(pdf: &str) {
        for key in [
            "/Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (FOGRA51)",
            "/DestOutputProfile",
            "/Trapped /False",
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Test &lt;1&gt;</rdf:li>",
            "<xmp:CreateDate>2026-09-28T12:00:00Z</xmp:CreateDate>",
            "<xmpMM:DocumentID>xmp.did:",
            "/CreationDate (D:20260928120000Z00'00')",
            "/TrimBox",
            "/BleedBox",
        ] {
            assert!(pdf.contains(key), "{key}");
        }
    }

    #[test]
    fn pdf_x4_has_the_fogra51_output_intent_and_keeps_transparency() {
        let pdf = clean(&export(&[busy()], Preset::X4, true, ColorMode::Cmyk));
        assert!(pdf.starts_with("%PDF-1.6"));
        assert_pdfx(&pdf);
        assert!(pdf.contains("<pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>"));
        assert!(pdf.contains("/GTS_PDFXVersion (PDF/X-4)"));
        assert!(pdf.contains("/ICCBased"));
        assert!(!pdf.contains("/DeviceRGB"));
        assert!(pdf.contains("/SMask"));
        assert!(pdf.contains("/ca .5"));
    }

    #[test]
    fn pdf_x1a_is_cmyk_and_opaque() {
        let pdf = clean(&export(&[busy()], Preset::X1a, true, ColorMode::Cmyk));
        assert!(pdf.starts_with("%PDF-1.4"));
        assert_pdfx(&pdf);
        assert!(pdf.contains("/GTS_PDFXConformance (PDF/X-1a:2003)"));
        assert!(pdf.contains("<pdfx:GTS_PDFXConformance>PDF/X-1a:2003</pdfx:GTS_PDFXConformance>"));
        assert!(pdf.contains("/ColorSpace /DeviceCMYK"));
        for no in [
            "/DeviceRGB",
            "/ICCBased",
            "/SMask",
            "/Transparency",
            "/ca ",
            "/CA ",
            "/BM",
            " rg ",
            " RG ",
            "/Pattern",
        ] {
            assert!(!pdf.contains(no), "{no}");
        }
    }

    #[test]
    fn the_pages_of_an_rgb_pdf_x4_blend_in_srgb() {
        let group = "/Group << /Type /Group /S /Transparency /CS [ /ICCBased";
        let rgb = clean(&export(&[busy(), busy()], Preset::X4, true, ColorMode::Rgb));
        assert_eq!(rgb.matches(group).count(), 2);
        let cmyk = clean(&export(&[busy()], Preset::X4, true, ColorMode::Cmyk));
        assert!(!cmyk.contains(group));
    }

    #[test]
    fn a_screen_pdf_is_rgb_and_shows_the_trim_box_only() {
        let pdf = clean(&export(&[busy()], Preset::Screen, true, ColorMode::Cmyk));
        for no in [
            "/DeviceCMYK",
            "/Separation",
            "/OutputIntents",
            "/BleedBox",
            "/All",
        ] {
            assert!(!pdf.contains(no), "{no}");
        }
        assert_close(&page_box(&pdf, "MediaBox"), &page_box(&pdf, "TrimBox"));
    }

    #[test]
    fn without_crop_marks_and_bleed_the_media_box_is_the_trim_box() {
        let pdf = clean(&export(&[busy()], Preset::X4, false, ColorMode::Cmyk));
        assert_close(&page_box(&pdf, "MediaBox"), &page_box(&pdf, "TrimBox"));
        assert_close(&page_box(&pdf, "BleedBox"), &page_box(&pdf, "TrimBox"));
        assert!(!pdf.contains("/All"));
    }
}
