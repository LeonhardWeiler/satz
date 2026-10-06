use crate::color::Color;
use crate::display_list::{Op, Paint, Shadow, Stop, rect};
use crate::geom::{LineStyle, arrow, closed, pattern};
use crate::image;
use crate::variable::Scope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Style {
    pub fills: Vec<Fill>,
    pub strokes: Vec<Fill>,
    pub stroke_weight: f32,
    pub stroke_align: Align,
    /// The stroke weights of the top, right, bottom and left side, or none for `stroke_weight` all round.
    pub stroke_sides: Vec<f32>,
    pub join: Join,
    pub cap: Cap,
    pub line_style: LineStyle,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub opacity: f32,
    pub blend: Blend,
    pub effects: Vec<Effect>,
    pub mask: bool,
    pub overprint_fill: bool,
    pub overprint_stroke: bool,
    pub constraints: Constraints,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fills: Vec::new(),
            strokes: Vec::new(),
            stroke_weight: 1.0,
            stroke_align: Align::Center,
            stroke_sides: Vec::new(),
            join: Join::Miter,
            cap: Cap::None,
            line_style: LineStyle::Solid,
            arrow_start: false,
            arrow_end: false,
            opacity: 1.0,
            blend: Blend::Normal,
            effects: Vec::new(),
            mask: false,
            overprint_fill: false,
            overprint_stroke: false,
            constraints: Constraints::default(),
        }
    }
}

/// How a frame's child follows the frame on resize: pinned to the start (left,
/// top), the end, both, the centre, or scaled.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Constraint {
    #[default]
    Min,
    Max,
    Stretch,
    Center,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Constraints {
    pub horizontal: Constraint,
    pub vertical: Constraint,
}

/// A fill or stroke paint. Gradients map their unit space, and images the unit
/// square they fill, into the node's unit box with `transform` [a b c d e f]; see
/// `display_list::Paint`. `image` is the hash of an image fill's file, and
/// `adjust` its brightness, contrast and saturation in -1..=1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fill {
    #[serde(rename = "type")]
    pub kind: FillKind,
    pub color: Color,
    pub stops: Vec<FillStop>,
    pub transform: [f32; 6],
    pub visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub adjust: [f32; 3],
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            kind: FillKind::Solid,
            color: Color::Rgb(0x000000ff),
            stops: Vec::new(),
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            visible: true,
            image: None,
            adjust: [0.0; 3],
        }
    }
}

impl Fill {
    pub fn solid(color: impl Into<Color>) -> Fill {
        Fill {
            color: color.into(),
            ..Fill::default()
        }
    }

    /// A fill with the image `hash` stretched over the node's box.
    pub fn image(hash: &str) -> Fill {
        Fill {
            kind: FillKind::Image,
            image: Some(hash.into()),
            ..Fill::default()
        }
    }

    /// `transform` in page space for a node with the box `frame`.
    fn place(&self, [x, y, w, h]: [f32; 4]) -> [f32; 6] {
        let [a, b, c, d, e, f] = self.transform;
        [w * a, h * b, w * c, h * d, x + w * e, y + h * f]
    }

