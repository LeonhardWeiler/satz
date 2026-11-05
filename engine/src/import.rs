//! SVG, PDF and Figma files as Satz documents: a page per sheet with a frame of its layers.

use crate::color::{Color, ColorMode};
use crate::display_list::{CLOSE, CUBIC, LINE, MOVE};
use crate::doc::{Command, Doc, NewKind, Props};
use crate::geom::{bounds, fit, valid};
use crate::style::{Align, Blend, Cap, Fill, FillKind, FillStop, Join};
use hayro_interpret::font::Glyph;
use hayro_interpret::hayro_syntax::Pdf;
use hayro_interpret::kurbo::{Affine, BezPath, Rect, Shape as _};
use hayro_interpret::{
    BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageData, ImageDrawProps,
    InterpreterCache, InterpreterSettings, Paint, SoftMask, interpret_page,
};
use linesweeper::{BinaryOp, FillRule, binary_op};

type Res<T> = Result<T, String>;

/// A fill in page space: its gradient's transform maps the unit gradient onto the page.
pub struct Shape {
    pub path: Vec<f32>,
    pub fills: Vec<Fill>,
    pub strokes: Vec<Fill>,
    pub weight: f32,
    pub join: Join,
    pub cap: Cap,
    pub opacity: f32,
    pub blend: Blend,
}

pub enum Item {
    Shape(Shape),
    /// PNG or JPEG bytes in [x, y, w, h].
    Image(Vec<u8>, [f32; 4], [f32; 4]),
}

pub struct Sheet {
    pub name: Option<String>,
    pub width: f64,
    pub height: f64,
    pub items: Vec<Item>,
}

/// The file `name` as a document; `Err` when its type is not one Satz imports or it does not read.
pub fn import(bytes: &[u8], name: &str) -> Res<Vec<u8>> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let stem = name.strip_suffix(&format!(".{ext}")).unwrap_or(name);
    let sheets = match ext.as_str() {
        "svg" => vec![svg(bytes)?],
        "pdf" => pdf(bytes)?,
        "fig" => crate::fig::sheets(bytes)?,
        _ => return Err("only SVG, PDF and Figma files import".into()),
    };
    build(stem, sheets)
}

fn build(name: &str, sheets: Vec<Sheet>) -> Res<Vec<u8>> {
    let Some(first) = sheets.first() else {
        return Err("it has no pages".into());
    };
    let mut d = Doc::blank(
        first.width,
        first.height,
        sheets.len(),
        false,
        ColorMode::Rgb,
    );
    let pages: Vec<String> = d
        .build_snapshot()
        .pages
        .iter()
        .map(|p| p.id.clone())
        .collect();
    for (i, (sheet, page)) in sheets.into_iter().zip(pages).enumerate() {
        d.apply(Command::SetPage {
            id: page.clone(),
            width: Some(sheet.width),
            height: Some(sheet.height),
            bleed: Some(0.0),
            scale: false,
        })?;
        let title = if i == 0 {
            name.to_string()
        } else {
            format!("{name} {}", i + 1)
        };
        let frame = create(
            &mut d,
            &page,
            NewKind::Frame,
            [0.0, 0.0, sheet.width as f32, sheet.height as f32],
        )?;
        set(
            &mut d,
            &frame,
            Props {
                name: Some(sheet.name.unwrap_or(title)),
                fills: Some(vec![]),
                clip: Some(true),
                ..Props::default()
            },
        )?;
        for item in sheet.items {
            match item {
                Item::Shape(s) => shape(&mut d, &frame, s)?,
                Item::Image(bytes, [x, y, w, h], crop) => {
                    let Ok(info) = d.add_image(&bytes) else {
                        continue;
                    };
                    let id = d
                        .apply(Command::PlaceImage {
                            parent: frame.clone(),
                            image: info.hash,
                            name: "Image".into(),
                            x: x.into(),
                            y: y.into(),
                            w: w.into(),
                            h: h.into(),
                        })?
                        .remove(0);
                    if crop != [x, y, w, h] {
                        let [x, y, w, h] = crop.map(f64::from);
                        d.apply(Command::SetFrame {
                            id,
                            x,
                            y,
                            w,
                            h,
                            crop: true,
                        })?;
                    }
                }
            }
        }
    }
    Ok(d.save())
}

