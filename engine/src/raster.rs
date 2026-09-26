use crate::display_list::{CLOSE, CUBIC, LINE, MOVE, Op, Paint as ListPaint, close};
use crate::geom;
use crate::image;
use crate::text::font_bytes;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient,
    Mask, MaskType, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Point, RadialGradient, Shader,
    SpreadMode, Stroke, Transform,
};

const BLENDS: [BlendMode; 16] = [
    BlendMode::SourceOver,
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

/// Renders `ops` into a pixmap that covers `rect` in pt at `ppi`.
pub fn rasterize(ops: &[Op], [x, y, w, h]: [f32; 4], ppi: f32) -> Option<Pixmap> {
    let scale = ppi / 72.0;
    let mut px = Pixmap::new(
        (w * scale).ceil().max(1.0) as u32,
        (h * scale).ceil().max(1.0) as u32,
    )?;
    let t = Transform::from_scale(scale, scale).pre_translate(-x, -y);
    render(&mut px, ops, t, None);
    Some(px)
}

/// Content bounds of `ops` in pt, generous for strokes and glyphs.
pub fn extent(ops: &[Op]) -> Option<[f32; 4]> {
    let mut b = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut add = |[x, y, w, h]: [f32; 4], pad: f32| {
        b = [
            b[0].min(x - pad),
            b[1].min(y - pad),
            b[2].max(x + w + pad),
            b[3].max(y + h + pad),
        ];
    };
    for op in ops {
        match op {
            Op::FillPath { path, .. } => add(geom::bounds(path), 0.0),
            Op::StrokePath { path, width, .. } => add(geom::bounds(path), 2.0 * width),
            Op::GlyphRun {
                positions, size, ..
            } => {
                for p in positions.chunks(2) {
                    add([p[0], p[1], 0.0, 0.0], *size);
                }
            }
            Op::Image {
                transform: [a, b, c, d, e, f],
                ..
            } => {
                let corners = [
                    MOVE,
                    *e,
                    *f,
                    LINE,
                    e + a,
                    f + b,
                    LINE,
                    e + a + c,
                    f + b + d,
                    LINE,
                    e + c,
                    f + d,
                ];
                add(geom::bounds(&corners), 0.0);
            }
            _ => {}
        }
    }
    (b[0] <= b[2]).then(|| [b[0], b[1], b[2] - b[0], b[3] - b[1]])
}

/// Gaussian blur of premultiplied pixels, approximated by three box blurs.
pub fn blur(px: &mut Pixmap, sigma: f32) {
    if sigma <= 0.0 {
        return;
    }
    let (w, h) = (px.width() as usize, px.height() as usize);
    let data = px.data_mut();
    let ideal = (12.0 * sigma * sigma / 3.0 + 1.0).sqrt();
    let mut lo = ideal.floor() as i32;
    if lo % 2 == 0 {
        lo -= 1;
    }
    let lof = lo as f32;
    let m = ((12.0 * sigma * sigma - 3.0 * lof * lof - 12.0 * lof - 9.0) / (-4.0 * lof - 4.0))
        .round() as i32;
    let mut tmp = vec![0u8; data.len()];
    for pass in 0..3 {
        let r = (if pass < m { lo } else { lo + 2 } - 1) as usize / 2;
        box_blur(data, &mut tmp, w, h, r, true);
        box_blur(&tmp, data, w, h, r, false);
    }
}

fn box_blur(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len) = if horizontal { (h, w) } else { (w, h) };
    let at = |line: usize, i: usize| {
        if horizontal {
            (line * w + i) * 4
        } else {
            (i * w + line) * 4
        }
    };
    let n = (2 * r + 1) as u32;
    for line in 0..lines {
        for c in 0..4 {
            let mut sum: u32 = (0..r.min(len)).map(|i| src[at(line, i) + c] as u32).sum();
            for i in 0..len {
                if i + r < len {
                    sum += src[at(line, i + r) + c] as u32;
                }
                dst[at(line, i) + c] = ((sum + n / 2) / n) as u8;
                if i >= r {
                    sum -= src[at(line, i - r) + c] as u32;
                }
            }
        }
    }
}

