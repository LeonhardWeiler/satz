use crate::color::Color;
use crate::display_list::{CLOSE, Op, Paint, Shadow, Stop};
use crate::geom::arrow;
use crate::variable::Scope;
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
    pub constraints: Constraints,
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

fn paint(f: &Fill, [x, y, w, h]: [f32; 4], s: &Scope) -> Paint {
    let [a, b, c, d, e, g] = f.transform;
    let transform = [w * a, h * b, w * c, h * d, x + w * e, y + h * g];
    let stops = f
        .stops
        .iter()
        .map(|stop| Stop {
            at: stop.at,
            color: stop.color.rgba(s),
            ink: stop.color.ink(s),
        })
        .collect();
    match f.kind {
        FillKind::Solid => Paint::Solid {
            color: f.color.rgba(s),
            ink: f.color.ink(s),
        },
        FillKind::Linear => Paint::Linear { transform, stops },
        FillKind::Radial => Paint::Radial { transform, stops },
    }
}

pub fn paints<'a>(
    fills: &'a [Fill],
    frame: [f32; 4],
    s: &'a Scope<'a>,
) -> impl Iterator<Item = Paint> + 'a {
    fills
        .iter()
        .filter(|f| f.visible)
        .map(move |f| paint(f, frame, s))
}

impl Style {
    /// Fill and stroke ops for `path`; closed paths honour the stroke alignment.
    pub fn shape(&self, path: &[f32], frame: [f32; 4], s: &Scope) -> Vec<Op> {
        let mut ops: Vec<Op> = paints(&self.fills, frame, s)
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
        for paint in paints(&self.strokes, frame, s) {
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
                blend: self.blend as u32,
                blur,
                shadows,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::{Modes, Palette};

    fn scope<T>(f: impl FnOnce(&Scope) -> T) -> T {
        let (palette, modes) = (Palette::default(), Modes::new());
        f(&Scope {
            palette: &palette,
            modes: &modes,
        })
    }

    const SQUARE: [f32; 11] = [0.0, 0.0, 0.0, 1.0, 10.0, 0.0, 1.0, 10.0, 10.0, CLOSE, 0.0];

    #[test]
    fn a_gradient_maps_its_unit_box_into_the_frame() {
        let f = Fill {
            kind: FillKind::Linear,
            transform: [1.0, 0.0, 0.0, 1.0, 0.5, 0.0],
            ..Fill::solid(Color::Rgb(0xff))
        };
        match scope(|s| paint(&f, [10.0, 20.0, 100.0, 50.0], s)) {
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
        assert_eq!(scope(|s| paints(&fills, [0.0; 4], s).count()), 0);
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
        let ops = scope(|s| style.shape(&SQUARE, [0.0; 4], s));
        assert!(matches!(
            ops[..],
            [
                Op::PushClip { invert: true, .. },
                Op::StrokePath { width: 4.0, .. },
                Op::PopClip
            ]
        ));
        let open = scope(|s| style.shape(&SQUARE[..9], [0.0; 4], s));
        assert!(matches!(open[..], [Op::StrokePath { width: 2.0, .. }]));
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