fn create(d: &mut Doc, parent: &str, kind: NewKind, [x, y, w, h]: [f32; 4]) -> Res<String> {
    Ok(d.apply(Command::Create {
        parent: parent.into(),
        kind,
        x: x.into(),
        y: y.into(),
        w: w.into(),
        h: h.into(),
    })?
    .remove(0))
}

fn set(d: &mut Doc, id: &str, props: Props) -> Res<()> {
    d.apply(Command::Set {
        id: id.into(),
        props,
    })
    .map(|_| ())
}

fn shape(d: &mut Doc, parent: &str, s: Shape) -> Res<()> {
    if !valid(&s.path) || s.path.is_empty() || s.fills.is_empty() && s.strokes.is_empty() {
        return Ok(());
    }
    let frame = bounds(&s.path);
    let local = |f: Vec<Fill>| f.into_iter().map(|f| unit(f, frame)).collect();
    let id = create(d, parent, NewKind::Path, frame)?;
    set(
        d,
        &id,
        Props {
            path: Some(fit(&s.path, [0.0, 0.0, 1.0, 1.0])),
            fills: Some(local(s.fills)),
            strokes: Some(local(s.strokes)),
            stroke_weight: Some(s.weight),
            stroke_align: Some(Align::Center),
            join: Some(s.join),
            cap: Some(s.cap),
            opacity: (s.opacity < 1.0).then_some(s.opacity),
            blend: (s.blend != Blend::Normal).then_some(s.blend),
            ..Props::default()
        },
    )
}

/// `f` with its gradient in the unit box of `frame`, or solid in its first color when `frame` has no area.
fn unit(mut f: Fill, [x, y, w, h]: [f32; 4]) -> Fill {
    if f.kind == FillKind::Solid {
        return f;
    }
    if w <= 0.0 || h <= 0.0 {
        let color = f.stops.first().map_or(f.color, |s| s.color.clone());
        return Fill::solid(color);
    }
    let [a, b, c, dd, e, ff] = f.transform;
    f.transform = [a / w, b / h, c / w, dd / h, (e - x) / w, (ff - y) / h];
    f
}

pub(crate) fn rgba([r, g, b, a]: [u8; 4]) -> Color {
    Color::Rgb(u32::from_be_bytes([r, g, b, a]))
}

pub(crate) fn gradient(kind: FillKind, transform: [f32; 6], stops: Vec<FillStop>) -> Fill {
    Fill {
        kind,
        transform,
        stops,
        ..Fill::default()
    }
}

pub(crate) fn then([a, b, c, d, e, f]: [f32; 6], [p, q, r, s, t, u]: [f32; 6]) -> [f32; 6] {
    [
        a * p + c * q,
        b * p + d * q,
        a * r + c * s,
        b * r + d * s,
        a * t + c * u + e,
        b * t + d * u + f,
    ]
}

fn scale([a, b, c, d, ..]: [f32; 6]) -> f32 {
    (a * d - b * c).abs().sqrt()
}