/// Replaces the colour of every pixel with `color`, keeping coverage.
pub fn tint(px: &mut Pixmap, color: [f32; 4]) {
    for p in px.data_mut().chunks_mut(4) {
        let a = p[3] as f32 / 255.0 * color[3];
        for c in 0..3 {
            p[c] = (color[c] * a * 255.0).round() as u8;
        }
        p[3] = (a * 255.0).round() as u8;
    }
}

fn render(px: &mut Pixmap, ops: &[Op], t: Transform, clip: Option<&Mask>) {
    let mut i = 0;
    while i < ops.len() {
        match &ops[i] {
            Op::FillPath { paint, path } => {
                if let (Some(p), Some(paint)) = (build(path), convert(paint)) {
                    px.fill_path(&p, &paint, FillRule::Winding, t, clip);
                }
            }
            Op::StrokePath {
                paint,
                width,
                cap,
                join,
                path,
            } => {
                if let (Some(p), Some(paint)) = (build(path), convert(paint)) {
                    let stroke = Stroke {
                        width: *width,
                        miter_limit: 4.0,
                        line_cap: [LineCap::Butt, LineCap::Round, LineCap::Square][*cap as usize],
                        line_join: [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel]
                            [*join as usize],
                        dash: None,
                    };
                    px.stroke_path(&p, &paint, &stroke, t, clip);
                }
            }
            Op::GlyphRun {
                font,
                size,
                paint,
                glyphs,
                positions,
                ..
            } => {
                if let (Some(p), Some(paint)) =
                    (glyph_path(*font, *size, glyphs, positions), convert(paint))
                {
                    px.fill_path(&p, &paint, FillRule::Winding, t, clip);
                }
            }
            Op::Image { image, transform } => {
                if let Some(pm) = image::pixmap(*image) {
                    let [a, b, c, d, e, f] = *transform;
                    let to = t
                        .pre_concat(Transform::from_row(a, b, c, d, e, f))
                        .pre_scale(1.0 / pm.width() as f32, 1.0 / pm.height() as f32);
                    let paint = PixmapPaint {
                        quality: FilterQuality::Bilinear,
                        ..PixmapPaint::default()
                    };
                    px.draw_pixmap(0, 0, pm.as_ref().as_ref(), &paint, to, clip);
                }
            }
            Op::PushClip { path, invert } => {
                let end = close(ops, i);
                if let Some(p) = build(path) {
                    let mut m = full(px, clip);
                    if *invert {
                        let mut sub = Mask::new(px.width(), px.height()).unwrap();
                        sub.fill_path(&p, FillRule::Winding, true, t);
                        sub.invert();
                        multiply(&mut m, &sub);
                    } else {
                        m.intersect_path(&p, FillRule::Winding, true, t);
                    }
                    render(px, &ops[i + 1..end], t, Some(&m));
                }
                i = end;
            }
            Op::PushLayer {
                opacity,
                blend,
                blur: sigma,
                shadows,
            } => {
                let end = close(ops, i);
                let scale = t.sx;
                let mut content = layer(px);
                render(&mut content, &ops[i + 1..end], t, None);
                let mut out = layer(px);
                for s in shadows {
                    let mut shadow = content.clone();
                    tint(&mut shadow, s.color);
                    blur(&mut shadow, s.blur * scale);
                    let (dx, dy) = (s.offset[0] * scale, s.offset[1] * scale);
                    out.draw_pixmap(
                        0,
                        0,
                        shadow.as_ref(),
                        &PixmapPaint::default(),
                        Transform::from_translate(dx, dy),
                        None,
                    );
                }
                blur(&mut content, sigma * scale);
                out.draw_pixmap(
                    0,
                    0,
                    content.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
                let paint = PixmapPaint {
                    opacity: *opacity,
                    blend_mode: BLENDS[*blend as usize],
                    ..PixmapPaint::default()
                };
                px.draw_pixmap(0, 0, out.as_ref(), &paint, Transform::identity(), clip);
                i = end;
            }
            Op::BeginMask => {
                let mid = close(ops, i);
                let end = close(ops, mid);
                let mut alpha = layer(px);
                render(&mut alpha, &ops[i + 1..mid], t, None);
                let mut content = layer(px);
                render(&mut content, &ops[mid + 1..end], t, None);
                let mut m = Mask::from_pixmap(alpha.as_ref(), MaskType::Alpha);
                if let Some(c) = clip {
                    multiply(&mut m, c);
                }
                px.draw_pixmap(
                    0,
                    0,
                    content.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    Some(&m),
                );
                i = end;
            }
            _ => {}
        }
        i += 1;
    }
}

fn layer(px: &Pixmap) -> Pixmap {
    Pixmap::new(px.width(), px.height()).unwrap()
}

fn full(px: &Pixmap, clip: Option<&Mask>) -> Mask {
    clip.cloned().unwrap_or_else(|| {
        let mut m = Mask::new(px.width(), px.height()).unwrap();
        m.invert();
        m
    })
}

fn multiply(m: &mut Mask, by: &Mask) {
    for (a, b) in m.data_mut().iter_mut().zip(by.data()) {
        *a = ((*a as u32 * *b as u32 + 127) / 255) as u8;
    }
}

fn color(c: &[f32; 4]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], c[3]).unwrap_or(Color::TRANSPARENT)
}

