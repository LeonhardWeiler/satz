use crate::color::{Color, Swatch};
use crate::display_list::{CLOSE, Op, Paint, Shadow, Stop};
use crate::geom::arrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Style {
    pub fills: Vec<Fill>,
    pub strokes: Vec<Fill>,
    pub stroke_weight: f32,
    pub stroke_align: Align,
    pub join: Join,
    pub cap: Cap,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub opacity: f32,
    pub blend: Blend,
    pub effects: Vec<Effect>,
    pub mask: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fills: Vec::new(),
            strokes: Vec::new(),
            stroke_weight: 1.0,
            stroke_align: Align::Center,
            join: Join::Miter,
            cap: Cap::None,
            arrow_start: false,
            arrow_end: false,
            opacity: 1.0,
            blend: Blend::Normal,
            effects: Vec::new(),
            mask: false,
        }
    }
}

/// A fill or stroke paint. Gradients map their unit space into the node's
/// unit box with `transform` [a b c d e f]; see `display_list::Paint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fill {
    #[serde(rename = "type")]
    pub kind: FillKind,
    pub color: Color,
    pub stops: Vec<FillStop>,
    pub transform: [f32; 6],
    pub visible: bool,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            kind: FillKind::Solid,
            color: Color::Rgb(0x000000ff),
            stops: Vec::new(),
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            visible: true,
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

fn paint(f: &Fill, [x, y, w, h]: [f32; 4], swatches: &[Swatch]) -> Paint {
    let [a, b, c, d, e, g] = f.transform;
    let transform = [w * a, h * b, w * c, h * d, x + w * e, y + h * g];
    let stops = f
        .stops
        .iter()
        .map(|s| Stop {
            at: s.at,
            color: s.color.rgba(swatches),
        })
        .collect();
    match f.kind {
        FillKind::Solid => Paint::Solid {
            color: f.color.rgba(swatches),
        },
        FillKind::Linear => Paint::Linear { transform, stops },
        FillKind::Radial => Paint::Radial { transform, stops },
    }
}

pub fn paints<'a>(
    fills: &'a [Fill],
    frame: [f32; 4],
    swatches: &'a [Swatch],
) -> impl Iterator<Item = Paint> + 'a {
    fills
        .iter()
        .filter(|f| f.visible)
        .map(move |f| paint(f, frame, swatches))
}

impl Style {
    /// Fill and stroke ops for `path`; closed paths honour the stroke alignment.
    pub fn shape(&self, path: &[f32], frame: [f32; 4], swatches: &[Swatch]) -> Vec<Op> {
        let mut ops: Vec<Op> = paints(&self.fills, frame, swatches)
            .map(|paint| Op::FillPath {
                paint,
                path: path.to_vec(),
            })
            .collect();
        let closed = path.contains(&CLOSE);
        let align = if closed {
            self.stroke_align
        } else {
            Align::Center
        };
        let mut line = path.to_vec();
        if !closed {
            for (on, end) in [(self.arrow_start, false), (self.arrow_end, true)] {
                if on {
                    line.extend(arrow(path, end, self.stroke_weight));
                }
            }
        }
        for paint in paints(&self.strokes, frame, swatches) {
            let stroke = Op::StrokePath {
                paint,
                width: self.stroke_weight * if align == Align::Center { 1.0 } else { 2.0 },
                cap: self.cap as u32,
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
    pub fn layer(&self, swatches: &[Swatch]) -> Option<Op> {
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
                color: e.color.rgba(swatches),
            })
            .collect();
        (self.opacity < 1.0 || self.blend != Blend::Normal || blur > 0.0 || !shadows.is_empty())
            .then(|| Op::PushLayer {
                opacity: self.opacity.clamp(0.0, 1.0),
                blend: self.blend as u32,
                blur,
                shadows,
            })
    }
}