fn svg(bytes: &[u8]) -> Res<Sheet> {
    let tree =
        usvg::Tree::from_data(bytes, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    svg_group(tree.root(), 1.0, &mut items);
    let size = tree.size();
    Ok(Sheet {
        name: None,
        width: size.width().into(),
        height: size.height().into(),
        items,
    })
}

fn affine(t: usvg::Transform) -> [f32; 6] {
    [t.sx, t.ky, t.kx, t.sy, t.tx, t.ty]
}

fn svg_group(g: &usvg::Group, opacity: f32, out: &mut Vec<Item>) {
    for n in g.children() {
        match n {
            usvg::Node::Group(g) => svg_group(g, opacity * g.opacity().get(), out),
            usvg::Node::Path(p) if p.is_visible() => {
                let t = affine(p.abs_transform());
                let mut path = Vec::new();
                for s in p.data().segments() {
                    use usvg::tiny_skia_path::PathSegment as S;
                    let at = |p: usvg::tiny_skia_path::Point| {
                        let [a, b, c, d, e, f] = t;
                        [a * p.x + c * p.y + e, b * p.x + d * p.y + f]
                    };
                    match s {
                        S::MoveTo(p) => path.extend([MOVE, at(p)[0], at(p)[1]]),
                        S::LineTo(p) => path.extend([LINE, at(p)[0], at(p)[1]]),
                        S::QuadTo(q, p) => quad(&mut path, at(q), at(p)),
                        S::CubicTo(a, b, p) => path.extend([
                            CUBIC,
                            at(a)[0],
                            at(a)[1],
                            at(b)[0],
                            at(b)[1],
                            at(p)[0],
                            at(p)[1],
                        ]),
                        S::Close => path.push(CLOSE),
                    }
                }
                let paint = |p: &usvg::Paint, alpha: f32| svg_paint(p, alpha, t);
                let stroke = p.stroke();
                out.push(Item::Shape(Shape {
                    path,
                    fills: p
                        .fill()
                        .and_then(|f| paint(f.paint(), f.opacity().get()))
                        .into_iter()
                        .collect(),
                    strokes: stroke
                        .and_then(|s| paint(s.paint(), s.opacity().get()))
                        .into_iter()
                        .collect(),
                    weight: stroke.map_or(1.0, |s| s.width().get() * scale(t)),
                    join: match stroke.map(|s| s.linejoin()) {
                        Some(usvg::LineJoin::Round) => Join::Round,
                        Some(usvg::LineJoin::Bevel) => Join::Bevel,
                        _ => Join::Miter,
                    },
                    cap: match stroke.map(|s| s.linecap()) {
                        Some(usvg::LineCap::Round) => Cap::Round,
                        Some(usvg::LineCap::Square) => Cap::Square,
                        _ => Cap::None,
                    },
                    opacity,
                    blend: Blend::Normal,
                }));
            }
            usvg::Node::Image(i) if i.is_visible() => {
                if let usvg::ImageKind::PNG(b) | usvg::ImageKind::JPEG(b) = i.kind() {
                    let r = i.abs_bounding_box();
                    let at = [r.x(), r.y(), r.width(), r.height()];
                    out.push(Item::Image(b.to_vec(), at, at));
                }
            }
            _ => {}
        }
    }
}

/// The last point of `path`.
fn last(path: &[f32]) -> [f32; 2] {
    let n = path.len();
    if n >= 2 {
        [path[n - 2], path[n - 1]]
    } else {
        [0.0; 2]
    }
}

pub(crate) fn quad(path: &mut Vec<f32>, [qx, qy]: [f32; 2], [x, y]: [f32; 2]) {
    let [sx, sy] = last(path);
    let k = 2.0 / 3.0;
    path.extend([
        CUBIC,
        sx + k * (qx - sx),
        sy + k * (qy - sy),
        x + k * (qx - x),
        y + k * (qy - y),
        x,
        y,
    ]);
}

fn svg_paint(p: &usvg::Paint, alpha: f32, t: [f32; 6]) -> Option<Fill> {
    let stops = |s: &[usvg::Stop]| {
        s.iter()
            .map(|s| {
                let c = s.color();
                let a = (s.opacity().get() * alpha * 255.0).round() as u8;
                FillStop {
                    at: s.offset().get(),
                    color: rgba([c.red, c.green, c.blue, a]),
                }
            })
            .collect()
    };
    Some(match p {
        usvg::Paint::Color(c) => Fill::solid(rgba([
            c.red,
            c.green,
            c.blue,
            (alpha * 255.0).round() as u8,
        ])),
        usvg::Paint::LinearGradient(g) => {
            let (dx, dy) = (g.x2() - g.x1(), g.y2() - g.y1());
            let base = [dx, dy, -dy, dx, g.x1(), g.y1()];
            gradient(
                FillKind::Linear,
                then(t, then(affine(g.transform()), base)),
                stops(g.stops()),
            )
        }
        usvg::Paint::RadialGradient(g) => {
            let r = g.r().get();
            let base = [r, 0.0, 0.0, r, g.cx(), g.cy()];
            gradient(
                FillKind::Radial,
                then(t, then(affine(g.transform()), base)),
                stops(g.stops()),
            )
        }
        usvg::Paint::Pattern(_) => return None,
    })
}

fn pdf(bytes: &[u8]) -> Res<Vec<Sheet>> {
    let pdf = Pdf::new(bytes.to_vec()).map_err(|e| format!("not a PDF ({e:?})"))?;
    let cache = InterpreterCache::new();
    let mut sheets = Vec::new();
    for page in pdf.pages().iter() {
        let (w, h) = page.render_dimensions();
        let t = Affine::new(page.initial_transform(true).as_coeffs());
        let mut ctx = Context::new(
            t,
            Rect::new(0.0, 0.0, w.into(), h.into()),
            &cache,
            page.xref(),
            InterpreterSettings::default(),
        );
        let mut dev = Sink {
            items: Vec::new(),
            groups: vec![],
            clips: vec![],
        };
        interpret_page(page, &mut ctx, &mut dev);
        sheets.push(Sheet {
            name: None,
            width: w.into(),
            height: h.into(),
            items: dev.items,
        });
    }
    Ok(sheets)
}

/// Collects what a PDF page draws, in the opacity of the transparency groups around it.
struct Sink {
    items: Vec<Item>,
    groups: Vec<(f32, BlendMode)>,
    clips: Vec<(BezPath, bool)>,
}

fn coeffs(t: Affine) -> [f32; 6] {
    t.as_coeffs().map(|v| v as f32)
}

const BLENDS: [Blend; 16] = {
    use Blend::*;
    [
        Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight,
        SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity,
    ]
};
fn shading(p: &hayro_interpret::pattern::ShadingPattern, alpha: f32) -> Option<Fill> {
    let hayro_interpret::shading::ShadingType::RadialAxial {
        coords: [x0, y0, a, b, c, d],
        domain: [t0, t1],
        function,
        axial,
        ..
    } = p.shading.shading_type.as_ref()
    else {
        return None;
    };
    let mut out = vec![0.0; function.num_outputs()];
    let stops = (0..=8)
        .map(|i| {
            let at = i as f32 / 8.0;
            function.eval(&[t0 + at * (t1 - t0)], &mut out)?;
            Some(FillStop {
                at: if *axial {
                    at
                } else {
                    (a + at * (d - a)) / d.max(f32::EPSILON)
                },
                color: rgba(
                    p.shading
                        .color_space
                        .to_rgba(&out, alpha * p.opacity)
                        .to_rgba8(),
                ),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let m = coeffs(p.matrix);
    Some(if *axial {
        let (dx, dy) = (a - x0, b - y0);
        gradient(
            FillKind::Linear,
            then(m, [dx, dy, -dy, dx, *x0, *y0]),
            stops,
        )
    } else {
        gradient(FillKind::Radial, then(m, [*d, 0.0, 0.0, *d, *b, *c]), stops)
    })
}

impl Sink {
    fn opacity(&self) -> f32 {
        self.groups.iter().map(|g| g.0).product()
    }

    fn blend(&self, mode: BlendMode) -> Blend {
        let mode = self
            .groups
            .iter()
            .map(|g| g.1)
            .chain([mode])
            .rfind(|&m| m != BlendMode::Normal);
        BLENDS[mode.unwrap_or_default() as usize]
    }

    /// The bounds of the clips around what is drawn, `None` when there are none.
    fn clip_box(&self) -> Option<Rect> {
        self.clips
            .iter()
            .map(|c| c.0.bounding_box())
            .reduce(|a, b| a.intersect(b))
    }

    fn shape(&mut self, mut path: BezPath, props: &DrawProps, mode: &DrawMode) {
        let paint = match &props.paint {
            Paint::Color(c) => Fill::solid(rgba(c.to_rgba().to_rgba8())),
            Paint::Pattern(p) => match p.as_ref() {
                hayro_interpret::pattern::Pattern::Shading(s) => match shading(s, 1.0) {
                    Some(f) => f,
                    None => return,
                },
                _ => return,
            },
        };
        let (fill, stroke) = match mode {
            DrawMode::Fill(_) => (true, None),
            DrawMode::Stroke(s) => (false, Some(s)),
            DrawMode::FillAndStroke(_, s) => (true, Some(s)),
            DrawMode::Invisible => return,
        };
        for (c, rect) in &self.clips {
            let (b, r) = (path.bounding_box(), c.bounding_box());
            if r.intersect(b).area() <= 0.0 && b.area() > 0.0 {
                return;
            }
            if stroke.is_none() && !(*rect && r.intersect(b) == b) {
                let Ok(cut) = binary_op(&path, c, FillRule::NonZero, BinaryOp::Intersection) else {
                    return;
                };
                path = cut
                    .contours()
                    .flat_map(|c| c.path.elements().to_vec())
                    .collect();
            }
        }
        let paint = || vec![paint.clone()];
        self.items.push(Item::Shape(Shape {
            path: crate::doc::path_of(&path),
            fills: if fill { paint() } else { vec![] },
            strokes: if stroke.is_some() { paint() } else { vec![] },
            weight: stroke.map_or(1.0, |s| s.line_width * scale(coeffs(props.transform))),
            join: match stroke.map(|s| s.line_join) {
                Some(hayro_interpret::kurbo::Join::Round) => Join::Round,
                Some(hayro_interpret::kurbo::Join::Bevel) => Join::Bevel,
                _ => Join::Miter,
            },
            cap: match stroke.map(|s| s.line_cap) {
                Some(hayro_interpret::kurbo::Cap::Round) => Cap::Round,
                Some(hayro_interpret::kurbo::Cap::Square) => Cap::Square,
                _ => Cap::None,
            },
            opacity: self.opacity(),
            blend: self.blend(props.blend_mode),
        }));
    }
}

impl<'a> Device<'a> for Sink {
    fn draw_path(&mut self, path: &BezPath, props: DrawProps<'a>, mode: &DrawMode) {
        self.shape(props.transform * path.clone(), &props, mode);
    }

    fn push_clip_path(&mut self, clip: &ClipPath) {
        self.clips.push((clip.path.clone(), false));
    }

    fn push_clip_rect(&mut self, r: &Rect) {
        self.clips.push((r.to_path(0.1), true));
    }

    fn push_transparency_group(&mut self, opacity: f32, _: Option<SoftMask<'a>>, mode: BlendMode) {
        self.groups.push((opacity, mode));
    }

    fn draw_glyph_run(
        &mut self,
        run: &hayro_interpret::font::GlyphRun<'_, 'a>,
        props: DrawProps<'a>,
        mode: &DrawMode,
    ) {
        let mut out = BezPath::new();
        for g in run.glyphs() {
            match &**g {
                Glyph::Outline(o) => out.extend(
                    (props.transform * g.transform() * o.outline())
                        .elements()
                        .iter()
                        .copied(),
                ),
                Glyph::Type3(t) => t.interpret(self, props.transform, g.transform(), &props.paint),
            }
        }
        self.shape(out, &props, mode);
    }

    fn draw_image(&mut self, image: Image<'a, '_>, props: ImageDrawProps<'a>) {
        let Image::Raster(r) = image else { return };
        let rect = props.transform.transform_rect_bbox(Rect::new(
            0.0,
            0.0,
            r.width().into(),
            r.height().into(),
        ));
        let crop = self.clip_box().map_or(rect, |c| c.intersect(rect));
        if crop.area() <= 0.0 {
            return;
        }
        let opacity = self.opacity();
        let at = |r: Rect| [r.x0, r.y0, r.width(), r.height()].map(|v| v as f32);
        r.with_rgba(
            |data, alpha| {
                let (iw, ih) = (data.width(), data.height());
                let rgb: Vec<[u8; 3]> = match data {
                    ImageData::Rgb(d) => d.data.as_chunks::<3>().0.to_vec(),
                    ImageData::Luma(d) => d.data.iter().map(|&v| [v; 3]).collect(),
                };
                let alpha = alpha
                    .filter(|a| a.width == iw && a.height == ih)
                    .map(|a| a.data);
                let rgba: Vec<u8> = rgb
                    .iter()
                    .enumerate()
                    .flat_map(|(i, [r, g, b])| {
                        let a = alpha.as_ref().map_or(255.0, |a| f32::from(a[i]));
                        [*r, *g, *b, (a * opacity).round() as u8]
                    })
                    .collect();
                if let Some(png) = crate::image::encode(&rgba, (iw, ih)) {
                    self.items.push(Item::Image(png, at(rect), at(crop)));
                }
            },
            None,
        );
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn pop_transparency_group(&mut self) {
        self.groups.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{Kind, Node};

    fn layers(bytes: &[u8]) -> Vec<(f64, f64, Vec<std::rc::Rc<Node>>)> {
        let d = Doc::load(bytes).unwrap();
        d.build_snapshot()
            .pages
            .iter()
            .map(|p| {
                let Kind::Frame { children, .. } = &p.children[0].kind else {
                    panic!("not a frame")
                };
                (p.width, p.height, children.clone())
            })
            .collect()
    }

    #[test]
    fn an_svg_imports_as_a_frame_of_paths_with_their_gradients() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
            <defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs>
            <rect x="10" y="10" width="80" height="80" fill="url(#g)"/>
            <g transform="translate(100 0)"><circle cx="50" cy="50" r="40" fill="#0f0" stroke="#000" stroke-width="2"/></g>
        </svg>"##;
        let bytes = import(svg, "logo.svg").unwrap();
        let pages = layers(&bytes);
        assert_eq!(pages.len(), 1);
        assert_eq!((pages[0].0, pages[0].1), (200.0, 100.0));
        assert_eq!(pages[0].2.len(), 2);
        let d = Doc::load(&bytes).unwrap();
        let s = d.build_snapshot();
        let Kind::Frame { children, .. } = &s.pages[0].children[0].kind else {
            panic!()
        };
        assert_eq!(s.pages[0].children[0].name, "logo");
        let [rect, circle] = [&children[0], &children[1]];
        assert_eq!([rect.x, rect.y, rect.w, rect.h], [10.0, 10.0, 80.0, 80.0]);
        assert_eq!(rect.style.fills[0].kind, FillKind::Linear);
        assert_eq!(
            rect.style.fills[0].transform,
            [1.0, 0.0, -0.0, 1.0, 0.0, 0.0]
        );
        assert!(
            (circle.x - 110.0).abs() < 0.01 && (circle.w - 80.0).abs() < 0.01,
            "{} {} {}",
            circle.x,
            circle.w,
            circle.h
        );
        assert_eq!(circle.style.fills[0].color, Color::Rgb(0x00ff00ff));
        assert_eq!(circle.style.stroke_weight, 2.0);
    }

    #[test]
    fn a_pdf_imports_with_a_page_of_vectors_for_each_of_its_pages() {
        let mut d = Doc::sample();
        d.apply(Command::AddPage { after: None }).unwrap();
        let pdf = d.pdf("Satz", "2026-09-28T12:00:00Z", &[0, 1]).unwrap();
        let pages = layers(&import(&pdf, "sample.pdf").unwrap());
        let page = &d.build_snapshot().pages[0];
        assert_eq!(pages.len(), 2);
        assert!(
            pages[0].0 > page.width && pages[0].1 > page.height,
            "the crop box with bleed and marks"
        );
        let all = || pages[0].2.iter();
        assert!(all().any(|n| n.style.blend == Blend::Multiply));
        assert!(all().any(|n| n.style.fills.iter().any(|f| f.kind == FillKind::Radial)));
        assert!(all().any(|n| n.style.fills.iter().any(|f| f.kind == FillKind::Linear)));
        assert!(all().any(|n| n.style.fills.iter().any(|f| f.kind == FillKind::Image)));
        assert!(pages[1].2.len() < pages[0].2.len());
    }

    #[test]
    fn a_file_that_is_neither_svg_nor_pdf_does_not_import() {
        assert!(import(b"<svg", "a.svg").is_err());
        assert!(import(b"%PDF-", "a.pdf").is_err());
        assert!(import(b"", "a.txt").is_err());
    }
}