fn convert(p: &ListPaint) -> Option<Paint<'static>> {
    let shader = match p {
        ListPaint::Solid { color: c, .. } => Shader::SolidColor(color(c)),
        ListPaint::Linear { transform, stops } | ListPaint::Radial { transform, stops } => {
            let [a, b, c, d, e, f] = *transform;
            let t = Transform::from_row(a, b, c, d, e, f);
            let stops = stops
                .iter()
                .map(|s| GradientStop::new(s.at, color(&s.color)))
                .collect();
            let zero = Point::from_xy(0.0, 0.0);
            match p {
                ListPaint::Linear { .. } => {
                    LinearGradient::new(zero, Point::from_xy(1.0, 0.0), stops, SpreadMode::Pad, t)?
                }
                _ => RadialGradient::new(zero, 0.0, zero, 1.0, stops, SpreadMode::Pad, t)?,
            }
        }
    };
    Some(Paint {
        shader,
        ..Paint::default()
    })
}

fn build(cmds: &[f32]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let mut i = 0;
    while i < cmds.len() {
        let c = &cmds[i..];
        match c[0] {
            MOVE => pb.move_to(c[1], c[2]),
            LINE => pb.line_to(c[1], c[2]),
            CUBIC => pb.cubic_to(c[1], c[2], c[3], c[4], c[5], c[6]),
            CLOSE => pb.close(),
            _ => return None,
        }
        i += match c[0] {
            CUBIC => 7,
            CLOSE => 1,
            _ => 3,
        };
    }
    pb.finish()
}

struct Pen<'a> {
    pb: &'a mut PathBuilder,
    origin: [f32; 2],
}