    /// Pixels per inch of an image fill on a node `w` × `h` pt, along the image's
    /// coarser axis.
    pub fn ppi(&self, w: f64, h: f64) -> Option<f64> {
        let info = image::info(self.image.as_deref()?)?;
        let [a, b, c, d, ..] = self.transform.map(f64::from);
        let across = (w * a).hypot(h * b);
        let down = (w * c).hypot(h * d);
        Some((f64::from(info.width) / across).min(f64::from(info.height) / down) * 72.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillStop {
    pub at: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FillKind {
    #[default]
    Solid,
    Linear,
    Radial,
    Image,
}

/// `radius` is the Figma blur radius, twice the Gaussian sigma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Effect {
    #[serde(rename = "type")]
    pub kind: EffectKind,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: Color,
    pub visible: bool,
}

impl Default for Effect {
    fn default() -> Self {
        Effect {
            kind: EffectKind::DropShadow,
            x: 0.0,
            y: 3.0,
            radius: 6.0,
            color: Color::Rgb(0x00000040),
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    #[default]
    DropShadow,
    Blur,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Align {
    Inside,
    Center,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cap {
    None,
    Round,
    Square,
}

/// In the order of the display list's blend numbers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Blend {
    pub fn krilla(self) -> krilla::blend::BlendMode {
        use krilla::blend::BlendMode as B;
        match self {
            Blend::Normal => B::Normal,
            Blend::Multiply => B::Multiply,
            Blend::Screen => B::Screen,
            Blend::Overlay => B::Overlay,
            Blend::Darken => B::Darken,
            Blend::Lighten => B::Lighten,
            Blend::ColorDodge => B::ColorDodge,
            Blend::ColorBurn => B::ColorBurn,
            Blend::HardLight => B::HardLight,
            Blend::SoftLight => B::SoftLight,
            Blend::Difference => B::Difference,
            Blend::Exclusion => B::Exclusion,
            Blend::Hue => B::Hue,
            Blend::Saturation => B::Saturation,
            Blend::Color => B::Color,
            Blend::Luminosity => B::Luminosity,
        }
    }

    pub fn skia(self) -> tiny_skia::BlendMode {
        use tiny_skia::BlendMode as B;
        match self {
            Blend::Normal => B::SourceOver,
            Blend::Multiply => B::Multiply,
            Blend::Screen => B::Screen,
            Blend::Overlay => B::Overlay,
            Blend::Darken => B::Darken,
            Blend::Lighten => B::Lighten,
            Blend::ColorDodge => B::ColorDodge,
            Blend::ColorBurn => B::ColorBurn,
            Blend::HardLight => B::HardLight,
            Blend::SoftLight => B::SoftLight,
            Blend::Difference => B::Difference,
            Blend::Exclusion => B::Exclusion,
            Blend::Hue => B::Hue,
            Blend::Saturation => B::Saturation,
            Blend::Color => B::Color,
            Blend::Luminosity => B::Luminosity,
        }
    }
}

/// The paint of a colour or gradient fill.
fn paint(f: &Fill, frame: [f32; 4], s: &Scope, overprint: bool) -> Option<Paint> {
    let transform = f.place(frame);
    let stops = f
        .stops
        .iter()
        .map(|stop| Stop {
            at: stop.at,
            color: stop.color.rgba(s),
            ink: stop.color.ink(s),
        })
        .collect();
    Some(match f.kind {
        FillKind::Solid => Paint::Solid {
            color: f.color.rgba(s),
            ink: f.color.ink(s),
            overprint,
        },
        FillKind::Linear => Paint::Linear {
            transform,
            stops,
            overprint,
        },
        FillKind::Radial => Paint::Radial {
            transform,
            stops,
            overprint,
        },
        FillKind::Image => return None,
    })
}

/// The paints of the visible colour and gradient fills among `fills`.
pub fn paints<'a>(
    fills: &'a [Fill],
    frame: [f32; 4],
    s: &'a Scope<'a>,
    overprint: bool,
) -> impl Iterator<Item = Paint> + 'a {
    fills
        .iter()
        .filter(|f| f.visible)
        .filter_map(move |f| paint(f, frame, s, overprint))
}

impl Style {
    /// Fill ops for `fill` and stroke ops for `path`; closed paths honour the stroke alignment.
    pub fn shape(&self, fill: &[f32], path: &[f32], frame: [f32; 4], s: &Scope) -> Vec<Op> {
        let mut ops = Vec::new();
        for f in self.fills.iter().filter(|f| f.visible) {
            if let Some(paint) = paint(f, frame, s, self.overprint_fill) {
                ops.push(Op::FillPath {
                    paint,
                    path: fill.to_vec(),
                });
            } else if let Some(image) = f
                .image
                .as_deref()
                .and_then(|h| image::adjusted(h, f.adjust))
            {
                ops.extend([
                    Op::PushClip {
                        path: fill.to_vec(),
                        invert: false,
                    },
                    Op::Image {
                        image,
                        transform: f.place(frame),
                    },
                    Op::PopClip,
                ]);
            }
        }
        if self.stroke_sides.len() == 4 && closed(path) {
            ops.extend(self.sides(path, frame, s));
        } else {
            ops.extend(self.stroke(path, frame, s));
        }
        ops
    }

    /// The strokes of each side of `frame` at its own weight, clipped to `path` when inside.
    fn sides(&self, path: &[f32], [x, y, w, h]: [f32; 4], s: &Scope) -> Vec<Op> {
        let [t, r, b, l] = [0, 1, 2, 3].map(|i| self.stroke_sides[i]);
        let k = match self.stroke_align {
            Align::Inside => 0.0,
            Align::Center => 0.5,
            Align::Outside => 1.0,
        };
        let [x, y, w, h] = [x - l * k, y - t * k, w + (l + r) * k, h + (t + b) * k];
        let [iw, ih] = [(w - l - r).max(0.0), (h - t - b).max(0.0)];
        let mut ring = rect(x, y, w, h);
        ring.extend(rect(x + l + iw, y + t, -iw, ih));
        let mut ops = Vec::new();
        for paint in paints(&self.strokes, [x, y, w, h], s, self.overprint_stroke) {
            ops.push(Op::FillPath {
                paint,
                path: ring.clone(),
            });
        }
        if self.stroke_align == Align::Inside && !ops.is_empty() {
            ops.insert(
                0,
                Op::PushClip {
                    path: path.to_vec(),
                    invert: false,
                },
            );
            ops.push(Op::PopClip);
        }
        ops
    }

    /// The strokes along `path`, aligned when it is closed and with arrows when open.
    pub fn stroke(&self, path: &[f32], frame: [f32; 4], s: &Scope) -> Vec<Op> {
        let mut ops = Vec::new();
        let closed = closed(path);
        let align = if closed {
            self.stroke_align
        } else {
            Align::Center
        };
        let mut line = pattern(path, self.line_style, self.stroke_weight);
        if !closed {
            for (on, end) in [(self.arrow_start, false), (self.arrow_end, true)] {
                if on {
                    line.extend(arrow(path, end, self.stroke_weight));
                }
            }
        }
        for paint in paints(&self.strokes, frame, s, self.overprint_stroke) {
            let stroke = Op::StrokePath {
                paint,
                width: self.stroke_weight * if align == Align::Center { 1.0 } else { 2.0 },
                cap: if self.line_style == LineStyle::Dotted {
                    Cap::Round
                } else {
                    self.cap
                } as u32,
                join: self.join as u32,
                path: line.clone(),
            };
            if align == Align::Center {
                ops.push(stroke);
            } else {
                ops.push(Op::PushClip {
                    path: path.to_vec(),
                    invert: align == Align::Outside,
                });
                ops.push(stroke);
                ops.push(Op::PopClip);
            }
        }
        ops
    }

    /// The layer that wraps a node's ops, if it needs one.
    pub fn layer(&self, s: &Scope) -> Option<Op> {
        let effects = || self.effects.iter().filter(|e| e.visible);
        let blur = effects()
            .filter(|e| e.kind == EffectKind::Blur)
            .map(|e| e.radius / 2.0)
            .fold(0.0, f32::max);
        let shadows: Vec<Shadow> = effects()
            .filter(|e| e.kind == EffectKind::DropShadow)
            .map(|e| Shadow {
                offset: [e.x, e.y],
                blur: e.radius / 2.0,
                color: e.color.rgba(s),
                ink: e.color.ink(s),
            })
            .collect();
        (self.opacity < 1.0 || self.blend != Blend::Normal || blur > 0.0 || !shadows.is_empty())
            .then(|| Op::PushLayer {
                opacity: self.opacity.clamp(0.0, 1.0),
                blend: self.blend,
                blur,
                shadows,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_list::{CLOSE, LINE, MOVE};
    use crate::geom;
    use crate::variable::{Modes, Palette};

    fn scope<T>(f: impl FnOnce(&Scope) -> T) -> T {
        let (palette, modes) = (Palette::default(), Modes::new());
        f(&Scope {
            palette: &palette,
            modes: &modes,
        })
    }

    const SQUARE: [f32; 10] = [MOVE, 0.0, 0.0, LINE, 10.0, 0.0, LINE, 10.0, 10.0, CLOSE];

    #[test]
    fn a_gradient_maps_its_unit_box_into_the_frame() {
        let f = Fill {
            kind: FillKind::Linear,
            transform: [1.0, 0.0, 0.0, 1.0, 0.5, 0.0],
            ..Fill::solid(Color::Rgb(0xff))
        };
        match scope(|s| paint(&f, [10.0, 20.0, 100.0, 50.0], s, false)).unwrap() {
            Paint::Linear { transform, .. } => {
                assert_eq!(transform, [100.0, 0.0, 0.0, 50.0, 60.0, 20.0])
            }
            p => panic!("{p:?}"),
        }
    }

    #[test]
    fn hidden_fills_paint_nothing() {
        let fills = [Fill {
            visible: false,
            ..Fill::solid(Color::Rgb(0xff))
        }];
        assert_eq!(scope(|s| paints(&fills, [0.0; 4], s, false).count()), 0);
    }

    #[test]
    fn inside_and_outside_strokes_clip_to_the_shape_at_twice_the_width_but_open_paths_stroke_centred()
     {
        let style = Style {
            strokes: vec![Fill::solid(Color::Rgb(0xff))],
            stroke_align: Align::Outside,
            stroke_weight: 2.0,
            ..Style::default()
        };
        let ops = scope(|s| style.shape(&SQUARE, &SQUARE, [0.0; 4], s));
        assert!(matches!(
            ops[..],
            [
                Op::PushClip { invert: true, .. },
                Op::StrokePath { width: 4.0, .. },
                Op::PopClip
            ]
        ));
        let open = scope(|s| style.shape(&SQUARE[..9], &SQUARE[..9], [0.0; 4], s));
        assert!(matches!(open[..], [Op::StrokePath { width: 2.0, .. }]));
    }

    #[test]
    fn sides_fill_a_ring_of_their_own_weights() {
        let style = Style {
            strokes: vec![Fill::solid(Color::Rgb(0xff))],
            stroke_align: Align::Outside,
            stroke_sides: vec![1.0, 0.0, 2.0, 0.0],
            ..Style::default()
        };
        let ops = scope(|s| style.shape(&SQUARE, &SQUARE, [0.0, 0.0, 10.0, 10.0], s));
        let [Op::FillPath { path, .. }] = &ops[..] else {
            panic!("{ops:?}")
        };
        assert_eq!(geom::bounds(path), [0.0, -1.0, 10.0, 13.0]);
        assert!(!geom::contains(path, 5.0, 5.0));
        assert!(geom::contains(path, 5.0, -0.5));
        assert!(geom::contains(path, 5.0, 11.0));
    }

    #[test]
    fn a_layer_wraps_a_node_only_for_opacity_blend_or_visible_effects() {
        assert_eq!(scope(|s| Style::default().layer(s)), None);
        let hidden = Style {
            effects: vec![Effect {
                visible: false,
                ..Effect::default()
            }],
            ..Style::default()
        };
        assert_eq!(scope(|s| hidden.layer(s)), None);
        let blurred = Style {
            effects: vec![Effect {
                kind: EffectKind::Blur,
                radius: 8.0,
                ..Effect::default()
            }],
            ..Style::default()
        };
        assert!(matches!(
            scope(|s| blurred.layer(s)),
            Some(Op::PushLayer { blur: 4.0, .. })
        ));
    }
}