impl OutlinePen for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.pb.move_to(self.origin[0] + x, self.origin[1] - y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.pb.line_to(self.origin[0] + x, self.origin[1] - y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let [ox, oy] = self.origin;
        self.pb.quad_to(ox + cx, oy - cy, ox + x, oy - y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let [ox, oy] = self.origin;
        self.pb
            .cubic_to(ox + cx0, oy - cy0, ox + cx1, oy - cy1, ox + x, oy - y);
    }
    fn close(&mut self) {
        self.pb.close();
    }
}

fn glyph_path(font: u32, size: f32, glyphs: &[u16], positions: &[f32]) -> Option<Path> {
    let bytes = font_bytes(font);
    let font = FontRef::new(&bytes).ok()?;
    let outlines = font.outline_glyphs();
    let mut pb = PathBuilder::new();
    for (g, p) in glyphs.iter().zip(positions.chunks(2)) {
        if let Some(outline) = outlines.get(GlyphId::new(*g as u32)) {
            let settings = DrawSettings::unhinted(Size::new(size), LocationRef::default());
            let mut pen = Pen {
                pb: &mut pb,
                origin: [p[0], p[1]],
            };
            outline.draw(settings, &mut pen).ok()?;
        }
    }
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_list::{Shadow, rect};

    const BLACK: ListPaint = ListPaint::Solid {
        color: [0.0, 0.0, 0.0, 1.0],
        ink: crate::color::Ink::Rgb,
    };

    fn alpha(px: &Pixmap, x: u32, y: u32) -> u8 {
        px.pixel(x, y).unwrap().alpha()
    }

    #[test]
    fn rasterizes_a_rect_at_the_given_resolution() {
        let ops = [Op::FillPath {
            paint: BLACK,
            path: rect(1.0, 1.0, 2.0, 2.0),
        }];
        let px = rasterize(&ops, [0.0, 0.0, 4.0, 4.0], 144.0).unwrap();
        assert_eq!((px.width(), px.height()), (8, 8));
        assert_eq!(alpha(&px, 3, 3), 255);
        assert_eq!(alpha(&px, 1, 1), 0);
    }

    #[test]
    fn an_image_fills_the_box_its_transform_maps_it_into() {
        let png = crate::image::tests::png(2, 2);
        let hash = image::register(png.into()).unwrap().hash;
        let ops = [Op::Image {
            image: image::id(&hash).unwrap(),
            transform: [2.0, 0.0, 0.0, 2.0, 1.0, 1.0],
        }];
        assert_eq!(extent(&ops), Some([1.0, 1.0, 2.0, 2.0]));
        let px = rasterize(&ops, [0.0, 0.0, 4.0, 4.0], 144.0).unwrap();
        assert_eq!(px.pixel(4, 4).unwrap().red(), 255);
        assert_eq!(alpha(&px, 4, 4), 255);
        assert_eq!(alpha(&px, 1, 1), 0);
    }

    #[test]
    fn blur_spreads_coverage_and_keeps_its_sum() {
        let ops = [Op::FillPath {
            paint: BLACK,
            path: rect(10.0, 10.0, 2.0, 2.0),
        }];
        let mut px = rasterize(&ops, [0.0, 0.0, 22.0, 22.0], 72.0).unwrap();
        let sum = |px: &Pixmap| px.pixels().iter().map(|p| p.alpha() as u32).sum::<u32>();
        let before = sum(&px);
        blur(&mut px, 2.0);
        assert!(alpha(&px, 8, 11) > 0);
        assert!(alpha(&px, 11, 11) < 255);
        assert!(sum(&px).abs_diff(before) < before / 20);
    }

    #[test]
    fn a_layer_draws_its_shadow_below_the_content() {
        let ops = [
            Op::PushLayer {
                opacity: 1.0,
                blend: 0,
                blur: 0.0,
                shadows: vec![Shadow {
                    offset: [4.0, 0.0],
                    blur: 0.0,
                    color: [1.0, 0.0, 0.0, 1.0],
                    ink: crate::color::Ink::Rgb,
                }],
            },
            Op::FillPath {
                paint: BLACK,
                path: rect(0.0, 0.0, 4.0, 4.0),
            },
            Op::PopLayer,
        ];
        let px = rasterize(&ops, [0.0, 0.0, 8.0, 4.0], 72.0).unwrap();
        let p = px.pixel(6, 2).unwrap();
        assert_eq!((p.red(), p.alpha()), (255, 255));
        assert_eq!(px.pixel(2, 2).unwrap().red(), 0);
    }

    #[test]
    fn glyphs_are_rasterized_from_their_outlines() {
        let ops = [Op::GlyphRun {
            font: 0,
            size: 20.0,
            paint: BLACK,
            glyphs: vec![36],
            positions: vec![2.0, 18.0],
            text: String::new(),
            ranges: vec![],
        }];
        let px = rasterize(&ops, [0.0, 0.0, 20.0, 20.0], 72.0).unwrap();
        assert!(px.pixels().iter().any(|p| p.alpha() == 255));
    }

    #[test]
    fn extent_covers_paths_and_glyphs() {
        let ops = [
            Op::FillPath {
                paint: BLACK,
                path: rect(1.0, 2.0, 3.0, 4.0),
            },
            Op::GlyphRun {
                font: 0,
                size: 10.0,
                paint: BLACK,
                glyphs: vec![36],
                positions: vec![20.0, 20.0],
                text: String::new(),
                ranges: vec![],
            },
        ];
        assert_eq!(extent(&ops), Some([1.0, 2.0, 29.0, 28.0]));
        assert_eq!(extent(&[]), None);
    }
}
